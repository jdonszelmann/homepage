use std::fmt::Display;

use eyre::Context;
use serde::Deserialize;
use time::UtcDateTime;
use uuid::Uuid;

use crate::{
    auth::User,
    pages::lists::list::raw::{
        self, all_lists, get_list, list_set_public, public_lists, rename_list,
    },
    state::ArcRouteState,
};

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(transparent)]
pub struct ListId(pub Uuid);

impl Display for ListId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Deserialize)]
pub struct CreateList {
    name: String,
}

impl List {
    pub fn link(&self) -> String {
        format!("/list/{}", self.id)
    }

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

#[derive(Deserialize, Clone)]
#[serde(tag = "kind")]
#[serde(rename_all = "snake_case")]
pub enum EditListKind {
    SetName {
        name: String,
    },
    Public {
        #[serde(deserialize_with = "crate::pages::lists::de_from_str")]
        public: bool,
    },
}

pub struct List {
    pub id: ListId,
    pub name: String,

    pub public: bool,

    pub added: UtcDateTime,
    pub updated: UtcDateTime,
    pub deleted: Option<UtcDateTime>,
}

pub async fn get_lists(user: Option<&User>, state: ArcRouteState) -> eyre::Result<Vec<List>> {
    let mut conn = state.db.acquire().await.context("aqcuire")?;

    let res = if user.is_some() {
        all_lists(&mut conn).await?
    } else {
        public_lists(&mut conn).await?
    };

    res.into_iter()
        .map(List::from_raw)
        .collect::<Result<_, _>>()
}

pub(super) async fn create_list_unauthenticated(
    state: ArcRouteState,
    CreateList { name }: CreateList,
) -> eyre::Result<List> {
    let mut conn = state.db.acquire().await.context("aqcuire")?;
    let list = raw::create_list(&mut conn, &name).await?;

    List::from_raw(list)
}

pub async fn create_list(_user: &User, state: ArcRouteState, cl: CreateList) -> eyre::Result<List> {
    create_list_unauthenticated(state, cl).await
}

pub async fn delete_list(_user: &User, state: ArcRouteState, list: ListId) -> eyre::Result<()> {
    let mut conn = state.db.acquire().await.context("aqcuire")?;
    raw::list_delete(&mut conn, list.0).await?;

    Ok(())
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

    List::from_raw(list)
}
