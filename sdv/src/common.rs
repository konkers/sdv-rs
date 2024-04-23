use std::convert::{TryFrom, TryInto};
use std::fmt::Display;

use anyhow::{anyhow, Error, Result};
use indexmap::IndexMap;
use nom::{branch::alt, bytes::complete::tag, combinator::map_res, combinator::value, IResult};
use num_derive::FromPrimitive;
use num_traits::{FromPrimitive, Num};
use roxmltree::Node;
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use serde_repr::Serialize_repr;
use strum::EnumString;
use xnb::xnb_name;

use crate::gamedata::sub_field_value;
use crate::gamedata::{decimal, sub_field};
use crate::save::{Finder, NodeFinder, SaveError, SaveResult};

pub use sdv_core::ItemId;
pub use xnb::XnbType;

#[derive(Clone, Debug, Deserialize, EnumString, Eq, PartialEq, Serialize, XnbType)]
pub enum ItemType {
    BigCraftable,
    Boot,
    Flooring,
    Fruniture,
    Hat,
    Object,
    Mannequin,
    Pants,
    Shirt,
    Tool,
    Trinket,
    Wallpaper,
    Weapon,
}

impl ItemType {
    pub fn prefix(&self) -> &str {
        match self {
            ItemType::BigCraftable => "(BC)",
            ItemType::Boot => "(B)",
            ItemType::Flooring => "(FL)",
            ItemType::Fruniture => "(F)",
            ItemType::Hat => "(H)",
            ItemType::Mannequin => "(M)",
            ItemType::Object => "(O)",
            ItemType::Pants => "(P)",
            ItemType::Shirt => "(S)",
            ItemType::Tool => "(T)",
            ItemType::Trinket => "(TR)",
            ItemType::Wallpaper => "(WP)",
            ItemType::Weapon => "(W)",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, EnumString, Eq, PartialEq, Serialize, XnbType)]
#[strum(ascii_case_insensitive)]
pub enum ObjectType {
    #[default]
    Unknown,
    Arch,
    #[serde(rename = "asdf")]
    Asdf,
    Basic,
    Cooking,
    Crafting,
    Fish,
    #[serde(rename = "interactive")]
    Interactive,
    Minerals,
    Quest,
    Ring,
    Seeds,
    Litter,
}

#[derive(
    Clone,
    Copy,
    Default,
    Deserialize_repr,
    EnumString,
    Eq,
    Debug,
    FromPrimitive,
    Hash,
    PartialEq,
    Serialize_repr,
    XnbType,
)]
#[repr(i32)]
pub enum ObjectCategory {
    #[default]
    None = 0,
    Gem = -2,
    Fish = -4,
    Egg = -5,
    Milk = -6,
    Cooking = -7,
    Crafting = -8,
    BigCraftable = -9,
    Mineral = -12,
    Meat = -14,
    Metal = -15,
    Building = -16,
    SellAtPierres = -17,
    SellAtPierresAndMarines = -18,
    Fertilizer = -19,
    Junk = -20,
    Bait = -21,
    Tackle = -22,
    SellAtFishShop = -23,
    Furniture = -24,
    Artisan = -26,
    Syrup = -27,
    MonsterLoot = -28,
    Equipment = -29,
    Seed = -74,
    Vegitable = -75,
    Fruit = -79,
    Flower = -80,
    Green = -81,
    Hat = -95,
    Ring = -96,
    Boots = -97, // unsure
    Weapon = -98,
    Tool = -99,
    Clothing = -100, // unsure
    Trinket = -101,
    Books = -102,
    SkillBooks = -103,
    WildSeed = -777,
    Litter = -999,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ObjectOrCategory {
    Category(ObjectCategory),
    Item(String),
}

impl ObjectOrCategory {
    fn parse_category(i: &str) -> IResult<&str, Self> {
        let (i, category) = map_res(decimal, |val| {
            ObjectCategory::from_i32(val).ok_or_else(|| anyhow!("Invalid category {val}"))
        })(i)?;
        Ok((i, Self::Category(category)))
    }

    fn parse_item(i: &str) -> IResult<&str, Self> {
        let (i, item) = sub_field(i)?;
        Ok((i, Self::Item(item.to_string())))
    }

    pub fn parse(i: &str) -> IResult<&str, Self> {
        let (i, val) = alt((Self::parse_category, Self::parse_item))(i)?;
        Ok((i, val))
    }

    pub fn id(&self) -> String {
        match self {
            ObjectOrCategory::Category(category) => (*category as i32).to_string(),
            ObjectOrCategory::Item(id) => id.clone(),
        }
    }
}

// This should, perhaps, be moved to `xnb-rs`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, XnbType)]
#[xnb_name("Microsoft.Xna.Framework.Point")]
#[xnb(untagged)]
pub struct XnaPoint {
    pub x: i32,
    pub y: i32,
}

// This should, perhaps, be moved to `xnb-rs`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Serialize, PartialEq)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Serialize, PartialEq)]
pub struct Size<T> {
    pub h: T,
    pub w: T,
}

// impl From<xnb::value::map::Size> for Size<usize> {
//     fn from(size: xnb::value::map::Size) -> Self {
//         Self {
//             h: size.h as usize,
//             w: size.w as usize,
//         }
//     }
// }

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for Point<i32> {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        let node = &finder.node()?;
        let x = node.child("X").try_into()?;
        let y = node.child("Y").try_into()?;
        Ok(Point { x, y })
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for Point<f32> {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        let node = finder.node()?;
        let x = node.child("X").try_into()?;
        let y = node.child("Y").try_into()?;
        Ok(Point { x, y })
    }
}

// This should, perhaps, be moved to `xnb-rs`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, XnbType)]
#[xnb_name("Microsoft.Xna.Framework.Rectangle")]
#[xnb(untagged)]
pub struct XnaRectangle {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

enum BorderPhase {
    Top,
    Right,
    Bottom,
    Left,
    Done,
}

pub struct BorderIterator<'a, T: Num + From<i8> + Clone + Copy> {
    rect: &'a Rect<T>,
    phase: BorderPhase,
    next_point: Point<T>,
}

impl<'a, T: Num + From<i8> + Clone + Copy + std::cmp::PartialOrd> Iterator
    for BorderIterator<'a, T>
{
    type Item = Point<T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.phase {
                BorderPhase::Top => {
                    let point = self.next_point;

                    let mut next_point = self.next_point;
                    next_point.x = next_point.x + T::from(1i8);
                    if next_point.x == self.rect.p2.x {
                        self.phase = BorderPhase::Right;
                        continue;
                    }

                    self.next_point = next_point;
                    return Some(point);
                }
                BorderPhase::Right => {
                    let point = self.next_point;

                    let mut next_point = self.next_point;
                    next_point.y = next_point.y + T::from(1i8);
                    if next_point.y == self.rect.p2.y {
                        self.phase = BorderPhase::Bottom;
                        continue;
                    }

                    self.next_point = next_point;
                    return Some(point);
                }
                BorderPhase::Bottom => {
                    let point = self.next_point;

                    let mut next_point = self.next_point;
                    next_point.x = next_point.x - T::from(1i8);
                    if next_point.x < self.rect.p1.x {
                        self.phase = BorderPhase::Left;
                        continue;
                    }

                    self.next_point = next_point;
                    return Some(point);
                }
                BorderPhase::Left => {
                    let point = self.next_point;

                    let mut next_point = self.next_point;
                    next_point.y = next_point.y - T::from(1i8);
                    if next_point.y < self.rect.p1.y {
                        self.phase = BorderPhase::Done;
                        continue;
                    }

                    self.next_point = next_point;
                    return Some(point);
                }
                BorderPhase::Done => return None,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Serialize, PartialEq)]
pub struct Rect<T> {
    p1: Point<T>,
    p2: Point<T>,
}

impl<T: Num + Clone + Copy> Rect<T> {
    pub fn from_xywh(x: T, y: T, w: T, h: T) -> Self {
        let p1 = Point { x, y };
        let p2 = Point { x: x + w, y: y + h };
        Self { p1, p2 }
    }

    pub fn width(&self) -> T {
        self.p2.x - self.p1.x
    }

    pub fn inflate(&mut self, dx: T, dy: T) {
        self.p1.x = self.p1.x - dx;
        self.p1.y = self.p1.y - dy;
        self.p2.x = self.p2.x + dx;
        self.p2.y = self.p2.y + dy;
    }
}

impl<T: Num + Clone + From<i8> + Copy> Rect<T> {
    pub fn border_points(&self) -> BorderIterator<'_, T> {
        BorderIterator {
            rect: self,
            phase: BorderPhase::Top,
            next_point: self.p1,
        }
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for Rect<i32> {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        let node = finder.node()?;
        let x = node.child("X").try_into()?;
        let y = node.child("Y").try_into()?;
        let width: i32 = node.child("Width").try_into()?;
        let height: i32 = node.child("Height").try_into()?;
        Ok(Rect {
            p1: Point { x, y },
            p2: Point {
                x: x + width,
                y: y + height,
            },
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TimeSpan {
    pub start: i32,
    pub end: i32,
}

impl TimeSpan {
    pub(crate) fn parse(i: &str) -> IResult<&str, Self> {
        let (i, start) = sub_field_value(decimal)(i)?;
        let (i, end) = sub_field_value(decimal)(i)?;

        Ok((i, TimeSpan { start, end }))
    }

    fn fmt_time(f: &mut std::fmt::Formatter<'_>, time: i32) -> std::fmt::Result {
        let time = time % 2400;
        let (time, meridiem) = if time < 1200 {
            (time, "am")
        } else if time < 1300 {
            (time, "pm")
        } else {
            (time - 1200, "pm")
        };

        let hour = time / 100;
        let hour = if hour == 0 { 12 } else { hour };
        let min = time % 100;
        f.write_fmt(format_args!("{hour:02}:{min:02}{meridiem}"))
    }
}

impl Display for TimeSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Self::fmt_time(f, self.start)?;
        f.write_str("-")?;
        Self::fmt_time(f, self.end)
    }
}

#[derive(Clone, Debug, Deserialize_repr, PartialEq, Serialize_repr, XnbType, strum::Display)]
#[repr(i32)]
pub enum Season {
    Spring,
    Summer,
    Fall,
    Winter,
}

impl Season {
    pub(crate) fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            value(Season::Spring, tag("spring")),
            value(Season::Summer, tag("summer")),
            value(Season::Fall, tag("fall")),
            value(Season::Winter, tag("winter")),
        ))(i)
    }

