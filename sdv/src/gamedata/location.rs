use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;

use serde::Deserialize;
use serde_repr::Deserialize_repr;

use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};
use xnb::{xnb_name, XnbType};

use crate::common::{
    GenericSpawnItemDataWithCondition, QuantityModifier, QuantityModifierMode, Season, XnaPoint,
    XnaRectangle,
};

#[derive(Clone, Debug, Deserialize_repr, PartialEq, XnbType)]
#[repr(i32)]
pub enum MusicContext {
    Default,
    SubLocation,
    MusicPlayer,
    Event,
    MiniGame,
    ImportantSplitScreenMusic,
    Max,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Locations.LocationMusicData")]
pub struct LocationMusicData {
    id: String,
    track: String,
    condition: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Locations.SpawnForageData")]
pub struct SpawnForageData {
    #[serde(flatten)]
    pub parent: GenericSpawnItemDataWithCondition,

    pub chance: f64,
    pub season: Option<Season>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Locations.SpawnFishData")]
pub struct SpawnFishData {
    #[serde(flatten)]
    pub parent: GenericSpawnItemDataWithCondition,

    pub chance: f32,
    pub season: Option<Season>,
    pub fish_area_id: Option<String>,
    pub bobber_position: Option<XnaRectangle>,
    pub player_position: Option<XnaRectangle>,
    pub min_fishing_level: i32,
    pub min_distance_from_shore: i32,
    pub max_distance_from_shore: i32,
    pub apply_daily_luck: bool,
    pub curiousity_lure_buff: f32,
    pub catch_limit: i32,
    pub is_boss_fish: bool,
    pub set_flag_on_catch: Option<String>,
    pub require_magic_bait: bool,
    pub precedence: i32,
    pub ignore_fish_data_requirements: bool,
    pub can_be_inherited: bool,
    pub chance_modifiers: Option<Vec<QuantityModifier>>,
    pub chance_modifier_mode: QuantityModifierMode,
    pub chance_boost_per_luck_level: f32,
    pub use_fish_caught_seeded_random: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Locations.FishAreaData")]
pub struct FishAreaData {
    pub display_name: Option<String>,
    pub position: Option<XnaRectangle>,
    pub crab_pot_fish_types: Vec<String>,
    pub crab_pot_junk_chance: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Locations.ArtifactSpotDropData")]
pub struct ArtifactSpotDropData {
    #[serde(flatten)]
    pub parent: GenericSpawnItemDataWithCondition,

    pub chance: f64,
    pub apply_generous_enchantment: bool,
    pub one_debris_per_drop: bool,
    pub precedence: i32,
    pub continue_on_drop: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Locations.CreateLocationData")]
pub struct CreateLocationData {
    pub map_path: String,
    pub ty: Option<String>,
    pub always_active: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Locations.LocationData")]
pub struct LocationData {
    pub display_name: Option<String>,
    pub default_arrival_tile: Option<XnaPoint>,
    pub exclude_from_npc_pathfinding: bool,
    pub create_on_load: Option<CreateLocationData>,
    pub former_location_names: Option<Vec<String>>,
    pub can_plant_here: Option<bool>,
    pub can_have_green_rain_spawns: bool,
    pub artifact_spots: Option<Vec<ArtifactSpotDropData>>,
    pub fish_areas: Option<IndexMap<String, FishAreaData>>,
    pub fish: Option<Vec<SpawnFishData>>,
    pub forage: Vec<SpawnForageData>,
    pub min_daily_weeds: i32,
    pub max_daily_weeds: i32,
    pub first_daily_weed_multiplier: i32,
    pub min_daily_forage_spawn: i32,
    pub max_daily_forage_spawn: i32,
    pub max_spawned_forage_at_once: i32,
    pub chance_for_clay: f64,
    pub music: Vec<LocationMusicData>,
    pub music_default: Option<String>,
    pub music_context: MusicContext,
    pub music_ignored_in_rain: bool,
    pub music_ignored_in_spring: bool,
    pub music_ignored_in_summer: bool,
    pub music_ignored_in_fall: bool,
    pub music_ignored_in_fall_debris: bool,
    pub music_ignored_in_winter: bool,
    pub music_ignored_is_town_theme: bool,
    pub custom_fields: Option<IndexMap<String, String>>,
}

pub fn load_locations<P: AsRef<Path>>(file: P) -> Result<IndexMap<String, LocationData>> {
    let file = file.as_ref();
    let f = File::open(file).context(anyhow!("Can't open location file {}", file.display()))?;
    let mut r = BufReader::new(f);
    let mut data: Vec<u8> = Vec::new();

    r.read_to_end(&mut data)?;
    xnb::from_bytes(&data)
}
