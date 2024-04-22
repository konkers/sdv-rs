use indexmap::IndexMap;

use serde::{Deserialize, Serialize};

use xnb::{xnb_name, XnbType};

use crate::common::GenericSpawnItemDataWithCondition;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.GarbageCans.GarbageCanItemData")]
pub struct GarbageCanItemData {
    #[serde(flatten)]
    pub parent: GenericSpawnItemDataWithCondition,

    pub ignore_base_chance: bool,
    pub is_mega_success: bool,
    pub is_double_mega_success: bool,
    pub add_to_inventory_directly: bool,
    pub create_multiple_debris: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.GarbageCans.GarbageCanEntryData")]
pub struct GarbageCanEntryData {
    pub base_chance: f32,
    pub items: Vec<GarbageCanItemData>,
    pub custom_fields: Option<IndexMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.GarbageCans.GarbageCanData")]
pub struct GarbageCanData {
    pub default_base_chance: f32, // is this actually read?
    pub before_all: Vec<GarbageCanItemData>,
    pub after_all: Vec<GarbageCanItemData>,
    pub garbage_cans: IndexMap<String, GarbageCanEntryData>,
}
