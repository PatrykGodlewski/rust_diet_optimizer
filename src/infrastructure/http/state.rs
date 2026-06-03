use std::sync::Arc;

use crate::application::DietOptimizationService;
use crate::optimization::BranchAndBoundOptimizer;

pub type AppState = Arc<DietOptimizationService<BranchAndBoundOptimizer>>;
