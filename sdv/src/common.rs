use anyhow::Result;
use nom::{branch::alt, bytes::complete::tag, combinator::value, IResult};
use num_traits::FromPrimitive;
use roxmltree::Node;
use serde::Deserialize;
use std::convert::TryFrom;
use std::convert::TryInto;

use crate::save::{Finder, NodeFinder, SaveError, SaveResult};

pub use xnb::value::{ObjectCategory, ObjectType};

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

impl From<xnb::value::map::Size> for Size<usize> {
    fn from(size: xnb::value::map::Size) -> Self {
        Self {
            h: size.h as usize,
            w: size.w as usize,
        }
    }
}

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

#[derive(Clone, Debug, Deserialize, PartialEq)]
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
