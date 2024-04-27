use indexmap::IndexMap;

use serde::{Deserialize, Serialize};

use xnb::{xnb_name, XnbType};

use crate::common::Season;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.PassiveFestivalData")]
pub struct PassiveFestivalData {
    #[serde(skip)]
    pub id: String,

    pub display_name: String,
    pub condition: String,
    pub show_on_calendar: bool,
    pub season: Season,
    pub start_day: i32,
    pub end_day: i32,
    pub start_time: i32,
    pub start_message: String,
    pub only_show_message_on_first_day: bool,
    pub map_replacements: Option<IndexMap<String, String>>,
    pub daily_setup_method: Option<String>,
    pub daily_cleanup_method: Option<String>,
    pub custom_fields: Option<IndexMap<String, String>>,
}
