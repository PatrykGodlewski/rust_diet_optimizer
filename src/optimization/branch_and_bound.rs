use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rayon::prelude::*;

use crate::domain::{
    CostFunction, DietOptimizer, DietPlan, DomainError, NutritionalProfile, Recipe,
    UserConstraints,
};

/// Branch-and-bound combinatorial search over meal combinations.
/// Explores combinations of size `min_meals..=max_meals` without repetition (order-independent).
pub struct BranchAndBoundOptimizer {
    cost_fn: CostFunction,
}

impl Default for BranchAndBoundOptimizer {
    fn default() -> Self {
        Self {
            cost_fn: CostFunction::default(),
        }
    }
}

impl BranchAndBoundOptimizer {
    pub fn new(cost_fn: CostFunction) -> Self {
        Self { cost_fn }
    }

    fn search_meal_count(
        &self,
        recipes: &[Recipe],
        constraints: &UserConstraints,
        resolved_macros: &crate::domain::ResolvedMacroTargets,
        meal_count: usize,
        global_best: &AtomicU64,
    ) -> Option<(Vec<usize>, f64)> {
        let n = recipes.len();
        if meal_count > n {
            return None;
        }

        let mut local_best: Option<(Vec<usize>, f64)> = None;
        let mut indices = (0..meal_count).collect::<Vec<_>>();

        loop {
            let partial_nutrition: NutritionalProfile = indices
                .iter()
                .map(|&i| recipes[i].nutrition.clone())
                .fold(
                    NutritionalProfile {
                        calories: 0.0,
                        carbs_g: 0.0,
                        protein_g: 0.0,
                        fat_g: 0.0,
                    },
                    |acc, p| NutritionalProfile {
                        calories: acc.calories + p.calories,
                        carbs_g: acc.carbs_g + p.carbs_g,
                        protein_g: acc.protein_g + p.protein_g,
                        fat_g: acc.fat_g + p.fat_g,
                    },
                );

            let cost = self.cost_fn.score(
                &partial_nutrition,
                constraints,
                resolved_macros,
            );

            let global_bits = global_best.load(Ordering::Relaxed);
            let global_cost = f64::from_bits(global_bits);

            if cost < global_cost {
                if local_best.as_ref().is_none_or(|(_, c)| cost < *c) {
                    local_best = Some((indices.clone(), cost));
                    global_best.fetch_min(cost.to_bits(), Ordering::Relaxed);
                }
            }

            if !next_combination(&mut indices, n) {
                break;
            }
        }

        local_best
    }
}

impl DietOptimizer for BranchAndBoundOptimizer {
    fn optimize(
        &self,
        recipes: &[Recipe],
        constraints: &UserConstraints,
    ) -> Result<DietPlan, DomainError> {
        if recipes.is_empty() {
            return Err(DomainError::NoEligibleRecipes);
        }

        constraints.validate_meal_bounds()?;
        let resolved_macros = constraints.resolved_macros()?;

        let global_best = Arc::new(AtomicU64::new(f64::MAX.to_bits()));

        let meal_range = constraints.min_meals..=constraints.max_meals;
        let results: Vec<Option<(Vec<usize>, f64)>> = meal_range
            .into_par_iter()
            .map(|meal_count| {
                self.search_meal_count(
                    recipes,
                    constraints,
                    &resolved_macros,
                    meal_count,
                    &global_best,
                )
            })
            .collect();

        let mut best: Option<(Vec<usize>, f64)> = None;
        for result in results.into_iter().flatten() {
            if best.as_ref().is_none_or(|(_, c)| result.1 < *c) {
                best = Some(result);
            }
        }

        let (indices, cost) = best.ok_or(DomainError::NoFeasiblePlan)?;
        let selected: Vec<Recipe> = indices.iter().map(|&i| recipes[i].clone()).collect();
        Ok(DietPlan::from_recipes(&selected, cost))
    }
}

