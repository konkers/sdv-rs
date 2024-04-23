use std::{cmp::max, convert::TryFrom};

use anyhow::{anyhow, Result};

use crate::{
    common::{GenericSpawnItemDataWithCondition, ItemId, Season},
    generate_day_save_seed, item_id,
    rng::{Rng, SeedGenerator},
};

pub mod bubbles;
pub mod garbage;
pub mod geode;
pub mod night_event;

/// Game state data for garbage prediction.
#[derive(Clone, Debug, Default)]
pub struct PredictionGameState {
    pub game_id: u32,
    pub days_played: u32,
    pub daily_luck: f64,
    pub has_trash_book: bool,
    pub trash_cans_checked: usize,
    pub qi_beans_quest_active: bool,
    pub has_cc_movie_theater_mail: bool,
    pub has_cc_movie_theater_joja_mail: bool,
    pub seen_event_191383: bool,
    pub cc_pantry_complete: bool,
    pub raccoon_tree_fallen: bool,
    pub has_fairy_rose: bool,
    pub has_mail_got_capsule: bool,
}

impl PredictionGameState {
    pub fn create_day_save_random<G: SeedGenerator>(&self, a: f64, b: f64, c: f64) -> Rng {
        Rng::new(generate_day_save_seed!(
            G,
            self.days_played,
            self.game_id,
            a,
            b,
            c
        ))
    }

    pub const fn year(&self) -> u32 {
        (self.days_played - 1) / (28 * 4) + 1
    }

    pub const fn season(&self) -> Season {
        match ((self.days_played - 1) / 28) % 4 {
            0 => Season::Spring,
            1 => Season::Summer,
            2 => Season::Fall,
            _ => Season::Winter,
        }
    }

    pub const fn day_of_month(&self) -> u32 {
        (self.days_played - 1) % 28 + 1
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DropReward {
    pub item: ItemId,
    pub quantity: usize,
}

impl DropReward {
    pub const fn new(item: ItemId, quantity: usize) -> Self {
        Self { item, quantity }
    }
}

#[derive(Clone, Debug)]
enum DropItems {
    Item(ItemId),
    Random(Vec<ItemId>),
}

/// Cached drop information
///
/// In order to speed up successive prediction for seed finding, we pre-resolve
/// any lookups or parsing that can either fail or take significant time during
/// predition.
#[derive(Clone, Debug)]
pub struct Drop {
    condition: Option<String>,
    min_stack: i32,
    max_stack: i32,
    drop: DropItems,
}

impl Drop {
    pub fn try_resolve(&self, rng: &mut Rng) -> Result<DropReward> {
        let item = match &self.drop {
            DropItems::Item(item) => item,
            DropItems::Random(items) => rng.chooose_from(items),
        };

        // This is the quanity logic from ItemQueryResolve.ApplyItemfields.
        let min_stack_size = self.min_stack;
        let max_stack_size = self.max_stack;

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
        if *item == item_id!("RANDOM_BASE_SEASON_ITEM") {
            let _ = rng.next_double();
        }

        Ok(DropReward::new(item.clone(), stack_size as usize))
    }
}

impl TryFrom<&GenericSpawnItemDataWithCondition> for Drop {
    type Error = anyhow::Error;

    fn try_from(
        drop: &GenericSpawnItemDataWithCondition,
    ) -> std::result::Result<Self, Self::Error> {
        let condition = drop.condition.clone();
        let min_stack = drop.parent.min_stack;
        let max_stack = drop.parent.max_stack;
        let drop: Result<DropItems> = {
            let random_item_id = &drop.parent.random_item_id.as_ref();
            if random_item_id.is_some() && !random_item_id.unwrap().is_empty() {
                let ids = random_item_id
                    .unwrap()
                    .iter()
                    .map(|id| {
                        id.parse::<ItemId>()
                            .map_err(|e| anyhow!("Can't parse item id {id}: {e}"))
                    })
                    .collect::<Result<Vec<ItemId>>>()?;
                Ok(DropItems::Random(ids))
            } else {
                Ok(DropItems::Item(
                    drop.parent
                        .item_id
                        .as_ref()
                        .ok_or_else(|| anyhow!("no item id for geode drop"))?
                        .parse::<ItemId>()?,
                ))
            }
        };

        Ok(Self {
            condition,
            min_stack,
            max_stack,
            drop: drop?,
        })
    }
}
//pub use geode::{Geode, GeodeType};
