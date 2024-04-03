use anyhow::{anyhow, Error, Result};
use indexmap::IndexMap;
use nom::{branch::alt, bytes::complete::tag, combinator::value, IResult};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use roxmltree::Node;

use serde::Deserialize;
use serde_repr::Deserialize_repr;

use std::convert::TryFrom;
use std::convert::TryInto;
use strum::EnumString;
use xnb::{xnb_name, xnb_untagged};

use crate::save::{Finder, NodeFinder, SaveError, SaveResult};

pub use xnb::XnbType;

#[derive(Clone, Debug, Default, Deserialize, EnumString, Eq, PartialEq, XnbType)]
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
    Clone, Default, Deserialize_repr, EnumString, Eq, Debug, FromPrimitive, Hash, PartialEq, XnbType,
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
    Pants = -100, // unsure
    Unknown102 = -102,
    Unknown103 = -103,
    Unknown999 = -999,
}

// This should, perhaps, be moved to `xnb-rs`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, XnbType)]
#[xnb_name("Microsoft.Xna.Framework.Point")]
#[xnb_untagged]
pub struct XnaPoint {
    pub x: i32,
    pub y: i32,
}

// This should, perhaps, be moved to `xnb-rs`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
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
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, XnbType)]
#[xnb_name("Microsoft.Xna.Framework.Rectangle")]
#[xnb_untagged]
pub struct XnaRectangle {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Rect<T> {
    p1: Point<T>,
    p2: Point<T>,
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

#[derive(Clone, Debug, Deserialize_repr, PartialEq, XnbType)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, Deserialize_repr, FromPrimitive, PartialEq, XnbType)]
#[repr(i32)]
pub enum ModificationType {
    Add = 0,
    Subtract,
    Multiply,
    Divide,
    Set,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.QuantityModifier")]
pub struct QuantityModifier {
    pub id: String,
    pub condition: String,
    pub modification: ModificationType,
    pub amount: f32,
    pub random_amount: Option<Vec<f32>>,
}

#[derive(Clone, Debug, Deserialize_repr, FromPrimitive, PartialEq, XnbType)]
#[repr(i32)]
pub enum QuantityModifierMode {
    Stack = 0,
    Minimum,
    Maximum,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
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

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.GenericSpawnItemDataWithCondition")]
pub struct GenericSpawnItemDataWithCondition {
    #[serde(flatten)]
    pub parent: GenericSpawnItemData,

    pub condition: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, XnbType)]
#[xnb_name("StardewValley.GameData.Characters.CharacterData")]
// TODO: Generate this from game data.
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
    Geode = 535,
    FrozenGeode = 536,
    MagmaGeode = 537,
    OmniGeode = 749,
    GoldenCoconut = 791,
    QiBean = 890,
}
