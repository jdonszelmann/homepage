use std::fmt::Display;

use eyre::{Context, bail};
use serde::{Deserialize, Deserializer, de};
use std::str::FromStr;
use time::UtcDateTime;
use tracing::error;
use uuid::Uuid;

use crate::{
    auth::User,
    pages::lists::raw::{
        self, all_lists, get_item, get_list, get_list_all_items, get_list_public_items,
        item_set_public, list_is_public, list_set_public, move_item, public_lists, rename_list,
        set_item_link, set_item_note,
    },
    state::ArcRouteState,
};

#[derive(Deserialize, Clone, Copy)]
#[serde(transparent)]
pub struct ListId(Uuid);

impl Display for ListId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Deserialize, Clone, Copy)]
#[serde(transparent)]
pub struct ItemId(Uuid);

impl Display for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Deserialize)]
pub struct CreateList {
    name: String,
}

#[derive(Deserialize)]
pub struct CreateItem {
    list: ListId,
    note: String,
    link: Option<String>,
    link_type: Option<LinkType>,
}

#[derive(Deserialize, Clone)]
#[serde(tag = "kind")]
#[serde(rename_all = "snake_case")]
pub enum EditItemKind {
    SetNote {
        note: String,
    },
    SetLink {
        link: String,
        ty: LinkType,
    },
    MoveItem {
        list: ListId,
    },
    Public {
        #[serde(deserialize_with = "de_from_str")]
        public: bool,
    },
}

#[derive(Deserialize, Clone)]
#[serde(tag = "kind")]
#[serde(rename_all = "snake_case")]
pub enum EditListKind {
    SetName {
        name: String,
    },
    Public {
        #[serde(deserialize_with = "de_from_str")]
        public: bool,
    },
}

fn de_from_str<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    bool::from_str(&s).map_err(de::Error::custom)
}

pub struct List {
    pub id: ListId,
    pub name: String,

    pub public: bool,

    pub added: UtcDateTime,
    pub updated: UtcDateTime,
    pub deleted: Option<UtcDateTime>,
}

impl List {
    pub fn from_raw(
        raw::List {
            id,
            name,
            public,
            added,
            updated,
            deleted,
        }: raw::List,
    ) -> eyre::Result<Self> {
        Ok(Self {
            id: ListId(id),
            name,
            public,
            added: added.as_utc(),
            updated: updated.as_utc(),
            deleted: deleted.map(|i| i.as_utc()),
        })
    }
}

#[derive(Deserialize, Clone, Copy, Default)]
pub enum LinkType {
    Url,
    #[default]
    Unset,
}

impl LinkType {
    pub fn into_raw(&self) -> &'static str {
        match self {
            LinkType::Url => "url",
            LinkType::Unset => "",
        }
    }

    pub fn from_raw(s: String) -> eyre::Result<Self> {
        Ok(match s.as_str() {
            "url" => Self::Url,
            "" => Self::Unset,
            other => {
                error!("invalid link type: {other}");
                bail!("invalid link type: {other}")
            }
        })
    }
}

pub struct Item {
    pub id: ItemId,
    pub list: ListId,

    pub note: String,

    pub link: String,
    pub link_type: LinkType,

    pub public: bool,

    pub added: UtcDateTime,
    pub updated: UtcDateTime,
    pub deleted: Option<UtcDateTime>,
}

impl Item {
    pub fn from_raw(
        raw::Item {
            id,
            list,
            note,
            link,
            link_type,
            public,
            added,
            updated,
            deleted,
        }: raw::Item,
    ) -> eyre::Result<Self> {
        Ok(Self {
            id: ItemId(id),
            list: ListId(list),
            note,
            link,
            link_type: LinkType::from_raw(link_type)?,
            public,
            added: added.as_utc(),
            updated: updated.as_utc(),
            deleted: deleted.map(|i| i.as_utc()),
        })
    }
}

