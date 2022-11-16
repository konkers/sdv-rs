use std::collections::HashMap;

use anyhow::{anyhow, Result};
use strum::{EnumIter, IntoEnumIterator};

use crate::rng::Rng;
use crate::{gamedata, save, GameData, SaveGame};

// TODO: Convert to an enum and move to crate::common.
const PRISMATIC_SHARD_ID: i32 = 74;
const FIRE_QUARTZ_ID: i32 = 82;
const FROZEN_TEAR_ID: i32 = 84;
const EARTH_CRYSTAL_ID: i32 = 86;
const CLAY_ID: i32 = 330;
const COPPER_ORE_ID: i32 = 378;
const IRON_ORE_ID: i32 = 380;
const COAL_ID: i32 = 382;
const GOLD_ORE_ID: i32 = 384;
const IRIDIUM_ORE_ID: i32 = 386;
const STONE_ID: i32 = 390;
const QI_BEAN_ID: i32 = 890;

#[derive(Clone, Copy, Debug, EnumIter, Eq, Hash, PartialEq)]
pub enum GeodeType {
    // 538 542 548 549 552 555 556 557 558 566 568 569 571 574 576 121
    Geode,         // 535
    FrozenGeode,   // 536
    MagmaGeode,    // 537
    OmniGeode,     // 749
    ArtifactTrove, // 275
    GoldenCoconut, // 791
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
        println!("seed: {}", save.unique_id_for_this_game);
        println!("num predictons: {}", num_predictions);
        println!(
            "geodes cracked: {}",
            save.player.stats.geodes_cracked as i32 + offset
        );
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
                game_data.get_object(QI_BEAN_ID)?,
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
                        game_data.get_object(PRISMATIC_SHARD_ID)?,
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
                        game_data.get_object(STONE_ID)?,
                        quantity,
                    )),
                    2 => Ok(save::Object::from_gamedata(
                        game_data.get_object(CLAY_ID)?,
                        1,
                    )),
                    3 | _ => {
                        let id = match geode.ty {
                            GeodeType::OmniGeode => FIRE_QUARTZ_ID + rng.next_max(3) * 2,
                            GeodeType::Geode => EARTH_CRYSTAL_ID,
                            GeodeType::FrozenGeode => FROZEN_TEAR_ID,
                            _ => FIRE_QUARTZ_ID,
                        };
                        Ok(save::Object::from_gamedata(game_data.get_object(id)?, 1))
                    }
                };
            } else {
                let id = match geode.ty {
                    GeodeType::Geode => match rng.next_max(3) {
                        0 => COPPER_ORE_ID,
                        1 => {
                            if save.player.deepest_mine_level > 25 {
                                IRON_ORE_ID
                            } else {
                                COPPER_ORE_ID
                            }
                        }
                        2 | _ => COAL_ID,
                    },
                    GeodeType::FrozenGeode => match rng.next_max(4) {
                        0 => COPPER_ORE_ID,
                        1 => IRON_ORE_ID,
                        2 => COAL_ID,
                        3 | _ => {
                            if save.player.deepest_mine_level > 75 {
                                GOLD_ORE_ID
                            } else {
                                IRON_ORE_ID
                            }
                        }
                    },

                    _ => match rng.next_max(5) {
                        0 => COPPER_ORE_ID,
                        1 => IRON_ORE_ID,
                        2 => COAL_ID,
                        3 => GOLD_ORE_ID,
                        4 | _ => {
                            quantity = quantity / 2 + 1;
                            IRIDIUM_ORE_ID
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