    pub(crate) fn from_node<'a, 'input: 'a>(
        node: Node<'a, 'input>,
    ) -> SaveResult<'a, 'input, Self> {
        let text = &node.text().unwrap_or("");
        let (_, season) = Self::parse(text).map_err(|e| SaveError::Generic {
            message: format!("error parsing season {}: {}", text, e),
            node,
        })?;

        Ok(season)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, strum::Display)]
pub enum DayOfWeek {
    Sunday = 0,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl TryFrom<i32> for DayOfWeek {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self> {
        if !(1..=28).contains(&value) {
            return Err(anyhow!("invalid day of month {value}"));
        }

        let day = match (value as u32) % 7 {
            0 => Self::Sunday,
            1 => Self::Monday,
            2 => Self::Tuesday,
            3 => Self::Wednesday,
            4 => Self::Thursday,
            5 => Self::Friday,
            6 => Self::Saturday,
            _ => unreachable!(),
        };
        Ok(day)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Weather {
    Sunny,
    Rainy,
    Both,
}

impl Weather {
    pub(crate) fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            value(Weather::Sunny, tag("sunny")),
            value(Weather::Rainy, tag("rainy")),
            value(Weather::Both, tag("both")),
        ))(i)
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for ObjectCategory {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        let node = finder.node()?;
        let id: i32 = node.finder().try_into()?;
        Self::from_i32(id).ok_or(SaveError::Generic {
            message: format!("unknown ObjectCategory {}", id),
            node,
        })
    }
}

#[derive(Clone, Debug, Deserialize_repr, FromPrimitive, PartialEq, Serialize_repr, XnbType)]
#[repr(i32)]
pub enum ModificationType {
    Add = 0,
    Subtract,
    Multiply,
    Divide,
    Set,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.QuantityModifier")]
pub struct QuantityModifier {
    pub id: String,
    pub condition: String,
    pub modification: ModificationType,
    pub amount: f32,
    pub random_amount: Option<Vec<f32>>,
}

#[derive(Clone, Debug, Deserialize_repr, FromPrimitive, PartialEq, Serialize_repr, XnbType)]
#[repr(i32)]
pub enum QuantityModifierMode {
    Stack = 0,
    Minimum,
    Maximum,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.GenericSpawnItemData")]
pub struct GenericSpawnItemData {
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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.GenericSpawnItemDataWithCondition")]
pub struct GenericSpawnItemDataWithCondition {
    #[serde(flatten)]
    pub parent: GenericSpawnItemData,

    pub condition: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, XnbType)]
#[xnb_name("StardewValley.GameData.Characters.CharacterData")]
// TODO: Generate this from game data.
// TODO: This needs to be compatible with string IDs.
#[repr(i32)]
pub enum ObjectId {
    PrismaticShard = 74,
    FireQuartz = 82,
    FrozenTear = 84,
    EarthCrystal = 86,
    ArtifactTrove = 275,
    Clay = 330,
    CopperOre = 378,
    IronOre = 380,
    Coal = 382,
    GoldOre = 384,
    IridiumOre = 386,
    Stone = 390,
    CoffeeBean = 433,
    Geode = 535,
    FrozenGeode = 536,
    MagmaGeode = 537,
    OmniGeode = 749,
    GoldenCoconut = 791,
    QiBean = 890,
}

pub mod items {
    use crate::item_id;

    use super::ItemId;

