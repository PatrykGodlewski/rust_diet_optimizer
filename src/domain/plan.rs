use serde::{Deserialize, Serialize};

use crate::domain::{NutritionalProfile, Recipe};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedMeal {
    pub recipe_id: String,
    pub recipe_name: String,
    pub nutrition: NutritionalProfile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DietPlan {
    pub meals: Vec<PlannedMeal>,
    pub total_nutrition: NutritionalProfile,
    /// Lower is better (fitness / cost score).
    pub cost_score: f64,
}

impl DietPlan {
    pub fn from_recipes(recipes: &[Recipe], cost_score: f64) -> Self {
        let meals: Vec<PlannedMeal> = recipes
            .iter()
            .map(|r| PlannedMeal {
                recipe_id: r.id.clone(),
                recipe_name: r.name.clone(),
                nutrition: r.nutrition.clone(),
            })
            .collect();
        let total_nutrition =
            NutritionalProfile::sum(meals.iter().map(|m| m.nutrition.clone()));
        DietPlan {
            meals,
            total_nutrition,
            cost_score,
        }
    }
}
