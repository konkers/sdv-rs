use anyhow::{anyhow, Result};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use roxmltree::Node;
use std::convert::TryInto;

use super::Finder;

#[derive(Clone, Eq, Debug, FromPrimitive, Hash, PartialEq)]
pub enum Weather {
    Sunny = 0,
    Rain = 1,
    Windy = 2,
    Lightning = 3,
    Festival = 4,
    Snow = 5,
    Wedding = 6,
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
    pub(crate) fn from_node(node: &Node) -> Result<Self> {
        let weather_for_tomorrow_raw = node.child("weatherForTomorrow").child("int").try_into()?;
        let weather_for_tomorrow = Weather::from_i32(weather_for_tomorrow_raw)
            .ok_or(anyhow!("Unknown weather {}", weather_for_tomorrow_raw))?;
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
            Weather::Windy
        } else if self.is_lightning {
            Weather::Lightning
        } else if self.is_raining {
            Weather::Snow
        } else {
            Weather::Sunny
        }
    }

    pub fn tomorrow(&self) -> Weather {
        // TODO: implement logic for specific day overrides of weather
        self.weather_for_tomorrow.clone()
    }
}