    pub const WEEDS: ItemId = item_id!("(O)0");
    pub const DIAMOND_STONE: ItemId = item_id!("(O)2");
    pub const RUBY_STONE: ItemId = item_id!("(O)4");
    pub const JADE_STONE: ItemId = item_id!("(O)6");
    pub const AMETHYST_STONE: ItemId = item_id!("(O)8");
    pub const TOPAZ_STONE: ItemId = item_id!("(O)10");
    pub const EMERALD_STONE: ItemId = item_id!("(O)12");
    pub const AQUAMARINE_STONE: ItemId = item_id!("(O)14");
    pub const WILD_HORSERADISH: ItemId = item_id!("(O)16");
    pub const DAFFODIL: ItemId = item_id!("(O)18");
    pub const LEEK: ItemId = item_id!("(O)20");
    pub const DANDELION: ItemId = item_id!("(O)22");
    pub const PARSNIP: ItemId = item_id!("(O)24");
    pub const LUMBER: ItemId = item_id!("(O)30");
    pub const STONE_32: ItemId = item_id!("(O)32");
    pub const STONE_34: ItemId = item_id!("(O)34");
    pub const STONE_36: ItemId = item_id!("(O)36");
    pub const STONE_38: ItemId = item_id!("(O)38");
    pub const STONE_40: ItemId = item_id!("(O)40");
    pub const STONE_42: ItemId = item_id!("(O)42");
    pub const GEM_STONE: ItemId = item_id!("(O)44");
    pub const MYSTIC_STONE: ItemId = item_id!("(O)46");
    pub const SNOWY_STONE_48: ItemId = item_id!("(O)48");
    pub const SNOWY_STONE_50: ItemId = item_id!("(O)50");
    pub const SNOWY_STONE_52: ItemId = item_id!("(O)52");
    pub const SNOWY_STONE_54: ItemId = item_id!("(O)54");
    pub const SNOWY_STONE_56: ItemId = item_id!("(O)56");
    pub const SNOWY_STONE_58: ItemId = item_id!("(O)58");
    pub const EMERALD: ItemId = item_id!("(O)60");
    pub const AQUAMARINE: ItemId = item_id!("(O)62");
    pub const RUBY: ItemId = item_id!("(O)64");
    pub const AMETHYST: ItemId = item_id!("(O)66");
    pub const TOPAZ: ItemId = item_id!("(O)68");
    pub const JADE: ItemId = item_id!("(O)70");
    pub const TRIMMED_LUCKY_PURPLE_SHORTS: ItemId = item_id!("(O)71");
    pub const DIAMOND: ItemId = item_id!("(O)72");
    pub const PRISMATIC_SHARD: ItemId = item_id!("(O)74");
    pub const GEODE_STONE: ItemId = item_id!("(O)75");
    pub const FROZEN_GEODE_STONE: ItemId = item_id!("(O)76");
    pub const MAGMA_GEODE_STONE: ItemId = item_id!("(O)77");
    pub const CAVE_CARROT: ItemId = item_id!("(O)78");
    pub const SECRET_NOTE: ItemId = item_id!("(O)79");
    pub const QUARTZ: ItemId = item_id!("(O)80");
    pub const FIRE_QUARTZ: ItemId = item_id!("(O)82");
    pub const FROZEN_TEAR: ItemId = item_id!("(O)84");
    pub const EARTH_CRYSTAL: ItemId = item_id!("(O)86");
    pub const COCONUT: ItemId = item_id!("(O)88");
    pub const CACTUS_FRUIT: ItemId = item_id!("(O)90");
    pub const SAP: ItemId = item_id!("(O)92");
    pub const TORCH: ItemId = item_id!("(O)93");
    pub const SPIRIT_TORCH: ItemId = item_id!("(O)94");
    pub const RADIOACTIVE_STONE: ItemId = item_id!("(O)95");
    pub const DWARF_SCROLL_I: ItemId = item_id!("(O)96");
    pub const DWARF_SCROLL_II: ItemId = item_id!("(O)97");
    pub const DWARF_SCROLL_III: ItemId = item_id!("(O)98");
    pub const DWARF_SCROLL_IV: ItemId = item_id!("(O)99");
    pub const CHIPPED_AMPHORA: ItemId = item_id!("(O)100");
    pub const ARROWHEAD: ItemId = item_id!("(O)101");
    pub const LOST_BOOK: ItemId = item_id!("(O)102");
    pub const ANCIENT_DOLL: ItemId = item_id!("(O)103");
    pub const ELVISH_JEWELRY: ItemId = item_id!("(O)104");
    pub const CHEWING_STICK: ItemId = item_id!("(O)105");
    pub const ORNAMENTAL_FAN: ItemId = item_id!("(O)106");
    pub const DINOSAUR_EGG: ItemId = item_id!("(O)107");
    pub const RARE_DISC: ItemId = item_id!("(O)108");
    pub const ANCIENT_SWORD: ItemId = item_id!("(O)109");
    pub const RUSTY_SPOON: ItemId = item_id!("(O)110");
    pub const RUSTY_SPUR: ItemId = item_id!("(O)111");
    pub const RUSTY_COG: ItemId = item_id!("(O)112");
    pub const CHICKEN_STATUE: ItemId = item_id!("(O)113");
    pub const ANCIENT_SEED: ItemId = item_id!("(O)114");
    pub const PREHISTORIC_TOOL: ItemId = item_id!("(O)115");
    pub const DRIED_STARFISH: ItemId = item_id!("(O)116");
    pub const ANCHOR: ItemId = item_id!("(O)117");
    pub const GLASS_SHARDS: ItemId = item_id!("(O)118");
    pub const BONE_FLUTE: ItemId = item_id!("(O)119");
    pub const PREHISTORIC_HANDAXE: ItemId = item_id!("(O)120");
    pub const DWARVISH_HELM: ItemId = item_id!("(O)121");
    pub const DWARF_GADGET: ItemId = item_id!("(O)122");
    pub const ANCIENT_DRUM: ItemId = item_id!("(O)123");
    pub const GOLDEN_MASK: ItemId = item_id!("(O)124");
    pub const GOLDEN_RELIC: ItemId = item_id!("(O)125");
    pub const STRANGE_DOLL_126: ItemId = item_id!("(O)126");
    pub const STRANGE_DOLL_127: ItemId = item_id!("(O)127");
    pub const PUFFERFISH: ItemId = item_id!("(O)128");
    pub const ANCHOVY: ItemId = item_id!("(O)129");
    pub const TUNA: ItemId = item_id!("(O)130");
    pub const SARDINE: ItemId = item_id!("(O)131");
    pub const BREAM: ItemId = item_id!("(O)132");
    pub const LARGEMOUTH_BASS: ItemId = item_id!("(O)136");
    pub const SMALLMOUTH_BASS: ItemId = item_id!("(O)137");
    pub const RAINBOW_TROUT: ItemId = item_id!("(O)138");
    pub const SALMON: ItemId = item_id!("(O)139");
    pub const WALLEYE: ItemId = item_id!("(O)140");
    pub const PERCH: ItemId = item_id!("(O)141");
    pub const CARP: ItemId = item_id!("(O)142");
    pub const CATFISH: ItemId = item_id!("(O)143");
    pub const PIKE: ItemId = item_id!("(O)144");
    pub const SUNFISH: ItemId = item_id!("(O)145");
    pub const RED_MULLET: ItemId = item_id!("(O)146");
    pub const HERRING: ItemId = item_id!("(O)147");
    pub const EEL: ItemId = item_id!("(O)148");
    pub const OCTOPUS: ItemId = item_id!("(O)149");
    pub const RED_SNAPPER: ItemId = item_id!("(O)150");
    pub const SQUID: ItemId = item_id!("(O)151");
    pub const SEAWEED: ItemId = item_id!("(O)152");
    pub const GREEN_ALGAE: ItemId = item_id!("(O)153");
    pub const SEA_CUCUMBER: ItemId = item_id!("(O)154");
    pub const SUPER_CUCUMBER: ItemId = item_id!("(O)155");
    pub const GHOSTFISH: ItemId = item_id!("(O)156");
    pub const WHITE_ALGAE: ItemId = item_id!("(O)157");
    pub const STONEFISH: ItemId = item_id!("(O)158");
    pub const CRIMSONFISH: ItemId = item_id!("(O)159");
    pub const ANGLER: ItemId = item_id!("(O)160");
    pub const ICE_PIP: ItemId = item_id!("(O)161");
    pub const LAVA_EEL: ItemId = item_id!("(O)162");
    pub const LEGEND: ItemId = item_id!("(O)163");
    pub const SANDFISH: ItemId = item_id!("(O)164");
    pub const SCORPION_CARP: ItemId = item_id!("(O)165");
    pub const TREASURE_CHEST: ItemId = item_id!("(O)166");
    pub const JOJA_COLA: ItemId = item_id!("(O)167");
    pub const TRASH: ItemId = item_id!("(O)168");
    pub const DRIFTWOOD: ItemId = item_id!("(O)169");
    pub const BROKEN_GLASSES: ItemId = item_id!("(O)170");
    pub const BROKEN_CD: ItemId = item_id!("(O)171");
    pub const SOGGY_NEWSPAPER: ItemId = item_id!("(O)172");
    pub const EGG: ItemId = item_id!("(O)176");
    pub const LARGE_EGG: ItemId = item_id!("(O)174");
    pub const HAY: ItemId = item_id!("(O)178");
    pub const EGG_180: ItemId = item_id!("(O)180");
    pub const LARGE_EGG_182: ItemId = item_id!("(O)182");
    pub const MILK: ItemId = item_id!("(O)184");
    pub const LARGE_MILK: ItemId = item_id!("(O)186");
    pub const GREEN_BEAN: ItemId = item_id!("(O)188");
    pub const CAULIFLOWER: ItemId = item_id!("(O)190");
    pub const ORNATE_NECKLACE: ItemId = item_id!("(O)191");
    pub const POTATO: ItemId = item_id!("(O)192");
    pub const FRIED_EGG: ItemId = item_id!("(O)194");
    pub const OMELET: ItemId = item_id!("(O)195");
    pub const SALAD: ItemId = item_id!("(O)196");
    pub const CHEESE_CAULIFLOWER: ItemId = item_id!("(O)197");
    pub const BAKED_FISH: ItemId = item_id!("(O)198");
    pub const PARSNIP_SOUP: ItemId = item_id!("(O)199");
    pub const VEGETABLE_MEDLEY: ItemId = item_id!("(O)200");
    pub const COMPLETE_BREAKFAST: ItemId = item_id!("(O)201");
    pub const FRIED_CALAMARI: ItemId = item_id!("(O)202");
    pub const STRANGE_BUN: ItemId = item_id!("(O)203");
    pub const LUCKY_LUNCH: ItemId = item_id!("(O)204");
    pub const FRIED_MUSHROOM: ItemId = item_id!("(O)205");
    pub const PIZZA: ItemId = item_id!("(O)206");
    pub const BEAN_HOTPOT: ItemId = item_id!("(O)207");
    pub const GLAZED_YAMS: ItemId = item_id!("(O)208");
    pub const CARP_SURPRISE: ItemId = item_id!("(O)209");
    pub const HASHBROWNS: ItemId = item_id!("(O)210");
    pub const PANCAKES: ItemId = item_id!("(O)211");
    pub const SALMON_DINNER: ItemId = item_id!("(O)212");
    pub const FISH_TACO: ItemId = item_id!("(O)213");
    pub const CRISPY_BASS: ItemId = item_id!("(O)214");
    pub const PEPPER_POPPERS: ItemId = item_id!("(O)215");
    pub const BREAD: ItemId = item_id!("(O)216");
    pub const TOM_KHA_SOUP: ItemId = item_id!("(O)218");
    pub const TROUT_SOUP: ItemId = item_id!("(O)219");
    pub const CHOCOLATE_CAKE: ItemId = item_id!("(O)220");
    pub const PINK_CAKE: ItemId = item_id!("(O)221");
    pub const RHUBARB_PIE: ItemId = item_id!("(O)222");
    pub const COOKIES: ItemId = item_id!("(O)223");
    pub const SPAGHETTI: ItemId = item_id!("(O)224");
    pub const FRIED_EEL: ItemId = item_id!("(O)225");
    pub const SPICY_EEL: ItemId = item_id!("(O)226");
    pub const SASHIMI: ItemId = item_id!("(O)227");
    pub const MAKI_ROLL: ItemId = item_id!("(O)228");
    pub const TORTILLA: ItemId = item_id!("(O)229");
    pub const RED_PLATE: ItemId = item_id!("(O)230");
    pub const EGGPLANT_PARMESAN: ItemId = item_id!("(O)231");
    pub const RICE_PUDDING: ItemId = item_id!("(O)232");
    pub const ICE_CREAM: ItemId = item_id!("(O)233");
    pub const BLUEBERRY_TART: ItemId = item_id!("(O)234");
    pub const AUTUMNS_BOUNTY: ItemId = item_id!("(O)235");
    pub const PUMPKIN_SOUP: ItemId = item_id!("(O)236");
    pub const SUPER_MEAL: ItemId = item_id!("(O)237");
    pub const CRANBERRY_SAUCE: ItemId = item_id!("(O)238");
    pub const STUFFING: ItemId = item_id!("(O)239");
    pub const FARMERS_LUNCH: ItemId = item_id!("(O)240");
    pub const SURVIVAL_BURGER: ItemId = item_id!("(O)241");
    pub const DISH_O_THE_SEA: ItemId = item_id!("(O)242");
    pub const MINERS_TREAT: ItemId = item_id!("(O)243");
    pub const ROOTS_PLATTER: ItemId = item_id!("(O)244");
    pub const SUGAR: ItemId = item_id!("(O)245");
    pub const WHEAT_FLOUR: ItemId = item_id!("(O)246");
    pub const OIL: ItemId = item_id!("(O)247");
    pub const GARLIC: ItemId = item_id!("(O)248");
    pub const KALE: ItemId = item_id!("(O)250");
    pub const TEA_SAPLING: ItemId = item_id!("(O)251");
    pub const RHUBARB: ItemId = item_id!("(O)252");
    pub const TRIPLE_SHOT_ESPRESSO: ItemId = item_id!("(O)253");
    pub const MELON: ItemId = item_id!("(O)254");
    pub const TOMATO: ItemId = item_id!("(O)256");
    pub const MOREL: ItemId = item_id!("(O)257");
    pub const BLUEBERRY: ItemId = item_id!("(O)258");
    pub const FIDDLEHEAD_FERN: ItemId = item_id!("(O)259");
    pub const HOT_PEPPER: ItemId = item_id!("(O)260");
    pub const WARP_TOTEM_DESERT: ItemId = item_id!("(O)261");
    pub const WHEAT: ItemId = item_id!("(O)262");
    pub const RADISH: ItemId = item_id!("(O)264");
    pub const RED_CABBAGE: ItemId = item_id!("(O)266");
    pub const STARFRUIT: ItemId = item_id!("(O)268");
    pub const CORN: ItemId = item_id!("(O)270");
    pub const UNMILLED_RICE: ItemId = item_id!("(O)271");
    pub const EGGPLANT: ItemId = item_id!("(O)272");
    pub const RICE_SHOOT: ItemId = item_id!("(O)273");
    pub const ARTICHOKE: ItemId = item_id!("(O)274");
    pub const ARTIFACT_TROVE: ItemId = item_id!("(O)275");
    pub const PUMPKIN: ItemId = item_id!("(O)276");
    pub const WILTED_BOUQUET: ItemId = item_id!("(O)277");
    pub const BOK_CHOY: ItemId = item_id!("(O)278");
    pub const MAGIC_ROCK_CANDY: ItemId = item_id!("(O)279");
    pub const YAM: ItemId = item_id!("(O)280");
    pub const CHANTERELLE: ItemId = item_id!("(O)281");
    pub const CRANBERRIES: ItemId = item_id!("(O)282");
    pub const HOLLY: ItemId = item_id!("(O)283");
    pub const BEET: ItemId = item_id!("(O)284");
    pub const CHERRY_BOMB: ItemId = item_id!("(O)286");
    pub const BOMB: ItemId = item_id!("(O)287");
    pub const MEGA_BOMB: ItemId = item_id!("(O)288");
    pub const IRON_STONE_290: ItemId = item_id!("(O)290");
    pub const BRICK_FLOOR: ItemId = item_id!("(O)293");
    pub const TWIG_294: ItemId = item_id!("(O)294");
    pub const TWIG_295: ItemId = item_id!("(O)295");
    pub const SALMONBERRY: ItemId = item_id!("(O)296");
    pub const GRASS_STARTER: ItemId = item_id!("(O)297");
    pub const HARDWOOD_FENCE: ItemId = item_id!("(O)298");
    pub const AMARANTH_SEEDS: ItemId = item_id!("(O)299");
    pub const AMARANTH: ItemId = item_id!("(O)300");
    pub const GRAPE_STARTER: ItemId = item_id!("(O)301");
    pub const HOPS_STARTER: ItemId = item_id!("(O)302");
    pub const PALE_ALE: ItemId = item_id!("(O)303");
    pub const HOPS: ItemId = item_id!("(O)304");
    pub const VOID_EGG: ItemId = item_id!("(O)305");
    pub const MAYONNAISE: ItemId = item_id!("(O)306");
    pub const DUCK_MAYONNAISE: ItemId = item_id!("(O)307");
    pub const VOID_MAYONNAISE: ItemId = item_id!("(O)308");
    pub const ACORN: ItemId = item_id!("(O)309");
    pub const MAPLE_SEED: ItemId = item_id!("(O)310");
    pub const PINE_CONE: ItemId = item_id!("(O)311");
    pub const WEEDS_313: ItemId = item_id!("(O)313");
    pub const WEEDS_314: ItemId = item_id!("(O)314");
    pub const WEEDS_315: ItemId = item_id!("(O)315");
    pub const WEEDS_316: ItemId = item_id!("(O)316");
    pub const WEEDS_317: ItemId = item_id!("(O)317");
    pub const WEEDS_318: ItemId = item_id!("(O)318");
    pub const ICE_CRYSTAL_319: ItemId = item_id!("(O)319");
    pub const ICE_CRYSTAL_320: ItemId = item_id!("(O)320");
    pub const ICE_CRYSTAL_321: ItemId = item_id!("(O)321");
    pub const WOOD_FENCE: ItemId = item_id!("(O)322");
    pub const STONE_FENCE: ItemId = item_id!("(O)323");
    pub const IRON_FENCE: ItemId = item_id!("(O)324");
    pub const GATE: ItemId = item_id!("(O)325");
    pub const DWARVISH_TRANSLATION_GUIDE: ItemId = item_id!("(O)326");
    pub const WOOD_FLOOR: ItemId = item_id!("(O)328");
    pub const STONE_FLOOR: ItemId = item_id!("(O)329");
    pub const CLAY: ItemId = item_id!("(O)330");
    pub const WEATHERED_FLOOR: ItemId = item_id!("(O)331");
    pub const CRYSTAL_FLOOR: ItemId = item_id!("(O)333");
    pub const COPPER_BAR: ItemId = item_id!("(O)334");
    pub const IRON_BAR: ItemId = item_id!("(O)335");
    pub const GOLD_BAR: ItemId = item_id!("(O)336");
    pub const IRIDIUM_BAR: ItemId = item_id!("(O)337");
    pub const REFINED_QUARTZ: ItemId = item_id!("(O)338");
    pub const HONEY: ItemId = item_id!("(O)340");
    pub const TEA_SET: ItemId = item_id!("(O)341");
    pub const PICKLES: ItemId = item_id!("(O)342");
    pub const STONE_343: ItemId = item_id!("(O)343");
    pub const JELLY: ItemId = item_id!("(O)344");
    pub const BEER: ItemId = item_id!("(O)346");
    pub const RARE_SEED: ItemId = item_id!("(O)347");
    pub const WINE: ItemId = item_id!("(O)348");
    pub const ENERGY_TONIC: ItemId = item_id!("(O)349");
    pub const JUICE: ItemId = item_id!("(O)350");
    pub const MUSCLE_REMEDY: ItemId = item_id!("(O)351");
    pub const BASIC_FERTILIZER: ItemId = item_id!("(O)368");
    pub const QUALITY_FERTILIZER: ItemId = item_id!("(O)369");
    pub const BASIC_RETAINING_SOIL: ItemId = item_id!("(O)370");
    pub const QUALITY_RETAINING_SOIL: ItemId = item_id!("(O)371");
    pub const CLAM: ItemId = item_id!("(O)372");
    pub const GOLDEN_PUMPKIN: ItemId = item_id!("(O)373");
    pub const COPPER_ORE: ItemId = item_id!("(O)378");
    pub const IRON_ORE: ItemId = item_id!("(O)380");
    pub const COAL: ItemId = item_id!("(O)382");
    pub const GOLD_ORE: ItemId = item_id!("(O)384");
    pub const IRIDIUM_ORE: ItemId = item_id!("(O)386");
    pub const WOOD: ItemId = item_id!("(O)388");
    pub const STONE: ItemId = item_id!("(O)390");
    pub const NAUTILUS_SHELL: ItemId = item_id!("(O)392");
    pub const CORAL: ItemId = item_id!("(O)393");
    pub const RAINBOW_SHELL: ItemId = item_id!("(O)394");
    pub const COFFEE: ItemId = item_id!("(O)395");
    pub const SPICE_BERRY: ItemId = item_id!("(O)396");
    pub const SEA_URCHIN: ItemId = item_id!("(O)397");
    pub const GRAPE: ItemId = item_id!("(O)398");
    pub const SPRING_ONION: ItemId = item_id!("(O)399");
    pub const STRAWBERRY: ItemId = item_id!("(O)400");
    pub const STRAW_FLOOR: ItemId = item_id!("(O)401");
    pub const SWEET_PEA: ItemId = item_id!("(O)402");
    pub const FIELD_SNACK: ItemId = item_id!("(O)403");
    pub const COMMON_MUSHROOM: ItemId = item_id!("(O)404");
    pub const WOOD_PATH: ItemId = item_id!("(O)405");
    pub const WILD_PLUM: ItemId = item_id!("(O)406");
    pub const GRAVEL_PATH: ItemId = item_id!("(O)407");
    pub const HAZELNUT: ItemId = item_id!("(O)408");
    pub const CRYSTAL_PATH: ItemId = item_id!("(O)409");
    pub const BLACKBERRY: ItemId = item_id!("(O)410");
    pub const COBBLESTONE_PATH: ItemId = item_id!("(O)411");
    pub const WINTER_ROOT: ItemId = item_id!("(O)412");
    pub const BLUE_SLIME_EGG: ItemId = item_id!("(O)413");
    pub const CRYSTAL_FRUIT: ItemId = item_id!("(O)414");
    pub const STEPPING_STONE_PATH: ItemId = item_id!("(O)415");
    pub const SNOW_YAM: ItemId = item_id!("(O)416");
    pub const SWEET_GEM_BERRY: ItemId = item_id!("(O)417");
    pub const CROCUS: ItemId = item_id!("(O)418");
    pub const VINEGAR: ItemId = item_id!("(O)419");
    pub const RED_MUSHROOM: ItemId = item_id!("(O)420");
    pub const SUNFLOWER: ItemId = item_id!("(O)421");
    pub const PURPLE_MUSHROOM: ItemId = item_id!("(O)422");
    pub const RICE: ItemId = item_id!("(O)423");
    pub const CHEESE: ItemId = item_id!("(O)424");
    pub const GOAT_CHEESE: ItemId = item_id!("(O)426");
    pub const CLOTH: ItemId = item_id!("(O)428");
    pub const TRUFFLE: ItemId = item_id!("(O)430");
    pub const TRUFFLE_OIL: ItemId = item_id!("(O)432");
    pub const COFFEE_BEAN: ItemId = item_id!("(O)433");
    pub const STARDROP: ItemId = item_id!("(O)434");
    pub const GOAT_MILK: ItemId = item_id!("(O)436");
    pub const RED_SLIME_EGG: ItemId = item_id!("(O)437");
    pub const L_GOAT_MILK: ItemId = item_id!("(O)438");
    pub const PURPLE_SLIME_EGG: ItemId = item_id!("(O)439");
    pub const WOOL: ItemId = item_id!("(O)440");
    pub const EXPLOSIVE_AMMO: ItemId = item_id!("(O)441");
    pub const DUCK_EGG: ItemId = item_id!("(O)442");
    pub const DUCK_FEATHER: ItemId = item_id!("(O)444");
    pub const RABBITS_FOOT: ItemId = item_id!("(O)446");
    pub const AGED_ROE: ItemId = item_id!("(O)447");
    pub const STONE_BASE: ItemId = item_id!("(O)449");
    pub const STONE_450: ItemId = item_id!("(O)450");
    pub const WEEDS_452: ItemId = item_id!("(O)452");
    pub const ANCIENT_FRUIT: ItemId = item_id!("(O)454");
    pub const ALGAE_SOUP: ItemId = item_id!("(O)456");
    pub const PALE_BROTH: ItemId = item_id!("(O)457");
    pub const BOUQUET: ItemId = item_id!("(O)458");
    pub const MEAD: ItemId = item_id!("(O)459");
    pub const MERMAIDS_PENDANT: ItemId = item_id!("(O)460");
    pub const DECORATIVE_POT: ItemId = item_id!("(O)461");
    pub const DRUM_BLOCK: ItemId = item_id!("(O)463");
    pub const FLUTE_BLOCK: ItemId = item_id!("(O)464");
    pub const SPEED_GRO: ItemId = item_id!("(O)465");
    pub const DELUXE_SPEED_GRO: ItemId = item_id!("(O)466");
    pub const PARSNIP_SEEDS: ItemId = item_id!("(O)472");
    pub const BEAN_STARTER: ItemId = item_id!("(O)473");
    pub const CAULIFLOWER_SEEDS: ItemId = item_id!("(O)474");
    pub const POTATO_SEEDS: ItemId = item_id!("(O)475");
    pub const GARLIC_SEEDS: ItemId = item_id!("(O)476");
    pub const KALE_SEEDS: ItemId = item_id!("(O)477");
    pub const RHUBARB_SEEDS: ItemId = item_id!("(O)478");
    pub const MELON_SEEDS: ItemId = item_id!("(O)479");
    pub const TOMATO_SEEDS: ItemId = item_id!("(O)480");
    pub const BLUEBERRY_SEEDS: ItemId = item_id!("(O)481");
    pub const PEPPER_SEEDS: ItemId = item_id!("(O)482");
    pub const WHEAT_SEEDS: ItemId = item_id!("(O)483");
    pub const RADISH_SEEDS: ItemId = item_id!("(O)484");
    pub const RED_CABBAGE_SEEDS: ItemId = item_id!("(O)485");
    pub const STARFRUIT_SEEDS: ItemId = item_id!("(O)486");
    pub const CORN_SEEDS: ItemId = item_id!("(O)487");
    pub const EGGPLANT_SEEDS: ItemId = item_id!("(O)488");
    pub const ARTICHOKE_SEEDS: ItemId = item_id!("(O)489");
    pub const PUMPKIN_SEEDS: ItemId = item_id!("(O)490");
    pub const BOK_CHOY_SEEDS: ItemId = item_id!("(O)491");
    pub const YAM_SEEDS: ItemId = item_id!("(O)492");
    pub const CRANBERRY_SEEDS: ItemId = item_id!("(O)493");
    pub const BEET_SEEDS: ItemId = item_id!("(O)494");
    pub const SPRING_SEEDS: ItemId = item_id!("(O)495");
    pub const SUMMER_SEEDS: ItemId = item_id!("(O)496");
    pub const FALL_SEEDS: ItemId = item_id!("(O)497");
    pub const WINTER_SEEDS: ItemId = item_id!("(O)498");
    pub const ANCIENT_SEEDS: ItemId = item_id!("(O)499");
    pub const TULIP_BULB: ItemId = item_id!("(O)427");
    pub const JAZZ_SEEDS: ItemId = item_id!("(O)429");
    pub const POPPY_SEEDS: ItemId = item_id!("(O)453");
    pub const SPANGLE_SEEDS: ItemId = item_id!("(O)455");
    pub const SUNFLOWER_SEEDS: ItemId = item_id!("(O)431");
    pub const FAIRY_SEEDS: ItemId = item_id!("(O)425");
    pub const SMALL_GLOW_RING: ItemId = item_id!("(O)516");
    pub const GLOW_RING: ItemId = item_id!("(O)517");
    pub const SMALL_MAGNET_RING: ItemId = item_id!("(O)518");
    pub const MAGNET_RING: ItemId = item_id!("(O)519");
    pub const SLIME_CHARMER_RING: ItemId = item_id!("(O)520");
    pub const WARRIOR_RING: ItemId = item_id!("(O)521");
    pub const VAMPIRE_RING: ItemId = item_id!("(O)522");
    pub const SAVAGE_RING: ItemId = item_id!("(O)523");
    pub const RING_OF_YOBA: ItemId = item_id!("(O)524");
    pub const STURDY_RING: ItemId = item_id!("(O)525");
    pub const BURGLARS_RING: ItemId = item_id!("(O)526");
    pub const IRIDIUM_BAND: ItemId = item_id!("(O)527");
    pub const JUKEBOX_RING: ItemId = item_id!("(O)528");
    pub const AMETHYST_RING: ItemId = item_id!("(O)529");
    pub const TOPAZ_RING: ItemId = item_id!("(O)530");
    pub const AQUAMARINE_RING: ItemId = item_id!("(O)531");
    pub const JADE_RING: ItemId = item_id!("(O)532");
    pub const EMERALD_RING: ItemId = item_id!("(O)533");
    pub const RUBY_RING: ItemId = item_id!("(O)534");
    pub const GEODE: ItemId = item_id!("(O)535");
    pub const FROZEN_GEODE: ItemId = item_id!("(O)536");
    pub const MAGMA_GEODE: ItemId = item_id!("(O)537");
    pub const ALAMITE: ItemId = item_id!("(O)538");
    pub const BIXITE: ItemId = item_id!("(O)539");
    pub const BARYTE: ItemId = item_id!("(O)540");
    pub const AERINITE: ItemId = item_id!("(O)541");
    pub const CALCITE: ItemId = item_id!("(O)542");
    pub const DOLOMITE: ItemId = item_id!("(O)543");
    pub const ESPERITE: ItemId = item_id!("(O)544");
    pub const FLUORAPATITE: ItemId = item_id!("(O)545");
    pub const GEMINITE: ItemId = item_id!("(O)546");
    pub const HELVITE: ItemId = item_id!("(O)547");
    pub const JAMBORITE: ItemId = item_id!("(O)548");
    pub const JAGOITE: ItemId = item_id!("(O)549");
    pub const KYANITE: ItemId = item_id!("(O)550");
    pub const LUNARITE: ItemId = item_id!("(O)551");
    pub const MALACHITE: ItemId = item_id!("(O)552");
    pub const NEPTUNITE: ItemId = item_id!("(O)553");
    pub const LEMON_STONE: ItemId = item_id!("(O)554");
    pub const NEKOITE: ItemId = item_id!("(O)555");
    pub const ORPIMENT: ItemId = item_id!("(O)556");
    pub const PETRIFIED_SLIME: ItemId = item_id!("(O)557");
    pub const THUNDER_EGG: ItemId = item_id!("(O)558");
    pub const PYRITE: ItemId = item_id!("(O)559");
    pub const OCEAN_STONE: ItemId = item_id!("(O)560");
    pub const GHOST_CRYSTAL: ItemId = item_id!("(O)561");
    pub const TIGERSEYE: ItemId = item_id!("(O)562");
    pub const JASPER: ItemId = item_id!("(O)563");
    pub const OPAL: ItemId = item_id!("(O)564");
    pub const FIRE_OPAL: ItemId = item_id!("(O)565");
    pub const CELESTINE: ItemId = item_id!("(O)566");
    pub const MARBLE: ItemId = item_id!("(O)567");
    pub const SANDSTONE: ItemId = item_id!("(O)568");
    pub const GRANITE: ItemId = item_id!("(O)569");
    pub const BASALT: ItemId = item_id!("(O)570");
    pub const LIMESTONE: ItemId = item_id!("(O)571");
    pub const SOAPSTONE: ItemId = item_id!("(O)572");
    pub const HEMATITE: ItemId = item_id!("(O)573");
    pub const MUDSTONE: ItemId = item_id!("(O)574");
    pub const OBSIDIAN: ItemId = item_id!("(O)575");
    pub const SLATE: ItemId = item_id!("(O)576");
    pub const FAIRY_STONE: ItemId = item_id!("(O)577");
    pub const STAR_SHARDS: ItemId = item_id!("(O)578");
    pub const PREHISTORIC_SCAPULA: ItemId = item_id!("(O)579");
    pub const PREHISTORIC_TIBIA: ItemId = item_id!("(O)580");
    pub const PREHISTORIC_SKULL: ItemId = item_id!("(O)581");
    pub const SKELETAL_HAND: ItemId = item_id!("(O)582");
    pub const PREHISTORIC_RIB: ItemId = item_id!("(O)583");
    pub const PREHISTORIC_VERTEBRA: ItemId = item_id!("(O)584");
    pub const SKELETAL_TAIL: ItemId = item_id!("(O)585");
    pub const NAUTILUS_FOSSIL: ItemId = item_id!("(O)586");
    pub const AMPHIBIAN_FOSSIL: ItemId = item_id!("(O)587");
    pub const PALM_FOSSIL: ItemId = item_id!("(O)588");
    pub const TRILOBITE: ItemId = item_id!("(O)589");
    pub const ARTIFACT_SPOT: ItemId = item_id!("(O)590");
    pub const TULIP: ItemId = item_id!("(O)591");
    pub const SUMMER_SPANGLE: ItemId = item_id!("(O)593");
    pub const FAIRY_ROSE: ItemId = item_id!("(O)595");
    pub const BLUE_JAZZ: ItemId = item_id!("(O)597");
    pub const SPRINKLER: ItemId = item_id!("(O)599");
    pub const POPPY: ItemId = item_id!("(O)376");
    pub const PLUM_PUDDING: ItemId = item_id!("(O)604");
    pub const ARTICHOKE_DIP: ItemId = item_id!("(O)605");
    pub const STIR_FRY: ItemId = item_id!("(O)606");
    pub const ROASTED_HAZELNUTS: ItemId = item_id!("(O)607");
    pub const PUMPKIN_PIE: ItemId = item_id!("(O)608");
    pub const RADISH_SALAD: ItemId = item_id!("(O)609");
    pub const FRUIT_SALAD: ItemId = item_id!("(O)610");
    pub const BLACKBERRY_COBBLER: ItemId = item_id!("(O)611");
    pub const CRANBERRY_CANDY: ItemId = item_id!("(O)612");
    pub const APPLE: ItemId = item_id!("(O)613");
    pub const GREEN_TEA: ItemId = item_id!("(O)614");
    pub const BRUSCHETTA: ItemId = item_id!("(O)618");
    pub const QUALITY_SPRINKLER: ItemId = item_id!("(O)621");
    pub const IRIDIUM_SPRINKLER: ItemId = item_id!("(O)645");
    pub const COLESLAW: ItemId = item_id!("(O)648");
    pub const FIDDLEHEAD_RISOTTO: ItemId = item_id!("(O)649");
    pub const POPPYSEED_MUFFIN: ItemId = item_id!("(O)651");
    pub const CHERRY_SAPLING: ItemId = item_id!("(O)628");
    pub const APRICOT_SAPLING: ItemId = item_id!("(O)629");
    pub const ORANGE_SAPLING: ItemId = item_id!("(O)630");
    pub const PEACH_SAPLING: ItemId = item_id!("(O)631");
    pub const POMEGRANATE_SAPLING: ItemId = item_id!("(O)632");
    pub const APPLE_SAPLING: ItemId = item_id!("(O)633");
    pub const APRICOT: ItemId = item_id!("(O)634");
    pub const ORANGE: ItemId = item_id!("(O)635");
    pub const PEACH: ItemId = item_id!("(O)636");
    pub const POMEGRANATE: ItemId = item_id!("(O)637");
    pub const CHERRY: ItemId = item_id!("(O)638");
    pub const STONE_668: ItemId = item_id!("(O)668");
    pub const STONE_670: ItemId = item_id!("(O)670");
    pub const WEEDS_674: ItemId = item_id!("(O)674");
    pub const WEEDS_675: ItemId = item_id!("(O)675");
    pub const WEEDS_676: ItemId = item_id!("(O)676");
    pub const WEEDS_677: ItemId = item_id!("(O)677");
    pub const WEEDS_678: ItemId = item_id!("(O)678");
    pub const WEEDS_679: ItemId = item_id!("(O)679");
    pub const GREEN_SLIME_EGG: ItemId = item_id!("(O)680");
    pub const RAIN_TOTEM: ItemId = item_id!("(O)681");
    pub const MUTANT_CARP: ItemId = item_id!("(O)682");
    pub const BUG_MEAT: ItemId = item_id!("(O)684");
    pub const BAIT: ItemId = item_id!("(O)685");
    pub const SPINNER: ItemId = item_id!("(O)686");
    pub const DRESSED_SPINNER: ItemId = item_id!("(O)687");
    pub const WARP_TOTEM_FARM: ItemId = item_id!("(O)688");
    pub const WARP_TOTEM_MOUNTAINS: ItemId = item_id!("(O)689");
    pub const WARP_TOTEM_BEACH: ItemId = item_id!("(O)690");
    pub const BARBED_HOOK: ItemId = item_id!("(O)691");
    pub const LEAD_BOBBER: ItemId = item_id!("(O)692");
    pub const TREASURE_HUNTER: ItemId = item_id!("(O)693");
    pub const TRAP_BOBBER: ItemId = item_id!("(O)694");
    pub const CORK_BOBBER: ItemId = item_id!("(O)695");
    pub const STURGEON: ItemId = item_id!("(O)698");
    pub const TIGER_TROUT: ItemId = item_id!("(O)699");
    pub const BULLHEAD: ItemId = item_id!("(O)700");
    pub const TILAPIA: ItemId = item_id!("(O)701");
    pub const CHUB: ItemId = item_id!("(O)702");
    pub const MAGNET: ItemId = item_id!("(O)703");
    pub const DORADO: ItemId = item_id!("(O)704");
    pub const ALBACORE: ItemId = item_id!("(O)705");
    pub const SHAD: ItemId = item_id!("(O)706");
    pub const LINGCOD: ItemId = item_id!("(O)707");
    pub const HALIBUT: ItemId = item_id!("(O)708");
    pub const HARDWOOD: ItemId = item_id!("(O)709");
    pub const CRAB_POT: ItemId = item_id!("(O)710");
    pub const LOBSTER: ItemId = item_id!("(O)715");
    pub const CRAYFISH: ItemId = item_id!("(O)716");
    pub const CRAB: ItemId = item_id!("(O)717");
    pub const COCKLE: ItemId = item_id!("(O)718");
    pub const MUSSEL: ItemId = item_id!("(O)719");
    pub const SHRIMP: ItemId = item_id!("(O)720");
    pub const SNAIL: ItemId = item_id!("(O)721");
    pub const PERIWINKLE: ItemId = item_id!("(O)722");
    pub const OYSTER: ItemId = item_id!("(O)723");
    pub const MAPLE_SYRUP: ItemId = item_id!("(O)724");
    pub const OAK_RESIN: ItemId = item_id!("(O)725");
    pub const PINE_TAR: ItemId = item_id!("(O)726");
    pub const CHOWDER: ItemId = item_id!("(O)727");
    pub const LOBSTER_BISQUE: ItemId = item_id!("(O)730");
    pub const FISH_STEW: ItemId = item_id!("(O)728");
    pub const ESCARGOT: ItemId = item_id!("(O)729");
    pub const MAPLE_BAR: ItemId = item_id!("(O)731");
    pub const CRAB_CAKES: ItemId = item_id!("(O)732");
    pub const SHRIMP_COCKTAIL: ItemId = item_id!("(O)733");
    pub const WOODSKIP: ItemId = item_id!("(O)734");
    pub const HALEYS_LOST_BRACELET: ItemId = item_id!("(O)742");
    pub const STRAWBERRY_SEEDS: ItemId = item_id!("(O)745");
    pub const JACK_O_LANTERN: ItemId = item_id!("(O)746");
    pub const ROTTEN_PLANT_747: ItemId = item_id!("(O)747");
    pub const ROTTEN_PLANT_748: ItemId = item_id!("(O)748");
    pub const OMNI_GEODE: ItemId = item_id!("(O)749");
    pub const WEEDS_750: ItemId = item_id!("(O)750");
    pub const COPPER_STONE_751: ItemId = item_id!("(O)751");
    pub const STONE_760: ItemId = item_id!("(O)760");
    pub const STONE_762: ItemId = item_id!("(O)762");
    pub const GOLD_STONE: ItemId = item_id!("(O)764");
    pub const IRIDIUM_STONE: ItemId = item_id!("(O)765");
    pub const SLIME: ItemId = item_id!("(O)766");
    pub const BAT_WING: ItemId = item_id!("(O)767");
    pub const SOLAR_ESSENCE: ItemId = item_id!("(O)768");
    pub const VOID_ESSENCE: ItemId = item_id!("(O)769");
    pub const MIXED_SEEDS: ItemId = item_id!("(O)770");
    pub const FIBER: ItemId = item_id!("(O)771");
    pub const OIL_OF_GARLIC: ItemId = item_id!("(O)772");
    pub const LIFE_ELIXIR: ItemId = item_id!("(O)773");
    pub const WILD_BAIT: ItemId = item_id!("(O)774");
    pub const GLACIERFISH: ItemId = item_id!("(O)775");
    pub const WEEDS_784: ItemId = item_id!("(O)784");
    pub const WEEDS_785: ItemId = item_id!("(O)785");
    pub const WEEDS_786: ItemId = item_id!("(O)786");
    pub const BATTERY_PACK: ItemId = item_id!("(O)787");
    pub const LOST_AXE: ItemId = item_id!("(O)788");
    pub const LUCKY_PURPLE_SHORTS: ItemId = item_id!("(O)789");
    pub const BERRY_BASKET: ItemId = item_id!("(O)790");
    pub const WEEDS_792: ItemId = item_id!("(O)792");
    pub const WEEDS_793: ItemId = item_id!("(O)793");
    pub const WEEDS_794: ItemId = item_id!("(O)794");
    pub const VOID_SALMON: ItemId = item_id!("(O)795");
    pub const SLIMEJACK: ItemId = item_id!("(O)796");
    pub const PEARL: ItemId = item_id!("(O)797");
    pub const MIDNIGHT_SQUID: ItemId = item_id!("(O)798");
    pub const SPOOK_FISH: ItemId = item_id!("(O)799");
    pub const BLOBFISH: ItemId = item_id!("(O)800");
    pub const WEDDING_RING: ItemId = item_id!("(O)801");
    pub const CACTUS_SEEDS: ItemId = item_id!("(O)802");
    pub const IRIDIUM_MILK: ItemId = item_id!("(O)803");
    pub const TREE_FERTILIZER: ItemId = item_id!("(O)805");
    pub const DINOSAUR_MAYONNAISE: ItemId = item_id!("(O)807");
    pub const VOID_GHOST_PENDANT: ItemId = item_id!("(O)808");
    pub const MOVIE_TICKET: ItemId = item_id!("(O)809");
    pub const CRABSHELL_RING: ItemId = item_id!("(O)810");
    pub const NAPALM_RING: ItemId = item_id!("(O)811");
    pub const ROE: ItemId = item_id!("(O)812");
    pub const CAVIAR: ItemId = item_id!("(O)445");
    pub const SQUID_INK: ItemId = item_id!("(O)814");
    pub const TEA_LEAVES: ItemId = item_id!("(O)815");
    pub const FLOUNDER: ItemId = item_id!("(O)267");
    pub const SEAFOAM_PUDDING: ItemId = item_id!("(O)265");
    pub const MIDNIGHT_CARP: ItemId = item_id!("(O)269");
    pub const MAHOGANY_SEED: ItemId = item_id!("(O)292");
    pub const OSTRICH_EGG: ItemId = item_id!("(O)289");
    pub const MUSSEL_STONE: ItemId = item_id!("(O)25");
    pub const GOLDEN_WALNUT: ItemId = item_id!("(O)73");
    pub const BANANA_SAPLING: ItemId = item_id!("(O)69");
    pub const BANANA: ItemId = item_id!("(O)91");
    pub const GOLDEN_COCONUT: ItemId = item_id!("(O)791");
    pub const FOSSIL_STONE_816: ItemId = item_id!("(O)816");
    pub const FOSSIL_STONE_817: ItemId = item_id!("(O)817");
    pub const CLAY_STONE: ItemId = item_id!("(O)818");
    pub const OMNI_GEODE_STONE: ItemId = item_id!("(O)819");
    pub const FOSSILIZED_SKULL: ItemId = item_id!("(O)820");
    pub const FOSSILIZED_SPINE: ItemId = item_id!("(O)821");
    pub const FOSSILIZED_TAIL: ItemId = item_id!("(O)822");
    pub const FOSSILIZED_LEG: ItemId = item_id!("(O)823");
    pub const FOSSILIZED_RIBS: ItemId = item_id!("(O)824");
    pub const SNAKE_SKULL: ItemId = item_id!("(O)825");
    pub const SNAKE_VERTEBRAE: ItemId = item_id!("(O)826");
    pub const MUMMIFIED_BAT: ItemId = item_id!("(O)827");
    pub const MUMMIFIED_FROG: ItemId = item_id!("(O)828");
    pub const GINGER: ItemId = item_id!("(O)829");
    pub const TARO_ROOT: ItemId = item_id!("(O)830");
    pub const TARO_TUBER: ItemId = item_id!("(O)831");
    pub const PINEAPPLE: ItemId = item_id!("(O)832");
    pub const PINEAPPLE_SEEDS: ItemId = item_id!("(O)833");
    pub const MANGO: ItemId = item_id!("(O)834");
    pub const MANGO_SAPLING: ItemId = item_id!("(O)835");
    pub const STINGRAY: ItemId = item_id!("(O)836");
    pub const LIONFISH: ItemId = item_id!("(O)837");
    pub const BLUE_DISCUS: ItemId = item_id!("(O)838");
    pub const THORNS_RING: ItemId = item_id!("(O)839");
    pub const RUSTIC_PLANK_FLOOR: ItemId = item_id!("(O)840");
    pub const STONE_WALKWAY_FLOOR: ItemId = item_id!("(O)841");
    pub const JOURNAL_SCRAP: ItemId = item_id!("(O)842");
    pub const CINDER_SHARD_STONE_843: ItemId = item_id!("(O)843");
    pub const CINDER_SHARD_STONE_844: ItemId = item_id!("(O)844");
    pub const STONE_845: ItemId = item_id!("(O)845");
    pub const STONE_846: ItemId = item_id!("(O)846");
    pub const STONE_847: ItemId = item_id!("(O)847");
    pub const CINDER_SHARD: ItemId = item_id!("(O)848");
    pub const COPPER_STONE_849: ItemId = item_id!("(O)849");
    pub const IRON_STONE_850: ItemId = item_id!("(O)850");
    pub const MAGMA_CAP: ItemId = item_id!("(O)851");
    pub const DRAGON_TOOTH: ItemId = item_id!("(O)852");
    pub const CURIOSITY_LURE: ItemId = item_id!("(O)856");
    pub const TIGER_SLIME_EGG: ItemId = item_id!("(O)857");
    pub const QI_GEM: ItemId = item_id!("(O)858");
    pub const LUCKY_RING: ItemId = item_id!("(O)859");
    pub const HOT_JAVA_RING: ItemId = item_id!("(O)860");
    pub const PROTECTION_RING: ItemId = item_id!("(O)861");
    pub const SOUL_SAPPER_RING: ItemId = item_id!("(O)862");
    pub const PHOENIX_RING: ItemId = item_id!("(O)863");
    pub const WAR_MEMENTO: ItemId = item_id!("(O)864");
    pub const GOURMET_TOMATO_SALT: ItemId = item_id!("(O)865");
    pub const STARDEW_VALLEY_ROSE: ItemId = item_id!("(O)866");
    pub const ADVANCED_TV_REMOTE: ItemId = item_id!("(O)867");
    pub const ARCTIC_SHARD: ItemId = item_id!("(O)868");
    pub const WRIGGLING_WORM: ItemId = item_id!("(O)869");
    pub const PIRATES_LOCKET: ItemId = item_id!("(O)870");
    pub const FAIRY_DUST: ItemId = item_id!("(O)872");
    pub const PINA_COLADA: ItemId = item_id!("(O)873");
    pub const BUG_STEAK: ItemId = item_id!("(O)874");
    pub const ECTOPLASM: ItemId = item_id!("(O)875");
    pub const PRISMATIC_JELLY: ItemId = item_id!("(O)876");
    pub const QUALITY_BOBBER: ItemId = item_id!("(O)877");
    pub const MONSTER_MUSK: ItemId = item_id!("(O)879");
    pub const COMBINED_RING: ItemId = item_id!("(O)880");
    pub const BONE_FRAGMENT: ItemId = item_id!("(O)881");
    pub const WEEDS_882: ItemId = item_id!("(O)882");
    pub const WEEDS_883: ItemId = item_id!("(O)883");
    pub const WEEDS_884: ItemId = item_id!("(O)884");
    pub const FIBER_SEEDS: ItemId = item_id!("(O)885");
    pub const WARP_TOTEM_ISLAND: ItemId = item_id!("(O)886");
    pub const IMMUNITY_BAND: ItemId = item_id!("(O)887");
    pub const GLOWSTONE_RING: ItemId = item_id!("(O)888");
    pub const QI_FRUIT: ItemId = item_id!("(O)889");
    pub const QI_BEAN: ItemId = item_id!("(O)890");
    pub const MUSHROOM_TREE_SEED: ItemId = item_id!("(O)891");
    pub const WARP_TOTEM_QIS_ARENA: ItemId = item_id!("(O)892");
    pub const FIREWORKS_RED: ItemId = item_id!("(O)893");
    pub const FIREWORKS_PURPLE: ItemId = item_id!("(O)894");
    pub const FIREWORKS_GREEN: ItemId = item_id!("(O)895");
    pub const GALAXY_SOUL: ItemId = item_id!("(O)896");
    pub const PIERRES_MISSING_STOCKLIST: ItemId = item_id!("(O)897");
    pub const SON_OF_CRIMSONFISH: ItemId = item_id!("(O)898");
    pub const MS_ANGLER: ItemId = item_id!("(O)899");
    pub const LEGEND_II: ItemId = item_id!("(O)900");
    pub const RADIOACTIVE_CARP: ItemId = item_id!("(O)901");
    pub const GLACIERFISH_JR: ItemId = item_id!("(O)902");
    pub const GINGER_ALE: ItemId = item_id!("(O)903");
    pub const BANANA_PUDDING: ItemId = item_id!("(O)904");
    pub const MANGO_STICKY_RICE: ItemId = item_id!("(O)905");
    pub const POI: ItemId = item_id!("(O)906");
    pub const TROPICAL_CURRY: ItemId = item_id!("(O)907");
    pub const MAGIC_BAIT: ItemId = item_id!("(O)908");
    pub const RADIOACTIVE_ORE: ItemId = item_id!("(O)909");
    pub const RADIOACTIVE_BAR: ItemId = item_id!("(O)910");
    pub const HORSE_FLUTE: ItemId = item_id!("(O)911");
    pub const ENRICHER: ItemId = item_id!("(O)913");
    pub const PRESSURE_NOZZLE: ItemId = item_id!("(O)915");
    pub const QI_SEASONING: ItemId = item_id!("(O)917");
    pub const HYPER_SPEED_GRO: ItemId = item_id!("(O)918");
    pub const DELUXE_FERTILIZER: ItemId = item_id!("(O)919");
    pub const DELUXE_RETAINING_SOIL: ItemId = item_id!("(O)920");
    pub const SQUID_INK_RAVIOLI: ItemId = item_id!("(O)921");
    pub const SUPPLYCRATE_922: ItemId = item_id!("(O)922");
    pub const SUPPLYCRATE_923: ItemId = item_id!("(O)923");
    pub const SUPPLYCRATE_924: ItemId = item_id!("(O)924");
    pub const SLIME_CRATE: ItemId = item_id!("(O)925");
    pub const COOKOUT_KIT: ItemId = item_id!("(O)926");
    pub const CAMPING_STOVE: ItemId = item_id!("(O)927");
    pub const GOLDEN_EGG: ItemId = item_id!("(O)928");
    pub const HEDGE: ItemId = item_id!("(O)929");
    pub const QUESTION_MARKS: ItemId = item_id!("(O)930");
    pub const FAR_AWAY_STONE: ItemId = item_id!("(O)FarAwayStone");
    pub const CALICO_EGG: ItemId = item_id!("(O)CalicoEgg");
    pub const MIXED_FLOWER_SEEDS: ItemId = item_id!("(O)MixedFlowerSeeds");
    pub const GOLDEN_BOBBER: ItemId = item_id!("(O)GoldenBobber");
    pub const CALICO_EGG_STONE_0: ItemId = item_id!("(O)CalicoEggStone_0");
    pub const CALICO_EGG_STONE_1: ItemId = item_id!("(O)CalicoEggStone_1");
    pub const CALICO_EGG_STONE_2: ItemId = item_id!("(O)CalicoEggStone_2");
    pub const MYSTERY_BOX: ItemId = item_id!("(O)MysteryBox");
    pub const GOLDEN_TAG: ItemId = item_id!("(O)TroutDerbyTag");
    pub const DELUXE_BAIT: ItemId = item_id!("(O)DeluxeBait");
    pub const MOSS: ItemId = item_id!("(O)Moss");
    pub const MOSSY_SEED: ItemId = item_id!("(O)MossySeed");
    pub const GREEN_RAIN_WEEDS_0: ItemId = item_id!("(O)GreenRainWeeds0");
    pub const GREEN_RAIN_WEEDS_1: ItemId = item_id!("(O)GreenRainWeeds1");
    pub const GREEN_RAIN_WEEDS_2: ItemId = item_id!("(O)GreenRainWeeds2");
    pub const GREEN_RAIN_WEEDS_3: ItemId = item_id!("(O)GreenRainWeeds3");
    pub const GREEN_RAIN_WEEDS_4: ItemId = item_id!("(O)GreenRainWeeds4");
    pub const GREEN_RAIN_WEEDS_5: ItemId = item_id!("(O)GreenRainWeeds5");
    pub const GREEN_RAIN_WEEDS_6: ItemId = item_id!("(O)GreenRainWeeds6");
    pub const GREEN_RAIN_WEEDS_7: ItemId = item_id!("(O)GreenRainWeeds7");
    pub const SONAR_BOBBER: ItemId = item_id!("(O)SonarBobber");
    pub const SPECIFIC_BAIT: ItemId = item_id!("(O)SpecificBait");
    pub const TENT_KIT: ItemId = item_id!("(O)TentKit");
    pub const VOLCANO_GOLD_NODE: ItemId = item_id!("(O)VolcanoGoldNode");
    pub const MYSTIC_TREE_SEED: ItemId = item_id!("(O)MysticTreeSeed");
    pub const MYSTIC_SYRUP: ItemId = item_id!("(O)MysticSyrup");
    pub const RAISINS: ItemId = item_id!("(O)Raisins");
    pub const DRIED_FRUIT: ItemId = item_id!("(O)DriedFruit");
    pub const DRIED_MUSHROOMS: ItemId = item_id!("(O)DriedMushrooms");
    pub const STARDROP_TEA: ItemId = item_id!("(O)StardropTea");
    pub const PRIZE_TICKET: ItemId = item_id!("(O)PrizeTicket");
    pub const GOLD_COIN: ItemId = item_id!("(O)GoldCoin");
    pub const TREASURE_TOTEM: ItemId = item_id!("(O)TreasureTotem");
    pub const CHALLENGE_BAIT: ItemId = item_id!("(O)ChallengeBait");
    pub const CARROT_SEEDS: ItemId = item_id!("(O)CarrotSeeds");
    pub const CARROT: ItemId = item_id!("(O)Carrot");
    pub const SUMMER_SQUASH_SEEDS: ItemId = item_id!("(O)SummerSquashSeeds");
    pub const SUMMER_SQUASH: ItemId = item_id!("(O)SummerSquash");
    pub const BROCCOLI_SEEDS: ItemId = item_id!("(O)BroccoliSeeds");
    pub const BROCCOLI: ItemId = item_id!("(O)Broccoli");
    pub const POWDERMELON_SEEDS: ItemId = item_id!("(O)PowdermelonSeeds");
    pub const POWDERMELON: ItemId = item_id!("(O)Powdermelon");
    pub const SEED_SPOT: ItemId = item_id!("(O)SeedSpot");
    pub const SMOKED_FISH: ItemId = item_id!("(O)SmokedFish");
    pub const BOOK_OF_STARS: ItemId = item_id!("(O)PurpleBook");
    pub const STARDEW_VALLEY_ALMANAC: ItemId = item_id!("(O)SkillBook_0");
    pub const WOODCUTTERS_WEEKLY: ItemId = item_id!("(O)SkillBook_2");
    pub const BAIT_AND_BOBBER: ItemId = item_id!("(O)SkillBook_1");
    pub const MINING_MONTHLY: ItemId = item_id!("(O)SkillBook_3");
    pub const COMBAT_QUARTERLY: ItemId = item_id!("(O)SkillBook_4");
    pub const THE_ALLEYWAY_BUFFET: ItemId = item_id!("(O)Book_Trash");
    pub const THE_ART_O_CRABBING: ItemId = item_id!("(O)Book_Crabbing");
    pub const DWARVISH_SAFETY_MANUAL: ItemId = item_id!("(O)Book_Bombs");
    pub const JEWELS_OF_THE_SEA: ItemId = item_id!("(O)Book_Roe");
    pub const RACCOON_JOURNAL: ItemId = item_id!("(O)Book_WildSeeds");
    pub const WOODYS_SECRET: ItemId = item_id!("(O)Book_Woodcutting");
    pub const JACK_BE_NIMBLE_JACK_BE_THICK: ItemId = item_id!("(O)Book_Defense");
    pub const FRIENDSHIP_101: ItemId = item_id!("(O)Book_Friendship");
    pub const MONSTER_COMPENDIUM: ItemId = item_id!("(O)Book_Void");
    pub const WAY_OF_THE_WIND_PT_1: ItemId = item_id!("(O)Book_Speed");
    pub const MAPPING_CAVE_SYSTEMS: ItemId = item_id!("(O)Book_Marlon");
    pub const PRICE_CATALOGUE: ItemId = item_id!("(O)Book_PriceCatalogue");
    pub const QUEEN_OF_SAUCE_COOKBOOK: ItemId = item_id!("(O)Book_QueenOfSauce");
    pub const THE_DIAMOND_HUNTER: ItemId = item_id!("(O)Book_Diamonds");
    pub const BOOK_OF_MYSTERIES: ItemId = item_id!("(O)Book_Mystery");
    pub const ANIMAL_CATALOGUE: ItemId = item_id!("(O)Book_AnimalCatalogue");
    pub const WAY_OF_THE_WIND_PT_2: ItemId = item_id!("(O)Book_Speed2");
    pub const GOLDEN_ANIMAL_CRACKER: ItemId = item_id!("(O)GoldenAnimalCracker");
    pub const GOLDEN_MYSTERY_BOX: ItemId = item_id!("(O)GoldenMysteryBox");
    pub const SEA_JELLY: ItemId = item_id!("(O)SeaJelly");
    pub const CAVE_JELLY: ItemId = item_id!("(O)CaveJelly");
    pub const RIVER_JELLY: ItemId = item_id!("(O)RiverJelly");
    pub const GOBY: ItemId = item_id!("(O)Goby");
    pub const VOLCANO_COAL_NODE_0: ItemId = item_id!("(O)VolcanoCoalNode0");
    pub const VOLCANO_COAL_NODE_1: ItemId = item_id!("(O)VolcanoCoalNode1");
    pub const POT_OF_GOLD: ItemId = item_id!("(O)PotOfGold");
    pub const TREASURE_APPRAISAL_GUIDE: ItemId = item_id!("(O)Book_Artifact");
    pub const HORSE_THE_BOOK: ItemId = item_id!("(O)Book_Horse");
    pub const BUTTERFLY_POWDER: ItemId = item_id!("(O)ButterflyPowder");
    pub const PET_LICENSE: ItemId = item_id!("(O)PetLicense");
    pub const BLUE_GRASS_STARTER: ItemId = item_id!("(O)BlueGrassStarter");
    pub const MOSS_SOUP: ItemId = item_id!("(O)MossSoup");
    pub const OL_SLITHERLEGS: ItemId = item_id!("(O)Book_Grass");
    pub const BASIC_COAL_NODE_0: ItemId = item_id!("(O)BasicCoalNode0");
    pub const BASIC_COAL_NODE_1: ItemId = item_id!("(O)BasicCoalNode1");
}

impl std::cmp::PartialEq<String> for ObjectId {
    fn eq(&self, other: &String) -> bool {
        *other == (*self as i32).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p<T>(x: T, y: T) -> Point<T> {
        Point { x, y }
    }

    #[test]
    fn one_by_one_boarder_iterator_works_correctly() {
        let points: Vec<_> = Rect::from_xywh(1, 4, 1, 1).border_points().collect();
        assert_eq!(points, vec![]);
    }

    #[test]
    fn one_by_two_boarder_iterator_works_correctly() {
        let points: Vec<_> = Rect::from_xywh(1, 4, 1, 2).border_points().collect();
        assert_eq!(points, vec![p(1, 4), p(1, 5)]);
    }

    #[test]
    fn two_by_two_boarder_iterator_works_correctly() {
        let points: Vec<_> = Rect::from_xywh(1, 4, 2, 2).border_points().collect();
        assert_eq!(points, vec![p(1, 4), p(2, 4), p(2, 5), p(1, 5)]);
    }

    #[test]
    fn three_by_two_boarder_iterator_works_correctly() {
        let points: Vec<_> = Rect::from_xywh(1, 4, 3, 2).border_points().collect();
        assert_eq!(
            points,
            vec![p(1, 4), p(2, 4), p(3, 4), p(3, 5), p(2, 5), p(1, 5)]
        );
    }

    #[test]
    fn three_by_three_boarder_iterator_works_correctly() {
        let points: Vec<_> = Rect::from_xywh(1, 4, 3, 3).border_points().collect();
        assert_eq!(
            points,
            vec![
                p(1, 4),
                p(2, 4),
                p(3, 4),
                p(3, 5),
                p(3, 6),
                p(2, 6),
                p(1, 6),
                p(1, 5)
            ]
        );
    }
}
