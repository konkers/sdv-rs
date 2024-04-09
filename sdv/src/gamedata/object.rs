use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;

use serde::Deserialize;

use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};
use xnb::{xnb_name, XnbType};

use crate::common::{GenericSpawnItemDataWithCondition, ObjectCategory, ObjectId, ObjectType};

use super::Locale;

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Objects.ObjectGeodeDropData")]
pub struct ObjectGeodeDropData {
    #[serde(flatten)]
    pub parent: GenericSpawnItemDataWithCondition,

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
    #[serde(skip)]
    pub id: String,
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

impl ObjectData {
    pub fn display_name<'a>(&'a self, locale: &'a Locale) -> &'a str {
        // First check if there's a collections tab name for this object.
        // That will turn items like "Dried" into "Dried Mushrooms".
        if let Some(name) = locale.strings.get(&format!(
            "[LocalizedText Strings\\Objects:{}_CollectionsTabName]",
            &self.id
        )) {
            return name;
        }

        // If collections tab name is found, then look up it's display name.
        if let Some(name) = locale.strings.get(&self.display_name) {
            return name;
        }

        // If no locale name was found, return the raw object name._
        &self.name
    }

    pub fn is_potential_basic_shipped(&self) -> bool {
        if ObjectId::CoffeeBean == self.id {
            return false;
        }

        if [
            ObjectType::Arch,
            ObjectType::Fish,
            ObjectType::Minerals,
            ObjectType::Cooking,
        ]
        .contains(&self.ty)
        {
            return false;
        }

        if [
            ObjectCategory::Litter,
            ObjectCategory::SkillBooks,
            ObjectCategory::Books,
            ObjectCategory::Ring,
            ObjectCategory::Seed,
            ObjectCategory::Equipment,
            ObjectCategory::Furniture,
            ObjectCategory::Tackle,
            ObjectCategory::Bait,
            ObjectCategory::Junk,
            ObjectCategory::Fertilizer,
            ObjectCategory::Meat,
            ObjectCategory::Mineral,
            ObjectCategory::Crafting,
            ObjectCategory::Cooking,
            ObjectCategory::Gem,
            ObjectCategory::None,
        ]
        .contains(&self.category)
        {
            return false;
        }

        !self.exclude_from_fishing_collection
    }
}

pub fn load_objects<P: AsRef<Path>>(file: P) -> Result<IndexMap<String, ObjectData>> {
    let file = file.as_ref();
    let f = File::open(file).context(anyhow!("Can't open object file {}", file.display()))?;
    let mut r = BufReader::new(f);
    let mut data: Vec<u8> = Vec::new();

    r.read_to_end(&mut data)?;
    let mut objects: IndexMap<String, ObjectData> = xnb::from_bytes(&data)?;

    // Populate Object ID
    objects
        .iter_mut()
        .for_each(|(id, object)| object.id = id.clone());

    Ok(objects)
}

#[cfg(test)]
mod tests {}
