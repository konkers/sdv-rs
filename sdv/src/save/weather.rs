use num_derive::FromPrimitive;
use roxmltree::Node;
use std::convert::TryInto;
use strum::EnumString;

use super::{Finder, SaveError, SaveResult};

#[derive(Clone, Debug, EnumString, Eq, FromPrimitive, Hash, PartialEq)]
pub enum Weather {
    Sun = 0,
    Rain = 1,
    Wind = 2,
    Storm = 3,
    Festival = 4,
    Snow = 5,
    Wedding = 6,
    GreenRain = 7,
}

#[derive(Debug)]
pub struct LocationWeather {
    pub weather_for_tomorrow: Weather,
    pub is_raining: bool,
    pub is_snowing: bool,
    pub is_lightning: bool,
    pub is_debris: bool,
}

impl LocationWeather {
    pub(crate) fn from_node<'a, 'input: 'a>(
        node: Node<'a, 'input>,
    ) -> SaveResult<'a, 'input, Self> {
        let weather_for_tomorrow_raw: String = node
            .child("weatherForTomorrow")
            .child("string")
            .try_into()?;
        let weather_for_tomorrow =
            weather_for_tomorrow_raw
                .parse::<Weather>()
                .map_err(|e| SaveError::Generic {
                    message: format!("Unknown weather {}: {e}", weather_for_tomorrow_raw),
                    node,
                })?;
        let is_raining = node.child("isRaining").child("boolean").try_into()?;
        let is_snowing = node.child("isSnowing").child("boolean").try_into()?;
        let is_lightning = node.child("isLightning").child("boolean").try_into()?;
        let is_debris = node.child("isDebrisWeather").child("boolean").try_into()?;
        Ok(LocationWeather {
            weather_for_tomorrow,
            is_raining,
            is_snowing,
            is_lightning,
            is_debris,
        })
    }

    pub fn today(&self) -> Weather {
        if self.is_snowing {
            Weather::Snow
        } else if self.is_debris {
            Weather::Wind
        } else if self.is_lightning {
            Weather::Storm
        } else if self.is_raining {
            Weather::Rain
        } else {
            Weather::Sun
        }
    }

    pub fn tomorrow(&self) -> Weather {
        // TODO: implement logic for specific day overrides of weather
        self.weather_for_tomorrow.clone()
    }
}
