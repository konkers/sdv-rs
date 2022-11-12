use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use roxmltree::Node;
use std::{
    convert::{TryFrom, TryInto},
    fmt::{Debug, Display},
    hash::Hash,
    io::Read,
    str::FromStr,
};

use crate::common::Season;

mod location;
mod object;
mod weather;

pub use location::Location;
pub use weather::{LocationWeather, Weather};

pub(crate) enum NodeFinder<'a, 'input: 'a> {
    Node(Node<'a, 'input>),
    Err(anyhow::Error),
}

impl<'a, 'input: 'a> NodeFinder<'a, 'input> {
    pub(crate) fn child(mut self, name: &str) -> Self {
        if let Self::Node(node) = self {
            self = match node.children().find(|n| n.tag_name().name() == name) {
                Some(n) => Self::Node(n),
                None => Self::Err(anyhow!("can't find {} element", name)),
            };
        }

        self
    }

    pub(crate) fn node(self) -> Result<Node<'a, 'input>> {
        match self {
            Self::Node(n) => Ok(n),
            Self::Err(e) => Err(e),
        }
    }

    pub(crate) fn convert<T: FromStr>(self) -> Result<T>
    where
        T::Err: Display,
    {
        match self {
            Self::Node(node) => {
                let text = node.text().unwrap_or("");
                text.parse().map_err(|e| {
                    anyhow!(
                        "error parsing {} {}: {}",
                        std::any::type_name::<T>(),
                        text,
                        e
                    )
                })
            }
            Self::Err(e) => Err(e),
        }
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for Node<'a, 'input> {
    type Error = anyhow::Error;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        finder.node()
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for i32 {
    type Error = anyhow::Error;
    fn try_from(finder: NodeFinder) -> Result<Self, Self::Error> {
        finder.convert()
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for i64 {
    type Error = anyhow::Error;
    fn try_from(finder: NodeFinder) -> Result<Self, Self::Error> {
        finder.convert()
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for f32 {
    type Error = anyhow::Error;
    fn try_from(finder: NodeFinder) -> Result<Self, Self::Error> {
        finder.convert()
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for String {
    type Error = anyhow::Error;
    fn try_from(finder: NodeFinder) -> Result<Self, Self::Error> {
        Ok(finder.node()?.text().unwrap_or("").to_string())
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for bool {
    type Error = anyhow::Error;
    fn try_from(finder: NodeFinder) -> Result<Self, Self::Error> {
        finder.convert()
    }
}

pub(crate) trait Finder {
    fn child<'a, 'input>(&'input self, name: &str) -> NodeFinder<'a, 'input>;
}

impl<'b, 'binput> Finder for Node<'b, 'binput> {
    fn child<'a, 'input>(&'input self, name: &str) -> NodeFinder<'a, 'input> {
        let finder = NodeFinder::Node(self.clone());
        finder.child(name)
    }
}

pub fn array_of<T: FromStr>(
    node: &Node,
    array_node_name: &str,
    value_node_name: &str,
) -> Result<Vec<T>>
where
    T::Err: Display,
{
    let array_node = node.child(array_node_name).node()?;
    let vals: Result<Vec<T>> = array_node
        .children()
        .filter(|n| n.tag_name().name() == value_node_name)
        .map(|n| -> Result<T> {
            let text = n.text().unwrap_or("");
            let val: T = text
                .parse()
                .map_err(|e| anyhow!("error parsing array value {}: {}", text, e))?;

            Ok(val)
        })
        .collect();

    Ok(vals?)
}

pub fn array_of_i32(node: &Node) -> Result<Vec<i32>> {
    array_of(node, "ArrayOfInt", "int")
}

pub fn array_of_bool(node: &Node) -> Result<Vec<bool>> {
    array_of(node, "ArrayOfBoolean", "boolean")
}

pub(crate) fn map_from_node<'a, 'input, K: Eq + Hash + TryFrom<NodeFinder<'a, 'input>>, V, F>(
    node: &'input Node,
    key_name: &str,
    parse_value: F,
) -> Result<IndexMap<K, V>>
where
    F: Fn(&Node) -> Result<V>,
    <K as TryFrom<NodeFinder<'a, 'input>>>::Error: Display,
    <K as TryFrom<NodeFinder<'a, 'input>>>::Error: Debug,
    'input: 'a,
{
    let vals: Result<IndexMap<K, V>> = node
        .children()
        .by_ref()
        .filter(|n| n.tag_name().name() == "item")
        .map(|n| -> Result<(K, V)> {
            let finder = NodeFinder::Node(n);
            let id = finder
                .child("key")
                .child(key_name)
                .try_into()
                .map_err(|e| anyhow!("can't parse key: {}", e))?;
            let value_node = n.child("value").try_into()?;
            let res = parse_value(&value_node);
            let value = res?;
            Ok((id, value))
        })
        .collect();

    Ok(vals?)
}

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub fish_caught: IndexMap<i32, FishCaught>,
}

impl Player {
    fn from_node(node: &Node) -> Result<Self> {
        let name = node.child("name").try_into()?;

        let fish_caught_node = node.child("fishCaught").try_into()?;
        let fish_caught_i32 = map_from_node(&fish_caught_node, "int", array_of_i32)?;
        let mut fish_caught = IndexMap::new();
        for (id, values) in fish_caught_i32 {
            if values.len() != 2 {
                return Err(anyhow!(
                    "expected fish caught to have 2 values instead of {}",
                    values.len()
                ));
            }
            fish_caught.insert(
                id,
                FishCaught {
                    num: values[0],
                    max_size: values[1],
                },
            );
        }

        Ok(Player { name, fish_caught })
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
        let save: Node = root.child("SaveGame").try_into()?;
        let player = Player::from_node(&save.child("player").try_into()?)?;
        let mut locations = IndexMap::new();

        for node in save
            .child("locations")
            .node()?
            .children()
            .filter(|n| n.tag_name().name() == "GameLocation")
        {
            let location = Location::from_node(&node)?;
            locations.insert(location.name.clone(), location);
        }

        let current_season = Season::from_node(&save.child("currentSeason").try_into()?)?;
        let day_of_month = save.child("dayOfMonth").try_into()?;
        let year = save.child("year").try_into()?;

        let weather = map_from_node(
            &save.child("locationWeather").try_into()?,
            "LocationContext",
            |node| LocationWeather::from_node(&node.child("LocationWeather").try_into()?),
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