pub async fn get_lists(user: Option<&User>, state: ArcRouteState) -> eyre::Result<Vec<List>> {
    let mut conn = state.db.acquire().await.context("aqcuire")?;

    let res = if let Some(_) = user {
        all_lists(&mut conn).await?
    } else {
        public_lists(&mut conn).await?
    };

    Ok(res
        .into_iter()
        .map(List::from_raw)
        .collect::<Result<_, _>>()?)
}

pub async fn get_items(
    user: Option<&User>,
    state: ArcRouteState,
    list: ListId,
) -> eyre::Result<Vec<Item>> {
    let mut conn = state.db.begin().await.context("start tx")?;

    let res = if let Some(_) = user {
        get_list_all_items(&mut conn, list.0).await?
    } else {
        if !list_is_public(&mut conn, list.0).await? {
            return Ok(Vec::new());
        }

        get_list_public_items(&mut conn, list.0).await?
    };

    conn.commit().await.context("commit tx")?;

    Ok(res
        .into_iter()
        .map(Item::from_raw)
        .collect::<Result<_, _>>()?)
}

pub async fn create_list(
    _user: &User,
    state: ArcRouteState,
    CreateList { name }: CreateList,
) -> eyre::Result<()> {
    let mut conn = state.db.acquire().await.context("aqcuire")?;
    raw::create_list(&mut conn, &name).await?;

    Ok(())
}

pub async fn delete_list(_user: &User, state: ArcRouteState, list: ListId) -> eyre::Result<()> {
    let mut conn = state.db.acquire().await.context("aqcuire")?;
    raw::list_delete(&mut conn, list.0).await?;

    Ok(())
}

pub async fn delete_item(_user: &User, state: ArcRouteState, item: ItemId) -> eyre::Result<()> {
    let mut conn = state.db.acquire().await.context("aqcuire")?;
    raw::item_delete(&mut conn, item.0).await?;

    Ok(())
}

pub async fn create_item(
    _user: &User,
    state: ArcRouteState,
    CreateItem {
        list,
        note,
        link,
        link_type,
    }: CreateItem,
) -> eyre::Result<()> {
    let mut conn = state.db.begin().await.context("start tx")?;

    let item = raw::create_item(&mut conn, list.0, &note).await?;
    if let Some(link) = link {
        raw::set_item_link(
            &mut conn,
            item,
            &link,
            link_type.unwrap_or_default().into_raw(),
        )
        .await?;
    }

    conn.commit().await.context("commit tx")?;

    Ok(())
}

pub async fn edit_item(
    _user: &User,
    state: ArcRouteState,
    item: ItemId,
    edit: EditItemKind,
) -> eyre::Result<Item> {
    let mut conn = state.db.begin().await.context("start tx")?;

    match edit {
        EditItemKind::SetNote { note } => set_item_note(&mut conn, item.0, &note)
            .await
            .context("set note")?,
        EditItemKind::SetLink { link, ty } => {
            set_item_link(&mut conn, item.0, &link, ty.into_raw())
                .await
                .context("set link")?
        }
        EditItemKind::MoveItem { list } => move_item(&mut conn, item.0, list.0)
            .await
            .context("move item")?,
        EditItemKind::Public { public } => item_set_public(&mut conn, item.0, public)
            .await
            .context("set public")?,
    }

    let item = get_item(&mut conn, item.0).await?;
    conn.commit().await.context("commit tx")?;

    Ok(Item::from_raw(item)?)
}

pub async fn edit_list(
    _user: &User,
    state: ArcRouteState,
    list: ListId,
    edit: EditListKind,
) -> eyre::Result<List> {
    let mut conn = state.db.begin().await.context("start tx")?;

    match edit {
        EditListKind::SetName { name } => rename_list(&mut conn, list.0, &name)
            .await
            .context("set name")?,
        EditListKind::Public { public } => list_set_public(&mut conn, list.0, public)
            .await
            .context("set public")?,
    }

    let list = get_list(&mut conn, list.0).await?;

    conn.commit().await.context("commit tx")?;

    Ok(List::from_raw(list)?)
}
