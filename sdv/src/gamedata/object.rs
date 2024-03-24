use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;

use serde::Deserialize;

use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};
use xnb::{xnb_name, XnbType};

use crate::common::{ObjectCategory, ObjectType, QuantityModifier, QuantityModifierMode};

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Objects.ObjectGeodeDropData")]
pub struct ObjectGeodeDropData {
    // Fields from GenericSpawnItemData
    pub id: String,
    pub item_id: Option<String>,
    pub random_item_id: Option<Vec<String>>,
    pub max_items: Option<i32>,
    pub min_stack: i32,
    pub max_stack: i32,
    pub quality: i32,
    pub internal_name: Option<String>,
    pub display_name: Option<String>,
    pub tool_upgrade_level: i32,
    pub is_recipe: bool,
    pub stack_modifiers: Option<Vec<QuantityModifier>>,
    pub stack_modifier_mode: QuantityModifierMode,
    pub quality_modifiers: Option<Vec<QuantityModifier>>,
    pub quality_modifier_mode: QuantityModifierMode,
    pub mod_data: Option<IndexMap<String, String>>,
    pub per_item_condition: Option<String>,

    // Fields from GenericSpawnItemDataWithCondition
    pub condition: Option<String>,

    // Fields from ObjectGeodeDropData
    pub chance: f64,
    pub set_flag_on_pickup: Option<String>,
    pub precedence: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Buffs.BuffAttributesData")]
pub struct BuffAttributesData {
    pub farming_level: f32,
    pub fishing_level: f32,
    pub mining_level: f32,
    pub luck_level: f32,
    pub foraging_level: f32,
    pub max_stamina: f32,
    pub magnetic_radius: f32,
    pub speed: f32,
    pub defense: f32,
    pub attack: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Objects.ObjectBuffData")]
pub struct ObjectBuffData {
    pub id: String,
    pub buff_id: Option<String>,
    pub icon_texture: Option<String>,
    pub icon_sprite_index: i32,
    pub duration: i32,
    pub is_debuf: bool,
    pub glow_color: Option<String>,
    pub custom_attributes: Option<BuffAttributesData>,
    pub custom_fields: Option<IndexMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Objects.ObjectData")]
pub struct ObjectData {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub ty: ObjectType,
    pub category: ObjectCategory,
    pub price: i32,
    pub texture: Option<String>,
    pub sprite_index: i32,
    pub edibility: i32,
    pub is_drink: bool,
    pub buffs: Option<Vec<ObjectBuffData>>,
    pub geode_drops_default_items: bool,
    pub geode_drops: Option<Vec<ObjectGeodeDropData>>,
    pub artifact_spot_chances: Option<IndexMap<String, f32>>,
    pub exclude_from_fishing_collection: bool,
    pub exclude_from_shipping_collection: bool,
    pub exclude_from_random_sale: bool,
    pub context_tags: Option<Vec<String>>,
    pub custom_fields: Option<IndexMap<String, String>>,
}

pub fn load_objects<P: AsRef<Path>>(file: P) -> Result<IndexMap<String, ObjectData>> {
    let file = file.as_ref();
    let f = File::open(file).context(anyhow!("Can't open object file {}", file.display()))?;
    let mut r = BufReader::new(f);
    let mut data: Vec<u8> = Vec::new();

    r.read_to_end(&mut data)?;
    xnb::from_bytes(&data)
}

#[cfg(test)]
mod tests {}
