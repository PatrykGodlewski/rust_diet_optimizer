use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use diet_optimizer::application::DietOptimizationService;
use diet_optimizer::infrastructure::http::create_router;
use diet_optimizer::infrastructure::repository::InMemoryRecipeRepository;
use diet_optimizer::optimization::BranchAndBoundOptimizer;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn test_app() -> axum::Router {
    let repository = Arc::new(InMemoryRecipeRepository::with_defaults());
    let optimizer = Arc::new(BranchAndBoundOptimizer::default());
    let service = Arc::new(DietOptimizationService::new(repository, optimizer));
    create_router(service)
}

async fn post_optimize(app: &axum::Router, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
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

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(json!({ "raw": String::from_utf8_lossy(&bytes) }));
    (status, json)
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = test_app();

    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["service"], "diet-optimizer");
}

#[tokio::test]
async fn optimize_diet_with_percentage_macros_returns_plan() {
    let app = test_app();

    let body = json!({
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

    let (status, json) = post_optimize(&app, body).await;
    assert_eq!(status, StatusCode::OK);

    let meals = json["plan"]["meals"].as_array().unwrap();
    assert!(meals.len() >= 3 && meals.len() <= 4);
    assert!(json["plan"]["cost_score"].as_f64().unwrap() >= 0.0);
    assert!(json["plan"]["total_nutrition"]["calories"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn optimize_diet_with_gram_macros_returns_plan() {
    let app = test_app();

    let body = json!({
        "target_calories": 2200.0,
        "macro_targets": {
            "type": "grams",
            "carbs_g": 200.0,
            "protein_g": 180.0,
            "fat_g": 70.0
        },
        "min_meals": 3,
        "max_meals": 3
    });

    let (status, json) = post_optimize(&app, body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["plan"]["meals"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn dietary_exclusions_remove_tagged_recipes() {
    let app = test_app();

    let body = json!({
        "target_calories": 2000.0,
        "macro_targets": {
            "type": "percentages",
            "carbs_pct": 50.0,
            "protein_pct": 25.0,
            "fat_pct": 25.0
        },
        "dietary_exclusions": ["dairy"],
        "min_meals": 3,
        "max_meals": 5
    });

    let (status, json) = post_optimize(&app, body).await;
    assert_eq!(status, StatusCode::OK);

    let dairy_recipe_ids = ["greek-salad", "egg-scramble", "cottage-cheese"];
    for meal in json["plan"]["meals"].as_array().unwrap() {
        let id = meal["recipe_id"].as_str().unwrap();
        assert!(
            !dairy_recipe_ids.contains(&id),
            "recipe {id} should be excluded when dairy is excluded"
        );
    }
}

#[tokio::test]
async fn plan_meals_include_required_fields() {
    let app = test_app();

    let body = json!({
        "target_calories": 1800.0,
        "macro_targets": {
            "type": "grams",
            "carbs_g": 200.0,
            "protein_g": 120.0,
            "fat_g": 60.0
        },
        "min_meals": 3,
        "max_meals": 3
    });

    let (status, json) = post_optimize(&app, body).await;
    assert_eq!(status, StatusCode::OK);

    for meal in json["plan"]["meals"].as_array().unwrap() {
        assert!(meal["recipe_id"].is_string());
        assert!(meal["recipe_name"].is_string());
        assert!(meal["nutrition"]["calories"].is_number());
        assert!(meal["nutrition"]["carbs_g"].is_number());
        assert!(meal["nutrition"]["protein_g"].is_number());
        assert!(meal["nutrition"]["fat_g"].is_number());
    }
}

#[tokio::test]
async fn invalid_request_returns_validation_error() {
    let app = test_app();

    let body = json!({
        "target_calories": -100.0,
        "macro_targets": {
            "type": "grams",
            "carbs_g": 250.0,
            "protein_g": 150.0,
            "fat_g": 67.0
        }
    });

    let (status, json) = post_optimize(&app, body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn bad_macro_percentages_return_400() {
    let app = test_app();

    let body = json!({
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

    let (status, json) = post_optimize(&app, body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "INVALID_CONSTRAINTS");
}

#[tokio::test]
async fn min_meals_greater_than_max_returns_400() {
    let app = test_app();

    let body = json!({
        "target_calories": 2000.0,
        "macro_targets": {
            "type": "grams",
            "carbs_g": 250.0,
            "protein_g": 150.0,
            "fat_g": 67.0
        },
        "min_meals": 5,
        "max_meals": 2
    });

    let (status, json) = post_optimize(&app, body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "INVALID_CONSTRAINTS");
}

#[tokio::test]
async fn exclusions_that_remove_all_recipes_return_422() {
    let app = test_app();

    // Every recipe in the default catalog has at least one meal-time tag.
    let body = json!({
        "target_calories": 2000.0,
        "macro_targets": {
            "type": "grams",
            "carbs_g": 250.0,
            "protein_g": 150.0,
            "fat_g": 67.0
        },
        "dietary_exclusions": ["breakfast", "lunch", "dinner", "snack"],
        "min_meals": 3,
        "max_meals": 3
    });

    let (status, json) = post_optimize(&app, body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json["code"], "OPTIMIZATION_FAILED");
}
