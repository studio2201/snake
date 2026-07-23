//! Process entrypoint.
//!
//! Delegates the entire startup sequence to [`bootstrap::build_runtime`]
//! and the URL assembly to [`router::build_router`]. Anything substantive
//! lives in those modules; this file should stay under ~30 lines.

use std::net::SocketAddr;

use backend::bootstrap;
use backend::router::build_router;

#[tokio::main]
async fn main() {
    let runtime = match bootstrap::build_runtime().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("startup failed: {e}");
            std::process::exit(1);
        }
    };

    tracing::info!(
        target: "main",
        port = runtime.port,
        version = %runtime.state.config.version,
        environment = %runtime.state.config.node_env,
        base_url = %runtime.state.config.base_url,
        "snake backend ready"
    );

    let app = build_router(runtime.state, &runtime.web_root);
    let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", runtime.port)).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(target: "main", error = %e, port = runtime.port, "bind failed");
            std::process::exit(1);
        }
    };
    if let Err(e) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    {
        tracing::error!(target: "main", error = %e, "server stopped with error");
        std::process::exit(1);
    }
}
