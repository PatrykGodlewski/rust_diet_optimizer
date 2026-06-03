use crate::domain::{NutritionalProfile, ResolvedMacroTargets, UserConstraints};

/// Weighted squared-error cost function for diet plan fitness.
#[derive(Debug, Clone)]
pub struct CostFunction {
    pub calorie_weight: f64,
    pub carbs_weight: f64,
    pub protein_weight: f64,
    pub fat_weight: f64,
}

impl Default for CostFunction {
    fn default() -> Self {
        Self {
            calorie_weight: 1.0,
            carbs_weight: 2.0,
            protein_weight: 2.0,
            fat_weight: 2.0,
        }
    }
}

impl CostFunction {
    pub fn score(
        &self,
        actual: &NutritionalProfile,
        constraints: &UserConstraints,
        resolved_macros: &ResolvedMacroTargets,
    ) -> f64 {
        let cal_err = relative_error(actual.calories, constraints.target_calories);
        let carb_err = relative_error(actual.carbs_g, resolved_macros.carbs_g);
        let protein_err = relative_error(actual.protein_g, resolved_macros.protein_g);
        let fat_err = relative_error(actual.fat_g, resolved_macros.fat_g);

        self.calorie_weight * cal_err.powi(2)
            + self.carbs_weight * carb_err.powi(2)
            + self.protein_weight * protein_err.powi(2)
            + self.fat_weight * fat_err.powi(2)
    }

    /// Lower bound for a partial plan: assumes remaining slots contribute zero
    /// (optimistic for pruning — only valid when we haven't filled all meals).
    pub fn partial_lower_bound(
        &self,
        partial: &NutritionalProfile,
        constraints: &UserConstraints,
        resolved_macros: &ResolvedMacroTargets,
    ) -> f64 {
        self.score(partial, constraints, resolved_macros)
    }
}

fn relative_error(actual: f64, target: f64) -> f64 {
    if target.abs() < f64::EPSILON {
        actual.abs()
    } else {
        (actual - target) / target
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MacroTargets;

    #[test]
    fn perfect_match_has_zero_cost() {
        let cost_fn = CostFunction::default();
        let constraints = UserConstraints {
            target_calories: 2000.0,
            macro_targets: MacroTargets::Grams {
                carbs_g: 250.0,
                protein_g: 150.0,
                fat_g: 67.0,
            },
            dietary_exclusions: vec![],
            min_meals: 3,
            max_meals: 5,
        };
        let macros = constraints.resolved_macros().unwrap();
        let nutrition = NutritionalProfile {
            calories: 2000.0,
            carbs_g: 250.0,
            protein_g: 150.0,
            fat_g: 67.0,
        };
        assert!((cost_fn.score(&nutrition, &constraints, &macros) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn deviation_increases_cost() {
        let cost_fn = CostFunction::default();
        let constraints = UserConstraints {
            target_calories: 2000.0,
            macro_targets: MacroTargets::Grams {
                carbs_g: 250.0,
                protein_g: 150.0,
                fat_g: 67.0,
            },
            dietary_exclusions: vec![],
            min_meals: 3,
            max_meals: 5,
        };
        let macros = constraints.resolved_macros().unwrap();
        let close = NutritionalProfile {
            calories: 2050.0,
            carbs_g: 255.0,
            protein_g: 152.0,
            fat_g: 68.0,
        };
        let far = NutritionalProfile {
            calories: 3000.0,
            carbs_g: 400.0,
            protein_g: 50.0,
            fat_g: 20.0,
        };
        assert!(
            cost_fn.score(&far, &constraints, &macros)
                > cost_fn.score(&close, &constraints, &macros)
        );
    }
}
