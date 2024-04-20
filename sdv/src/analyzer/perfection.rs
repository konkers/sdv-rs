use std::collections::HashMap;
use std::hash::Hash;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::common::Point;
use crate::gamedata::Recipe;
use crate::save::Object;
use crate::{GameData, SaveGame};

// TODO move to common place
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(tag = "type")]
pub enum ItemLocation {
    Player,
    Map { name: String, point: Point<i32> },
    Chest { map: String, point: Point<i32> },
}

impl std::fmt::Display for ItemLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Player => write!(f, "Player"),
            Self::Map { name, point: loc } => write!(f, "{}: {}, {}", name, loc.x, loc.y),
            Self::Chest {
                map: name,
                point: loc,
            } => write!(f, "{} Chest: {}, {}", name, loc.x, loc.y),
        }
    }
}

#[derive(Debug)]
struct Item<'a> {
    object: &'a Object,
    location: ItemLocation,
}

fn get_all_items(save: &SaveGame, include_map_items: bool) -> Vec<Item> {
    let mut items = Vec::new();

    for item in &save.player.items {
        items.push(Item {
            object: item,
            location: ItemLocation::Player,
        });
    }

    for (name, location) in &save.locations {
        for (pos, object) in &location.objects {
            if let Some(chest_items) = &object.items {
                for item in chest_items {
                    items.push(Item {
                        object: item,
                        location: ItemLocation::Chest {
                            map: name.clone(),
                            point: *pos,
                        },
                    });
                }
            } else if include_map_items {
                items.push(Item {
                    object,
                    location: ItemLocation::Map {
                        name: name.clone(),
                        point: *pos,
                    },
                });
            }
        }
    }

    items
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
pub struct ItemQuantityAndLocations {
    pub quantity: usize,
    pub locations: Vec<ItemLocation>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
pub struct ItemInfo {
    pub id: String,
    pub normal: ItemQuantityAndLocations,
    pub iron: ItemQuantityAndLocations,
    pub gold: ItemQuantityAndLocations,
    pub irridium: ItemQuantityAndLocations,
}

impl ItemInfo {
    pub fn quantity(&self) -> usize {
        self.normal.quantity + self.iron.quantity + self.gold.quantity + self.irridium.quantity
    }
}

fn aggregate_items(items: Vec<Item>) -> Result<HashMap<String, ItemInfo>> {
    items.iter().try_fold(HashMap::new(), |mut acc, item| {
        let info: &mut ItemInfo = acc.entry(item.object.id.clone()).or_default();
        let quantity_and_locations = match item.object.quality {
            Some(1) => &mut info.iron,
            Some(2) => &mut info.gold,
            Some(4) => &mut info.irridium,
            _ => &mut info.normal,
        };
        quantity_and_locations.quantity += item.object.stack as usize;
        quantity_and_locations.locations.push(item.location.clone());
        info.id = item.object.id.clone();
        Ok(acc)
    })
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
pub struct GoalItem {
    pub id: String,
    pub completed: bool,
    pub locations: Option<ItemInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
pub struct GoalRecipe {
    pub name: String,
    pub learned: bool,
    pub completed: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
pub struct NeededItem {
    pub id: String,
    pub needed: usize,
    pub total_on_hand: usize,
    pub on_hand: Option<ItemInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
pub struct PerfectionAnalysis {
    pub basic_shipped: Vec<GoalItem>,
    pub cooking_recipes: Vec<GoalRecipe>,
    pub needed_items: Vec<NeededItem>,
}

fn add_recipe_ingredients(
    craftable_items: &HashMap<String, Recipe>,
    needed_items: &mut HashMap<String, usize>,
    recipe: &Recipe,
    quantity: usize,
) {
    for ingredient in &recipe.ingredients {
        let id = ingredient.item.id();
        let qualified_id = format!("(O){id}");
        if let Some(recipe) = craftable_items.get(&qualified_id) {
            add_recipe_ingredients(
                craftable_items,
                needed_items,
                recipe,
                quantity * (ingredient.quantity as usize),
            );
        } else {
            *needed_items.entry(id.clone()).or_default() +=
                quantity * (ingredient.quantity as usize);
        }
    }
}

pub fn analyze_perfection(game_data: &GameData, save: &SaveGame) -> Result<PerfectionAnalysis> {
    let items = get_all_items(save, false);
    let aggregate_items = aggregate_items(items)?;

    let mut needed_items: HashMap<String, usize> = HashMap::new();

    let basic_shipped: Vec<_> = game_data
        .objects
        .iter()
        .filter(|(_, o)| o.is_potential_basic_shipped())
        .map(|(_, o)| {
            let completed = save.player.basic_shipped.get(&o.id).map_or(0, |num| *num) > 0;
            let locations = aggregate_items.get(&o.id).cloned();

            if !completed {
                needed_items.insert(o.id.clone(), 1);
            }

            GoalItem {
                id: o.id.clone(),
                completed,
                locations,
            }
        })
        .collect();

    let craftable_items = game_data
        .cooking_recipies
        .iter()
        .chain(game_data.crafting_recipies.iter())
        .fold(HashMap::new(), |mut acc, (_, recipe)| {
            acc.insert(recipe.yield_item.clone(), recipe.clone());
            acc
        });

    let cooking_recipes: Vec<_> = game_data
        .cooking_recipies
        .iter()
        .map(|(_, o)| {
            let name = o.name.clone();
            let learned = save.player.cooking_recipes.contains_key(&o.name);
            let id = o.yield_item.strip_prefix("(O)").unwrap_or(&o.yield_item);
            let completed = save.player.recipes_cooked.contains_key(id);

            GoalRecipe {
                name,
                learned,
                completed,
            }
        })
        .collect();

    let crafting_recipes: Vec<_> = game_data
        .crafting_recipies
        .iter()
        .map(|(_, o)| {
            let name = o.name.clone();
            let (learned, completed) = save
                .player
                .crafting_recipes
                .get(&o.name)
                .map(|n| (true, *n > 0))
                .unwrap_or((false, false));
            GoalRecipe {
                name,
                learned,
                completed,
            }
        })
        .collect();

    for recipe in &cooking_recipes {
        if !recipe.completed {
            add_recipe_ingredients(
                &craftable_items,
                &mut needed_items,
                game_data
                    .cooking_recipies
                    .get(&recipe.name)
                    .ok_or_else(|| anyhow!("can't get cooking recipe '{}'", recipe.name))?,
                1,
            );
        }
    }

    for recipe in &crafting_recipes {
        if !recipe.completed {
            add_recipe_ingredients(
                &craftable_items,
                &mut needed_items,
                game_data
                    .crafting_recipies
                    .get(&recipe.name)
                    .ok_or_else(|| anyhow!("can't get crafting recipe '{}'", recipe.name))?,
                1,
            );
        }
    }

    let needed_items = needed_items
        .into_iter()
        .map(|(id, quantity)| {
            let on_hand = aggregate_items.get(&id).cloned();
            let total_on_hand = on_hand.as_ref().map(|v| v.quantity()).unwrap_or(0);
            NeededItem {
                id,
                needed: quantity,
                total_on_hand,
                on_hand,
            }
        })
        .filter(|needed| needed.total_on_hand < needed.needed)
        .collect();

    Ok(PerfectionAnalysis {
        basic_shipped,
        cooking_recipes,
        needed_items,
    })
}
