use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use nom::{
    branch::alt,
    combinator::{map_parser, map_res},
    multi::many0,
    IResult,
};
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

use super::{decimal, field, sub_field, sub_field_value};
use crate::common::ObjectCategory;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Taste {
    Category(ObjectCategory),
    Item(String),
}

impl Taste {
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

    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, val) = alt((Self::parse_category, Self::parse_item))(i)?;
        Ok((i, val))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tastes {
    pub response: String,
    pub tastes: Vec<Taste>,
    categories: HashSet<ObjectCategory>,
    items: HashSet<String>,
}

impl Tastes {
    fn parse_tastes(i: &str) -> IResult<&str, Vec<Taste>> {
        map_parser(field, many0(sub_field_value(Taste::parse)))(i)
    }

    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, response) = field(i)?;
        let (i, tastes) = Self::parse_tastes(i)?;

        let (categories, items) = Self::calculate_lookup_tables(&tastes);

        Ok((
            i,
            Self {
                response: response.to_string(),
                tastes,
                categories,
                items,
            },
        ))
    }

    fn parse_universal(i: &str) -> IResult<&str, Self> {
        let (i, tastes) = Self::parse_tastes(i)?;
        let (categories, items) = Self::calculate_lookup_tables(&tastes);

        Ok((
            i,
            Self {
                response: String::new(),
                tastes,
                categories,
                items,
            },
        ))
    }

    fn calculate_lookup_tables(tastes: &[Taste]) -> (HashSet<ObjectCategory>, HashSet<String>) {
        let categories = tastes
            .iter()
            .filter_map(|taste| match taste {
                Taste::Category(c) => Some(c.clone()),
                _ => None,
            })
            .collect();
        let items = tastes
            .iter()
            .filter_map(|taste| match taste {
                Taste::Item(i) => Some(i.clone()),
                _ => None,
            })
            .collect();
        (categories, items)
    }

    pub fn has_category(&self, category: &ObjectCategory) -> bool {
        self.categories.contains(category)
    }

    pub fn has_item(&self, id: &str) -> bool {
        self.items.contains(id)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NpcGiftTastes {
    pub love: Tastes,
    pub like: Tastes,
    pub neutral: Tastes,
    pub dislike: Tastes,
    pub hate: Tastes,
}

impl NpcGiftTastes {
    pub fn load<P: AsRef<Path>>(file: P) -> Result<IndexMap<String, Self>> {
        let data = std::fs::read(file)?;

        let mut entries: IndexMap<String, String> = xnb::from_bytes(&data)?;

        let universal_love = entries
            .remove("Universal_Love")
            .ok_or_else(|| anyhow!("No Universal_Love in NPCGiftTastes"))?;
        let universal_like = entries
            .remove("Universal_Like")
            .ok_or_else(|| anyhow!("No Universal_Love in NPCGiftTastes"))?;
        let universal_neutral = entries
            .remove("Universal_Neutral")
            .ok_or_else(|| anyhow!("No Universal_Love in NPCGiftTastes"))?;
        let universal_dislike = entries
            .remove("Universal_Dislike")
            .ok_or_else(|| anyhow!("No Universal_Love in NPCGiftTastes"))?;
        let universal_hate = entries
            .remove("Universal_Hate")
            .ok_or_else(|| anyhow!("No Universal_Love in NPCGiftTastes"))?;

        let mut tastes = entries
            .into_iter()
            .map(|(k, v)| {
                Ok((
                    k,
                    Self::parse(&v)
                        .map_err(|e| anyhow!("error parsing tastes: {e}"))?
                        .1,
                ))
            })
            .collect::<Result<IndexMap<String, Self>>>()?;

        tastes.insert(
            "Universal".to_string(),
            Self {
                love: Tastes::parse_universal(&universal_love)
                    .map_err(|e| anyhow!("error parsing tastes: {e}"))?
                    .1,
                like: Tastes::parse_universal(&universal_like)
                    .map_err(|e| anyhow!("error parsing tastes: {e}"))?
                    .1,
                neutral: Tastes::parse_universal(&universal_neutral)
                    .map_err(|e| anyhow!("error parsing tastes: {e}"))?
                    .1,
                dislike: Tastes::parse_universal(&universal_dislike)
                    .map_err(|e| anyhow!("error parsing tastes: {e}"))?
                    .1,
                hate: Tastes::parse_universal(&universal_hate)
                    .map_err(|e| anyhow!("error parsing tastes: {e}"))?
                    .1,
            },
        );

        Ok(tastes)
    }

    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, love) = Tastes::parse(i)?;
        let (i, like) = Tastes::parse(i)?;
        let (i, dislike) = Tastes::parse(i)?;
        let (i, hate) = Tastes::parse(i)?;
        let (i, neutral) = Tastes::parse(i)?;

        Ok((
            i,
            Self {
                love,
                like,
                neutral,
                dislike,
                hate,
            },
        ))
    }
}
