use askama::Template;
use axum::{
    extract::{Form, Path, State},
    http::header::HeaderName,
    response::{Html, IntoResponse, Redirect},
};

use crate::{
    auth::User,
    pages::{
        LoggedinBase,
        error::RequestError,
        lists::{
            item::hl::{CreateItem, EditItemKind, ItemId},
            list::ListTemplate,
        },
    },
    state::ArcRouteState,
};

pub mod hl;
mod raw;

pub async fn create_item(
    user: User,
    State(state): State<ArcRouteState>,
    Form(item): Form<CreateItem>,
) -> Result<impl IntoResponse, RequestError> {
    hl::create_item(&user, state, item).await?;

    Ok([(HeaderName::from_static("hx-refresh"), "true")])
}

pub async fn delete_item(
    user: User,
    State(state): State<ArcRouteState>,
    Path(item): Path<ItemId>,
) -> Result<impl IntoResponse, RequestError> {
    hl::delete_item(&user, state, item).await?;

    Ok(())
}

pub async fn edit_item(
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
            rss_sources: vec![],
            base: base.0,
        }
        .as_listitem()
        .render()?,
    )
    .into_response())
}
