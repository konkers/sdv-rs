use std::{convert::TryFrom};

use anyhow::{anyhow, Result};
use strum::{Display, EnumIter, EnumString};
use xxhash_rust::xxh32::xxh32;

use super::{Drop, DropReward, PredictionGameState};
use crate::{
    gamedata::garbage::GarbageCanData,
    generate_day_save_seed, generate_seed,
    rng::{Rng, SeedGenerator},
};

#[derive(Clone, Copy, Debug, Display, EnumIter, EnumString, Eq, Hash, PartialEq)]
pub enum GarbageCanLocation {
    JodiAndKent,
    EmilyAndHaley,
    Mayor,
    Museum,
    Blacksmith,
    Saloon,
    Evelyn,
    JojaMart,
}
#[derive(Clone, Debug)]
struct GarbageDrop {
    drop: Drop,
    ignore_base_chance: bool,
}

/// Cached can data for garbage prediction.
#[derive(Clone, Debug)]
pub struct GarbageCan {
    pub location: GarbageCanLocation,
    base_chance: f32, // Includes default base chance and book calculation
    hashed_id: i32,
    items: Vec<GarbageDrop>,
}

impl GarbageCan {
    pub fn new(
        location: GarbageCanLocation,
        data: &GarbageCanData,
        state: &PredictionGameState,
    ) -> Result<Self> {
        let location_str = location.to_string();
        let can_data = data
            .garbage_cans
            .get(&location_str)
            .ok_or_else(|| anyhow!("Can't get trash can data for {location_str}"))?;
        let mut base_chance = if can_data.base_chance > 0. {
            can_data.base_chance
        } else {
            data.default_base_chance
        };
        if state.has_trash_book {
            base_chance += 0.2;
        }
        let hashed_id = 777 + xxh32(location_str.as_bytes(), 0) as i32;

        let mut items = data.before_all.clone();
        items.append(&mut can_data.items.clone());
        items.append(&mut data.after_all.clone());

        let items = items
            .iter()
            .map(|item| {
                let drop = Drop::try_from(&item.parent)?;
                Ok(GarbageDrop {
                    drop,
                    ignore_base_chance: item.ignore_base_chance,
                })
            })
            .collect::<Result<Vec<GarbageDrop>>>()?;

        Ok(Self {
            location,
            base_chance,
            hashed_id,
            items,
        })
    }
}

fn daily_luck_bool(rng: &mut Rng, chance: f64) -> ConditionResult {
    let roll = rng.next_double();
    ConditionResult::WithDailyLuck(roll - chance)
}

macro_rules! synced_random {
    (day, $generator:ty, $key:literal, $chance:expr, $state: expr) => {
        ConditionResult::Static(
            Rng::new(generate_seed!(
                G,
                xxh32($key.as_bytes(), 0) as i32, // i32 conversion here is very important
                $state.game_id,
                $state.days_played
            ))
            .next_weighted_bool($chance),
        )
    };
    (day_luck, $generator:ty, $key:literal, $chance:expr, $state: expr) => {{
        let mut rng = Rng::new(generate_seed!(
            G,
            xxh32($key.as_bytes(), 0) as i32, // i32 conversion here is very important
            $state.game_id as f64,
            $state.days_played as f64
        ));
        daily_luck_bool(&mut rng, $chance)
    }};
}

#[derive(Clone, Copy, Debug)]
enum ConditionResult {
    Static(bool),
    WithDailyLuck(f64),
}

impl ConditionResult {
    pub fn and(self, b: bool) -> Self {
        if !b {
            return ConditionResult::Static(false);
        }
        self
    }

    pub fn and_result(self, b: Self) -> Self {
        match self {
            ConditionResult::Static(a) => match b {
                ConditionResult::Static(b) => ConditionResult::Static(a && b),
                ConditionResult::WithDailyLuck(_) => {
                    if a {
                        b
                    } else {
                        self
                    }
                }
            },
            ConditionResult::WithDailyLuck(a) => match b {
                ConditionResult::Static(b) => {
                    if b {
                        self
                    } else {
                        ConditionResult::Static(false)
                    }
                }
                ConditionResult::WithDailyLuck(b) => ConditionResult::WithDailyLuck(a.max(b)),
            },
        }
    }
}

impl From<ConditionResult> for bool {
    fn from(value: ConditionResult) -> Self {
        match value {
            ConditionResult::Static(v) => v,
            // Treat daily luck rolls as success as long as they require less
            // than 0.100001 daily luck so we can accumlate min_daily_luck.
            ConditionResult::WithDailyLuck(min_luck) => min_luck < 0.100001,
        }
    }
}

