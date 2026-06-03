use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NutritionalProfile {
    pub calories: f64,
    pub carbs_g: f64,
    pub protein_g: f64,
    pub fat_g: f64,
}

impl NutritionalProfile {
    pub fn sum(profiles: impl IntoIterator<Item = NutritionalProfile>) -> Self {
        profiles
            .into_iter()
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
            )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub nutrition: NutritionalProfile,
    /// Dietary tags such as "vegan", "gluten-free", "dairy".
    pub tags: Vec<String>,
}

impl Recipe {
    pub fn has_excluded_tag(&self, exclusions: &[String]) -> bool {
        exclusions.iter().any(|ex| {
            self.tags
                .iter()
                .any(|tag| tag.eq_ignore_ascii_case(ex))
        })
    }
}
