use std::{
    ffi::OsString,
    iter,
    panic::{UnwindSafe, catch_unwind, resume_unwind},
    sync::Arc,
};

use crate::{Args, init_database};
use axum_test::TestServer;
use clap::Parser;
use sqlx::{Connection, Executor, PgConnection};
use tracing::info;
use uuid::Uuid;

use crate::{RouteState, init_app, shared_setup};

fn start_test_server<O, F: UnwindSafe + AsyncFnOnce(TestServer) -> eyre::Result<O>>(
    f: F,
) -> yre::Result<O> {
    let mut args = Args::parse_from(iter::empty::<OsString>());

    // ignore errors, they only happen if we've already initialized for this process which we expect
    // given that tests run in the same process
    let _ = shared_setup();
    info!("starting test");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");

    // test overrides for args
    {
        args.db_user = Some("postgres".to_string());
        args.db_password = Some("postgres".to_string());
        args.db_name = Uuid::new_v4().to_string();
    }

    let res = catch_unwind(|| {
        runtime.block_on(async {
            let pool = init_database(&args, true).await?;
            let state = Arc::new(RouteState { db: pool });

            let app = init_app(state).await;
            let server = TestServer::new(app);

            f(server).await
        })
    });

    info!("deleting database {}", args.db_name);
    runtime.block_on(async {
        let mut connection = PgConnection::connect(&args.db_base())
            .await
            .expect("new connection to delete db");
        let _ = connection
            .execute(sqlx::AssertSqlSafe(format!(
                r#"DROP DATABASE "{}";"#,
                args.db_name
            )))
            .await;
    });

    match res {
        Ok(i) => i,
        Err(e) => {
            resume_unwind(e);
        }
    }
}

#[test]
fn test_server_starts() {
    start_test_server(async |s| Ok(())).unwrap();
}
