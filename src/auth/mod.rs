use std::convert::Infallible;

use axum::{
    RequestPartsExt, Router,
    error_handling::HandleErrorLayer,
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::{Uri, request::Parts},
    response::{IntoResponse, Redirect},
    routing::{any, get},
};
use axum_oidc::{
    AdditionalClaims, EmptyAdditionalClaims, OidcAuthLayer, OidcClaims, OidcClient, OidcLoginLayer,
    OidcRpInitiatedLogout, OidcSession,
    error::{ExtractorError, MiddlewareError},
    handle_oidc_redirect,
    openidconnect::{ClientId, ClientSecret, IssuerUrl, Scope, core::CoreGenderClaim},
};
use color_eyre::eyre::Context;
use tower::ServiceBuilder;
use tower_sessions::{
    Expiry, SessionManagerLayer,
    cookie::{SameSite, time::Duration},
};

use crate::state::ArcRouteState;

mod session_store;

#[derive(Debug)]
pub struct User {
    pub name: String,
}

impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = ExtractorError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let claims: OidcClaims<EmptyAdditionalClaims> = parts.extract().await?;

        let name = claims
            .name()
            .and_then(|i| i.iter().next())
            .expect("no username in claims")
            .1
            .to_string();

        Ok(User { name })
    }
}

impl<S> OptionalFromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Option<Self>, Self::Rejection> {
        let Ok(claims): Result<OidcClaims<EmptyAdditionalClaims>, _> = parts.extract().await else {
            return Ok(None);
        };

        let name = claims
            .nickname()
            .and_then(|i| i.iter().next())
            .expect("no username in claims")
            .1
            .to_string();

        Ok(Some(User { name }))
    }
}

struct SessionWrapper(tower_sessions::Session);
impl<S: Send + Sync> FromRequestParts<S> for SessionWrapper {
    type Rejection = <tower_sessions::Session as FromRequestParts<S>>::Rejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = tower_sessions::Session::from_request_parts(parts, state).await?;
        Ok(Self(session))
    }
}

impl<AC: AdditionalClaims> axum_oidc::Session<AC> for SessionWrapper {
    type Error = tower_sessions::session::Error;
    async fn get(&self) -> Result<OidcSession<AC, CoreGenderClaim>, Self::Error> {
        Ok(self.0.get("axum-oidc").await?.unwrap_or_default())
    }
    async fn set(&mut self, value: OidcSession<AC, CoreGenderClaim>) -> Result<(), Self::Error> {
        self.0.insert("axum-oidc", value).await?;
        Ok(())
    }
}

pub async fn auth_routes<S: Clone + Send + Sync + 'static>(
    r: Router<S>,
    state: ArcRouteState,
) -> color_eyre::Result<Router<S>> {
    let session_layer = SessionManagerLayer::new(state.clone())
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::hours(12)));

    let oidc_login_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e: MiddlewareError| async {
            dbg!(&e);
            e.into_response()
        }))
        .layer(OidcLoginLayer::<EmptyAdditionalClaims, SessionWrapper>::new());

    const OIDC_URL: &str = "/auth";

    let oidc_client = OidcClient::<EmptyAdditionalClaims>::builder()
        .with_default_http_client()
        .with_redirect_url(
            Uri::try_from(format!("{}{}", state.args.base_url, OIDC_URL))
                .context("bad base url")?,
        )
        .with_client_id(ClientId::new(state.args.client_id.clone()))
        .add_scope(Scope::new("profile".into()))
        .add_scope(Scope::new("email".into()))
        .with_client_secret(ClientSecret::new(state.args.client_secret.clone()));

    let oidc_client = oidc_client
        .discover(IssuerUrl::new(state.args.auth_server.clone()).context("invalid auth server")?)
        .await
        .unwrap()
        .build();

    let oidc_auth_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e: MiddlewareError| async {
            dbg!(&e);
            e.into_response()
        }))
        .layer(OidcAuthLayer::<_, SessionWrapper>::new(oidc_client));

    let r = r
        .route("/logout", get(logout))
        .route("/login", get(login))
        .layer(oidc_login_service)
        .route(
            OIDC_URL,
            any(handle_oidc_redirect::<EmptyAdditionalClaims, SessionWrapper>),
        )
        .layer(oidc_auth_service)
        .layer(session_layer);

    Ok(r)
}

async fn login(_: User) -> impl IntoResponse {
    Redirect::temporary("/")
}

async fn logout(logout: OidcRpInitiatedLogout) -> impl IntoResponse {
    logout.with_post_logout_redirect(Uri::from_static("/"))
}
