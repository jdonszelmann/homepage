use axum::{Router, extract::Request, routing::get};
use clap::Parser;
use color_eyre::eyre::WrapErr;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_tree::HierarchicalLayer;

use crate::{
    auth::auth_routes,
    state::{ArcRouteState, RouteState, init_database},
};

mod auth;
mod pages;
mod state;
#[cfg(test)]
mod tests;

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

fn init_tracing() -> color_eyre::Result<()> {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("debug"))
        .unwrap();
    let tree_layer = HierarchicalLayer::new(2)
        .with_ansi(true)
        .with_indent_lines(true)
        .with_verbose_entry(true)
        .with_verbose_exit(true);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(tree_layer)
        .try_init()
        .context("init tracing")?;

    Ok(())
}

fn shared_setup() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();
    color_eyre::install()?;
    init_tracing()?;

    Ok(())
}

async fn init_app(state: ArcRouteState) -> color_eyre::Result<Router> {
    let app = Router::new().with_state(state.clone());
    let app = routes(app).await;
    let app = auth_routes(app, state).await.context("auth routes")?;

    Ok(app)
}

async fn routes<S: Clone + Send + Sync + 'static>(r: Router<S>) -> Router<S> {
    r.route("/", get(pages::index))
        .nest_service("/static", ServeDir::new("public"))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let request_id = uuid::Uuid::new_v4().to_string();

                tracing::span!(
                    Level::INFO,
                    "request",
                    %request_id,
                    method = ?request.method(),
                    uri = %request.uri(),
                    version = ?request.version(),
                )
            }),
        )
}

async fn start() -> color_eyre::Result<()> {
    shared_setup()?;

    let args = Args::parse();
    let pool = init_database(&args, false).await?;
    let state = ArcRouteState::new(RouteState {
        db: pool,
        args: args.clone(),
    });

    let app = init_app(state).await.context("init app")?;

    let bind_addr = args.bind_addr();
    info!("listening on {bind_addr:?}");
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .context("bind tcp listener")?;
    axum::serve(listener, app).await.context("serve failed")?;

    Ok(())
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    start().await?;

    Ok(())
}
