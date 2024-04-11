use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use nom::{branch::alt, bytes::complete::tag, combinator::value, multi::many1, IResult};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use super::{decimal, field, field_value, float, sub_field_value};
use crate::common::{Season, Weather};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum FishBehavior {
    Mixed,
    Smooth,
    Sinker,
    Floater,
    Dart,
}

impl FishBehavior {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            value(FishBehavior::Mixed, tag("mixed")),
            value(FishBehavior::Smooth, tag("smooth")),
            value(FishBehavior::Sinker, tag("sinker")),
            value(FishBehavior::Floater, tag("floater")),
            value(FishBehavior::Dart, tag("dart")),
        ))(i)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, strum::Display)]
pub enum TrapLocation {
    Ocean,
    Freshwater,
}

impl TrapLocation {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            value(TrapLocation::Ocean, tag("ocean/")),
            value(TrapLocation::Freshwater, tag("freshwater/")),
        ))(i)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BaitAffinity {
    bait_id: i32,
    affinity: f32,
}

impl BaitAffinity {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, bait_id) = sub_field_value(decimal)(i)?;
        let (i, affinity) = sub_field_value(float)(i)?;

        Ok((i, BaitAffinity { bait_id, affinity }))
    }

    fn parse_list(i: &str) -> IResult<&str, Vec<Self>> {
        alt((value(vec![], tag("-1")), many1(BaitAffinity::parse)))(i)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TimeSpan {
    start: i32,
    end: i32,
}

impl TimeSpan {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, start) = sub_field_value(decimal)(i)?;
        let (i, end) = sub_field_value(decimal)(i)?;

        Ok((i, TimeSpan { start, end }))
    }

    fn fmt_time(f: &mut std::fmt::Formatter<'_>, time: i32) -> std::fmt::Result {
        let time = time % 2400;
        let (time, meridiem) = if time < 1200 {
            (time, "am")
        } else if time < 1300 {
            (time, "pm")
        } else {
            (time - 1200, "pm")
        };
        f.write_fmt(format_args!(
            "{:02}:{:02}{}",
            time / 100,
            time % 100,
            meridiem
        ))
    }
}

impl Display for TimeSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Self::fmt_time(f, self.start)?;
        f.write_str("-")?;
        Self::fmt_time(f, self.end)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Fish {
    Line {
        name: String,
        difficulty: i32,
        behavior: FishBehavior,
        min_size: i32,
        max_size: i32,
        times: Vec<TimeSpan>,
        seasons: Vec<Season>,
        weather: Weather,
        bait_affinity: Vec<BaitAffinity>,
        min_depth: i32,
        spawn_mult: f32,
        depth_mult: f32,
        min_level: i32,
    },
    Trap {
        name: String,
        weight: f32,
        bait_affinity: Vec<BaitAffinity>,
        location: TrapLocation,
        min_size: i32,
        max_size: i32,
    },
}

