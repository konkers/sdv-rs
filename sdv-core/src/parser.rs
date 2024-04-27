use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1},
    combinator::recognize,
    multi::many0_count,
    sequence::pair,
    IResult,
};
use xxhash_rust::xxh32::xxh32;

use crate::ItemId;

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

pub fn item_id(input: &str) -> IResult<&str, ItemId> {
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
