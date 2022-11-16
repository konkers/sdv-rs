use anyhow::{anyhow, Result};
use indexmap::{IndexMap, IndexSet};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use roxmltree::{Children, Node, TextPos};
use std::{
    convert::{TryFrom, TryInto},
    fmt::{Debug, Display},
    hash::Hash,
    io::Read,
    str::FromStr,
};
use strum::{Display, EnumString};

use crate::common::Season;

mod location;
mod object;
mod stats;
mod weather;

pub use location::Location;
pub use object::Object;
pub use stats::Stats;
pub use weather::{LocationWeather, Weather};

#[derive(Debug, Clone)]
pub(crate) enum SaveError<'a, 'input: 'a> {
    ChildNotFound {
        name: String,
        node: Node<'a, 'input>,
    },
    Generic {
        message: String,
        node: Node<'a, 'input>,
    },
}
impl<'a, 'input: 'a> SaveError<'a, 'input> {
    fn node_loc(node: Node<'a, 'input>) -> TextPos {
        let doc = node.document();
        doc.text_pos_at(node.range().start)
    }
}

impl<'a, 'input: 'a> std::fmt::Display for SaveError<'a, 'input> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChildNotFound { name, node } => write!(
                f,
                "child element '{}' not found at {}",
                name,
                Self::node_loc(*node)
            ),
            Self::Generic { message, node } => {
                write!(f, "{} at {}", message, Self::node_loc(*node))
            }
        }
    }
}

pub(crate) type SaveResult<'a, 'input, T> = std::result::Result<T, SaveError<'a, 'input>>;

pub(crate) enum NodeFinder<'a, 'input: 'a> {
    Node(Node<'a, 'input>),
    Err(SaveError<'a, 'input>),
}

