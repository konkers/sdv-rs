use anyhow::{anyhow, Result};
use nom::{
    branch::alt,
    bytes::complete::is_not,
    bytes::complete::tag,
    character::complete::{char, one_of},
    combinator::{map_res, opt, recognize, value},
    multi::{many0, many1},
    sequence::{pair, preceded, terminated, tuple},
    IResult, Parser,
};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use roxmltree::Node;
use serde::Deserialize;
use std::convert::TryFrom;
use std::convert::TryInto;
use strum::EnumString;

pub mod gamedata;
pub mod save;

pub use save::SaveGame;
use save::{Finder, NodeFinder};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Point<T> {
    x: T,
    y: T,
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for Point<i32> {
    type Error = anyhow::Error;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        let node = finder.node()?;
        let x = node.child("X").try_into()?;
        let y = node.child("Y").try_into()?;
        Ok(Point { x, y })
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for Point<f32> {
    type Error = anyhow::Error;
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
    type Error = anyhow::Error;
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
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            value(Season::Spring, tag("spring")),
            value(Season::Summer, tag("summer")),
            value(Season::Fall, tag("fall")),
            value(Season::Winter, tag("winter")),
        ))(i)
    }

    fn from_node(node: &Node) -> Result<Self> {
        let text = node.text().unwrap_or("");
        let (_, season) =
            Self::parse(text).map_err(|e| anyhow!("error parsing season {}: {}", text, e))?;

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
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            value(Weather::Sunny, tag("sunny")),
            value(Weather::Rainy, tag("rainy")),
            value(Weather::Both, tag("both")),
        ))(i)
    }
}

#[derive(Clone, EnumString, Eq, Debug, FromPrimitive, Hash, PartialEq)]
pub enum ObjectCategory {
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
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for ObjectCategory {
    type Error = anyhow::Error;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        let id: i32 = finder.try_into()?;
        Self::from_i32(id).ok_or(anyhow!("unknown ObjectCategory {}", id))
    }
}

fn field_seperator(input: &str) -> IResult<&str, ()> {
    let (i, _) = opt(tag("/"))(input)?;
    Ok((i, ()))
}

fn sub_field_seperator(input: &str) -> IResult<&str, ()> {
    let (i, _) = opt(alt((tag("/"), tag(" "))))(input)?;
    Ok((i, ()))
}

fn decimal(input: &str) -> IResult<&str, i32> {
    map_res(
        recognize(pair(
            opt(char('-')),
            many1(terminated(one_of("0123456789"), many0(char('_')))),
        )),
        |out: &str| i32::from_str_radix(&str::replace(&out, "_", ""), 10),
    )(input)
}

fn float(input: &str) -> IResult<&str, f32> {
    map_res(
        alt((
            // Case one: .42
            recognize(tuple((
                char('.'),
                decimal,
                opt(tuple((one_of("eE"), opt(one_of("+-")), decimal))),
            ))), // Case two: 42e42 and 42.42e42
            recognize(tuple((
                decimal,
                opt(preceded(char('.'), decimal)),
                one_of("eE"),
                opt(one_of("+-")),
                decimal,
            ))), // Case three: 42. and 42.42
            recognize(tuple((decimal, char('.'), opt(decimal)))),
            // Case four: 42
            recognize(decimal),
        )),
        |out: &str| out.parse::<f32>(),
    )(input)
}

fn field(i: &str) -> IResult<&str, &str> {
    let (i, value) = recognize(many0(is_not("/")))(i)?;
    let (i, _) = opt(field_seperator)(i)?;
    Ok((i, value))
}

pub fn field_value<'a, O2, G>(mut f: G) -> impl FnMut(&'a str) -> IResult<&'a str, O2>
where
    G: FnMut(&'a str) -> IResult<&'a str, O2>,
{
    move |input: &str| {
        let (i, o1) = field.parse(input)?;
        let (_, value) = f(o1)?;

        Ok((i, value))
    }
}

fn sub_field(i: &str) -> IResult<&str, &str> {
    let (i, value) = recognize(many0(is_not(" /")))(i)?;
    let (i, _) = opt(sub_field_seperator)(i)?;
    Ok((i, value))
}

pub fn sub_field_value<'a, O2, G>(mut f: G) -> impl FnMut(&'a str) -> IResult<&'a str, O2>
where
    G: FnMut(&'a str) -> IResult<&'a str, O2>,
{
    move |input: &str| {
        let (i, o1) = sub_field.parse(input)?;
        let (_, value) = f(o1)?;

        Ok((i, value))
    }
}

fn remaining_fields<'a>(i: &'a str) -> IResult<&'a str, Vec<String>> {
    let (i, fields) = many0(|i: &'a str| {
        let (i, value) = recognize(many1(is_not("/")))(i)?;
        let (i, _) = opt(field_seperator)(i)?;
        Ok((i, value))
    })(i)?;

    Ok((i, fields.iter().map(|s| s.to_string()).collect()))
}
