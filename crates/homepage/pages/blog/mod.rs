use std::{convert::Infallible, ops::Deref};

use crate::pages::lists::{Item, LINKS_LIST, get_items};
use crate::pages::{Base, error::RequestError};
use crate::state::ArcRouteState;
use askama::Template;
use axum::response::Html;
use axum::routing::MethodRouter;
use axum::routing::get;
use eyre::{Context, bail};
use homepage_live::LiveTemplate;
use homepage_markdown::BlogPost;
use homepage_route_gen::generate_blog_routes;

// used in the expansion of generate_blog_routes
mod prelude {
    pub use crate::state::ArcRouteState;
    pub use axum::{response::IntoResponse, routing::Router};
    pub use homepage_markdown::{BlogPost, Preamble, Variant};
}

macro_rules! generate_route {
    ($source: literal, $path: literal, $data: expr) => {{
        pub async fn blog_route(mut base: Base) -> Result<impl IntoResponse, RequestError> {
            #[derive(Template, LiveTemplate)]
            #[template(source = $source, ext="html")]
            #[template_disambiguator = $path]
            struct Template {
                base: Base,
                post: &'static BlogPost,
            }

            impl Deref for Template {
                type Target = Base;

                fn deref(&self) -> &Self::Target {
                    &self.base
                }
            }

            let post = &$data;
            base.wide |= matches!(post.preamble.variant, Variant::Music);
            let template = Template { base, post };

            Ok(Html(template.render_live()?))
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
    posts: &'a [(&'a str, BlogPost)],
    links: Vec<Link>,
}

impl Deref for Overview<'_> {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

fn overview_route(
    posts: &'static [(&'static str, BlogPost)],
    show_links: bool,
) -> MethodRouter<ArcRouteState, Infallible> {
    let num_links = if show_links { 5 } else { 0 };
    get(async move |base: Base| -> Result<_, RequestError> {
        let links = get_links(&base, num_links).await.context("get links")?;
        let template = Overview { base, posts, links };
        Ok(Html(template.render()?))
    })
}
