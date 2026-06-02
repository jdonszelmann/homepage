use std::fmt::Display;

use crate::auth::User;
use askama::{Template, filters::HtmlSafe};
use axum::{Router, routing::get};

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

pub struct GithubUrlParts {
    org: String,
    repo: String,
    number: String,
}

impl Base {
    fn split_github_url(url: &str) -> GithubUrlParts {
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

pub fn routes<S: Clone + Send + Sync + 'static>(r: Router<S>) -> Router<S> {
    let r = r.route("/", get(index::index));

    blog::routes(r)
}
