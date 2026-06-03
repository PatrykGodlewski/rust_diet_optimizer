use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;

/// Resolved macro targets in grams for a single day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedMacroTargets {
    pub carbs_g: f64,
    pub protein_g: f64,
    pub fat_g: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MacroTargets {
    Grams {
        carbs_g: f64,
        protein_g: f64,
        fat_g: f64,
    },
    /// Percentages of total calories (must sum to ~100).
    Percentages {
        carbs_pct: f64,
        protein_pct: f64,
        fat_pct: f64,
    },
}

impl MacroTargets {
    pub fn resolve(&self, target_calories: f64) -> Result<ResolvedMacroTargets, DomainError> {
        match self {
            MacroTargets::Grams {
                carbs_g,
                protein_g,
                fat_g,
            } => {
                if *carbs_g < 0.0 || *protein_g < 0.0 || *fat_g < 0.0 {
                    return Err(DomainError::InvalidMacroTargets(
                        "macro grams must be non-negative".into(),
                    ));
                }
                Ok(ResolvedMacroTargets {
                    carbs_g: *carbs_g,
                    protein_g: *protein_g,
                    fat_g: *fat_g,
                })
            }
            MacroTargets::Percentages {
                carbs_pct,
                protein_pct,
                fat_pct,
            } => {
                let sum = carbs_pct + protein_pct + fat_pct;
                if sum < 99.0 || sum > 101.0 {
                    return Err(DomainError::InvalidMacroTargets(format!(
                        "macro percentages must sum to 100, got {sum}"
                    )));
                }
                // 1g carbs = 4 kcal, protein = 4 kcal, fat = 9 kcal
                Ok(ResolvedMacroTargets {
                    carbs_g: (target_calories * carbs_pct / 100.0) / 4.0,
                    protein_g: (target_calories * protein_pct / 100.0) / 4.0,
                    fat_g: (target_calories * fat_pct / 100.0) / 9.0,
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserConstraints {
    pub target_calories: f64,
    pub macro_targets: MacroTargets,
    /// Tags to exclude (allergies, vegan requirement = exclude non-vegan tags, etc.).
    pub dietary_exclusions: Vec<String>,
    pub min_meals: usize,
    pub max_meals: usize,
}

impl UserConstraints {
    pub const DEFAULT_MIN_MEALS: usize = 3;
    pub const DEFAULT_MAX_MEALS: usize = 5;

    pub fn resolved_macros(&self) -> Result<ResolvedMacroTargets, DomainError> {
        if self.target_calories <= 0.0 {
            return Err(DomainError::InvalidMacroTargets(
                "target calories must be positive".into(),
            ));
        }
        self.macro_targets.resolve(self.target_calories)
    }

    pub fn validate_meal_bounds(&self) -> Result<(), DomainError> {
        if self.min_meals < 1 || self.max_meals < self.min_meals {
            return Err(DomainError::InvalidMealCount {
                min: Self::DEFAULT_MIN_MEALS,
                max: Self::DEFAULT_MAX_MEALS,
                actual: self.min_meals,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_percentages_to_grams() {
        let targets = MacroTargets::Percentages {
            carbs_pct: 50.0,
            protein_pct: 25.0,
            fat_pct: 25.0,
        };
        let resolved = targets.resolve(2000.0).unwrap();
        assert!((resolved.carbs_g - 250.0).abs() < 0.01);
        assert!((resolved.protein_g - 125.0).abs() < 0.01);
        assert!((resolved.fat_g - 55.56).abs() < 0.1);
    }

    #[test]
    fn rejects_invalid_percentage_sum() {
        let targets = MacroTargets::Percentages {
            carbs_pct: 40.0,
            protein_pct: 40.0,
            fat_pct: 40.0,
        };
        assert!(targets.resolve(2000.0).is_err());
    }
}
