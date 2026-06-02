use std::ops::Deref;

use crate::pages::Base;
use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use thiserror::Error;
use tracing::error;

#[derive(Template)]
#[template(path = "pages/error.html")]
struct ErrorTemplate {
    base: Base,
}

impl Deref for ErrorTemplate {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("could not render template: {0:?}")]
    Render(#[from] askama::Error),
    #[error(transparent)]
    Generic(#[from] color_eyre::Report),
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        error!("{self:?}");
        let status = match &self {
            RequestError::Render(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RequestError::Generic(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let tmpl = ErrorTemplate {
            base: Base {
                gay: false,
                wide: false,
                user: None,
            },
        };
        if let Ok(body) = tmpl.render() {
            (status, Html(body)).into_response()
        } else {
            (status, "Something went wrong").into_response()
        }
    }
}
