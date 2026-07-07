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

pub async fn create_list(conn: &mut PgConnection, name: &str) -> sqlx::Result<List> {
    let list = sqlx::query_as!(
        List,
        "insert into list values ($1, $2, false) returning *",
        Uuid::new_v4(),
        name,
    )
    .fetch_one(conn)
    .await?;

    Ok(list)
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
