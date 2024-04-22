use std::str::FromStr;

use anyhow::anyhow;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1},
    combinator::recognize,
    multi::many0_count,
    sequence::pair,
    IResult,
};
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh32::xxh32;

fn big_craftable(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(BC)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::BigCraftable(id)))
}

fn boot(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(B)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Boot(id)))
}

fn flooring(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(FL)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Flooring(id)))
}

fn furniture(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(F)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Furniture(id)))
}

fn hat(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(H)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Hat(id)))
}

fn mannequin(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(M)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Mannequin(id)))
}

fn object(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(O)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Object(id)))
}

fn pants(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(P)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Pants(id)))
}

fn shirt(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(S)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Shirt(id)))
}

fn tool(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(T)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Tool(id)))
}

fn trinket(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(TR)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Trinket(id)))
}

fn wallpaper(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(WP)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Wallpaper(id)))
}

fn weapon(input: &str) -> IResult<&str, ItemId> {
    let (input, _) = tag("(W)")(input)?;
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Weapon(id)))
}

fn untagged(input: &str) -> IResult<&str, ItemId> {
    let (input, id) = bare_item_id(input)?;

    Ok((input, ItemId::Object(id)))
}

// From nom::recipes
pub fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0_count(alt((alphanumeric1, tag("_")))),
    ))(input)
}

fn bare_item_id(input: &str) -> IResult<&str, u32> {
    let (input, id) = alt((identifier, alphanumeric1))(input)?;
    let hashed_id = xxh32(id.as_bytes(), 0);

    Ok((input, hashed_id))
}

fn item_id(input: &str) -> IResult<&str, ItemId> {
    alt((
        big_craftable,
        boot,
        flooring,
        furniture,
        hat,
        mannequin,
        object,
        pants,
        shirt,
        tool,
        trinket,
        wallpaper,
        weapon,
        untagged,
    ))(input)
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ItemId {
    BigCraftable(u32),
    Boot(u32),
    Flooring(u32),
    Furniture(u32),
    Hat(u32),
    Object(u32),
    Mannequin(u32),
    Pants(u32),
    Shirt(u32),
    Tool(u32),
    Trinket(u32),
    Wallpaper(u32),
    Weapon(u32),
}

impl FromStr for ItemId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (rest, id) = item_id(s).map_err(|_| anyhow!("Can't parse item id \"{s}\""))?;
        if !rest.is_empty() {
            return Err(anyhow!("trailing input at end of valid item id \"{s}\""));
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! tagged_test {
        ($variant:ident, $tag:literal) => {
            assert_eq!(
                format!("{}0", $tag).parse::<ItemId>().unwrap(),
                ItemId::$variant(xxh32("0".as_bytes(), 0))
            );
            assert_eq!(
                format!("{}123", $tag).parse::<ItemId>().unwrap(),
                ItemId::$variant(xxh32("123".as_bytes(), 0))
            );
            assert_eq!(
                format!("{}ItemId", $tag).parse::<ItemId>().unwrap(),
                ItemId::$variant(xxh32("ItemId".as_bytes(), 0))
            );
            assert_eq!(
                format!("{}CalicoEggStone_0", $tag)
                    .parse::<ItemId>()
                    .unwrap(),
                ItemId::$variant(xxh32("CalicoEggStone_0".as_bytes(), 0))
            );
        };
    }

    #[test]
    fn item_ids_parse_correctly() {
        tagged_test!(BigCraftable, "(BC)");
        tagged_test!(Boot, "(B)");
        tagged_test!(Flooring, "(FL)");
        tagged_test!(Furniture, "(F)");
        tagged_test!(Hat, "(H)");
        tagged_test!(Mannequin, "(M)");
        tagged_test!(Object, "(O)");
        tagged_test!(Pants, "(P)");
        tagged_test!(Shirt, "(S)");
        tagged_test!(Tool, "(T)");
        tagged_test!(Trinket, "(TR)");
        tagged_test!(Wallpaper, "(WP)");
        tagged_test!(Weapon, "(W)");

        tagged_test!(Object, "");
    }

    #[test]
    fn unknown_item_tag_returns_error() {
        assert!("(💣)123".parse::<ItemId>().is_err());
    }

    #[test]
    fn bad_item_id_returns_error() {
        assert!("(BC)-1".parse::<ItemId>().is_err());
    }

    #[test]
    fn trailing_input_returns_error() {
        assert!("(BC)123💣".parse::<ItemId>().is_err());
    }
}
