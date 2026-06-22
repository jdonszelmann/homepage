use std::{convert::Infallible, ops::Deref};

use crate::pages::lists::{Item, LINKS_LIST, get_items};
use crate::pages::{Base, error::RequestError};
use crate::state::ArcRouteState;
use askama::{DynTemplate, Template};
use axum::response::Html;
use axum::routing::MethodRouter;
use axum::routing::get;
use eyre::{Context, bail};
use homepage_live::LiveTemplate;
use homepage_markdown::BlogPost;
use homepage_route_gen::generate_blog_routes;

pub const BLOGPOST_INFO: &[RouteInfo] = generated::ALL_POSTS;

pub trait PostTemplate: DynTemplate + LiveTemplate {
    fn render_contents(&self) -> Result<String, askama::Error>;
}

pub type RenderFn = fn(Option<Base>) -> Box<dyn PostTemplate>;
pub type RouteInfo<'a> = &'a (&'a str, BlogPost, RenderFn);

// used in the expansion of generate_blog_routes
mod prelude {
    pub(super) use super::overview_route;
    pub use crate::pages::{Base, error::RequestError};
    pub use crate::state::ArcRouteState;
    pub use askama::Template;
    pub use axum::{
        response::IntoResponse,
        routing::{Router, get},
    };
    pub use homepage_live::LiveTemplate;
    pub use homepage_markdown::{BlogPost, Preamble, Variant};
    pub use std::ops::Deref;

    pub(super) use super::{PostTemplate, RouteInfo};
}

macro_rules! generate_route {
    ($data: expr) => {{
        pub async fn blog_route(mut base: Base) -> Result<impl IntoResponse, RequestError> {
            use axum::response::Html;
            let data = $data;

            base.wide |= matches!(data.1.preamble.variant, Variant::Music);
            let res = (data.2)(Some(base));

            Ok(Html(res.render_live()?))
        }

        get(blog_route)
    }};
}

generate_blog_routes!("../../../..", generate_route, overview_route);

pub struct Link {
    note: String,
}

async fn get_links(base: &Base, limit: usize) -> eyre::Result<Vec<Link>> {
    let Some(state) = &base.state else {
        bail!("no state")
    };

    let items = get_items(None, state, LINKS_LIST, Some(limit))
        .await
        .context("get links")?
        .0;

    Ok(items
        .into_iter()
        .map(|Item { id, note, .. }| Link { note })
        .collect())
}

#[derive(Template)]
#[template(path = "layouts/overview.html")]
struct Overview<'a> {
    base: Base,
    posts: &'a [RouteInfo<'a>],
    links: Vec<Link>,
}

impl Deref for Overview<'_> {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

fn overview_route(
    posts: &'static [RouteInfo<'static>],
    show_links: bool,
) -> MethodRouter<ArcRouteState, Infallible> {
    let num_links = if show_links { 5 } else { 0 };
    get(async move |base: Base| -> Result<_, RequestError> {
        let links = get_links(&base, num_links).await.context("get links")?;
        let template = Overview { base, posts, links };
        Ok(Html(template.render()?))
    })
}
