use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};
use std::io::Read;

use super::Season;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub name: String,
    #[serde(deserialize_with = "deserialize_fish_caught")]
    pub fish_caught: IndexMap<i32, FishCaught>,
}

#[derive(Debug, Deserialize)]
pub struct ArrayOfInt {
    pub int: Vec<i32>,
}

#[derive(Debug, Deserialize)]
pub struct FishValue {
    #[serde(alias = "ArrayOfInt")]
    pub array_of_int: ArrayOfInt,
}

#[derive(Debug, Deserialize)]
pub struct FishKey {
    pub int: i32,
}

#[derive(Debug, Deserialize)]
pub struct FishItem {
    pub key: FishKey,
    pub value: FishValue,
}

#[derive(Debug, Deserialize)]
pub struct FishCaughtProxy {
    pub item: Vec<FishItem>,
}

#[derive(Debug)]
pub struct FishCaught {
    pub num: i32,
    pub max_size: i32,
}

fn deserialize_fish_caught<'de, D>(
    deserializer: D,
) -> std::result::Result<IndexMap<i32, FishCaught>, D::Error>
where
    D: Deserializer<'de>,
{
    let proxy: FishCaughtProxy = Deserialize::deserialize(deserializer)?;
    let mut fishes = IndexMap::new();

    for i in &proxy.item {
        let id = i.key.int;
        let val = &i.value.array_of_int.int;
        let num = val[0];
        let max_size = val[1];

        fishes.insert(id, FishCaught { num, max_size });
    }

    Ok(fishes)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveGame {
    pub player: Player,
    pub current_season: Season,
    pub day_of_month: u32,
    pub year: u32,
}

impl SaveGame {
    pub fn from_reader(r: &mut impl Read) -> Result<Self> {
        // save files have a unicode <U+FEFF> at the start whcih is 3 bytes.
        //  Drop that.
        let mut buf = [0; 3];
        r.read_exact(&mut buf).unwrap();

        serde_xml_rs::from_reader(r).map_err(|e| anyhow!("can't load save: {}", e))
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