impl Fish {
    pub fn load<P: AsRef<Path>>(file: P) -> Result<IndexMap<String, Self>> {
        let f = File::open(file).context("Can't open fish file")?;
        let mut r = BufReader::new(f);
        let mut data: Vec<u8> = Vec::new();
        r.read_to_end(&mut data)?;

        let entries: IndexMap<String, String> = xnb::from_bytes(&data)?;
        let mut fishes = IndexMap::new();
        for (k, v) in &entries {
            let (_, fish) =
                Self::parse(v).map_err(|e| anyhow!("Error parsing fish \"{}\": {}", v, e))?;

            fishes.insert(k.clone(), fish);
        }

        Ok(fishes)
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Line { name, .. } => name,
            Self::Trap { name, .. } => name,
        }
    }

    pub fn in_season(&self, season: &Season) -> bool {
        match self {
            Self::Line { seasons, .. } => {
                for s in seasons {
                    if s == season {
                        return true;
                    }
                }
                false
            }
            Self::Trap { .. } => true,
        }
    }

    pub fn is_line_fish(&self) -> bool {
        matches!(self, Self::Line { .. })
    }

    pub fn is_pot_fish(&self) -> bool {
        matches!(self, Self::Trap { .. })
    }

    fn parse(i: &str) -> IResult<&str, Self> {
        alt((Self::parse_trap, Self::parse_line))(i)
    }

    fn parse_line(i: &str) -> IResult<&str, Self> {
        let (i, name) = field(i)?;
        let (i, difficulty) = field_value(decimal)(i)?;
        let (i, behavior) = field_value(FishBehavior::parse)(i)?;
        let (i, min_size) = field_value(decimal)(i)?;
        let (i, max_size) = field_value(decimal)(i)?;
        let (i, times) = many1(TimeSpan::parse)(i)?;
        let (i, seasons) = many1(sub_field_value(Season::parse))(i)?;
        let (i, weather) = field_value(Weather::parse)(i)?;
        let (i, bait_affinity) = field_value(BaitAffinity::parse_list)(i)?;
        let (i, min_depth) = field_value(decimal)(i)?;
        let (i, spawn_mult) = field_value(float)(i)?;
        let (i, depth_mult) = field_value(float)(i)?;
        let (i, min_level) = field_value(decimal)(i)?;

        // The legendary fishes are locked to seasons through a different
        // method than the XNB data.  We fix them up here.
        let seasons = match name {
            "Crimsonfish" => vec![Season::Summer],
            "Angler" => vec![Season::Fall],
            "Legend" => vec![Season::Spring],
            "Glacierfish" => vec![Season::Winter],
            _ => seasons,
        };

        Ok((
            i,
            Fish::Line {
                name: name.to_string(),
                difficulty,
                behavior,
                min_size,
                max_size,
                times,
                seasons,
                weather,
                bait_affinity,
                min_depth,
                spawn_mult,
                depth_mult,
                min_level,
            },
        ))
    }

    fn parse_trap(i: &str) -> IResult<&str, Self> {
        let (i, name) = field(i)?;
        let (i, _) = tag("trap/")(i)?;
        let (i, weight) = float(i)?;
        let (i, _) = tag("/")(i)?;
        let (i, bait_affinity) = BaitAffinity::parse_list(i)?;
        let (i, location) = TrapLocation::parse(i)?;
        let (i, min_size) = decimal(i)?;
        let (i, _) = tag("/")(i)?;
        let (i, max_size) = decimal(i)?;

        Ok((
            i,
            Fish::Trap {
                name: name.to_string(),
                weight,
                bait_affinity,
                location,
                min_size,
                max_size,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fish() {
        assert_eq!(
            Fish::parse(
                "Pufferfish/80/floater/1/36/1200 1600/summer/sunny/690 .4 685 .1/4/.3/.5/0"
            )
            .unwrap(),
            (
                "",
                Fish::Line {
                    name: "Pufferfish".to_string(),
                    difficulty: 80,
                    behavior: FishBehavior::Floater,
                    min_size: 1,
                    max_size: 36,
                    times: vec![TimeSpan {
                        start: 1200,
                        end: 1600
                    }],
                    seasons: vec![Season::Summer],
                    weather: Weather::Sunny,
                    bait_affinity: vec![
                        BaitAffinity {
                            bait_id: 690,
                            affinity: 0.4
                        },
                        BaitAffinity {
                            bait_id: 685,
                            affinity: 0.1
                        },
                    ],
                    min_depth: 4,
                    spawn_mult: 0.3,
                    depth_mult: 0.5,
                    min_level: 0,
                }
            )
        );

        assert_eq!(
            Fish::parse("Largemouth Bass/50/mixed/11/30/600 1900/spring summer fall winter/both/685 .35/3/.4/.2/0").unwrap(),
            (
                "",
                Fish::Line {
                    name: "Largemouth Bass".to_string(),
                    difficulty: 50,
                    behavior: FishBehavior::Mixed,
                    min_size: 11,
                    max_size: 30,
                    times: vec![TimeSpan {
                        start: 600,
                        end: 1900
                    }],
                    seasons: vec![
                        Season::Spring,
                        Season::Summer,
                        Season::Fall,
                        Season::Winter,
                    ],
                    weather: Weather::Both,
                    bait_affinity: vec![
                        BaitAffinity {
                            bait_id: 685,
                            affinity: 0.35,
                        },
                    ],
                    min_depth: 3,
                    spawn_mult: 0.4,
                    depth_mult: 0.2,
                    min_level: 0,
                }
            )
        );

        assert_eq!(
            Fish::parse("Lobster/trap/.05/688 .45 689 .35 690 .35/ocean/2/20").unwrap(),
            (
                "",
                Fish::Trap {
                    name: "Lobster".to_string(),
                    weight: 0.05,
                    bait_affinity: vec![
                        BaitAffinity {
                            bait_id: 688,
                            affinity: 0.45
                        },
                        BaitAffinity {
                            bait_id: 689,
                            affinity: 0.35
                        },
                        BaitAffinity {
                            bait_id: 690,
                            affinity: 0.35
                        },
                    ],
                    location: TrapLocation::Ocean,
                    min_size: 2,
                    max_size: 20,
                }
            )
        );

        assert_eq!(
            Fish::parse(
                "Green Algae/5/floater/5/30/600 2600/spring summer fall winter/both/-1/0/.3/0/0"
            )
            .unwrap(),
            (
                "",
                Fish::Line {
                    name: "Green Algae".to_string(),
                    difficulty: 5,
                    behavior: FishBehavior::Floater,
                    min_size: 5,
                    max_size: 30,
                    times: vec![TimeSpan {
                        start: 600,
                        end: 2600
                    }],
                    seasons: vec![Season::Spring, Season::Summer, Season::Fall, Season::Winter],
                    weather: Weather::Both,
                    bait_affinity: vec![],
                    min_depth: 0,
                    spawn_mult: 0.3,
                    depth_mult: 0.0,
                    min_level: 0
                }
            )
        );
    }
}
