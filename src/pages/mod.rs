use std::{convert::Infallible, fmt::Display};

use crate::{auth::User, state::ArcRouteState};
use askama::{Template, filters::HtmlSafe};
use axum::{
    RequestPartsExt, Router,
    extract::{FromRequestParts, Query},
    routing::get,
};
use serde::Deserialize;

struct Icon(&'static str);

impl HtmlSafe for Icon {}
impl Display for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r#"<i class="icon">{}</i>"#, self.0)
    }
}

macro_rules! icon {
    ($name: ident) => {
        $crate::pages::Icon(fontawesome_free_pack::$name.svg)
    };
}

pub mod blog;
pub mod error;
pub mod index;

#[derive(Template, Default)]
#[template(path = "layouts/base.html")]
pub struct Base {
    gay: bool,
    wide: bool,
    user: Option<User>,
}

impl<S: Send + Sync> FromRequestParts<S> for Base {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        let user: Option<User> = parts.extract().await?;

        #[derive(Deserialize, Default)]
        struct GayParams {
            gay: bool,
        }
        let Query(GayParams { gay }) = parts.extract().await.unwrap_or_default();

        Ok(Self {
            gay,
            wide: false,
            user,
        })
    }
}

pub struct GithubUrlParts {
    org: String,
    repo: String,
    number: String,
}

impl Base {
    fn split_github_url(&self, url: &str) -> GithubUrlParts {
        let url = url.trim_start_matches("https://github.com/");
        let (org, rest) = url.split_once("/").unwrap_or(("rust-lang", url));
        let (mut repo, number) = rest.rsplit_once("#").unwrap_or(("rust", url));
        if repo.is_empty() {
            repo = "rust";
        }

        GithubUrlParts {
            org: org.to_string(),
            repo: repo.to_string(),
            number: number.to_string(),
        }
    }
}

pub fn routes(r: Router<ArcRouteState>) -> Router<ArcRouteState> {
    let r = r.route("/", get(index::index));

    blog::routes(r)
}
