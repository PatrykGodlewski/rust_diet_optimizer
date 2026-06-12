use std::sync::Arc;

use crate::application::RecipeRepository;
use crate::domain::{DietOptimizer, DietPlan, DomainError, UserConstraints};

pub struct DietOptimizationService<O: DietOptimizer> {
    repository: Arc<dyn RecipeRepository>,
    optimizer: Arc<O>,
}

impl<O: DietOptimizer> DietOptimizationService<O> {
    pub fn new(repository: Arc<dyn RecipeRepository>, optimizer: Arc<O>) -> Self {
        Self {
            repository,
            optimizer,
        }
    }

    pub fn optimize_diet(&self, constraints: UserConstraints) -> Result<DietPlan, DomainError> {
        constraints.validate_meal_bounds()?;
        constraints.resolved_macros()?;

        let eligible = self
            .repository
            .filter_eligible(&constraints.dietary_exclusions)?;

        self.optimizer.optimize(&eligible, &constraints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MacroTargets;
    use crate::infrastructure::repository::InMemoryRecipeRepository;
    use crate::optimization::BranchAndBoundOptimizer;

    #[test]
    fn service_filters_excluded_tags() {
        let repo = Arc::new(InMemoryRecipeRepository::with_defaults());
        let optimizer = Arc::new(BranchAndBoundOptimizer::default());
        let service = DietOptimizationService::new(repo, optimizer);

        let constraints = UserConstraints {
            target_calories: 1500.0,
            macro_targets: MacroTargets::Percentages {
                carbs_pct: 50.0,
                protein_pct: 25.0,
                fat_pct: 25.0,
            },
            dietary_exclusions: vec!["dairy".into()],
            min_meals: 3,
            max_meals: 3,
        };

        let plan = service.optimize_diet(constraints).unwrap();
        for meal in &plan.meals {
            assert!(!meal.recipe_name.to_lowercase().contains("cheese"));
        }
    }
}
