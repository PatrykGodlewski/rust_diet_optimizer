use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::domain::{DietPlan, MacroTargets, UserConstraints};

#[derive(Debug, Deserialize, Validate)]
pub struct OptimizeDietRequest {
    #[validate(range(min = 1.0, max = 10000.0))]
    pub target_calories: f64,
    pub macro_targets: MacroTargets,
    #[serde(default)]
    pub dietary_exclusions: Vec<String>,
    #[serde(default = "default_min_meals")]
    #[validate(range(min = 1, max = 10))]
    pub min_meals: usize,
    #[serde(default = "default_max_meals")]
    #[validate(range(min = 1, max = 10))]
    pub max_meals: usize,
}

fn default_min_meals() -> usize {
    UserConstraints::DEFAULT_MIN_MEALS
}

fn default_max_meals() -> usize {
    UserConstraints::DEFAULT_MAX_MEALS
}

impl OptimizeDietRequest {
    pub fn into_constraints(self) -> UserConstraints {
        UserConstraints {
            target_calories: self.target_calories,
            macro_targets: self.macro_targets,
            dietary_exclusions: self.dietary_exclusions,
            min_meals: self.min_meals,
            max_meals: self.max_meals,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OptimizeDietResponse {
    pub plan: DietPlan,
}

impl From<DietPlan> for OptimizeDietResponse {
    fn from(plan: DietPlan) -> Self {
        Self { plan }
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}
