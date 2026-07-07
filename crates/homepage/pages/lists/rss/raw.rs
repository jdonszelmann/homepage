use time::PrimitiveDateTime;
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
