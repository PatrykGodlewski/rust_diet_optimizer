use crate::domain::{DietPlan, DomainError, Recipe, UserConstraints};

/// Port for combinatorial diet plan optimization algorithms.
pub trait DietOptimizer: Send + Sync {
    fn optimize(
        &self,
        recipes: &[Recipe],
        constraints: &UserConstraints,
    ) -> Result<DietPlan, DomainError>;
}
