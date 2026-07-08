use std::fmt::Display;

use eyre::Context;
use serde::Deserialize;
use time::UtcDateTime;
use uuid::Uuid;

use crate::{
    auth::User,
    pages::lists::{
        list::{
            get_list,
            hl::{List, ListId},
        },
        rss::raw::{self, delete_rss, rss_sources_for_list, set_rss_url},
    },
    state::ArcRouteState,
};

#[derive(Deserialize, Clone, Copy)]
#[serde(transparent)]
pub struct RssId(pub Uuid);

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

#[derive(Clone)]
pub struct Rss {
    pub id: RssId,
    pub list: ListId,
    pub url: String,

    pub last_error: Option<String>,

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
            last_error,
            added,
            updated,
            deleted,
        }: raw::Rss,
    ) -> eyre::Result<Self> {
        Ok(Self {
            id: RssId(id),
            list: ListId(list),
            url,
            last_error,
            added: added.as_utc(),
            updated: updated.as_utc(),
            deleted: deleted.map(|i| i.as_utc()),
        })
    }
}

pub async fn get_rss_sources(
    user: Option<&User>,
    state: &ArcRouteState,
    list: ListId,
) -> eyre::Result<Vec<Rss>> {
    if user.is_none() {
        return Ok(Vec::new());
    }

    let mut conn = state.db.acquire().await.context("aqcuire")?;
    let res = rss_sources_for_list(&mut conn, list.0).await?;

    res.into_iter().map(Rss::from_raw).collect::<Result<_, _>>()
}

pub async fn delete_rss_source(_user: &User, state: ArcRouteState, rss: RssId) -> eyre::Result<()> {
    let mut conn = state.db.acquire().await.context("aqcuire")?;
    delete_rss(&mut conn, rss.0).await?;

    Ok(())
}

pub async fn add_rss_source(
    _user: &User,
    state: &ArcRouteState,
    CreateRss { list, url }: CreateRss,
) -> eyre::Result<RssId> {
    let mut conn = state.db.begin().await.context("start tx")?;

    let rss = raw::create_rss(&mut conn, list.0, &url).await?;
    conn.commit().await.context("commit tx")?;

    Ok(RssId(rss))
}

pub async fn edit_rss_source(
    _user: &User,
    state: ArcRouteState,
    rss: RssId,
    edit: EditRssKind,
) -> eyre::Result<(Rss, List)> {
    let mut conn = state.db.begin().await.context("start tx")?;

    match edit {
        EditRssKind::SetUrl { url } => set_rss_url(&mut conn, rss.0, &url)
            .await
            .context("set note")?,
    }

    let rss = raw::get_rss(&mut conn, rss.0).await?;
    let list = get_list(&mut conn, rss.list).await?;
    conn.commit().await.context("commit tx")?;

    Ok((Rss::from_raw(rss)?, List::from_raw(list)?))
}

pub async fn get_rss(_user: &User, state: &ArcRouteState, rss: RssId) -> eyre::Result<Rss> {
    let mut conn = state.db.acquire().await.context("start tx")?;
    let rss = raw::get_rss(&mut conn, rss.0).await?;

    Rss::from_raw(rss)
}
