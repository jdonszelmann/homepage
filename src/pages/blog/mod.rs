use std::{convert::Infallible, ops::Deref};

use crate::{
    auth::User,
    pages::{Base, error::RequestError},
};
use askama::Template;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::{Router, routing::MethodRouter};
use homepage_macros::collect_blog_posts;
use homepage_markdown::{BlogPost, Preamble, Variant};
use std::borrow::Cow;

macro_rules! generate_route {
    ($source: literal, $data: expr) => {{
        // #[axum::debug_handler]
        pub async fn blog_route(user: Option<User>) -> Result<impl IntoResponse, RequestError> {
            #[derive(Template)]
            #[template(source = $source, ext="html")]
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

            let template = Template {
                base: Base {
                    gay: false,
                    wide: matches!(post.preamble.variant, Variant::Music),
                    user,
                },
                post,
            };

            Ok(Html(template.render()?))
        }

        get(blog_route)
    }};
}

collect_blog_posts!(generate_route);

#[derive(Template)]
#[template(path = "layouts/overview.html")]
struct Overview<'a> {
    base: Base,
    posts: &'a [(&'a str, BlogPost)],
}

impl Deref for Overview<'_> {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

fn overview_route<S: Clone + Send + Sync + 'static>(
    posts: &'static [(&'static str, BlogPost)],
) -> MethodRouter<S, Infallible> {
    // TODO: don't prompt login on Option<User> routes
    get(async move |user: Option<User>| -> Result<_, RequestError> {
        let template = Overview {
            base: Base {
                gay: false,
                wide: false,
                user,
            },
            posts,
        };

        Ok(Html(template.render()?))
    })
}