/// Advance `indices` to the next strictly increasing combination, lexicographically.
/// Returns false when all combinations are exhausted.
fn next_combination(indices: &mut [usize], n: usize) -> bool {
    let k = indices.len();
    let mut i = k;
    while i > 0 {
        i -= 1;
        if indices[i] < n - k + i {
            indices[i] += 1;
            for j in i + 1..k {
                indices[j] = indices[j - 1] + 1;
            }
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MacroTargets;

    fn sample_recipes() -> Vec<Recipe> {
        vec![
            Recipe {
                id: "r1".into(),
                name: "Oatmeal".into(),
                nutrition: NutritionalProfile {
                    calories: 350.0,
                    carbs_g: 55.0,
                    protein_g: 12.0,
                    fat_g: 8.0,
                },
                tags: vec!["vegan".into(), "gluten-free".into()],
            },
            Recipe {
                id: "r2".into(),
                name: "Grilled Chicken".into(),
                nutrition: NutritionalProfile {
                    calories: 450.0,
                    carbs_g: 5.0,
                    protein_g: 45.0,
                    fat_g: 18.0,
                },
                tags: vec!["gluten-free".into()],
            },
            Recipe {
                id: "r3".into(),
                name: "Salmon Bowl".into(),
                nutrition: NutritionalProfile {
                    calories: 520.0,
                    carbs_g: 40.0,
                    protein_g: 35.0,
                    fat_g: 22.0,
                },
                tags: vec!["gluten-free".into(), "dairy-free".into()],
            },
            Recipe {
                id: "r4".into(),
                name: "Greek Salad".into(),
                nutrition: NutritionalProfile {
                    calories: 280.0,
                    carbs_g: 15.0,
                    protein_g: 10.0,
                    fat_g: 20.0,
                },
                tags: vec!["vegetarian".into()],
            },
            Recipe {
                id: "r5".into(),
                name: "Quinoa Stir Fry".into(),
                nutrition: NutritionalProfile {
                    calories: 400.0,
                    carbs_g: 50.0,
                    protein_g: 18.0,
                    fat_g: 12.0,
                },
                tags: vec!["vegan".into()],
            },
        ]
    }

    fn constraints_near(recipes: &[Recipe], meal_count: usize) -> UserConstraints {
        let subset: Vec<_> = recipes.iter().take(meal_count).collect();
        let total = NutritionalProfile::sum(subset.into_iter().map(|r| r.nutrition.clone()));
        UserConstraints {
            target_calories: total.calories,
            macro_targets: MacroTargets::Grams {
                carbs_g: total.carbs_g,
                protein_g: total.protein_g,
                fat_g: total.fat_g,
            },
            dietary_exclusions: vec![],
            min_meals: meal_count,
            max_meals: meal_count,
        }
    }

    #[test]
    fn finds_zero_cost_plan_when_exact_subset_exists() {
        let recipes = sample_recipes();
        let constraints = constraints_near(&recipes, 3);
        let optimizer = BranchAndBoundOptimizer::default();
        let plan = optimizer.optimize(&recipes, &constraints).unwrap();
        assert!((plan.cost_score - 0.0).abs() < 1e-6);
        assert_eq!(plan.meals.len(), 3);
    }

    #[test]
    fn optimizer_picks_lower_cost_than_random_triple() {
        let recipes = sample_recipes();
        let constraints = UserConstraints {
            target_calories: 1800.0,
            macro_targets: MacroTargets::Percentages {
                carbs_pct: 50.0,
                protein_pct: 25.0,
                fat_pct: 25.0,
            },
            dietary_exclusions: vec![],
            min_meals: 3,
            max_meals: 3,
        };
        let optimizer = BranchAndBoundOptimizer::default();
        let plan = optimizer.optimize(&recipes, &constraints).unwrap();

        let cost_fn = CostFunction::default();
        let macros = constraints.resolved_macros().unwrap();
        let bad_combo = [0usize, 3, 4];
        let bad_nutrition: NutritionalProfile = bad_combo
            .iter()
            .map(|&i| recipes[i].nutrition.clone())
            .fold(
                NutritionalProfile {
                    calories: 0.0,
                    carbs_g: 0.0,
                    protein_g: 0.0,
                    fat_g: 0.0,
                },
                |acc, p| NutritionalProfile {
                    calories: acc.calories + p.calories,
                    carbs_g: acc.carbs_g + p.carbs_g,
                    protein_g: acc.protein_g + p.protein_g,
                    fat_g: acc.fat_g + p.fat_g,
                },
            );
        let bad_cost = cost_fn.score(&bad_nutrition, &constraints, &macros);
        assert!(plan.cost_score <= bad_cost);
    }

    #[test]
    fn matches_brute_force_optimum_for_small_catalog() {
        let recipes = sample_recipes();
        let constraints = UserConstraints {
            target_calories: 1600.0,
            macro_targets: MacroTargets::Percentages {
                carbs_pct: 45.0,
                protein_pct: 30.0,
                fat_pct: 25.0,
            },
            dietary_exclusions: vec![],
            min_meals: 3,
            max_meals: 3,
        };
        let optimizer = BranchAndBoundOptimizer::default();
        let plan = optimizer.optimize(&recipes, &constraints).unwrap();
        let brute = brute_force_best(&recipes, &constraints);
        assert!((plan.cost_score - brute).abs() < 1e-9);
    }

    fn brute_force_best(recipes: &[Recipe], constraints: &UserConstraints) -> f64 {
        let cost_fn = CostFunction::default();
        let macros = constraints.resolved_macros().unwrap();
        let mut best = f64::MAX;
        for meal_count in constraints.min_meals..=constraints.max_meals {
            let mut indices = (0..meal_count).collect::<Vec<_>>();
            let n = recipes.len();
            loop {
                let nutrition: NutritionalProfile = indices
                    .iter()
                    .map(|&i| recipes[i].nutrition.clone())
                    .fold(
                        NutritionalProfile {
                            calories: 0.0,
                            carbs_g: 0.0,
                            protein_g: 0.0,
                            fat_g: 0.0,
                        },
                        |acc, p| NutritionalProfile {
                            calories: acc.calories + p.calories,
                            carbs_g: acc.carbs_g + p.carbs_g,
                            protein_g: acc.protein_g + p.protein_g,
                            fat_g: acc.fat_g + p.fat_g,
                        },
                    );
                let cost = cost_fn.score(&nutrition, constraints, &macros);
                if cost < best {
                    best = cost;
                }
                if !next_combination(&mut indices, n) {
                    break;
                }
            }
        }
        best
    }

    #[test]
    fn returns_error_when_no_recipes() {
        let optimizer = BranchAndBoundOptimizer::default();
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
        let err = optimizer.optimize(&[], &constraints).unwrap_err();
        assert_eq!(err, DomainError::NoEligibleRecipes);
    }
}
