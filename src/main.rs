use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use diet_optimizer::application::DietOptimizationService;
use diet_optimizer::infrastructure::http::{create_router, AppState};
use diet_optimizer::infrastructure::repository::InMemoryRecipeRepository;
use diet_optimizer::optimization::BranchAndBoundOptimizer;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let repository = Arc::new(InMemoryRecipeRepository::with_defaults());
    let optimizer = Arc::new(BranchAndBoundOptimizer::default());
    let service: AppState = Arc::new(DietOptimizationService::new(repository, optimizer));

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .context("invalid HOST/PORT")?;

    let app = create_router(service);
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;

    info!(%addr, "diet-optimizer listening");
    axum::serve(listener, app)
        .await
        .context("server error")?;

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,diet_optimizer=debug,tower_http=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
