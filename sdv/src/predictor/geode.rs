use std::{
    cmp::max,
    convert::{TryFrom, TryInto},
};

use anyhow::{anyhow, Result};
use num_derive::FromPrimitive;
use strum::{EnumIter, EnumString};

use crate::{
    common::{items, ItemId},
    gamedata::object::ObjectGeodeDropData,
    generate_seed,
    rng::{Rng, SeedGenerator},
    GameData,
};

#[derive(Clone, Copy, Debug, EnumIter, EnumString, Eq, FromPrimitive, Hash, PartialEq)]
pub enum GeodeType {
    Geode = 535,
    FrozenGeode = 536,
    MagmaGeode = 537,
    OmniGeode = 749,
    ArtifactTrove = 275,
    GoldenCoconut = 791,
}

pub struct GeodeReward {
    pub item: ItemId,
    pub quantity: usize,
}

impl GeodeReward {
    pub const fn new(item: ItemId, quantity: usize) -> Self {
        Self { item, quantity }
    }
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

#[derive(Clone, Debug)]
enum GeodeDropItems {
    Item(ItemId),
    Random(Vec<ItemId>),
}

/// Cached drop information
///
/// In order to speed up successive prediction for seed finding, we pre-resolve
/// any lookups or parsing that can either fail or take significant time during
/// predition.
#[derive(Clone, Debug)]
struct GeodeDrop {
    chance: f64,
    condition: Option<String>,
    min_stack: i32,
    max_stack: i32,
    drop: GeodeDropItems,
}

impl TryFrom<ObjectGeodeDropData> for GeodeDrop {
    type Error = anyhow::Error;

    fn try_from(drop: ObjectGeodeDropData) -> std::result::Result<Self, Self::Error> {
        let chance = drop.chance;
        let condition = drop.parent.condition.clone();
        let min_stack = drop.parent.parent.min_stack;
        let max_stack = drop.parent.parent.max_stack;
        let drop: Result<GeodeDropItems> = {
            let random_item_id = &drop.parent.parent.random_item_id.as_ref();
            if random_item_id.is_some() && !random_item_id.unwrap().is_empty() {
                let ids = random_item_id
                    .unwrap()
                    .iter()
                    .map(|id| {
                        id.parse::<ItemId>()
                            .map_err(|e| anyhow!("Can't parse item id {id}: {e}"))
                    })
                    .collect::<Result<Vec<ItemId>>>()?;
                Ok(GeodeDropItems::Random(ids))
            } else {
                Ok(GeodeDropItems::Item(
                    drop.parent
                        .parent
                        .item_id
                        .as_ref()
                        .ok_or_else(|| anyhow!("no item id for geode drop"))?
                        .parse::<ItemId>()?,
                ))
            }
        };

        Ok(Self {
            chance,
            condition,
            min_stack,
            max_stack,
            drop: drop?,
        })
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

// Resolves a [`GeodeDrop`] into a [`GeodeReward`]
fn try_resolve(rng: &mut Rng, drop: &GeodeDrop) -> Result<GeodeReward> {
    let item = match &drop.drop {
        GeodeDropItems::Item(item) => item,
        GeodeDropItems::Random(items) => rng.chooose_from(items),
    };

    // This is the quanity logic from ItemQUeryResolve.ApplyItemfields.
    let min_stack_size = drop.min_stack;
    let max_stack_size = drop.max_stack;

    // The fact that we have to cases that return 1 is to mirror the exact
    // logic that the game uses.  I suspect that the first case here is
    // unneccesary but need further testing to prove.
    let stack_size = if min_stack_size == -1 && max_stack_size == -1 {
        1
    } else if max_stack_size > 1 {
        let min_stack_size = max(min_stack_size, 1);
        let max_stack_size = max(max_stack_size, min_stack_size);
        rng.next_range(min_stack_size, max_stack_size + 1)?
    } else if min_stack_size > 1 {
        min_stack_size
    } else {
        1
    };

    Ok(GeodeReward::new(item.clone(), stack_size as usize))
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
) -> Result<GeodeReward> {
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
        return Ok(GeodeReward::new(items::QI_BEAN, quantity));
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
                || !evaulate_condition(&drop.condition, geodes_cracked)
            {
                continue;
            }

            if let Ok(reward) = try_resolve(&mut rng, drop) {
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
        let item = match rng.next_max(4) {
            0 | 1 => items::STONE,
            2 => items::CLAY,
            _ => match geode.ty {
                GeodeType::OmniGeode => rng
                    .chooose_from(&[items::FIRE_QUARTZ, items::FROZEN_TEAR, items::EARTH_CRYSTAL])
                    .clone(),
                GeodeType::Geode => items::EARTH_CRYSTAL,
                GeodeType::FrozenGeode => items::FROZEN_TEAR,
                _ => items::FIRE_QUARTZ,
            },
        };
        return Ok(GeodeReward::new(item, amount));
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
            return Ok(GeodeReward::new(item, amount));
        }

        let (item, amount) = match rng.next_max(5) {
            0 => (items::COPPER_ORE, amount),
            1 => (items::IRON_ORE, amount),
            2 => (items::COAL, amount),
            3 => (items::GOLD_ORE, amount),
            _ => (items::IRIDIUM_ORE, amount / 2 + 1),
        };
        return Ok(GeodeReward::new(item, amount));
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
    Ok(GeodeReward::new(item, amount))
}
