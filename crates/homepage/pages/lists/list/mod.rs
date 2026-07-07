use std::ops::Deref;

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    http::{StatusCode, header::HeaderName},
    response::{Html, IntoResponse},
};
use eyre::Context;

use crate::{
    auth::User,
    pages::{
        Base, LoggedinBase,
        error::RequestError,
        lists::{
            item::hl::{Item, get_items},
            list::hl::{CreateList, EditListKind, List, ListId, get_lists},
        },
    },
    state::ArcRouteState,
};

/// A list of links that I publish on my blog. Special uuid, cannot be removed from the db, and is
/// automatically inserted if it doesn't exist.
pub const LINKS_LIST: ListId = ListId(uuid::uuid!("bec58e9c-bfad-4c66-9b95-4954f5917a34"));

pub mod hl;
mod raw;

pub(super) use raw::get_list;

/// This is the only way in which we allow creating a list *without required authentication*,
/// since this list is always expected to exist anyway.
async fn ensure_links_list_exists(state: &ArcRouteState) -> eyre::Result<()> {
    let mut conn = state.db.acquire().await.context("aqcuire")?;
    raw::ensure_list_exists(&mut conn, LINKS_LIST.0, "links", true).await?;

    Ok(())
}

pub async fn create_list(
    user: User,
    State(state): State<ArcRouteState>,
    Form(list): Form<CreateList>,
) -> Result<impl IntoResponse, RequestError> {
    let list = hl::create_list(&user, state, list).await?;

    Ok([(HeaderName::from_static("hx-redirect"), list.link())])
}

pub async fn delete_list(
    user: User,
    State(state): State<ArcRouteState>,
    Path(list): Path<ListId>,
) -> Result<impl IntoResponse, RequestError> {
    // can't delete the links list
    if list == LINKS_LIST {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    hl::delete_list(&user, state, list).await?;

    Ok(().into_response())
}

pub async fn edit_list(
    base: LoggedinBase,
    State(state): State<ArcRouteState>,
    Path(list): Path<ListId>,
    Form(edit): Form<EditListKind>,
) -> Result<impl IntoResponse, RequestError> {
    let list = hl::edit_list(base.user(), state, list, edit).await?;

    Ok(Html(
        AllListsTemplate {
            lists: vec![list],
            base: base.0,
        }
        .as_listitem()
        .render()?,
    ))
}

#[derive(Template)]
#[template(path = "pages/list.html", blocks=["listitem"])]
pub struct ListTemplate {
    pub base: Base,
    pub list: List,
    pub items: Vec<Item>,
}

impl Deref for ListTemplate {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

pub async fn list(
    base: Base,
    State(state): State<ArcRouteState>,
    Path(list): Path<ListId>,
) -> Result<impl IntoResponse, RequestError> {
    ensure_links_list_exists(&state).await?;
    let (items, list) = get_items(base.user.as_ref(), &state, list, None).await?;

    Ok(Html(ListTemplate { items, list, base }.render()?))
}

#[derive(Template)]
#[template(path = "pages/lists.html", blocks=["listitem"])]
struct AllListsTemplate {
    base: Base,
    lists: Vec<List>,
}

impl Deref for AllListsTemplate {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

pub async fn all_lists(
    base: Base,
    State(state): State<ArcRouteState>,
) -> Result<impl IntoResponse, RequestError> {
    ensure_links_list_exists(&state).await?;

    Ok(Html(
        AllListsTemplate {
            lists: get_lists(base.user.as_ref(), state).await?,
            base,
        }
        .render()?,
    ))
}
