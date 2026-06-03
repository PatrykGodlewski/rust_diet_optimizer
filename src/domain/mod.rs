pub mod cost;
pub mod error;
pub mod optimizer;
pub mod plan;
pub mod recipe;
pub mod constraints;

pub use constraints::{MacroTargets, ResolvedMacroTargets, UserConstraints};
pub use cost::CostFunction;
pub use error::DomainError;
pub use optimizer::DietOptimizer;
pub use plan::{DietPlan, PlannedMeal};
pub use recipe::{NutritionalProfile, Recipe};
