use serde::{Deserialize, Serialize};

use crate::{
    common::Season,
    generate_day_save_seed,
    rng::{Rng, SeedGenerator},
};

use super::PredictionGameState;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NightEvent {
    RacoonStump,
    Fairy,
    Witch,
    Meteorite,
    Owl,
    Capsule,
    None,
}

const fn season(days_played: u32) -> Season {
    match ((days_played - 1) / 28) % 4 {
        0 => Season::Spring,
        1 => Season::Summer,
        2 => Season::Fall,
        _ => Season::Winter,
    }
}

const fn day_of_month(days_played: u32) -> u32 {
    (days_played - 1) % 28 + 1
}

const fn year(days_played: u32) -> u32 {
    (days_played - 1) / (28 * 4) + 1
}

/// Predict a night event based on game state.
pub fn predict_night_event<G: SeedGenerator>(state: &mut PredictionGameState) -> NightEvent {
    // Logic appears in `Utility.pickFarmEvent()`

    // Night events are calculated at the end of the day after the `days_played`
    // is already incremented so we need to create our seeds off of `days_played + 1`.
    let days_played = state.days_played + 1;
    let mut random = Rng::new(generate_day_save_seed!(
        G,
        days_played,
        state.game_id,
        0.0,
        0.0,
        0.0
    ));

    // Warm up rng
    for _ in 0..10 {
        random.next_double();
    }

    // The below checks are taken from the game and are fairly self explainitory.

    if state.cc_pantry_complete && random.next_double() < 0.1 && !state.raccoon_tree_fallen {
        state.raccoon_tree_fallen = true;
        return NightEvent::RacoonStump;
    }

    // If a fairy rose matured tonight, there's a buf to fairy chance.
    let fairy_chance = 0.01 + if state.has_fairy_rose { 0.007 } else { 0.0 };
    state.has_fairy_rose = false;
    if random.next_double() < fairy_chance
        && season(days_played) != Season::Winter
        && day_of_month(days_played) != 1
    {
        return NightEvent::Fairy;
    }

    if random.next_double() < 0.01 && days_played > 20 {
        return NightEvent::Witch;
    }

    if random.next_double() < 0.01 && days_played > 5 {
        return NightEvent::Meteorite;
    }

    if random.next_double() < 0.005 {
        return NightEvent::Owl;
    }

    if random.next_double() < 0.008 && year(days_played) > 1 && !state.has_mail_got_capsule {
        // Only one capsule event per save.
        state.has_mail_got_capsule = true;
        return NightEvent::Capsule;
    }

    NightEvent::None
}

#[cfg(test)]
mod tests {

    use crate::rng::HashedSeedGenerator;

    use super::*;

    #[test]
    fn garbage_prediction_returns_correct_results() {
        let mut state = PredictionGameState {
            game_id: 7269403,
            ..Default::default()
        };
        for day in 1..(2 * 4 * 28) {
            state.days_played = day;
            let event = predict_night_event::<HashedSeedGenerator>(&mut state);
            // Data verified with mouseypounds which seems to be a day off.
            let expected = match (state.year(), state.season(), state.day_of_month()) {
                (1, Season::Spring, 23) => NightEvent::Meteorite,
                (1, Season::Summer, 8) => NightEvent::Meteorite,
                (1, Season::Fall, 11) => NightEvent::Witch,
                (2, Season::Spring, 12) => NightEvent::Meteorite,
                (2, Season::Summer, 1) => NightEvent::Witch,
                (2, Season::Summer, 10) => NightEvent::Witch,
                (2, Season::Fall, 4) => NightEvent::Capsule,
                (2, Season::Fall, 6) => NightEvent::Fairy,
                (2, Season::Fall, 19) => NightEvent::Meteorite,
                _ => NightEvent::None,
            };
            assert_eq!(event, expected, "{day}");
        }
    }
}
