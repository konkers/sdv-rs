use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use roxmltree::Node;
use std::{fmt::Display, hash::Hash, io::Read, str::FromStr};

use super::Season;

mod location;

pub use location::Location;

pub fn child_node<'a>(node: Node<'a, 'a>, name: &str) -> Result<Node<'a, 'a>> {
    node.children()
        .find(|n| n.tag_name().name() == name)
        .ok_or(anyhow!("can't find {} element", name))
}

pub fn child_node_text(node: Node, name: &str) -> Result<String> {
    Ok(child_node(node, name)?.text().unwrap_or("").to_string())
}

pub fn child_node_i32(node: Node, name: &str) -> Result<i32> {
    let text = child_node(node, name)?.text().unwrap_or("");
    text.parse()
        .map_err(|e| anyhow!("error parsing i32 {}: {}", text, e))
}

pub fn array_of<T: FromStr>(
    node: Node,
    array_node_name: &str,
    value_node_name: &str,
) -> Result<Vec<T>>
where
    T::Err: Display,
{
    let mut vals = Vec::new();
    let array_node = child_node(node, array_node_name)?;
    for elem in array_node
        .children()
        .filter(|n| n.tag_name().name() == value_node_name)
    {
        let text = elem.text().unwrap_or("");
        let val = text
            .parse()
            .map_err(|e| anyhow!("error parsing array value {}: {}", text, e))?;
        vals.push(val);
    }

    Ok(vals)
}

pub fn array_of_i32(node: Node) -> Result<Vec<i32>> {
    array_of(node, "ArrayOfInt", "int")
}

pub fn array_of_bool(node: Node) -> Result<Vec<bool>> {
    array_of(node, "ArrayOfBoolean", "boolean")
}

pub fn map_from_node<K: Eq + Hash + FromStr, V, F>(
    node: Node,
    key_name: &str,
    parse_value: F,
) -> Result<IndexMap<K, V>>
where
    F: Fn(Node) -> Result<V>,
    K::Err: Display,
{
    let mut vals = IndexMap::new();
    for item in node.children().filter(|n| n.tag_name().name() == "item") {
        let key = child_node(item, "key")?;
        let id_text = child_node(key, key_name)?.text().unwrap_or("");
        let id = id_text
            .parse()
            .map_err(|e| anyhow!("error parsing key {}: {}", id_text, e))?;
        let value_node = child_node(item, "value")?;
        let value = parse_value(value_node)?;
        vals.insert(id, value);
    }

    Ok(vals)
}

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub fish_caught: IndexMap<i32, FishCaught>,
}

impl Player {
    fn from_node(node: Node) -> Result<Self> {
        let name = child_node_text(node, "name")?;

        let fish_caught_node = child_node(node, "fishCaught")?;
        let fish_caught_i32 = map_from_node(fish_caught_node, "int", array_of_i32)?;
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
        let save = child_node(root, "SaveGame")?;
        let player = Player::from_node(child_node(save, "player")?)?;
        let locations = child_node(save, "locations")?
            .children()
            .filter_map(|n| {
                if n.tag_name().name() == "GameLocation" {
                    Location::from_node(n).map(|l| (l.name.clone(), l)).ok()
                } else {
                    None
                }
            })
            .collect();
        let current_season = Season::from_node(child_node(save, "currentSeason")?)?;
        let day_of_month = child_node_i32(save, "dayOfMonth")?;
        let year = child_node_i32(save, "year")?;

        Ok(SaveGame {
            player,
            locations,
            current_season,
            day_of_month,
            year,
        })
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
