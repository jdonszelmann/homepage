use std::ops::Deref;

use crate::pages::{Base, error::RequestError};
use askama::Template;
use axum::response::{Html, IntoResponse};

#[derive(Template)]
#[template(path = "pages/index.html")]
struct Index {
    base: Base,
}

impl Deref for Index {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[axum::debug_handler]
pub async fn index(base: Base) -> Result<impl IntoResponse, RequestError> {
    let template = Index { base };

    Ok(Html(template.render()?))
}
