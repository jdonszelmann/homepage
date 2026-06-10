use std::{ops::Deref, sync::Arc, time::Duration};

use eyre::Context;
use sqlx::{
    ConnectOptions, Connection, Executor, PgConnection, Pool, Postgres,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tracing::info;

use crate::Args;

pub async fn init_database(args: &Args, create_db: bool) -> eyre::Result<Pool<Postgres>> {
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

    let connection = args
        .db_connection_str()
        .parse::<PgConnectOptions>()
        .context("parse db connection str")?
        .log_slow_statements(log::LevelFilter::Debug, Duration::from_millis(200))
        .log_statements(log::LevelFilter::Debug);

    info!("connecting to database");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(connection)
        .await
        .context("connect to pool")?;

    info!("running migrations");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .context("migrations")?;

    Ok(pool)
}

#[derive(Debug)]
pub struct RouteState {
    pub db: Pool<Postgres>,
    pub args: Args,
}

#[derive(Debug, Clone)]
pub struct ArcRouteState(Arc<RouteState>);

impl Deref for ArcRouteState {
    type Target = RouteState;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl ArcRouteState {
    pub fn new(state: RouteState) -> Self {
        Self(Arc::new(state))
    }
}
