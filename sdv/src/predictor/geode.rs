use std::convert::{TryFrom, TryInto};

use anyhow::Result;
use num_derive::FromPrimitive;
use strum::{EnumIter, EnumString};

use crate::{
    common::items,
    gamedata::object::ObjectGeodeDropData,
    generate_seed,
    predictor::Drop,
    rng::{Rng, SeedGenerator},
    GameData,
};

use super::DropReward;

#[derive(Clone, Copy, Debug, EnumIter, EnumString, Eq, FromPrimitive, Hash, PartialEq)]
pub enum GeodeType {
    Geode = 535,
    FrozenGeode = 536,
    MagmaGeode = 537,
    OmniGeode = 749,
    ArtifactTrove = 275,
    GoldenCoconut = 791,
}

impl GeodeType {
    pub fn item_name(&self) -> &'static str {
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

/// Cached drop information
///
/// In order to speed up successive prediction for seed finding, we pre-resolve
/// any lookups or parsing that can either fail or take significant time during
/// predition.
#[derive(Clone, Debug)]
struct GeodeDrop {
    chance: f64,
    drop: Drop,
}

impl TryFrom<ObjectGeodeDropData> for GeodeDrop {
    type Error = anyhow::Error;

    fn try_from(drop: ObjectGeodeDropData) -> std::result::Result<Self, Self::Error> {
        let chance = drop.chance;
        let drop = Drop::try_from(&drop.parent)?;

        Ok(Self { chance, drop })
    }
}

/// Cached data for geode prediction.
#[derive(Clone, Debug)]
pub struct Geode {
    ty: GeodeType,
    default_drops: bool,
    drops: Vec<GeodeDrop>,
}

impl Geode {
    pub fn new(ty: GeodeType, game_data: &GameData) -> Result<Geode> {
        let object = game_data.get_object_by_name(ty.item_name())?.clone();

        let mut drops = object.geode_drops.unwrap_or_default();
        drops.sort_by(|a, b| a.precedence.partial_cmp(&b.precedence).unwrap());
        let drops = drops
            .into_iter()
            .map(|drop| {
                let drop = drop.try_into()?;
                Ok(drop)
            })
            .collect::<Result<Vec<GeodeDrop>>>()?;

        Ok(Self {
            ty,
            default_drops: object.geode_drops_default_items,
            drops,
        })
    }
}

// Stub until we get real condition parsing working
fn evaulate_condition(condition: &Option<String>, geodes_cracked: i32) -> bool {
    let Some(condition) = condition else {
        return true;
    };

    match condition.as_str() {
        "PLAYER_STAT Current GeodesCracked 16" => geodes_cracked >= 16,
        "!PLAYER_HAS_MAIL Current goldenCoconutHat" => true,
        _ => panic!("Unkown condition {}", condition),
    }
}

/// Predict a single geode pull
///
/// TODO: add mystery box support
pub fn predict_single_geode<G: SeedGenerator>(
    game_id: u32,
    multiplayer_id: i64,
    geodes_cracked: i32,
    geode: &Geode,
    deepest_mine_level: usize,
    qi_bean_quest_active: bool,
) -> Result<DropReward> {
    // Logic found in StardewValley.Utility.getTreasureFromGeode()

    let mut rng = Rng::new(generate_seed!(
        G,
        geodes_cracked,
        game_id / 2,
        (multiplayer_id as i32) / 2
    ));

    // The game "prewarms" the rng.
    for _ in 0..2 {
        let prewarm_count = rng.next_range(1, 10)?;
        for _ in 0..prewarm_count {
            rng.next_double();
        }
    }

    // TODO: Mystery Box logic goes here

    if rng.next_double() <= 0.1 && qi_bean_quest_active {
        let quantity = if rng.next_double() < 0.25 { 5 } else { 1 };
        return Ok(DropReward::new(items::QI_BEAN, quantity));
    }

    // The game contains a conditional here on getting the gode object by ID.
    // We're already done that and cached the results needed.  I don't belive that
    // check will ever fail in game so we're OK eliding it.

    // First process the geode specific drops
    //
    // The ordering of this conditional is important as it contains an RNG pull in it.
    if !geode.drops.is_empty() && (!geode.default_drops || rng.next_bool()) {
        // The game uses an `.OrderBy` iterator here.  We're already presorted the
        // drops accoring to precidence
        for drop in &geode.drops {
            if !rng.next_weighted_bool(drop.chance)
                || !evaulate_condition(&drop.drop.condition, geodes_cracked)
            {
                continue;
            }

            if let Ok(reward) = drop.drop.try_resolve(&mut rng) {
                return Ok(reward);
            }
        }
    }

    // If the geode specific drop processing failed above, proceed to generic geode processing.
    let mut amount = rng.next_max(3) as usize * 2 + 1;
    if rng.next_weighted_bool(0.1) {
        amount = 10;
    }
    if rng.next_weighted_bool(0.01) {
        amount = 20;
    }

    if rng.next_bool() {
        let (item, amount) = match rng.next_max(4) {
            0 | 1 => (items::STONE, amount),
            2 => (items::CLAY, 1),
            _ => match geode.ty {
                GeodeType::OmniGeode => (
                    rng.chooose_from(&[
                        items::FIRE_QUARTZ,
                        items::FROZEN_TEAR,
                        items::EARTH_CRYSTAL,
                    ])
                    .clone(),
                    1,
                ),
                GeodeType::Geode => (items::EARTH_CRYSTAL, 1),
                GeodeType::FrozenGeode => (items::FROZEN_TEAR, 1),
                _ => (items::FIRE_QUARTZ, 1),
            },
        };
        return Ok(DropReward::new(item, amount));
    }

    if !matches!(geode.ty, GeodeType::Geode) {
        if matches!(geode.ty, GeodeType::FrozenGeode) {
            let item = match rng.next_max(4) {
                0 => items::COPPER_ORE,
                1 => items::IRON_ORE,
                2 => items::COAL,
                _ => {
                    if deepest_mine_level > 75 {
                        items::GOLD_ORE
                    } else {
                        items::IRON_ORE
                    }
                }
            };
            return Ok(DropReward::new(item, amount));
        }

        let (item, amount) = match rng.next_max(5) {
            0 => (items::COPPER_ORE, amount),
            1 => (items::IRON_ORE, amount),
            2 => (items::COAL, amount),
            3 => (items::GOLD_ORE, amount),
            _ => (items::IRIDIUM_ORE, amount / 2 + 1),
        };
        return Ok(DropReward::new(item, amount));
    }

    let item = match rng.next_max(3) {
        0 => items::COPPER_ORE,
        1 => {
            if deepest_mine_level > 25 {
                items::IRON_ORE
            } else {
                items::COPPER_ORE
            }
        }
        _ => items::COAL,
    };
    Ok(DropReward::new(item, amount))
}

#[cfg(test)]
mod tests {
    use crate::rng::HashedSeedGenerator;

