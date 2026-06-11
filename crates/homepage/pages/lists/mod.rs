use std::ops::Deref;

use askama::Template;
use axum::{
    Router,
    extract::{Form, Path, State},
    http::header::HeaderName,
    response::{Html, IntoResponse, Redirect},
    routing::{delete, get, post, put},
};

use crate::{
    auth::User,
    pages::{
        Base, LoggedinBase,
        error::RequestError,
        lists::hl::{
            CreateItem, CreateList, EditItemKind, EditListKind, Item, ItemId, List, ListId,
            get_items, get_lists,
        },
    },
    state::ArcRouteState,
};

mod hl;
mod raw;

#[derive(Template)]
#[template(path = "pages/list.html", blocks=["listitem"])]
struct ListTemplate {
    base: Base,
    list: ListId,
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
    Ok(Html(
        ListTemplate {
            items: get_items(base.user.as_ref(), state, list).await?,
            list,
            base,
        }
        .render()?,
    ))
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
    hl::create_list(&user, state, list).await?;

    Ok([(HeaderName::from_static("hx-refresh"), "true")])
}

async fn delete_list(
    user: User,
    State(state): State<ArcRouteState>,
    Path(list): Path<ListId>,
) -> Result<impl IntoResponse, RequestError> {
    hl::delete_list(&user, state, list).await?;

    Ok(())
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
    let item = hl::edit_item(base.user(), state, item, edit.clone()).await?;

    if let EditItemKind::MoveItem { list } = edit {
        return Ok(Redirect::to(&format!("/list/{list}")).into_response());
    }

    Ok(Html(
        ListTemplate {
            list: item.list,
            items: vec![item],
            base: base.0,
        }
        .as_listitem()
        .render()?,
    )
    .into_response())
}

pub fn routes(app: Router<ArcRouteState>) -> Router<ArcRouteState> {
    app.route("/list", get(all_lists))
        .route("/list/{:id}", get(list))
        .route("/list", post(create_list))
        .route("/list/{:id}", delete(delete_list))
        .route("/list/{:id}", put(edit_list))
        .route("/list/item", post(create_item))
        .route("/list/item/{:id}", delete(delete_item))
        .route("/list/item/{:id}", put(edit_item))
}
