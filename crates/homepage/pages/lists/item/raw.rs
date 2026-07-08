use sqlx::PgConnection;
use time::{PrimitiveDateTime, UtcDateTime};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct Item {
    pub id: Uuid,
    pub list: Uuid,

    pub note: String,

    pub added_through: AddedThrough,
    pub rss_guid: Option<String>,

    pub public: bool,

    pub added: PrimitiveDateTime,
    pub updated: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
}

#[repr(i32)]
pub enum AddedThrough {
    Manual = 0,
    Rss = 1,
}

impl From<i32> for AddedThrough {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Manual,
            1 => Self::Rss,
            _ => {
                tracing::error!("{value} from database cannot be converted into `AddedThrough`");
                Self::Manual
            }
        }
    }
}

pub async fn get_item(conn: &mut PgConnection, item: Uuid) -> sqlx::Result<Item> {
    sqlx::query_as!(Item, "select * from item where id = $1", item)
        .fetch_one(conn)
        .await
}

pub async fn get_list_public_items(
    conn: &mut PgConnection,
    list: Uuid,
    limit: Option<usize>,
) -> sqlx::Result<Vec<Item>> {
    if let Some(limit) = limit {
        sqlx::query_as!(
            Item,
            "select * from item where list = $1 and public = true and deleted is NULL order by added desc limit $2",
            list,
            limit as i64,
        )
        .fetch_all(conn)
        .await
    } else {
        sqlx::query_as!(
            Item,
            "select * from item where list = $1 and public = true and deleted is NULL order by added desc",
            list
        )
        .fetch_all(conn)
        .await
    }
}

pub async fn get_list_all_items(
    conn: &mut PgConnection,
    list: Uuid,
    limit: Option<usize>,
) -> sqlx::Result<Vec<Item>> {
    if let Some(limit) = limit {
        sqlx::query_as!(
            Item,
            "select * from item where list = $1 and deleted is NULL order by added desc limit $2",
            list,
            limit as i64,
        )
        .fetch_all(conn)
        .await
    } else {
        sqlx::query_as!(
            Item,
            "select * from item where list = $1 and deleted is NULL order by added desc",
            list
        )
        .fetch_all(conn)
        .await
    }
}

pub async fn create_item(
    conn: &mut PgConnection,
    list: Uuid,
    note: &str,
    guid: Option<&str>,
    added_through: AddedThrough,
    added: Option<UtcDateTime>,
) -> sqlx::Result<Uuid> {
    let added = added.unwrap_or_else(UtcDateTime::now);
    let added = PrimitiveDateTime::new(added.date(), added.time());

    let res = sqlx::query!(
        "insert into item (id, list, note, public, rss_guid, added_through, added) values ($1, $2, $3, (select list.public from list where list.id = $2), $4, $5, $6) returning id",
        Uuid::new_v4(),
        list,
        note,
        guid,
        added_through as i32,
        added
    )
    .fetch_one(conn)
    .await?;

    Ok(res.id)
}

pub async fn item_set_public(
    conn: &mut PgConnection,
    item: Uuid,
    public: bool,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"update item set public = $2, updated = CURRENT_TIMESTAMP where id = $1"#,
        item,
        public,
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn item_delete(conn: &mut PgConnection, item: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        r#"update item set deleted = CURRENT_TIMESTAMP, updated = CURRENT_TIMESTAMP where id = $1"#,
        item,
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn move_item(conn: &mut PgConnection, item: Uuid, to_list: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        r#"update item set list = $2, updated = CURRENT_TIMESTAMP where id = $1"#,
        item,
        to_list
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn item_is_public(conn: &mut PgConnection, item: Uuid) -> sqlx::Result<bool> {
    let record = sqlx::query!(r#"select public from item where id = $1"#, item)
        .fetch_one(conn)
        .await?;

    Ok(record.public)
}

pub async fn set_item_note(conn: &mut PgConnection, item: Uuid, note: &str) -> sqlx::Result<()> {
    sqlx::query!(
        r#"update item set note = $2, updated = CURRENT_TIMESTAMP where id = $1"#,
        item,
        note
    )
    .execute(conn)
    .await?;

    Ok(())
}
