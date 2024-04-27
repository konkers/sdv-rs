use sdv_core::HashedString;
use xxhash_rust::xxh32::xxh32;

use crate::{
    common::{Season, Weather},
    gamedata::LocationContextData,
    generate_day_save_seed, generate_seed, hashed_match,
    rng::{Rng, SeedGenerator},
};

use super::PredictionGameState;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WeatherLocation {
    conditions: Vec<Option<HashedString>>,
}

impl From<&LocationContextData> for WeatherLocation {
    fn from(value: &LocationContextData) -> Self {
        let conditions = value
            .weather_condidtions
            .iter()
            .map(|condition| {
                condition
                    .condition
                    .as_ref()
                    .map(|value| HashedString::new(value))
            })
            .collect();
        Self { conditions }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WeatherPrediction {
    pub sun: f64,
    pub rain: f64,
    pub wind: f64,
    pub storm: f64,
    pub snow: f64,
    pub fesival: f64,
    pub green_rain: f64,
}

impl WeatherPrediction {
    pub fn sun() -> Self {
        Self {
            sun: 1.0,
            ..Default::default()
        }
    }

    pub fn rain() -> Self {
        Self {
            rain: 1.0,
            ..Default::default()
        }
    }

    pub fn wind() -> Self {
        Self {
            wind: 1.0,
            ..Default::default()
        }
    }

    pub fn storm() -> Self {
        Self {
            storm: 1.0,
            ..Default::default()
        }
    }

    pub fn snow() -> Self {
        Self {
            snow: 1.0,
            ..Default::default()
        }
    }

    pub fn fesival() -> Self {
        Self {
            fesival: 1.0,
            ..Default::default()
        }
    }

    pub fn green_rain() -> Self {
        Self {
            green_rain: 1.0,
            ..Default::default()
        }
    }

    pub fn from_pratials(partials: &[PartialPrediction]) -> Self {
        partials.iter().fold(Self::default(), |mut acc, partial| {
            match partial.weather {
                Weather::Sun => acc.sun += partial.chance,
                Weather::Rain => acc.rain += partial.chance,
                Weather::Wind => acc.wind += partial.chance,
                Weather::Storm => acc.storm += partial.chance,
                Weather::Snow => acc.snow += partial.chance,
                Weather::Festival => acc.fesival += partial.chance,
                Weather::GreenRain => acc.green_rain += partial.chance,
            };
            acc
        })
    }
}

#[derive(Clone, Debug)]
pub struct PartialPrediction {
    weather: Weather,
    chance: f64,

    // Used for debugging.
    #[allow(unused)]
    condition: &'static str,
}

impl PartialEq for PartialPrediction {
    fn eq(&self, other: &Self) -> bool {
        self.weather == other.weather && self.chance == other.chance
    }
}

impl PartialPrediction {
    pub fn new(weather: Weather, chance: f64) -> Self {
        Self {
            weather,
            chance,
            condition: "Code",
        }
    }

    pub fn new_condition(weather: Weather, chance: f64, condition: &'static str) -> Self {
        Self {
            weather,
            chance,
            condition,
        }
    }
}

fn synced_day_random<G: SeedGenerator>(
    state: &PredictionGameState,
    key: &str,
    chance: f64,
) -> bool {
    Rng::new(generate_seed!(
        G,
        xxh32(key.as_bytes(), 0) as i32, // i32 conversion here is very important
        state.game_id,
        state.days_played
    ))
    .next_weighted_bool(chance)
}

fn synced_summer_rain_random<G: SeedGenerator>(state: &PredictionGameState) -> bool {
    // i32 conversion here is very important
    let key = xxh32("summer_rain_chance".as_bytes(), 0) as i32;
    Rng::new(generate_day_save_seed!(
        G,
        state.days_played,
        state.game_id,
        key
    ))
    .next_weighted_bool((0.12f32 + (state.day_of_month() as f32) * 0.003) as f64)
}

fn is_green_rain_day<G: SeedGenerator>(state: &PredictionGameState) -> bool {
    if !state.is_season(Season::Summer) {
        return false;
    }
    let mut rng = Rng::new(generate_seed!(G, state.year() * 777, state.game_id));

    state.day_of_month() == *rng.chooose_from(&[5u32, 6, 7, 14, 15, 16, 18, 23])
}

// Stub until we get real condition parsing working
fn evaulate_condition<G: SeedGenerator>(
    //    r: &mut Rng,
    state: &PredictionGameState,
    condition: &Option<HashedString>,
) -> Option<PartialPrediction> {
    let Some(condition) = condition else {
        return Some(PartialPrediction::new_condition(
            Weather::Sun,
            1.0,
            "Default",
        ));
    };

    match condition {
        // GreenRain
        hashed_match!("IS_GREEN_RAIN_DAY") => {
            // Since conditions are evaluated as if they were the previous day,
            // `getWeatherModificationsForDate()` also check for green rain with
            // the correct day, and there can only be one green rain day per year,
            // This condition will never be valid.  We're forcing to `None` here
            // to avoid having to carry state between weather predictions.
            None
        }

        // FirstWeekSun
        hashed_match!("SEASON_DAY Spring 0 Spring 1 Spring 2 Spring 4, YEAR 1") => {
            if (state.is_season_day(Season::Spring, 0)
                || state.is_season_day(Season::Spring, 1)
                || state.is_season_day(Season::Spring, 2)
                || state.is_season_day(Season::Spring, 4))
                && state.is_year(1)
            {
                Some(PartialPrediction::new_condition(
                    Weather::Sun,
                    1.0,
                    "FirstWeekSun",
                ))
            } else {
                None
            }
        } // Sun

        // FirstWeekRain
        hashed_match!("SEASON_DAY Spring 3, YEAR 1") => {
            if state.is_season_day(Season::Spring, 3) && state.is_year(1) {
                Some(PartialPrediction::new_condition(
                    Weather::Rain,
                    1.0,
                    "FirstWeekRain",
                ))
            } else {
                None
            }
        }

        // SummerStorm
        hashed_match!("SEASON summer, SYNCED_SUMMER_RAIN_RANDOM, RANDOM .85") => {
            if state.is_season(Season::Summer) && synced_summer_rain_random::<G>(state) {
                Some(PartialPrediction::new_condition(
                    Weather::Storm,
                    0.85,
                    "SummerStorm",
                ))
            } else {
                None
            }
        }

        // SummerStorm2
        hashed_match!(
            "SEASON summer, SYNCED_SUMMER_RAIN_RANDOM, RANDOM .25, \
	     DAYS_PLAYED 28, !DAY_OF_MONTH 1, !DAY_OF_MONTH 2"
        ) => {
            if state.is_season(Season::Summer)
                && synced_summer_rain_random::<G>(state)
                && state.days_played > 28
                && !state.is_day_of_month(1)
                && !state.is_day_of_month(2)
            {
                Some(PartialPrediction::new_condition(
                    Weather::Storm,
                    0.25,
                    "SummerStorm2",
                ))
            } else {
                None
            }
        }

        // FallStorm
        hashed_match!(
            "SEASON spring fall, SYNCED_RANDOM day location_weather .183, \
	     RANDOM .25, DAYS_PLAYED 28, !DAY_OF_MONTH 1, !DAY_OF_MONTH 2"
        ) => {
            if (state.is_season(Season::Spring) || state.is_season(Season::Fall))
                && synced_day_random::<G>(state, "location_weather", 0.183)
                && state.days_played > 28
                && !state.is_day_of_month(1)
                && !state.is_day_of_month(2)
            {
                Some(PartialPrediction::new_condition(
                    Weather::Storm,
                    0.25,
                    "FallStorm",
                ))
            } else {
                None
            }
        }

        // WinterSnow
        hashed_match!("SEASON winter, SYNCED_RANDOM day location_weather 0.63") => {
            if state.is_season(Season::Winter)
                && synced_day_random::<G>(state, "location_weather", 0.63)
            {
                Some(PartialPrediction::new_condition(
                    Weather::Snow,
                    1.0,
                    "WinterSnow",
                ))
            } else {
                None
            }
        }

        // SummerRain
        hashed_match!("SEASON summer, SYNCED_SUMMER_RAIN_RANDOM, !DAY_OF_MONTH 1") => {
            if state.is_season(Season::Summer)
                && synced_summer_rain_random::<G>(state)
                && !state.is_day_of_month(1)
            {
                Some(PartialPrediction::new_condition(
                    Weather::Rain,
                    1.0,
                    "SummerRain",
                ))
            } else {
                None
            }
        }

        // FallRain
        hashed_match!("SEASON spring fall, SYNCED_RANDOM day location_weather 0.183") => {
            if (state.is_season(Season::Spring) || state.is_season(Season::Fall))
                && synced_day_random::<G>(state, "location_weather", 0.183)
            {
                Some(PartialPrediction::new_condition(
                    Weather::Rain,
                    1.0,
                    "FallRain",
                ))
            } else {
                None
            }
        }

        // SpringWind
        hashed_match!("DAYS_PLAYED 3, SEASON spring, RANDOM .20") => {
            if state.days_played > 3 && state.is_season(Season::Spring) {
                Some(PartialPrediction::new_condition(
                    Weather::Wind,
                    0.2,
                    "SpringWind",
                ))
            } else {
                None
            }
        }

        // FallWind
        hashed_match!("DAYS_PLAYED 3, SEASON fall, RANDOM .6") => {
            if state.days_played > 3 && state.is_season(Season::Spring) {
                Some(PartialPrediction::new_condition(
                    Weather::Wind,
                    0.3,
                    "FallWind",
                ))
            } else {
                None
            }
        }

        _ => panic!("unknown condition \"|{}\"", condition),
    }
}

pub fn predict_weather<G: SeedGenerator>(
    location: &WeatherLocation,
    state: &PredictionGameState,
) -> WeatherPrediction {
    // TODO: explore ways of returning weather that do not involve allocation.

    // See UpdateDailyWeather and getWeatherModificationsForDate

    // Weather in game is determeined by two functions: `UpdateDailyWeather()`
    // and `getWeatherModificationsForData()`.  I belive that `UpdateDailyWeather()`
    // run first when then `getWeatherModificationsForData()` is used to add some
    // hard coded modifications.  Here we process those modifications first then
    // run through the `UpdateDailyWeather()` logic if those don't match.
    //
    // Additionally many of these hard coded checks overlap with the `Default`
    // location context's weather conditions.  For accuracy's sake, no attepemt
    // has been made to optimize these checks away.

    // `getWeatherModificationsForData()` calculates a `day_offset` and uses that
    // in its calculations.  Here we assume that day_offset is always zero and
    // `state.day_played` represents the day of interest.

    // These are in reverse order that they are in the game because they return
    // early, yielding the same results.

    // Here the game checks passive festivals (aka desert festival and fishing derbies).
    // None of those are declared in a way that affect any weather so we elide the
    // calculation here.

    if state.is_festival_day() {
        return WeatherPrediction::fesival();
    }

    if state.is_season(Season::Summer) && (state.day_of_month() % 13) == 0 {
        return WeatherPrediction::storm();
    }

    if is_green_rain_day::<G>(state) {
        return WeatherPrediction::green_rain();
    }

    if state.days_played == 3 {
        return WeatherPrediction::rain();
    }

    if state.is_day_of_month(1) || state.days_played <= 4 {
        return WeatherPrediction::sun();
    }

    // Below is the logic for `UpdateDailyWeather()`.

    let mut state = state.clone();
    state.days_played -= 1;
    // `UpdateDailyWeather()` has a redundant festival and passive festival
    // checks here.  We elide those calculation because we would have returned
    // early above if they were true.

    let mut partial_predictions = Vec::new();
    let mut current_probability = 1.0;
    for condition in &location.conditions {
        if let Some(mut weather) = evaulate_condition::<G>(&state, condition) {
            let base_chance = weather.chance;
            weather.chance *= current_probability;
            current_probability *= 1.0 - base_chance;
            if weather.chance > 0.0 {
                partial_predictions.push(weather);
            }
        }
    }

    if current_probability > 0.0 {
        partial_predictions.push(PartialPrediction::new(Weather::Sun, current_probability))
    }

    WeatherPrediction::from_pratials(&partial_predictions)
}

#[cfg(test)]
mod tests {

    use crate::{rng::HashedSeedGenerator, GameData};

    use super::*;

    #[test]
    fn weather_prediction_returns_correct_restults() {
        let data =
            GameData::from_content_dir(crate::gamedata::get_game_content_path().unwrap()).unwrap();
        let location = data.location_contexts.get("Default").unwrap().into();
        let mut state = PredictionGameState {
            game_id: 7269403,
            ..Default::default()
        };

        for day in 1..(4 * 28) {
            state.days_played = day;
            let weather = predict_weather::<HashedSeedGenerator>(&location, &state);
            let expected = match (state.season(), state.day_of_month()) {
                (Season::Spring, 3) => WeatherPrediction::rain(),

                (Season::Spring, 10) => WeatherPrediction::rain(),
                (Season::Spring, 11) => WeatherPrediction::rain(),
                (Season::Spring, 13) => WeatherPrediction::fesival(),
                (Season::Spring, 18) => WeatherPrediction::rain(),
                (Season::Spring, 19) => WeatherPrediction::rain(),
                (Season::Spring, 20) => WeatherPrediction::rain(),
                (Season::Spring, 23) => WeatherPrediction::rain(),
                (Season::Spring, 24) => WeatherPrediction::fesival(),
                (Season::Spring, 28) => WeatherPrediction::rain(),

                (Season::Summer, 11) => WeatherPrediction::fesival(),
                (Season::Summer, 13) => WeatherPrediction::storm(),
                (Season::Summer, 14) => WeatherPrediction {
                    rain: 0.11250000000000002,
                    storm: 0.8875,
                    ..Default::default()
                },
                (Season::Summer, 16) => WeatherPrediction {
                    rain: 0.11250000000000002,
                    storm: 0.8875,
                    ..Default::default()
                },
                (Season::Summer, 17) => WeatherPrediction {
                    rain: 0.11250000000000002,
                    storm: 0.8875,
                    ..Default::default()
                },
                (Season::Summer, 21) => WeatherPrediction {
                    rain: 0.11250000000000002,
                    storm: 0.8875,
                    ..Default::default()
                },
                (Season::Summer, 23) => WeatherPrediction::green_rain(),
                (Season::Summer, 25) => WeatherPrediction {
                    rain: 0.11250000000000002,
                    storm: 0.8875,
                    ..Default::default()
                },
                (Season::Summer, 26) => WeatherPrediction::storm(),
                (Season::Summer, 27) => WeatherPrediction {
                    rain: 0.11250000000000002,
                    storm: 0.8875,
                    ..Default::default()
                },
                (Season::Summer, 28) => WeatherPrediction::fesival(),

                (Season::Fall, 16) => WeatherPrediction::fesival(),
                (Season::Fall, 21) => WeatherPrediction {
                    rain: 0.75,
                    storm: 0.25,
                    ..Default::default()
                },
                (Season::Fall, 27) => WeatherPrediction::fesival(),
                (Season::Fall, 28) => WeatherPrediction {
                    rain: 0.75,
                    storm: 0.25,
                    ..Default::default()
                },

                (Season::Winter, 3)
                | (Season::Winter, 4)
                | (Season::Winter, 7)
                | (Season::Winter, 9)
                | (Season::Winter, 11)
                | (Season::Winter, 13)
                | (Season::Winter, 16)
                | (Season::Winter, 22)
                | (Season::Winter, 24)
                | (Season::Winter, 26) => WeatherPrediction::snow(),
                (Season::Winter, 8) => WeatherPrediction::fesival(),
                (Season::Winter, 25) => WeatherPrediction::fesival(),

                _ => WeatherPrediction::sun(),
            };

            if weather.wind > 0.0 {
                continue;
            }

            assert_eq!(
                weather,
                expected,
                "unexpected weather on {:?} {}",
                state.season(),
                state.day_of_month()
            );
        }
    }
}
