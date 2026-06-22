use std::{convert::Infallible, fmt::Display, ops::Deref, sync::atomic::Ordering};

use crate::{
    GAY_MODE,
    auth::{User, require_login},
    pages::error::RequestError,
    state::ArcRouteState,
};
use askama::{
    Template,
    filters::{HtmlSafe, Safe},
};
use axum::{
    RequestPartsExt, Router,
    extract::{FromRef, FromRequestParts, Query},
    http::request,
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
pub mod dashboard;
pub mod error;
pub mod index;
pub mod lists;
pub mod rss;

#[askama::filter_fn]
pub fn markdown(value: impl Display, env: &dyn askama::Values) -> askama::Result<Safe<String>> {
    let input = value.to_string();
    Ok(Safe(
        homepage_markdown::markdown_to_html(&input).unwrap_or_else(|| {
            let Ok(escaped) = askama::filters::escape(input, askama::filters::Html);
            escaped.0.to_string()
        }),
    ))
}

#[derive(Template, Default)]
#[template(path = "layouts/base.html")]
pub struct Base {
    gay: bool,
    wide: bool,
    user: Option<User>,
    state: Option<ArcRouteState>,
}

async fn extract_base<E>(
    parts: &mut request::Parts,
    state: ArcRouteState,
    user: Option<User>,
) -> Result<Base, E> {
    #[derive(Deserialize, Default)]
    struct GayParams {
        gay: bool,
    }
    let Query(GayParams { gay }) = parts.extract().await.unwrap_or_else(|_| {
        Query(GayParams {
            gay: GAY_MODE.load(Ordering::Relaxed),
        })
    });

    Ok(Base {
        gay,
        wide: false,
        user,
        state: Some(state),
    })
}

impl<S: Send + Sync> FromRequestParts<S> for Base
where
    ArcRouteState: FromRef<S>,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let user: Option<User> = parts.extract().await?;
        let state = ArcRouteState::from_ref(state);

        extract_base(parts, state, user).await
    }
}

pub struct LoggedinBase(Base);

impl Deref for LoggedinBase {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl LoggedinBase {
    pub fn user(&self) -> &User {
        self.0.user.as_ref().unwrap()
    }
}

impl<S: Send + Sync> FromRequestParts<S> for LoggedinBase
where
    ArcRouteState: FromRef<S>,
{
    type Rejection = RequestError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let user: User = parts.extract().await?;
        let state = ArcRouteState::from_ref(state);

        extract_base(parts, state, Some(user))
            .await
            .map(LoggedinBase)
    }
}

pub struct GithubUrlParts {
    org: String,
    repo: String,
    number: String,
}

impl Base {
    // only takes self to be accessible in templates.
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

// I like this because it makes every line the same: "let app = something(app);"
#[allow(clippy::let_and_return)]
pub fn routes(app: Router<ArcRouteState>) -> Router<ArcRouteState> {
    let app = app.fallback(error::fallback);

    let app = app.route("/", get(index::index));
    let app = app.route("/rss.xml", get(crate::pages::rss::rss_xml));
    let app = app.route(
        "/dashboard",
        get(dashboard::dashboard).layer(require_login()),
    );

    let app = blog::routes(app);
    let app = lists::routes(app);

    app
}
