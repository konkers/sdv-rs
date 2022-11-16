use std::collections::HashMap;

use anyhow::{anyhow, Result};
use num_derive::FromPrimitive;
use strum::{EnumIter, IntoEnumIterator};

use crate::{common::ObjectId, gamedata, rng::Rng, save, GameData, SaveGame};

#[derive(Clone, Copy, Debug, EnumIter, Eq, FromPrimitive, Hash, PartialEq)]
pub enum GeodeType {
    Geode = 535,
    FrozenGeode = 536,
    MagmaGeode = 537,
    OmniGeode = 749,
    ArtifactTrove = 275,
    GoldenCoconut = 791,
}

impl GeodeType {
    fn item_name(&self) -> &'static str {
        match self {
            Self::Geode => "Geode",
            Self::FrozenGeode => "Frozen Geode",
            Self::MagmaGeode => "Magma Geode",
            Self::OmniGeode => "Omni Geode",
            Self::ArtifactTrove => "Artifact Trove",
            Self::GoldenCoconut => "Golden Coconut",
        }
    }
}

struct GeodeReward<'a> {
    item: &'a gamedata::Object,
    quantity: i32,
}

pub struct Geode<'a> {
    ty: GeodeType,
    rewards: Vec<GeodeReward<'a>>,
}

