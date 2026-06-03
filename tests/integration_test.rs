use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use diet_optimizer::application::DietOptimizationService;
use diet_optimizer::domain::MacroTargets;
use diet_optimizer::infrastructure::http::create_router;
use diet_optimizer::infrastructure::repository::InMemoryRecipeRepository;
use diet_optimizer::optimization::BranchAndBoundOptimizer;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn test_app() -> axum::Router {
    let repository = Arc::new(InMemoryRecipeRepository::with_defaults());
    let optimizer = Arc::new(BranchAndBoundOptimizer::default());
    let service = Arc::new(DietOptimizationService::new(repository, optimizer));
    create_router(service)
}

#[tokio::test]
async fn optimize_diet_endpoint_returns_plan() {
    let app = test_app();

    let body = serde_json::json!({
        "target_calories": 2000.0,
        "macro_targets": {
            "type": "percentages",
            "carbs_pct": 50.0,
            "protein_pct": 25.0,
            "fat_pct": 25.0
        },
        "dietary_exclusions": ["dairy"],
        "min_meals": 3,
        "max_meals": 4
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/optimize-diet")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let meals = json["plan"]["meals"].as_array().unwrap();
    assert!(meals.len() >= 3 && meals.len() <= 4);
    assert!(json["plan"]["cost_score"].as_f64().unwrap() >= 0.0);
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = test_app();

    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn invalid_request_returns_validation_error() {
    let app = test_app();

    let body = serde_json::json!({
        "target_calories": -100.0,
        "macro_targets": {
            "type": "grams",
            "carbs_g": 250.0,
            "protein_g": 150.0,
            "fat_g": 67.0
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/optimize-diet")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn bad_macro_percentages_return_400() {
    let app = test_app();

    let body = serde_json::json!({
        "target_calories": 2000.0,
        "macro_targets": {
            "type": "percentages",
            "carbs_pct": 80.0,
            "protein_pct": 80.0,
            "fat_pct": 80.0
        },
        "min_meals": 3,
        "max_meals": 3
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/optimize-diet")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
