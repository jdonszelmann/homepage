use sqlx::PgConnection;
use time::{OffsetDateTime, PrimitiveDateTime, UtcDateTime};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct Rss {
    pub id: Uuid,
    pub list: Uuid,

    pub url: String,

    pub added: PrimitiveDateTime,
    pub updated: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
}

pub async fn item_exists(conn: &mut PgConnection, list: Uuid, guid: &str) -> sqlx::Result<bool> {
    sqlx::query!(
        "select exists(select 1 from item where list = $1 and rss_guid = $2)",
        list,
        guid
    )
    .fetch_one(conn)
    .await
    .map(|i| i.exists.unwrap_or_default())
}

pub async fn all_rss_sources(conn: &mut PgConnection) -> sqlx::Result<Vec<Rss>> {
    sqlx::query_as!(Rss, "select * from rss where rss.deleted is null")
        .fetch_all(conn)
        .await
}

pub async fn rss_sources_for_list(conn: &mut PgConnection, list: Uuid) -> sqlx::Result<Vec<Rss>> {
    sqlx::query_as!(
        Rss,
        "select rss.* from rss join list on list.id = rss.list where list.id = $1 and list.deleted is null and rss.deleted is null",
        list
    )
    .fetch_all(conn)
    .await
}

pub async fn get_rss(conn: &mut PgConnection, rss: Uuid) -> sqlx::Result<Rss> {
    sqlx::query_as!(Rss, "select * from rss where id = $1", rss)
        .fetch_one(conn)
        .await
}

pub async fn create_rss(conn: &mut PgConnection, list: Uuid, url: &str) -> sqlx::Result<Uuid> {
    let res = sqlx::query!(
        "insert into rss (id, list, url) values ($1, $2, $3) returning id",
        Uuid::new_v4(),
        list,
        url
    )
    .fetch_one(conn)
    .await?;

    Ok(res.id)
}

pub async fn set_rss_url(conn: &mut PgConnection, rss: Uuid, url: &str) -> sqlx::Result<()> {
    sqlx::query!(
        r#"update rss set url = $2, updated = CURRENT_TIMESTAMP where id = $1"#,
        rss,
        url,
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn update_rss(
    conn: &mut PgConnection,
    rss: Uuid,
    timestamp: UtcDateTime,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"update rss set updated = $2 where id = $1"#,
        rss,
        PrimitiveDateTime::new(timestamp.date(), timestamp.time()),
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn delete_rss(conn: &mut PgConnection, rss: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        r#"update rss set deleted = CURRENT_TIMESTAMP, updated = CURRENT_TIMESTAMP where id = $1"#,
        rss,
    )
    .execute(conn)
    .await?;

    Ok(())
}
