use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::{map_res, opt, value},
    IResult,
};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::{convert::TryInto, fs::File, io::BufReader, path::Path};
use xnb::Xnb;

use crate::{decimal, field, field_value, remaining_fields, sub_field_value};

#[derive(Clone, Eq, Debug, Hash, PartialEq)]
pub enum ObjectType {
    Arch,
    Asdf,
    Basic,
    Cooking,
    Crafting,
    Fish,
    Minerals,
    Quest,
    Ring,
    Seeds,
}

#[derive(Clone, Eq, Debug, FromPrimitive, Hash, PartialEq)]
pub enum ObjectCategory {
    Gem = -2,
    Fish = -4,
    Egg = -5,
    Milk = -6,
    Cooking = -7,
    Crafting = -8,
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub name: String,
    pub price: i32,
    pub edibility: i32,
    pub ty: ObjectType,
    pub category: Option<ObjectCategory>,
    pub display_name: String,
    pub desc: String,
    pub extra: Vec<String>,
}

impl Object {
    pub fn load<P: AsRef<Path>>(file: P) -> Result<IndexMap<i32, Self>> {
        let f = File::open(file).context("Can't open object file")?;
        let mut r = BufReader::new(f);
        let xnb = Xnb::new(&mut r).context("Can't parse object xnb file")?;

        let entries: IndexMap<i32, String> = xnb.content.try_into()?;
        let mut objects = IndexMap::new();

        for (key, value) in &entries {
            let (_, object) = Self::parse(&value)
                .map_err(|e| anyhow!("Error parsing object \"{}\": {}", value, e))?;
            objects.insert(*key, object);
        }
        Ok(objects)
    }

    fn parse_type(i: &str) -> IResult<&str, ObjectType> {
        sub_field_value(alt((
            value(ObjectType::Asdf, tag("asdf")),
            value(ObjectType::Arch, tag("Arch")),
            value(ObjectType::Basic, tag("Basic")),
            value(ObjectType::Cooking, tag("Cooking")),
            value(ObjectType::Crafting, tag("Crafting")),
            value(ObjectType::Fish, tag("Fish")),
            value(ObjectType::Minerals, tag("Minerals")),
            value(ObjectType::Quest, tag("Quest")),
            value(ObjectType::Ring, tag("Ring")),
            value(ObjectType::Seeds, tag("Seeds")),
        )))(i)
    }

    fn parse_category(i: &str) -> IResult<&str, Option<ObjectCategory>> {
        opt(map_res(sub_field_value(decimal), |category: i32| {
            ObjectCategory::from_i32(category).ok_or(anyhow!("Unknown category {}", category))
        }))(i)
    }

    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, name) = field(i)?;
        let (i, price) = field_value(decimal)(i)?;
        let (i, edibility) = field_value(decimal)(i)?;
        let (i, ty) = Self::parse_type(i)?;
        let (i, category) = Self::parse_category(i)?;
        let (i, display_name) = field(i)?;
        let (i, desc) = field(i)?;

        let (i, extra) = remaining_fields(i)?;

        Ok((
            i,
            Object {
                name: name.to_string(),
                price,
                edibility,
                ty,
                category,
                display_name: display_name.to_string(),
                desc: desc.to_string(),
                extra,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fish_object() {
        assert_eq!(
        Object::parse(
        "Glacierfish/1000/10/Fish -4/Glacierfish/Builds a nest on the underside of glaciers./Day^Winter").unwrap(),
        ("",
        Object{
            name: "Glacierfish".to_string(),
            price: 1000,
            edibility: 10,
            ty: ObjectType::Fish,
            category: Some(ObjectCategory::Fish),
            display_name: "Glacierfish".to_string(),
            desc: "Builds a nest on the underside of glaciers.".to_string(),
            extra: vec!["Day^Winter".to_string()],
        }));
    }

    #[test]
    fn ring_object() {
        assert_eq!(
        Object::parse("Small Magnet Ring/100/-300/Ring/Small Magnet Ring/Slightly increases your radius for collecting items.").unwrap(),
        ("",
        Object{
            name: "Small Magnet Ring".to_string(),
            price: 100,
            edibility: -300,
            ty: ObjectType::Ring,
            category: None,
            display_name: "Small Magnet Ring".to_string(),
            desc: "Slightly increases your radius for collecting items.".to_string(),
            extra: Vec::new(),
        }));
    }
}
