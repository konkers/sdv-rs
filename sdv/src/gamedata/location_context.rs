use indexmap::IndexMap;

use serde::{Deserialize, Serialize};

use xnb::{xnb_name, XnbType};

use crate::common::{
    GenericSpawnItemDataWithCondition, ObjectCategory, ObjectId, ObjectType, Season, XnaPoint,
};

use super::location::LocationMusicData;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.LocationContexts.WeatherCondition")]
pub struct WeatherCondition {
    pub id: String,
    pub condition: Option<String>,
    pub weather: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.LocationContexts.ReviveLocation")]
pub struct ReviveLocation {
    pub id: String,
    pub condition: Option<String>,
    pub location: String,
    pub position: XnaPoint,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.LocationContexts.PassOutMailData")]
pub struct PassOutMailData {
    pub id: String,
    pub condition: Option<String>,
    pub mail: String,
    pub max_pass_out_cost: i32,
    pub skip_random_selection: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.LocationContexts.LocationContextData")]
pub struct LocationContextData {
    #[serde(skip)]
    pub id: String,
    pub season_override: Option<Season>,
    pub default_music: Option<String>,
    pub default_music_condition: Option<String>,
    pub default_music_delay_one_screen: bool,
    pub music: Vec<LocationMusicData>,
    pub day_ambience: Option<String>,
    pub night_ambience: Option<String>,
    pub play_random_ambient_sound: bool,
    pub allow_rain_totem: bool,
    pub rain_totem_affects_context: Option<String>,
    pub weather_condidtions: Vec<WeatherCondition>,
    pub copy_weather_from_location: bool,
    pub revive_locations: Vec<ReviveLocation>,
    pub max_pass_out_cost: i32,
    pub pass_out_mail: Option<Vec<PassOutMailData>>,
    pub pass_out_locations: Option<Vec<ReviveLocation>>,
    pub custom_fields: Option<IndexMap<String, String>>,
}
