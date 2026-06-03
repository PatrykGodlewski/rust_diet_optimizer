use axum::routing::{get, post};
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::infrastructure::http::handlers;
use crate::infrastructure::http::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/api/v1/optimize-diet", post(handlers::optimize_diet))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
