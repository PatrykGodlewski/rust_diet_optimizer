use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use validator::ValidationErrors;

use crate::domain::DomainError;

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub error: String,
    pub code: String,
}

pub struct ApiError {
    pub status: StatusCode,
    pub body: ApiErrorBody,
}

impl ApiError {
    pub fn validation(errors: ValidationErrors) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: ApiErrorBody {
                error: errors.to_string(),
                code: "VALIDATION_ERROR".into(),
            },
        }
    }

    pub fn domain(err: DomainError) -> Self {
        let (status, code) = match &err {
            DomainError::NoEligibleRecipes | DomainError::NoFeasiblePlan => {
                (StatusCode::UNPROCESSABLE_ENTITY, "OPTIMIZATION_FAILED")
            }
            DomainError::InvalidMealCount { .. } | DomainError::InvalidMacroTargets(_) => {
                (StatusCode::BAD_REQUEST, "INVALID_CONSTRAINTS")
            }
        };
        Self {
            status,
            body: ApiErrorBody {
                error: err.to_string(),
                code: code.into(),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
