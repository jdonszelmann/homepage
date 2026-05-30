use crate::auth::User;
use askama::Template;
use axum::response::IntoResponse;

mod blog;

// #[derive(Template)]
// #[template(path = "layouts/base.html")]
// struct Base {
//     gay: bool,
//     wide: bool,
// }

#[axum::debug_handler]
pub async fn index(user: User) -> impl IntoResponse {
    format!("hello, world! {user:?}")
}
