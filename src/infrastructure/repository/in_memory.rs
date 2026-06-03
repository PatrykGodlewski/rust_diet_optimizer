use crate::application::RecipeRepository;
use crate::domain::{DomainError, NutritionalProfile, Recipe};

pub struct InMemoryRecipeRepository {
    recipes: Vec<Recipe>,
}

impl InMemoryRecipeRepository {
    pub fn new(recipes: Vec<Recipe>) -> Self {
        Self { recipes }
    }

    pub fn with_defaults() -> Self {
        Self::new(default_recipe_catalog())
    }
}

impl RecipeRepository for InMemoryRecipeRepository {
    fn all_recipes(&self) -> &[Recipe] {
        &self.recipes
    }

    fn filter_eligible(&self, exclusions: &[String]) -> Result<Vec<Recipe>, DomainError> {
        let eligible: Vec<Recipe> = self
            .recipes
            .iter()
            .filter(|r| !r.has_excluded_tag(exclusions))
            .cloned()
            .collect();

        if eligible.is_empty() {
            return Err(DomainError::NoEligibleRecipes);
        }

        Ok(eligible)
    }
}

fn default_recipe_catalog() -> Vec<Recipe> {
    vec![
        Recipe {
            id: "oatmeal-berry".into(),
            name: "Berry Oatmeal".into(),
            nutrition: NutritionalProfile {
                calories: 380.0,
                carbs_g: 58.0,
                protein_g: 14.0,
                fat_g: 9.0,
            },
            tags: vec![
                "vegan".into(),
                "gluten-free".into(),
                "breakfast".into(),
            ],
        },
        Recipe {
            id: "chicken-rice".into(),
            name: "Grilled Chicken with Rice".into(),
            nutrition: NutritionalProfile {
                calories: 520.0,
                carbs_g: 45.0,
                protein_g: 48.0,
                fat_g: 14.0,
            },
            tags: vec!["gluten-free".into(), "lunch".into()],
        },
        Recipe {
            id: "salmon-quinoa".into(),
            name: "Salmon Quinoa Bowl".into(),
            nutrition: NutritionalProfile {
                calories: 580.0,
                carbs_g: 42.0,
                protein_g: 40.0,
                fat_g: 24.0,
            },
            tags: vec![
                "gluten-free".into(),
                "dairy-free".into(),
                "dinner".into(),
            ],
        },
        Recipe {
            id: "greek-salad".into(),
            name: "Greek Salad".into(),
            nutrition: NutritionalProfile {
                calories: 320.0,
                carbs_g: 18.0,
                protein_g: 12.0,
                fat_g: 24.0,
            },
            tags: vec!["vegetarian".into(), "lunch".into(), "dairy".into()],
        },
        Recipe {
            id: "tofu-stirfry".into(),
            name: "Tofu Vegetable Stir Fry".into(),
            nutrition: NutritionalProfile {
                calories: 410.0,
                carbs_g: 48.0,
                protein_g: 22.0,
                fat_g: 14.0,
            },
            tags: vec!["vegan".into(), "dinner".into()],
        },
        Recipe {
            id: "turkey-wrap".into(),
            name: "Turkey Avocado Wrap".into(),
            nutrition: NutritionalProfile {
                calories: 460.0,
                carbs_g: 38.0,
                protein_g: 32.0,
                fat_g: 18.0,
            },
            tags: vec!["lunch".into()],
        },
        Recipe {
            id: "egg-scramble".into(),
            name: "Vegetable Egg Scramble".into(),
            nutrition: NutritionalProfile {
                calories: 340.0,
                carbs_g: 8.0,
                protein_g: 24.0,
                fat_g: 22.0,
            },
            tags: vec!["vegetarian".into(), "breakfast".into(), "dairy".into()],
        },
        Recipe {
            id: "lentil-soup".into(),
            name: "Lentil Soup".into(),
            nutrition: NutritionalProfile {
                calories: 290.0,
                carbs_g: 42.0,
                protein_g: 18.0,
                fat_g: 6.0,
            },
            tags: vec!["vegan".into(), "gluten-free".into(), "lunch".into()],
        },
        Recipe {
            id: "beef-stew".into(),
            name: "Lean Beef Stew".into(),
            nutrition: NutritionalProfile {
                calories: 490.0,
                carbs_g: 28.0,
                protein_g: 42.0,
                fat_g: 20.0,
            },
            tags: vec!["dinner".into()],
        },
        Recipe {
            id: "cottage-cheese".into(),
            name: "Cottage Cheese with Fruit".into(),
            nutrition: NutritionalProfile {
                calories: 260.0,
                carbs_g: 22.0,
                protein_g: 28.0,
                fat_g: 6.0,
            },
            tags: vec!["vegetarian".into(), "snack".into(), "dairy".into()],
        },
    ]
}