impl<'a, 'input: 'a> NodeFinder<'a, 'input> {
    pub(crate) fn child(self, name: &str) -> Self {
        if let Self::Node(node) = self {
            let nodes = node.children().find(|n| n.tag_name().name() == name);
            return match nodes {
                Some(n) => Self::Node(n),
                None => Self::Err(SaveError::ChildNotFound {
                    name: name.to_string(),
                    node: node,
                }),
            };
        }

        self
    }

    pub(crate) fn node(self) -> SaveResult<'a, 'input, Node<'a, 'input>> {
        match self {
            Self::Node(n) => Ok(n),
            Self::Err(e) => Err(e.clone()),
        }
    }

    pub(crate) fn children(self) -> SaveResult<'a, 'input, Children<'a, 'input>> {
        match self {
            Self::Node(n) => Ok(n.children()),
            Self::Err(e) => Err(e.clone()),
        }
    }

    pub(crate) fn convert<T: FromStr>(self) -> SaveResult<'a, 'input, T>
    where
        T::Err: Display,
    {
        match self {
            Self::Node(node) => {
                let text = node.text().unwrap_or("");
                text.parse().map_err(|e| SaveError::Generic {
                    message: format!(
                        "error parsing {} {}: {}",
                        std::any::type_name::<T>(),
                        text,
                        e
                    ),
                    node: node,
                })
            }
            Self::Err(e) => Err(e.clone()),
        }
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for Node<'a, 'input> {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        finder.node()
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for i32 {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        finder.convert()
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for u32 {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        finder.convert()
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for i64 {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        finder.convert()
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for f32 {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        finder.convert()
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for String {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        Ok(finder.node()?.text().unwrap_or("").to_string())
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for bool {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        finder.convert()
    }
}

pub(crate) trait Finder<'a, 'input: 'a> {
    fn child(self, name: &str) -> NodeFinder<'a, 'input>;
    fn finder(self) -> NodeFinder<'a, 'input>;
}

impl<'a, 'input: 'a> Finder<'a, 'input> for Node<'a, 'input> {
    fn child(self, name: &str) -> NodeFinder<'a, 'input> {
        let finder = NodeFinder::Node(self);
        finder.child(name)
    }

    fn finder(self) -> NodeFinder<'a, 'input> {
        NodeFinder::Node(self)
    }
}

pub(crate) fn array_of<'a, 'input, T: FromStr>(
    node: Node<'a, 'input>,
    array_node_name: &str,
    value_node_name: &str,
) -> SaveResult<'a, 'input, Vec<T>>
where
    T::Err: Display,
{
    let array_node = node.child(array_node_name).node()?;
    let vals: SaveResult<'a, 'input, Vec<T>> = array_node
        .children()
        .filter(|n| n.tag_name().name() == value_node_name)
        .map(|n| -> SaveResult<'a, 'input, T> {
            let text = n.text().unwrap_or("");
            let val: T = text.parse().map_err(|e| SaveError::Generic {
                message: format!("error parsing array value {}: {}", text, e),
                node: n,
            })?;

            Ok(val)
        })
        .collect();

    Ok(vals?)
}

pub(crate) fn array_of_i32<'a, 'input>(node: Node<'a, 'input>) -> SaveResult<'a, 'input, Vec<i32>> {
    array_of(node, "ArrayOfInt", "int")
}

pub(crate) fn array_of_bool<'a, 'input>(
    node: Node<'a, 'input>,
) -> SaveResult<'a, 'input, Vec<bool>> {
    array_of(node, "ArrayOfBoolean", "boolean")
}

pub(crate) fn map_from_node<'a, 'input: 'a, K: Eq + Hash + TryFrom<NodeFinder<'a, 'input>>, V, F>(
    node: Node<'a, 'input>,
    key_name: &str,
    parse_value: F,
) -> SaveResult<'a, 'input, IndexMap<K, V>>
where
    F: Fn(Node<'a, 'input>) -> SaveResult<'a, 'input, V>,
    <K as TryFrom<NodeFinder<'a, 'input>>>::Error: Display,
    <K as TryFrom<NodeFinder<'a, 'input>>>::Error: Debug,
{
    let vals: SaveResult<'a, 'input, IndexMap<K, V>> = node
        .children()
        .by_ref()
        .filter(|n| n.tag_name().name() == "item")
        .map(|n| -> SaveResult<'a, 'input, (K, V)> {
            let id = n
                .child("key")
                .child(key_name)
                .try_into()
                .map_err(|e| SaveError::Generic {
                    message: format!("{}", e),
                    node: n,
                })?;
            let value_node = n.child("value").try_into()?;
            let value = parse_value(value_node).map_err(|e| SaveError::Generic {
                message: format!("{}", e),
                node: n,
            })?;
            Ok((id, value))
        })
        .collect();

    Ok(vals?)
}

#[derive(Clone, EnumString, Eq, Debug, FromPrimitive, Hash, PartialEq)]
pub enum Profession {
    Rancher = 0,
    Tiller = 1,
    Coopmaster = 2,
    Shepherd = 3,
    Artisan = 4,
    Agriculturist = 5,
    Fisher = 6,
    Trapper = 7,
    Angler = 8,
    Pirate = 9,
    Mariner = 10,
    Luremaster = 11,
    Forester = 12,
    Gatherer = 13,
    Lumberjack = 14,
    Tapper = 15,
    Botanist = 16,
    Tracker = 17,
    Miner = 18,
    Geologist = 19,
    Blacksmith = 20,
    Prospector = 21,
    Excavator = 22,
    Gemologist = 23,
    Fighter = 24,
    Scout = 25,
    Brute = 26,
    Defender = 27,
    Acrobat = 28,
    Desperado = 29,
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for Profession {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        let node = finder.node()?;
        let id: i32 = node.finder().try_into()?;
        Self::from_i32(id).ok_or(SaveError::Generic {
            message: format!("unknown Profession {}", id),
            node,
        })
    }
}

#[derive(Clone, Copy, Display, EnumString, Eq, Debug, FromPrimitive, Hash, PartialEq)]
pub enum Skill {
    Farming = 0,
    Fishing = 1,
    Foraging = 2,
    Mining = 3,
    Combat = 4,
}

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub stats: Stats,
    pub fish_caught: IndexMap<i32, FishCaught>,
    pub professions: IndexSet<Profession>,
    pub experience: IndexMap<Skill, i32>,
    pub items: Vec<Object>,
}

impl Player {
    fn from_node<'a, 'input: 'a>(node: Node<'a, 'input>) -> SaveResult<'a, 'input, Self> {
        let name = node.child("name").try_into()?;
        let stats = node.child("stats").try_into()?;

        let fish_caught_node = node.child("fishCaught").try_into()?;
        let fish_caught_i32 = map_from_node(fish_caught_node, "int", array_of_i32)?;
        let mut fish_caught = IndexMap::new();
        for (id, values) in fish_caught_i32 {
            if values.len() != 2 {
                return Err(SaveError::Generic {
                    message: format!(
                        "expected fish caught to have 2 values instead of {}",
                        values.len()
                    ),
                    node: fish_caught_node,
                });
            }
            fish_caught.insert(
                id,
                FishCaught {
                    num: values[0],
                    max_size: values[1],
                },
            );
        }

        let professions: SaveResult<IndexSet<Profession>> = node
            .child("professions")
            .node()?
            .children()
            .filter(|n| n.has_tag_name("int"))
            .map(|n| -> SaveResult<Profession> { n.finder().try_into() })
            .collect();

        let xp: SaveResult<Vec<i32>> = node
            .child("experiencePoints")
            .children()?
            .filter(|n| n.has_tag_name("int"))
            .map(|n| -> SaveResult<i32> { n.finder().try_into() })
            .collect();

        let experience = xp?
            .iter()
            .enumerate()
            .filter_map(|(index, value)| {
                if let Some(skill) = Skill::from_i32(index as i32) {
                    Some((skill, *value))
                } else {
                    None
                }
            })
            .collect();

        let items = match node.child("items").node().ok() {
            Some(node) => Object::array_from_node(node)?,
            None => Vec::new(),
        };

        Ok(Player {
            name,
            stats,
            fish_caught,
            professions: professions?,
            experience,
            items,
        })
    }

    pub fn levels(&self) -> IndexMap<Skill, (i32, i32)> {
        const LEVEL_XP: &[i32] = &[0, 100, 380, 770, 1300, 2150, 3300, 4800, 6900, 10000, 15000];
        self.experience
            .iter()
            .map(|(skill, xp)| {
                let (level, _) = LEVEL_XP
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_level, level_xp)| xp >= level_xp)
                    .unwrap();
                if level == 10 {
                    (*skill, (level as i32, 0))
                } else {
                    (*skill, (level as i32, LEVEL_XP[level + 1] - xp))
                }
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct FishCaught {
    pub num: i32,
    pub max_size: i32,
}

#[derive(Debug)]
pub struct SaveGame {
    pub player: Player,
    pub locations: IndexMap<String, Location>,
    pub current_season: Season,
    pub day_of_month: i32,
    pub year: i32,
    pub weather: IndexMap<String, LocationWeather>,
}

impl SaveGame {
    pub fn from_reader(r: &mut impl Read) -> Result<Self> {
        // save files have a unicode <U+FEFF> at the start whcih is 3 bytes.
        //  Drop that.
        let mut buf = [0; 3];
        r.read_exact(&mut buf).unwrap();

        let contents = std::io::read_to_string(r)?;
        let doc = roxmltree::Document::parse(&contents)?;
        let root = doc.root();
        Self::from_node(root).map_err(|e| anyhow!("{}", e))
    }

    fn from_node<'a, 'input: 'a>(node: Node<'a, 'input>) -> SaveResult<'a, 'input, Self> {
        let save: Node = node.child("SaveGame").try_into()?;
        let player = Player::from_node(save.child("player").try_into()?)?;
        let mut locations = IndexMap::new();

        for node in save
            .child("locations")
            .node()?
            .children()
            .filter(|n| n.tag_name().name() == "GameLocation")
        {
            let location = Location::from_node(node)?;
            locations.insert(location.name.clone(), location);
        }

        let current_season = Season::from_node(save.child("currentSeason").try_into()?)?;
        let day_of_month = save.child("dayOfMonth").try_into()?;
        let year = save.child("year").try_into()?;

        let weather = map_from_node(
            save.child("locationWeather").try_into()?,
            "LocationContext",
            |node| LocationWeather::from_node(node.child("LocationWeather").try_into()?),
        )?;

        Ok(SaveGame {
            player,
            locations,
            current_season,
            day_of_month,
            year,
            weather,
        })
    }

    pub fn get_location(&self, name: &str) -> Result<&Location> {
        self.locations
            .get(name)
            .ok_or(anyhow!("Can't find location {}", name))
    }

    pub fn get_bundles(&self) -> Result<&IndexMap<i32, Vec<bool>>> {
        let location = self.get_location("CommunityCenter")?;
        location
            .bundles
            .as_ref()
            .ok_or(anyhow!("Can't find bundles in CommunityCenter"))
    }

    pub fn get_weather(&self, location: &str) -> &LocationWeather {
        self.weather
            .get(location)
            .unwrap_or(self.weather.get("Default").unwrap())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufReader};

    use super::*;

    #[test]
    fn load_save() {
        let f = File::open("test-data/Serenity_268346611").unwrap();
        let mut r = BufReader::new(f);
        let save = SaveGame::from_reader(&mut r).unwrap();
        println!("{:?}", save);
    }
}
