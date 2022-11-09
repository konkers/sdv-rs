use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::{opt, value},
    multi::many0,
    IResult,
};
use std::{convert::TryInto, fs::File, io::BufReader, path::Path};
use xnb::Xnb;

use crate::{decimal, field, field_value, sub_field_value};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RewardType {
    Object,
    BigObject,
    Furniture,
    Hat,
    Clothing,
    Ring,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RoomId {
    Pantry,
    CraftsRoom,
    FishTank,
    BoilerRoom,
    Vault,
    BulletinBoard,
    AbandonedJojaMart,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BundleReward {
    pub ty: RewardType,
    pub id: i32,
    pub quantity: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BundleRequirement {
    pub id: i32,
    pub quantity: i32,
    pub minimum_quality: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bundle {
    pub room: RoomId,
    pub sprite_id: i32,
    pub name: String,
    pub reward: Option<BundleReward>,
    pub requirements: Vec<BundleRequirement>,
    pub color_index: i32,
    pub num_items_needed: i32,
}

impl Bundle {
    pub fn load<P: AsRef<Path>>(file: P) -> Result<Vec<Self>> {
        let f = File::open(file).context("Can't open bundle file")?;
        let mut r = BufReader::new(f);
        let xnb = Xnb::new(&mut r).context("Can't parse bundle xnb file")?;

        let entries: IndexMap<String, String> = xnb.content.try_into()?;
        let mut bundles = Vec::new();

        for (key, value) in &entries {
            let (_, (room, sprite_id)) = Bundle::parse_key(key).unwrap();

            let (_, bundle) = Self::parse(room, sprite_id, &value)
                .map_err(|e| anyhow!("Error parsing bundle \"{}\": {}", value, e))?;
            bundles.push(bundle);
        }
        Ok(bundles)
    }

    fn parse_room_id(i: &str) -> IResult<&str, RoomId> {
        field_value(alt((
            value(RoomId::Pantry, tag("Pantry")),
            value(RoomId::CraftsRoom, tag("Crafts Room")),
            value(RoomId::FishTank, tag("Fish Tank")),
            value(RoomId::BoilerRoom, tag("Boiler Room")),
            value(RoomId::Vault, tag("Vault")),
            value(RoomId::BulletinBoard, tag("Bulletin Board")),
            value(RoomId::AbandonedJojaMart, tag("Abandoned Joja Mart")),
        )))(i)
    }
    fn parse_reward_type(i: &str) -> IResult<&str, RewardType> {
        sub_field_value(alt((
            value(RewardType::Object, tag("O")),
            value(RewardType::BigObject, tag("BO")),
            value(RewardType::Furniture, tag("F")),
            value(RewardType::Hat, tag("H")),
            value(RewardType::Clothing, tag("C")),
            value(RewardType::Ring, tag("R")),
        )))(i)
    }

    fn parse_bundle_requirement(i: &str) -> IResult<&str, BundleRequirement> {
        let (i, id) = sub_field_value(decimal)(i)?;
        let (i, quantity) = sub_field_value(decimal)(i)?;
        let (i, minimum_quality) = sub_field_value(decimal)(i)?;

        Ok((
            i,
            BundleRequirement {
                id,
                quantity,
                minimum_quality,
            },
        ))
    }

    fn parse_bundle_reward(i: &str) -> IResult<&str, BundleReward> {
        let (i, ty) = Self::parse_reward_type(i)?;
        let (i, id) = sub_field_value(decimal)(i)?;
        let (i, quantity) = sub_field_value(decimal)(i)?;

        Ok((i, BundleReward { ty, id, quantity }))
    }

    fn parse_key(i: &str) -> IResult<&str, (RoomId, i32)> {
        let (i, room) = Self::parse_room_id(i)?;
        let (i, sprite_id) = field_value(decimal)(i)?;

        Ok((i, (room, sprite_id)))
    }

    fn parse(room: RoomId, sprite_id: i32, i: &str) -> IResult<&str, Self> {
        let (i, name) = field(i)?;
        let (i, reward) = field_value(opt(Self::parse_bundle_reward))(i)?;
        let (i, requirements) = field_value(many0(Self::parse_bundle_requirement))(i)?;
        let (i, color_index) = field_value(decimal)(i)?;
        let (i, num_items_needed) = opt(field_value(decimal))(i)?;
        let num_items_needed = num_items_needed.unwrap_or(requirements.len() as i32);

        Ok((
            i,
            Bundle {
                room,
                sprite_id,
                name: name.to_string(),
                reward,
                requirements,
                color_index,
                num_items_needed,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_crops() {
        let (_, (room, sprite_id)) = Bundle::parse_key("Pantry/0").unwrap();
        assert_eq!(
            Bundle::parse(
                room,
                sprite_id,
                "Spring Crops/O 465 20/24 1 0 188 1 0 190 1 0 192 1 0/0"
            )
            .unwrap(),
            (
                "",
                Bundle {
                    room: RoomId::Pantry,
                    sprite_id: 0,
                    name: "Spring Crops".to_string(),
                    reward: Some(BundleReward {
                        ty: RewardType::Object,
                        id: 465,
                        quantity: 20,
                    }),
                    requirements: vec![
                        BundleRequirement {
                            id: 24,
                            quantity: 1,
                            minimum_quality: 0
                        },
                        BundleRequirement {
                            id: 188,
                            quantity: 1,
                            minimum_quality: 0
                        },
                        BundleRequirement {
                            id: 190,
                            quantity: 1,
                            minimum_quality: 0
                        },
                        BundleRequirement {
                            id: 192,
                            quantity: 1,
                            minimum_quality: 0
                        },
                    ],
                    color_index: 0,
                    num_items_needed: 4,
                }
            )
        );
    }

    #[test]
    fn missing() {
        let (_, (room, sprite_id)) = Bundle::parse_key("Abandoned Joja Mart/36").unwrap();
        assert_eq!(
            Bundle::parse(
                room,
                sprite_id,
                "The Missing//348 1 1 807 1 0 74 1 0 454 5 2 795 1 2 445 1 0/1/5",
            )
            .unwrap(),
            (
                "",
                Bundle {
                    room: RoomId::AbandonedJojaMart,
                    sprite_id: 36,
                    name: "The Missing".to_string(),
                    reward: None,
                    requirements: vec![
                        BundleRequirement {
                            id: 348,
                            quantity: 1,
                            minimum_quality: 1
                        },
                        BundleRequirement {
                            id: 807,
                            quantity: 1,
                            minimum_quality: 0
                        },
                        BundleRequirement {
                            id: 74,
                            quantity: 1,
                            minimum_quality: 0
                        },
                        BundleRequirement {
                            id: 454,
                            quantity: 5,
                            minimum_quality: 2
                        },
                        BundleRequirement {
                            id: 795,
                            quantity: 1,
                            minimum_quality: 2
                        },
                        BundleRequirement {
                            id: 445,
                            quantity: 1,
                            minimum_quality: 0
                        },
                    ],
                    color_index: 1,
                    num_items_needed: 5,
                }
            )
        );
    }
}
