use std::sync::Arc;

use axum::{
    Json, Router,
    extract::Request,
    http::StatusCode,
    routing::{get, post},
};
use clap::Parser;
use color_eyre::eyre::WrapErr;
use serde::{Deserialize, Serialize};
use sqlx::{Connection, Executor, PgConnection, Pool, Postgres, postgres::PgPoolOptions};
use tower_http::trace::TraceLayer;
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(test)]
mod tests;

#[derive(Parser, Debug)]
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
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .try_init()
        .context("init tracing")?;

    Ok(())
}

async fn init_database(args: &Args, create_db: bool) -> color_eyre::Result<Pool<Postgres>> {
    if create_db {
        info!("creating database {}", args.db_name);
        // Connect to the default `postgres` database to create a new database
        let mut connection = PgConnection::connect(&args.db_base()).await?;

        // create unique logical database
        connection
            .execute(sqlx::AssertSqlSafe(format!(
                r#"CREATE DATABASE "{}";"#,
                args.db_name
            )))
            .await?;
    }

    info!("connecting to database");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&args.db_connection_str())
        .await
        .context("connect to pool")?;

    info!("running migrations");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("migrations")?;

    Ok(pool)
}

struct RouteState {
    db: Pool<Postgres>,
}

fn shared_setup() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();
    color_eyre::install()?;
    init_tracing()?;

    Ok(())
}

async fn init_app(state: Arc<RouteState>) -> Router {
    let app = Router::new().with_state(state);
    routes(app).await
}

async fn routes<S: Clone + Send + Sync + 'static>(r: Router<S>) -> Router<S> {
    r.layer(
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

    let args = Arc::new(Args::parse());
    let pool = init_database(&args, false).await?;
    let state = Arc::new(RouteState { db: pool });

    let app = init_app(state).await;

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