impl<'a> Geode<'a> {
    fn new(ty: GeodeType, game_data: &'a GameData) -> Result<Geode<'a>> {
        let geode_object = game_data.get_object_by_name(ty.item_name())?;

        let reward_ids = if ty == GeodeType::GoldenCoconut {
            // Golden Coconut's reward table is hard coded into the game.
            vec![
                (69, 1),
                (835, 1),
                (833, 5),
                (831, 5),
                (820, 1),
                (292, 1),
                (386, 5),
            ]
        } else {
            // Other geode's reward tables are encoded in their object data.
            let rewards: Result<Vec<(i32, i32)>> = geode_object.extra[0]
                .split(' ')
                .map(|s| {
                    Ok((
                        s.parse::<i32>()
                            .map_err(|e| anyhow!("Can't parse int {}: {}", s, e))?,
                        1i32,
                    ))
                })
                .collect();
            rewards?
        };

        let rewards: Result<Vec<GeodeReward>> = reward_ids
            .iter()
            .map(|(id, quantity)| {
                Ok(GeodeReward {
                    item: game_data.get_object(*id)?,
                    quantity: *quantity,
                })
            })
            .collect();

        Ok(Geode {
            ty,
            rewards: rewards?,
        })
    }

    pub fn predict(
        num_predictions: i32,
        offset: i32,
        game_data: &GameData,
        save: &SaveGame,
    ) -> Result<HashMap<GeodeType, Vec<save::Object>>> {
        GeodeType::iter()
            .map(|ty| -> Result<(GeodeType, Vec<save::Object>)> {
                let geode = Geode::new(ty, game_data)?;
                let predictions: Result<Vec<save::Object>> = (0..num_predictions)
                    .map(|i| -> Result<save::Object> {
                        Self::predict_single_geode(
                            &geode,
                            save.player.stats.geodes_cracked as i32 + offset + i,
                            game_data,
                            save,
                        )
                    })
                    .collect();
                Ok((ty, predictions?))
            })
            .collect()
    }

    fn predict_single_geode(
        geode: &Geode,
        geodes_cracked: i32,
        game_data: &GameData,
        save: &SaveGame,
    ) -> Result<save::Object> {
        let mut rng = Rng::new(geodes_cracked + save.unique_id_for_this_game / 2);

        // The game "prewarms" the rng.
        for _ in 0..2 {
            let prewarm_count = rng.next_range(1, 10)?;
            for _ in 0..prewarm_count {
                rng.next_double();
            }
        }

        // Need to implement SpecialOrderRuleActive("DROP_QI_BEANS"))
        if rng.next_double() <= 0.1 && false {
            let quantity = if rng.next_double() < 0.25 { 5 } else { 1 };
            return Ok(save::Object::from_gamedata(
                game_data.get_object(ObjectId::QiBean as i32)?,
                quantity,
            ));
        }

        if geode.ty == GeodeType::GoldenCoconut {
            // Need to implement hasOrWillReceiveMail("goldenCoconutHat"))
            if rng.next_double() <= 0.05 && false {
                return Err(anyhow!("hats not implemented"));
            }

            let reward = &geode.rewards[rng.next_max(geode.rewards.len() as i32) as usize];
            return Ok(save::Object::from_gamedata(reward.item, reward.quantity));
        } else {
            if geode.ty == GeodeType::ArtifactTrove || !(rng.next_double() < 0.5) {
                // For artifact troves and other geodes half the time, we get the
                // item from the object's definiton.
                let reward = &geode.rewards[rng.next_max(geode.rewards.len() as i32) as usize];

                // OmniGeode's have a prismatic shard as a rare reward.
                if geode.ty == GeodeType::OmniGeode
                    && rng.next_double() < 0.008
                    && geodes_cracked > 15
                {
                    return Ok(save::Object::from_gamedata(
                        game_data.get_object(ObjectId::PrismaticShard as i32)?,
                        1,
                    ));
                }

                return Ok(save::Object::from_gamedata(reward.item, reward.quantity));
            }

            let mut quantity = rng.next_max(3) * 2 + 1;
            if rng.next_double() < 0.1 {
                quantity = 10;
            }
            if rng.next_double() < 0.001 {
                quantity = 20;
            }

            if rng.next_double() < 0.5 {
                return match rng.next_max(4) {
                    0 | 1 => Ok(save::Object::from_gamedata(
                        game_data.get_object(ObjectId::Stone as i32)?,
                        quantity,
                    )),
                    2 => Ok(save::Object::from_gamedata(
                        game_data.get_object(ObjectId::Clay as i32)?,
                        1,
                    )),
                    3 | _ => {
                        let id = match geode.ty {
                            GeodeType::OmniGeode => {
                                ObjectId::FireQuartz as i32 + rng.next_max(3) * 2
                            }
                            GeodeType::Geode => ObjectId::EarthCrystal as i32,
                            GeodeType::FrozenGeode => ObjectId::FrozenTear as i32,
                            _ => ObjectId::FireQuartz as i32,
                        };
                        Ok(save::Object::from_gamedata(game_data.get_object(id)?, 1))
                    }
                };
            } else {
                let id = match geode.ty {
                    GeodeType::Geode => match rng.next_max(3) {
                        0 => ObjectId::CopperOre as i32,
                        1 => {
                            if save.player.deepest_mine_level > 25 {
                                ObjectId::IronOre as i32
                            } else {
                                ObjectId::CopperOre as i32
                            }
                        }
                        2 | _ => ObjectId::Coal as i32,
                    },
                    GeodeType::FrozenGeode => match rng.next_max(4) {
                        0 => ObjectId::CopperOre as i32,
                        1 => ObjectId::IronOre as i32,
                        2 => ObjectId::Coal as i32,
                        3 | _ => {
                            if save.player.deepest_mine_level > 75 {
                                ObjectId::GoldOre as i32
                            } else {
                                ObjectId::IronOre as i32
                            }
                        }
                    },

                    _ => match rng.next_max(5) {
                        0 => ObjectId::CopperOre as i32,
                        1 => ObjectId::IronOre as i32,
                        2 => ObjectId::Coal as i32,
                        3 => ObjectId::GoldOre as i32,
                        4 | _ => {
                            quantity = quantity / 2 + 1;
                            ObjectId::IridiumOre as i32
                        }
                    },
                };

                return Ok(save::Object::from_gamedata(
                    game_data.get_object(id)?,
                    quantity,
                ));
            }
        }
    }
}
