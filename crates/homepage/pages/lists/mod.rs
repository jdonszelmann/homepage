use std::ops::Deref;

use askama::Template;
use axum::{
    Router,
    extract::{Form, Path, State},
    http::{StatusCode, header::HeaderName},
    response::{Html, IntoResponse, Redirect},
    routing::{delete, get, post, put},
};
use eyre::Context;

use crate::{
    auth::User,
    pages::{
        Base, LoggedinBase,
        error::RequestError,
        lists::hl::{
            CreateItem, CreateList, EditItemKind, EditListKind, ItemId, List, ListId, get_lists,
        },
    },
    state::ArcRouteState,
};

pub use hl::{Item, get_items};

/// A list of links that I publish on my blog. Special uuid, cannot be removed from the db, and is
/// automatically inserted if it doesn't exist.
pub const LINKS_LIST: ListId = ListId(uuid::uuid!("bec58e9c-bfad-4c66-9b95-4954f5917a34"));

mod hl;
mod raw;

/// This is the only way in which we allow creating a list *without required authentication*,
/// since this list is always expected to exist anyway.
async fn ensure_links_list_exists(state: &ArcRouteState) -> eyre::Result<()> {
    let mut conn = state.db.acquire().await.context("aqcuire")?;
    raw::ensure_list_exists(&mut conn, LINKS_LIST.0, "links", true).await?;

    Ok(())
}

#[derive(Template)]
#[template(path = "pages/list.html", blocks=["listitem"])]
struct ListTemplate {
    base: Base,
    list: List,
    items: Vec<Item>,
}

impl Deref for ListTemplate {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

async fn list(
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

async fn all_lists(
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

async fn create_list(
    user: User,
    State(state): State<ArcRouteState>,
    Form(list): Form<CreateList>,
) -> Result<impl IntoResponse, RequestError> {
    let list = hl::create_list(&user, state, list).await?;

    Ok([(HeaderName::from_static("hx-redirect"), list.link())])
}

async fn delete_list(
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

async fn edit_list(
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

async fn create_item(
    user: User,
    State(state): State<ArcRouteState>,
    Form(item): Form<CreateItem>,
) -> Result<impl IntoResponse, RequestError> {
    hl::create_item(&user, state, item).await?;

    Ok([(HeaderName::from_static("hx-refresh"), "true")])
}

async fn delete_item(
    user: User,
    State(state): State<ArcRouteState>,
    Path(item): Path<ItemId>,
) -> Result<impl IntoResponse, RequestError> {
    hl::delete_item(&user, state, item).await?;

    Ok(())
}

async fn edit_item(
    base: LoggedinBase,
    State(state): State<ArcRouteState>,
    Path(item): Path<ItemId>,
    Form(edit): Form<EditItemKind>,
) -> Result<impl IntoResponse, RequestError> {
    let (item, list) = hl::edit_item(base.user(), state, item, edit.clone()).await?;

    if let EditItemKind::MoveItem { list } = edit {
        return Ok(Redirect::to(&format!("/list/{list}")).into_response());
    }

    Ok(Html(
        ListTemplate {
            list,
            items: vec![item],
            base: base.0,
        }
        .as_listitem()
        .render()?,
    )
    .into_response())
}

pub fn routes(app: Router<ArcRouteState>) -> Router<ArcRouteState> {
    app.route(
        "/links",
        get(async || Redirect::to(&format!("/list/{}", LINKS_LIST))),
    )
    .route("/list", get(all_lists))
    .route("/list/{:id}", get(list))
    .route("/list", post(create_list))
    .route("/list/{:id}", delete(delete_list))
    .route("/list/{:id}", put(edit_list))
    .route("/list/item", post(create_item))
    .route("/list/item/{:id}", delete(delete_item))
    .route("/list/item/{:id}", put(edit_item))
}
