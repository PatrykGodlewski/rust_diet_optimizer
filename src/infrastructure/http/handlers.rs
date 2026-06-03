use axum::extract::State;
use axum::Json;
use tracing::instrument;
use validator::Validate;

use crate::infrastructure::http::dto::{HealthResponse, OptimizeDietRequest, OptimizeDietResponse};
use crate::infrastructure::http::error_response::ApiError;
use crate::infrastructure::http::state::AppState;

#[instrument(skip(state, payload))]
pub async fn optimize_diet(
    State(state): State<AppState>,
    Json(payload): Json<OptimizeDietRequest>,
) -> Result<Json<OptimizeDietResponse>, ApiError> {
    payload
        .validate()
        .map_err(ApiError::validation)?;

    if payload.min_meals > payload.max_meals {
        return Err(ApiError::domain(
            crate::domain::DomainError::InvalidMealCount {
                min: payload.min_meals,
                max: payload.max_meals,
                actual: payload.min_meals,
            },
        ));
    }

    let constraints = payload.into_constraints();
    let plan = state
        .optimize_diet(constraints)
        .map_err(ApiError::domain)?;

    Ok(Json(OptimizeDietResponse::from(plan)))
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "diet-optimizer",
    })
}