    use super::*;

    #[track_caller]
    fn prediction_test(geode_type: GeodeType, geodes_cracked: i32) -> Result<Vec<DropReward>> {
        let data = GameData::from_content_dir(crate::gamedata::get_game_content_path().unwrap())?;
        let geode = Geode::new(geode_type, &data).unwrap();
        let results: Vec<_> = (0..10)
            .map(|i| {
                predict_single_geode::<HashedSeedGenerator>(
                    7269403u32,
                    -7347405514601242418i64,
                    geodes_cracked + i,
                    &geode,
                    0,     // deepest_mine_level
                    false, // qi_bean_quest_active
                )
                .unwrap()
            })
            .collect();

        Ok(results)
    }

    #[test]
    fn geode_prediction_returns_correct_results() {
        // Data was verified with mousepounds predictor.
        assert_eq!(
            prediction_test(GeodeType::Geode, 1).unwrap(),
            vec![
                DropReward::new(items::ALAMITE, 1),
                DropReward::new(items::STONE, 3),
                DropReward::new(items::LIMESTONE, 1),
                DropReward::new(items::LIMESTONE, 1),
                DropReward::new(items::GRANITE, 1),
                DropReward::new(items::NEKOITE, 1),
                DropReward::new(items::ALAMITE, 1),
                DropReward::new(items::COAL, 3),
                DropReward::new(items::EARTH_CRYSTAL, 1),
                DropReward::new(items::ALAMITE, 1),
            ],
        );
    }

