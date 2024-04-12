use indexmap::IndexMap;

use serde::{Deserialize, Serialize};

use xnb::{xnb_name, XnbType};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.BigCraftables.BigCraftableData")]
pub struct BigCraftableData {
    #[serde(skip)]
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub price: i32,
    pub fragility: i32,
    pub can_be_placed_outdoors: bool,
    pub can_be_placed_indoors: bool,
    pub is_lamp: bool,
    pub texture: Option<String>,
    pub sprite_index: i32,
    pub context_tags: Option<Vec<String>>,
    pub custom_fields: Option<IndexMap<String, String>>,
}
