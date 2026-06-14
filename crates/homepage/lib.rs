#[cfg(feature = "live")]
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::{
    Router, ServiceExt,
    extract::Request,
    http::{HeaderValue, header},
};
use clap::Parser;
use eyre::WrapErr;
use tower::{Layer, ServiceBuilder};
use tower_http::{
    normalize_path::{NormalizePath, NormalizePathLayer},
    services::ServeDir,
    set_header::SetResponseHeaderLayer,
    trace::{DefaultOnRequest, TraceLayer},
};
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_tree::HierarchicalLayer;

use crate::{
    auth::auth_routes,
    state::{ArcRouteState, RouteState, init_database},
};

#[cfg(feature = "live")]
pub use homepage_live::get_templates;

mod auth;
mod pages;
mod state;
#[cfg(test)]
mod tests;

static GAY_MODE: AtomicBool = AtomicBool::new(false);

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, env = "HOMEPAGE_DB_HOST")]
    db_host: String,

    #[arg(long, env = "HOMEPAGE_DB_PORT", default_value_t = 5432)]
    db_port: u16,

    #[arg(long, env = "HOMEPAGE_DB_USER")]
    db_user: Option<String>,

    #[arg(long, env = "HOMEPAGE_DB_PASSWORD")]
    db_password: Option<String>,

    #[arg(long, env = "HOMEPAGE_DB_NAME")]
    db_name: String,

    #[arg(long, env = "HOMEPAGE_HOST", default_value = "localhost")]
    host: String,

    #[arg(long, env = "HOMEPAGE_PORT", default_value_t = 3000)]
    port: u16,

    #[arg(long, env = "HOMEPAGE_CLIENT_ID")]
    client_id: String,

    #[arg(long, env = "HOMEPAGE_CLIENT_SECRET")]
    client_secret: String,

    #[arg(long, env = "HOMEPAGE_AUTH_SERVER")]
    auth_server: String,

    #[arg(long, env = "HOMEPAGE_BASE_URL")]
    base_url: String,

    #[arg(long, env = "HOMEPAGE_GAY")]
    gay: bool,
}

impl Args {
    fn db_base(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.db_user.as_ref().unwrap_or(&self.db_name),
            self.db_password.as_ref().unwrap_or(&self.db_name),
            self.db_host,
            self.db_port,
        )
    }

    fn db_connection_str(&self) -> String {
        format!("{}/{}", self.db_base(), self.db_name)
    }

    fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn init_tracing() -> eyre::Result<()> {
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();
    let tree_layer = HierarchicalLayer::new(2)
        .with_ansi(true)
        .with_indent_lines(true);
    // .with_verbose_entry(true)
    // .with_verbose_exit(true)
    // .with_span_retrace(true);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tree_layer)
        .try_init()
        .context("init tracing")?;

    Ok(())
}

fn shared_setup() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing()?;

    Ok(())
}

async fn init_app(state: ArcRouteState) -> eyre::Result<NormalizePath<Router>> {
    let app = Router::new();
    // static dir
    let app = app.nest_service(
        "/static",
        ServiceBuilder::new()
            .layer(SetResponseHeaderLayer::overriding(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, no-transform"),
            ))
            .service(ServeDir::new("public")),
    );

    // webpages
    let app = pages::routes(app);
    // authentication
    let app = auth_routes(app, state.clone())
        .await
        .context("auth routes")?;

    // tracing
    let app = app.layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &Request<_>| {
                let request_id = uuid::Uuid::new_v4().to_string();

                tracing::span!(
                    Level::INFO,
                    "request",
                    %request_id,
                    method = ?request.method(),
                    uri = %request.uri(),
                    version = ?request.version(),
                )
            })
            .on_request(DefaultOnRequest::new().level(Level::INFO)),
    );

    Ok(NormalizePathLayer::trim_trailing_slash().layer(app.with_state(state)))
}

async fn start() -> eyre::Result<()> {
    shared_setup()?;

    #[cfg(feature = "live")]
    homepage_live::start_watching(
        &[Path::new("./templates")],
        &[Path::new("./crates"), Path::new("Cargo.toml")],
    )
    .context("start watch")?;

    let args = Args::parse();
    let pool = init_database(&args, false).await?;
    let state = ArcRouteState::new(RouteState {
        db: pool,
        args: args.clone(),
    });

    if args.gay {
        GAY_MODE.store(true, Ordering::Relaxed);
    }

    let app = init_app(state).await.context("init app")?;

    let bind_addr = args.bind_addr();
    info!("listening on {bind_addr:?}");
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .context("bind tcp listener")?;
    axum::serve(listener, ServiceExt::<Request>::into_make_service(app))
        .await
        .context("serve failed")?;

    Ok(())
}

pub type MainResult = eyre::Result<()>;
#[tokio::main]
pub async fn main() -> MainResult {
    start().await?;

    Ok(())
}