    #[test]
    fn frozen_geode_prediction_returns_correct_results() {
        // Data was verified with mousepounds predictor.
        assert_eq!(
            prediction_test(GeodeType::FrozenGeode, 1).unwrap(),
            vec![
                DropReward::new(items::AERINITE, 1),
                DropReward::new(items::STONE, 3),
                DropReward::new(items::SOAPSTONE, 1),
                DropReward::new(items::SOAPSTONE, 1),
                DropReward::new(items::MARBLE, 1),
                DropReward::new(items::LUNARITE, 1),
                DropReward::new(items::AERINITE, 1),
                DropReward::new(items::IRON_ORE, 3),
                DropReward::new(items::FROZEN_TEAR, 1),
                DropReward::new(items::AERINITE, 1),
            ],
        );
    }

    #[test]
    fn magma_geode_prediction_returns_correct_results() {
        // Data was verified with mousepounds predictor.
        assert_eq!(
            prediction_test(GeodeType::MagmaGeode, 1).unwrap(),
            vec![
                DropReward::new(items::BIXITE, 1),
                DropReward::new(items::STONE, 3),
                DropReward::new(items::OBSIDIAN, 1),
                DropReward::new(items::BASALT, 1),
                DropReward::new(items::BASALT, 1),
                DropReward::new(items::NEPTUNITE, 1),
                DropReward::new(items::BIXITE, 1),
                DropReward::new(items::IRIDIUM_ORE, 2),
                DropReward::new(items::FIRE_QUARTZ, 1),
                DropReward::new(items::BIXITE, 1),
            ],
        );
    }

    #[test]
    fn omni_geode_prediction_returns_correct_results() {
        // Data was verified with mousepounds predictor.
        assert_eq!(
            prediction_test(GeodeType::OmniGeode, 1).unwrap(),
            vec![
                DropReward::new(items::LUNARITE, 1),
                DropReward::new(items::STONE, 3),
                DropReward::new(items::FLUORAPATITE, 1),
                DropReward::new(items::TIGERSEYE, 1),
                DropReward::new(items::GEMINITE, 1),
                DropReward::new(items::LIMESTONE, 1),
                DropReward::new(items::HEMATITE, 1),
                DropReward::new(items::IRIDIUM_ORE, 2),
                DropReward::new(items::EARTH_CRYSTAL, 1),
                DropReward::new(items::FIRE_OPAL, 1),
            ],
        );
    }

    #[test]
    fn artifact_trove_prediction_returns_correct_results() {
        // Data was verified with mousepounds predictor.
        assert_eq!(
            prediction_test(GeodeType::ArtifactTrove, 1).unwrap(),
            vec![
                DropReward::new(items::RARE_DISC, 1),
                DropReward::new(items::DWARVISH_HELM, 1),
                DropReward::new(items::ANCIENT_DOLL, 1),
                DropReward::new(items::ORNAMENTAL_FAN, 1),
                DropReward::new(items::CHIPPED_AMPHORA, 1),
                DropReward::new(items::CHIPPED_AMPHORA, 1),
                DropReward::new(items::ELVISH_JEWELRY, 1),
                DropReward::new(items::ANCIENT_DRUM, 1),
                DropReward::new(items::BONE_FLUTE, 1),
                DropReward::new(items::ANCIENT_DOLL, 1),
            ],
        );
    }
}
