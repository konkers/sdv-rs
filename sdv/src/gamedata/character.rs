
use indexmap::IndexMap;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use xnb::{xnb_name, XnbType};

use crate::common::{GenericSpawnItemDataWithCondition, Season, XnaPoint, XnaRectangle};

#[derive(Clone, Debug, Deserialize_repr, PartialEq, Serialize_repr, XnbType)]
#[repr(i32)]
pub enum NpcLanguage {
    Default,
    Dwarvish,
}

#[derive(Clone, Debug, Deserialize_repr, PartialEq, Serialize_repr, XnbType)]
#[repr(i32)]
pub enum Gender {
    Male,
    Female,
    Undefined,
}

#[derive(Clone, Debug, Deserialize_repr, PartialEq, Serialize_repr, XnbType)]
#[repr(i32)]
pub enum NpcAge {
    Adult,
    Teen,
    Child,
}

#[derive(Clone, Debug, Deserialize_repr, PartialEq, Serialize_repr, XnbType)]
#[repr(i32)]
pub enum NpcManner {
    Neutral,
    Polite,
    Rude,
}

#[derive(Clone, Debug, Deserialize_repr, PartialEq, Serialize_repr, XnbType)]
#[repr(i32)]
pub enum NpcSocialAnxiety {
    Outgoing,
    Shy,
    Neutral,
}

#[derive(Clone, Debug, Deserialize_repr, PartialEq, Serialize_repr, XnbType)]
#[repr(i32)]
pub enum NpcOptimism {
    Positive,
    Negative,
    Neutral,
}

#[derive(Clone, Debug, Deserialize_repr, PartialEq, Serialize_repr, XnbType)]
#[repr(i32)]
pub enum CalendarBehavior {
    AlwaysShown,
    HiddenUntilMet,
    HiddenAlways,
}

#[derive(Clone, Debug, Deserialize_repr, PartialEq, Serialize_repr, XnbType)]
#[repr(i32)]
pub enum SocialTabBehavior {
    UnknownUntilMet,
    AlwaysShown,
    HiddenUntilMet,
    HiddenAlways,
}

#[derive(Clone, Debug, Deserialize_repr, PartialEq, Serialize_repr, XnbType)]
#[repr(i32)]
pub enum EndSlideShowBehavior {
    Hidden,
    MainGroup,
    TrailingGroup,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.Characters.CharacterSpouseRoomData")]
pub struct CharacterSpouseRoomData {
    pub map_asset: Option<String>,
    pub map_source_rect: XnaRectangle,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.Characters.CharacterSpousePatioData")]
pub struct CharacterSpousePatioData {
    pub map_asset: Option<String>,
    pub map_source_rect: XnaRectangle,
    pub sprite_animation_frames: Vec<Vec<i32>>,
    pub sprite_animation_pixel_offset: XnaPoint,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.Characters.CharacterHomeData")]
pub struct CharacterHomeData {
    pub id: String,
    pub condition: Option<String>,
    pub location: String,
    pub tile: XnaPoint,
    pub direction: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.Characters.CharacterAppearanceData")]
pub struct CharacterAppearanceData {
    pub id: String,
    pub condition: Option<String>,
    pub season: Option<Season>,
    pub indoors: bool,
    pub outdoors: bool,
    pub portrait: Option<String>,
    pub sprite: String,
    pub is_island_attire: bool,
    pub precedence: i32,
    pub weight: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.Characters.CharacterShadowData")]
pub struct CharacterShadowData {
    pub visible: bool,
    pub offset: XnaPoint,
    pub scale: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.Characters.CharacterData")]
pub struct CharacterData {
    pub display_name: String,
    pub birth_season: Option<Season>,
    pub birthday: i32,
    pub home_region: String,
    pub language: NpcLanguage,
    pub gender: Gender,
    pub age: NpcAge,
    pub manner: NpcManner,
    pub social_anxiety: NpcSocialAnxiety,
    pub optimism: NpcOptimism,
    pub is_dark_skinned: bool,
    pub can_be_romanced: bool,
    pub love_intrest: Option<String>,
    pub calendar: CalendarBehavior,
    pub social_tab: SocialTabBehavior,
    pub can_socialize: Option<String>,
    pub can_receive_gifts: bool,
    pub can_greet_nearby_characters: bool,
    pub can_comment_on_purchased_shop_items: Option<bool>,
    pub can_visit_island: Option<String>,
    pub introductions_quest: Option<bool>,
    pub item_delivery_quests: Option<String>,
    pub perfection_score: bool,
    pub end_slide_show: EndSlideShowBehavior,
    pub spouse_adopts: Option<String>,
    pub spouse_wants_childern: Option<String>,
    pub spouse_gift_jealousy: Option<String>,
    pub spouse_gift_jealousy_friendship_change: i32,
    pub spouse_room: Option<CharacterSpouseRoomData>,
    pub spouse_patio: Option<CharacterSpousePatioData>,
    pub spouse_floors: Vec<String>,
    pub spouse_wallpapers: Vec<String>,
    pub dumpster_dive_friendship_effect: i32,
    pub dumpster_dive_emote: Option<i32>,
    pub friends_and_family: IndexMap<String, String>,
    pub flower_dance_can_dance: Option<bool>,
    pub winter_star_gifts: Vec<GenericSpawnItemDataWithCondition>,
    pub winter_star_participant: Option<String>,
    pub unlock_conditions: Option<String>,
    pub spawn_if_missing: bool,
    pub home: Option<Vec<CharacterHomeData>>,
    pub texture_name: Option<String>,
    pub appearance: Vec<CharacterAppearanceData>,
    pub mug_shot_source_rect: Option<XnaRectangle>,
    pub size: XnaPoint,
    pub breather: bool,
    pub breath_chest_rect: Option<XnaRectangle>,
    pub breath_chest_position: Option<XnaPoint>,
    pub shadow: Option<CharacterShadowData>,
    pub emote_offset: XnaPoint,
    pub shake_portraits: Vec<i32>,
    pub kiss_sprite_index: i32,
    pub kiss_sprite_facing_right: bool,
    pub hidden_profile_emote_sound: Option<String>,
    pub hidden_profile_emote_duration: i32,
    pub hidden_profile_emote_start_frame: i32,
    pub hidden_profile_emote_start_count: i32,
    pub hidden_profile_emote_frame_duration: f32,
    pub former_character_names: Option<Vec<String>>,
    pub festival_vanilla_actor_index: i32,
    pub custom_fields: Option<IndexMap<String, String>>,
}
