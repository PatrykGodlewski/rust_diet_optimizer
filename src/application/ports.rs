use crate::domain::{DomainError, Recipe};

/// Port for recipe catalog access.
pub trait RecipeRepository: Send + Sync {
    fn all_recipes(&self) -> &[Recipe];
    fn filter_eligible(&self, exclusions: &[String]) -> Result<Vec<Recipe>, DomainError>;
}
