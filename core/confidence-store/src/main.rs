use anyhow::Result;
use axum::{routing::get, Router};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let app = Router::new().route("/health", get(|| async { "ok" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8003").await?;
    tracing::info!("Confidence Store service listening on port 8003");
    axum::serve(listener, app).await?;

    Ok(())
}
