use std::fmt::Display;

use serde::Deserialize;
use time::UtcDateTime;
use uuid::Uuid;

use crate::pages::lists::{list::hl::ListId, rss::raw};

#[derive(Deserialize, Clone, Copy)]
#[serde(transparent)]
pub struct RssId(Uuid);

impl Display for RssId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Deserialize)]
pub struct CreateRss {
    list: ListId,
    url: String,
}

#[derive(Deserialize, Clone)]
#[serde(tag = "kind")]
#[serde(rename_all = "snake_case")]
pub enum EditRssKind {
    SetUrl { url: String },
}

pub struct Rss {
    pub id: RssId,
    pub list: ListId,
    pub url: String,

    pub added: UtcDateTime,
    pub updated: UtcDateTime,
    pub deleted: Option<UtcDateTime>,
}

impl Rss {
    pub fn from_raw(
        raw::Rss {
            id,
            list,
            url,
            added,
            updated,
            deleted,
        }: raw::Rss,
    ) -> eyre::Result<Self> {
        Ok(Self {
            id: RssId(id),
            list: ListId(list),
            url,
            added: added.as_utc(),
            updated: updated.as_utc(),
            deleted: deleted.map(|i| i.as_utc()),
        })
    }
}