// Stub until we get real condition parsing working
fn evaulate_condition<G: SeedGenerator>(
    r: &mut Rng,
    state: &PredictionGameState,
    condition: &Option<String>,
) -> ConditionResult {
    let Some(condition) = condition else {
        return ConditionResult::Static(true);
    };

    match condition.as_str() {
        "PLAYER_STAT Current trashCansChecked 20, RANDOM .002" => {
            ConditionResult::Static(state.trash_cans_checked >= 20 && r.next_weighted_bool(0.002))
        }
        "PLAYER_STAT Current trashCansChecked 50, RANDOM .002" => {
            ConditionResult::Static(state.trash_cans_checked >= 50 && r.next_weighted_bool(0.002))
        }
        "PLAYER_SPECIAL_ORDER_RULE_ACTIVE Current DROP_QI_BEANS, RANDOM 0.25" => {
            ConditionResult::Static(state.qi_beans_quest_active && r.next_weighted_bool(0.25))
        }
        "PLAYER_STAT Current trashCansChecked 20, RANDOM .01" => {
            ConditionResult::Static(state.trash_cans_checked >= 20 && r.next_weighted_bool(0.01))
        }
        "RANDOM 0.2 @addDailyLuck" => daily_luck_bool(r, 0.2),
        "SYNCED_RANDOM day garbage_joja 0.2, \
	 PLAYER_HAS_MAIL Host ccMovieTheater, \
         !PLAYER_HAS_MAIL Host ccMovieTheaterJoja" => {
            synced_random!(day, G, "garbage_joja", 0.2, state)
                .and(state.has_cc_movie_theater_mail)
                .and(!state.has_cc_movie_theater_joja_mail)
        }
        "SYNCED_RANDOM day garbage_joja 0.2, !PLAYER_HAS_SEEN_EVENT Any 191393" => {
            synced_random!(day, G, "garbage_joja", 0.2, state).and(!state.seen_event_191383)
        }
        "SYNCED_RANDOM day garbage_museum_535 0.2 @addDailyLuck, \
	 SYNCED_RANDOM day garbage_museum_749 0.05" => {
            synced_random!(day_luck, G, "garbage_museum_535", 0.2, state)
                .and_result(synced_random!(day, G, "garbage_museum_749", 0.05, state))
        }
        "SYNCED_RANDOM day garbage_museum_535 0.2 @addDailyLuck" => {
            synced_random!(day_luck, G, "garbage_museum_535", 0.2, state)
        }
        "SYNCED_RANDOM day garbage_saloon_dish 0.2 @addDailyLuck" => {
            synced_random!(day_luck, G, "garbage_saloon_dish", 0.2, state)
        }
        _ => panic!("Unkown condition {}", condition),
    }
}

pub fn predict_garbage<G: SeedGenerator>(
    can: &GarbageCan,
    state: &PredictionGameState,
) -> Result<Option<(DropReward, f64)>> {
    let mut r = Rng::new(generate_day_save_seed!(
        G,
        state.days_played,
        state.game_id,
        can.hashed_id
    ));

    for _ in 0..2 {
        let prewarm_count = r.next_range(0, 100)?;
        for _ in 0..prewarm_count {
            r.next_double();
        }
    }

    let base_chance_passed = daily_luck_bool(&mut r, can.base_chance as f64);

    for item in &can.items {
        if base_chance_passed.into() || item.ignore_base_chance {
            let result = evaulate_condition::<G>(&mut r, state, &item.drop.condition);
            let result = result.and_result(base_chance_passed);
            match result {
                ConditionResult::Static(success) => {
                    if success {
                        return Ok(Some((item.drop.try_resolve(&mut r)?, -1.)));
                    }
                }
                ConditionResult::WithDailyLuck(min_luck) => {
                    if state.daily_luck > min_luck {
                        return Ok(Some((item.drop.try_resolve(&mut r)?, min_luck)));
                    }
                }
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use crate::{common::items, rng::HashedSeedGenerator, GameData};

    use super::*;
    #[track_caller]
    fn prediction_test(days_played: u32) -> Result<Vec<(GarbageCanLocation, DropReward, f64)>> {
        let data = GameData::from_content_dir(crate::gamedata::get_game_content_path().unwrap())?;
        let state = PredictionGameState {
            game_id: 254546202,
            days_played,
            daily_luck: 0.0999,
            has_trash_book: false,
            trash_cans_checked: 0,
            qi_beans_quest_active: false,
            has_cc_movie_theater_mail: false,
            has_cc_movie_theater_joja_mail: false,
            seen_event_191383: false,
        };
        GarbageCanLocation::iter()
            .filter_map(|location| {
                let can = GarbageCan::new(location, &data.garbage_cans, &state).unwrap();
                let prediction = predict_garbage::<HashedSeedGenerator>(&can, &state).unwrap();
                prediction.map(|(drop, min_luck)| Ok((can.location, drop, min_luck)))
            })
            .collect::<Result<Vec<_>>>()
    }

    #[test]
    fn garbage_prediction_returns_correct_results() {
        // Confirmed with https://github.com/Underscore76/SeedFinding
        assert_eq!(
            prediction_test(1).unwrap(),
            vec![
                (
                    GarbageCanLocation::JodiAndKent,
                    DropReward::new(items::MAPLE_SEED, 1),
                    -0.004689127581514935
                ),
                (
                    GarbageCanLocation::Museum,
                    DropReward::new(items::GEODE, 1),
                    -0.05895514341953917
                ),
                (
                    GarbageCanLocation::Saloon,
                    DropReward::new(items::DISH_OF_THE_DAY, 1),
                    -0.09639927181247587
                ),
            ]
        );

        assert_eq!(
            prediction_test(2).unwrap(),
            vec![
                (
                    GarbageCanLocation::EmilyAndHaley,
                    DropReward::new(items::BROKEN_CD, 1),
                    0.01593356915560251
                ),
                (
                    GarbageCanLocation::Blacksmith,
                    DropReward::new(items::MAPLE_SEED, 1),
                    0.06764924492111862
                ),
            ]
        );

        assert_eq!(
            prediction_test(3).unwrap(),
            vec![(
                GarbageCanLocation::EmilyAndHaley,
                DropReward::new(items::JOJA_COLA, 1),
                -0.09772658250188761
            ),]
        );

        assert_eq!(
            prediction_test(4).unwrap(),
            vec![
                (
                    GarbageCanLocation::JodiAndKent,
                    DropReward::new(items::JOJA_COLA, 1),
                    0.04146529512548133
                ),
                (
                    GarbageCanLocation::Mayor,
                    DropReward::new(items::MAPLE_SEED, 1),
                    0.05827249505476678
                ),
            ]
        );
    }
}
