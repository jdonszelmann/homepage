use sqlx::PgConnection;
use time::PrimitiveDateTime;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct List {
    pub id: Uuid,
    pub name: String,

    pub public: bool,

    pub added: PrimitiveDateTime,
    pub updated: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
}

#[derive(sqlx::FromRow)]
pub struct Item {
    pub id: Uuid,
    pub list: Uuid,

    pub note: String,

    pub link: String,
    pub link_type: String,

    pub public: bool,

    pub added: PrimitiveDateTime,
    pub updated: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
}

pub async fn public_lists(conn: &mut PgConnection) -> sqlx::Result<Vec<List>> {
    sqlx::query_as!(
        List,
        "select * from list where public = true and deleted is NULL"
    )
    .fetch_all(conn)
    .await
}

pub async fn all_lists(conn: &mut PgConnection) -> sqlx::Result<Vec<List>> {
    sqlx::query_as!(List, "select * from list where deleted is NULL")
        .fetch_all(conn)
        .await
}

pub async fn get_list(conn: &mut PgConnection, list: Uuid) -> sqlx::Result<List> {
    sqlx::query_as!(List, "select * from list where id = $1", list)
        .fetch_one(conn)
        .await
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

pub async fn create_list(conn: &mut PgConnection, name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "insert into list values ($1, $2, false)",
        Uuid::new_v4(),
        name,
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn ensure_list_exists(
    conn: &mut PgConnection,
    uuid: Uuid,
    name: &str,
    public: bool,
) -> sqlx::Result<()> {
    sqlx::query!(
        "insert into list values ($1, $2, $3) ON CONFLICT(id) DO UPDATE SET name=EXCLUDED.name, public=EXCLUDED.public",
        uuid,
        name,
        public,
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn create_item(conn: &mut PgConnection, list: Uuid, note: &str) -> sqlx::Result<Uuid> {
    let res = sqlx::query!(
        "insert into item values ($1, $2, $3, '', '', (select list.public from list where list.id = $2)) returning id",
        Uuid::new_v4(),
        list,
        note,
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

pub async fn list_set_public(
    conn: &mut PgConnection,
    list: Uuid,
    public: bool,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"update list set public = $2, updated = CURRENT_TIMESTAMP where id = $1"#,
        list,
        public,
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn list_delete(conn: &mut PgConnection, list: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        r#"update list set deleted = CURRENT_TIMESTAMP, updated = CURRENT_TIMESTAMP where id = $1"#,
        list,
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

pub async fn rename_list(conn: &mut PgConnection, list: Uuid, name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        r#"update list set name = $2, updated = CURRENT_TIMESTAMP where id = $1"#,
        list,
        name
    )
    .execute(conn)
    .await?;

    Ok(())
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

pub async fn set_item_link(
    conn: &mut PgConnection,
    item: Uuid,
    link: &str,
    link_type: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"update item set link = $2, link_type = $3, updated = CURRENT_TIMESTAMP where id = $1"#,
        item,
        link,
        link_type,
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
