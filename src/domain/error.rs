use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum DomainError {
    #[error("no recipes available after applying dietary exclusions")]
    NoEligibleRecipes,

    #[error("meal count must be between {min} and {max}, got {actual}")]
    InvalidMealCount { min: usize, max: usize, actual: usize },

    #[error("no feasible diet plan found for the given constraints")]
    NoFeasiblePlan,

    #[error("invalid macro targets: {0}")]
    InvalidMacroTargets(String),
}
