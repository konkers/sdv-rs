use crate::{common::Season, rng::SeedGenerator};

use super::PredictionGameState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NightEvent {
    RacoonStump,
    Fairy,
    Witch,
    Meteorite,
    Owl,
    Capsule,
    None,
}

/// Predict a night event based on game state.
pub fn predict_night_event<G: SeedGenerator>(state: &mut PredictionGameState) -> NightEvent {
    // Night events are seeded to the game seed and day.
    let mut random = state.create_day_save_random::<G>(0., 0., 0.);

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
        && state.season() != Season::Winter
        && state.day_of_month() != 1
    {
        return NightEvent::Fairy;
    }

    if random.next_double() < 0.01 && state.days_played > 20 {
        return NightEvent::Witch;
    }

    if random.next_double() < 0.01 && state.days_played > 5 {
        return NightEvent::Meteorite;
    }

    if random.next_double() < 0.005 {
        return NightEvent::Owl;
    }

    if random.next_double() < 0.008 && state.year() > 1 && !state.has_mail_got_capsule {
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
                (1, Season::Spring, 24) => NightEvent::Meteorite,
                (1, Season::Summer, 9) => NightEvent::Meteorite,
                (1, Season::Fall, 12) => NightEvent::Witch,
                (2, Season::Spring, 13) => NightEvent::Meteorite,
                (2, Season::Summer, 2) => NightEvent::Witch,
                (2, Season::Summer, 11) => NightEvent::Witch,
                (2, Season::Fall, 5) => NightEvent::Capsule,
                (2, Season::Fall, 7) => NightEvent::Fairy,
                (2, Season::Fall, 20) => NightEvent::Meteorite,
                _ => NightEvent::None,
            };
            assert_eq!(event, expected, "{day}");
        }
    }
}
