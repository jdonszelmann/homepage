use askama::Template;
use axum::{
    Form,
    extract::{Path, State},
    http::HeaderName,
    response::{Html, IntoResponse},
};

use crate::{
    auth::User,
    pages::{
        LoggedinBase,
        error::RequestError,
        lists::{
            list::ListTemplate,
            rss::hl::{CreateRss, EditRssKind, RssId, get_rss},
        },
    },
    state::ArcRouteState,
};

pub mod hl;
mod raw;
pub mod update_feed;

pub async fn update_rss(
    user: User,
    Path(rss): Path<RssId>,
    State(state): State<ArcRouteState>,
) -> Result<impl IntoResponse, RequestError> {
    let rss = get_rss(&user, &state, rss).await?;
    update_feed::try_update_feed(&state, &rss, true).await?;

    Ok([(HeaderName::from_static("hx-refresh"), "true")])
}

pub async fn add_rss_source(
    user: User,
    State(state): State<ArcRouteState>,
    Form(rss): Form<CreateRss>,
) -> Result<impl IntoResponse, RequestError> {
    let rss = hl::add_rss_source(&user, &state, rss).await?;
    let rss = get_rss(&user, &state, rss).await?;
    update_feed::try_update_feed(&state, &rss, true).await?;

    Ok([(HeaderName::from_static("hx-refresh"), "true")])
}

pub async fn delete_rss_source(
    user: User,
    State(state): State<ArcRouteState>,
    Path(rss): Path<RssId>,
) -> Result<impl IntoResponse, RequestError> {
    hl::delete_rss_source(&user, state, rss).await?;

    Ok(())
}

pub async fn edit_rss_source(
    base: LoggedinBase,
    State(state): State<ArcRouteState>,
    Path(item): Path<RssId>,
    Form(edit): Form<EditRssKind>,
) -> Result<impl IntoResponse, RequestError> {
    let (rss_source, list) = hl::edit_rss_source(base.user(), state, item, edit).await?;

    Ok(Html(
        ListTemplate {
            list,
            items: vec![],
            rss_sources: vec![rss_source],
            base: base.0,
        }
        .as_rss_source()
        .render()?,
    )
    .into_response())
}
