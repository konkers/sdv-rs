use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use roxmltree::Node;
use std::io::Read;

use super::Season;

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

pub fn child_node_i32_array(node: Node, name: &str) -> Result<Vec<i32>> {
    let node = child_node(node, name)?;
    let mut vals = Vec::new();
    let array_node = child_node(node, "ArrayOfInt")?;
    for elem in array_node
        .children()
        .filter(|n| n.tag_name().name() == "int")
    {
        let text = elem.text().unwrap_or("");
        let val = text
            .parse()
            .map_err(|e| anyhow!("error parsing i32 {}: {}", text, e))?;
        vals.push(val);
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
        let mut fish_caught = IndexMap::new();
        for item in fish_caught_node
            .children()
            .filter(|n| n.tag_name().name() == "item")
        {
            let key = child_node(item, "key")?;
            let id = child_node_i32(key, "int")?;
            let values = child_node_i32_array(item, "value")?;
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
        let current_season = Season::from_node(child_node(save, "currentSeason")?)?;
        let day_of_month = child_node_i32(save, "dayOfMonth")?;
        let year = child_node_i32(save, "year")?;

        Ok(SaveGame {
            player,
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
