use std::ops::Deref;

use crate::{
    auth::User,
    pages::{Base, error::RequestError},
};
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
pub async fn index(user: Option<User>) -> Result<impl IntoResponse, RequestError> {
    let template = Index {
        base: Base {
            gay: false,
            wide: false,
            user,
        },
    };

    Ok(Html(template.render()?))
}
