use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::common::Point;
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
pub struct PerfectionAnalysis {
    pub basic_shipped: Vec<GoalItem>,
}

pub fn analyze_perfection(game_data: &GameData, save: &SaveGame) -> Result<PerfectionAnalysis> {
    let items = get_all_items(save, false);
    let aggregate_items = aggregate_items(items)?;
    println!("{aggregate_items:#?}");

    let basic_shipped: Vec<_> = game_data
        .objects
        .iter()
        .filter(|(_, o)| o.is_potential_basic_shipped())
        .map(|(_, o)| {
            let completed = save.player.basic_shipped.get(&o.id).map_or(0, |num| *num) > 0;
            let locations = aggregate_items.get(&o.id).cloned();
            GoalItem {
                id: o.id.clone(),
                completed,
                locations,
            }
        })
        .collect();

    Ok(PerfectionAnalysis { basic_shipped })
}
