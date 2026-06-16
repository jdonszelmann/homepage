use std::ops::Deref;

use crate::pages::{LoggedinBase, error::RequestError};
use askama::Template;
use axum::response::{Html, IntoResponse};

#[derive(Template)]
#[template(path = "pages/dashboard.html")]
struct Dashboard {
    base: LoggedinBase,
}

impl Deref for Dashboard {
    type Target = LoggedinBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

pub async fn dashboard(base: LoggedinBase) -> Result<impl IntoResponse, RequestError> {
    let template = Dashboard { base };

    Ok(Html(template.render()?))
}
