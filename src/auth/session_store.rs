use async_trait::async_trait;
use color_eyre::eyre::Context;
use sqlx::{Executor, PgConnection};
use time::OffsetDateTime;
use tower_sessions::{
    ExpiredDeletion, SessionStore,
    session::{Id, Record},
    session_store,
};

use crate::ArcRouteState;

impl ArcRouteState {
    async fn id_exists(&self, conn: &mut PgConnection, id: &Id) -> color_eyre::Result<bool> {
        let res = sqlx::query!(
            "select exists(select 1 from session where id = $1)",
            id.to_string()
        )
        .fetch_one(conn)
        .await
        .context("id exists")?;
        Ok(res.exists.unwrap_or_default())
    }

    async fn save_with_conn(
        &self,
        conn: &mut PgConnection,
        record: &Record,
    ) -> color_eyre::Result<()> {
        println!("{record:?}");
        let record_value = serde_json::to_value(record).context("encode")?;
        let query = sqlx::query!(
            "
            insert into session (id, data, expiry_date)
            values ($1, $2, $3)
            on conflict (id) do update
            set
              data = excluded.data,
              expiry_date = excluded.expiry_date
            ",
            record.id.to_string(),
            record_value,
            record.expiry_date,
        );
        conn.execute(query).await.context("save with conn")?;
        Ok(())
    }
}

#[async_trait]
impl ExpiredDeletion for ArcRouteState {
    async fn delete_expired(&self) -> session_store::Result<()> {
        transform_result("delete expired", async || {
            self.db
                .execute(sqlx::query!(
                    "delete from session where expiry_date < (now() at time zone 'utc')"
                ))
                .await
                .context("delete expired")?;
            Ok(())
        })
        .await
    }
}

async fn transform_result<T>(
    context: &'static str,
    f: impl AsyncFnOnce() -> color_eyre::Result<T>,
) -> session_store::Result<T> {
    f().await
        .context(context)
        .map_err(|e| session_store::Error::Backend(format!("{e:?}")))
}

#[async_trait]
impl SessionStore for ArcRouteState {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        transform_result("create", async || {
            let mut tx = self.db.begin().await.context("begin transaction")?;

            // try random ids till we find a new one,
            // shouldn't take more than a few cycles.
            while self.id_exists(&mut tx, &record.id).await? {
                record.id = Id::default();
            }
            self.save_with_conn(&mut tx, record).await?;

            tx.commit().await.context("end transaction")?;

            Ok(())
        })
        .await
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        transform_result("save", async || {
            let mut conn = self.db.acquire().await.context("aqcuire")?;
            self.save_with_conn(&mut conn, record).await
        })
        .await
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        transform_result("load", async || {
            let data = sqlx::query!(
                "
            select data from session
            where id = $1 and expiry_date > $2
            ",
                session_id.to_string(),
                OffsetDateTime::now_utc()
            )
            .fetch_optional(&self.db)
            .await
            .context("query")?;

            data.map(|i| serde_json::from_value(i.data).context("decode"))
                .transpose()
        })
        .await
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        transform_result("delete", async || {
            self.db
                .execute(sqlx::query!(
                    "delete from session where id = $1",
                    session_id.to_string()
                ))
                .await?;
            Ok(())
        })
        .await
    }
}
