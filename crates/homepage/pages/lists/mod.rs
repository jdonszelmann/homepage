use axum::{
    Router,
    response::Redirect,
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Deserializer, de};

use crate::{
    pages::lists::{
        item::{create_item, delete_item, edit_item},
        list::{LINKS_LIST, all_lists, create_list, delete_list, edit_list, list},
    },
    state::ArcRouteState,
};
use std::str::FromStr;

pub mod item;
pub mod list;
pub mod rss;

fn de_from_str<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    bool::from_str(&s).map_err(de::Error::custom)
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
