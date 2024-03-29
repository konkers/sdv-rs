use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use nom::{
    branch::alt,
    bytes::complete::is_not,
    bytes::complete::tag,
    character::complete::{char, one_of},
    combinator::{map_res, opt, recognize},
    multi::{many0, many1},
    sequence::{pair, preceded, terminated, tuple},
    IResult, Parser,
};
use std::{collections::HashMap, path::Path};

pub mod bundle;
pub mod character;
pub mod fish;
pub mod npc_gift_tastes;
pub mod object;
// Needs to be updated for Serde
// pub mod map;
// pub mod texture;

pub use bundle::Bundle;
pub use character::CharacterData;
pub use fish::Fish;
pub use npc_gift_tastes::NpcGiftTastes;
pub use object::ObjectData;


// Needs to be updated for Serde
// pub use map::{Map, Tile};
// pub use texture::Texture;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectTaste {
    Love,
    Like,
    Neutral,
    Dislike,
    Hate,
}

fn field_seperator(input: &str) -> IResult<&str, ()> {
    let (i, _) = opt(tag("/"))(input)?;
    Ok((i, ()))
}

fn sub_field_seperator(input: &str) -> IResult<&str, ()> {
    let (i, _) = opt(alt((tag("/"), tag(" "))))(input)?;
    Ok((i, ()))
}

fn decimal(input: &str) -> IResult<&str, i32> {
    map_res(
        recognize(pair(
            opt(char('-')),
            many1(terminated(one_of("0123456789"), many0(char('_')))),
        )),
        |out: &str| str::replace(out, "_", "").parse::<i32>(),
    )(input)
}

fn float(input: &str) -> IResult<&str, f32> {
    map_res(
        alt((
            // Case one: .42
            recognize(tuple((
                char('.'),
                decimal,
                opt(tuple((one_of("eE"), opt(one_of("+-")), decimal))),
            ))), // Case two: 42e42 and 42.42e42
            recognize(tuple((
                decimal,
                opt(preceded(char('.'), decimal)),
                one_of("eE"),
                opt(one_of("+-")),
                decimal,
            ))), // Case three: 42. and 42.42
            recognize(tuple((decimal, char('.'), opt(decimal)))),
            // Case four: 42
            recognize(decimal),
        )),
        |out: &str| out.parse::<f32>(),
    )(input)
}

fn field(i: &str) -> IResult<&str, &str> {
    let (i, value) = recognize(many0(is_not("/")))(i)?;
    let (i, _) = opt(field_seperator)(i)?;
    Ok((i, value))
}

pub fn field_value<'a, O2, G>(mut f: G) -> impl FnMut(&'a str) -> IResult<&'a str, O2>
where
    G: FnMut(&'a str) -> IResult<&'a str, O2>,
{
    move |input: &str| {
        let (i, o1) = field.parse(input)?;
        let (_, value) = f(o1)?;

        Ok((i, value))
    }
}

fn sub_field(i: &str) -> IResult<&str, &str> {
    let (i, value) = recognize(many1(is_not(" /")))(i)?;
    let (i, _) = opt(sub_field_seperator)(i)?;
    Ok((i, value))
}

pub fn sub_field_value<'a, O2, G>(mut f: G) -> impl FnMut(&'a str) -> IResult<&'a str, O2>
where
    G: FnMut(&'a str) -> IResult<&'a str, O2>,
{
    move |input: &str| {
        let (i, o1) = sub_field(input)?;
        let (_, value) = f(o1)?;

        Ok((i, value))
    }
}

#[allow(dead_code)]
fn remaining_fields<'a>(i: &'a str) -> IResult<&'a str, Vec<String>> {
    let (i, fields) = many0(|i: &'a str| {
        let (i, value) = recognize(many1(is_not("/")))(i)?;
        let (i, _) = opt(field_seperator)(i)?;
        Ok((i, value))
    })(i)?;

    Ok((i, fields.iter().map(|s| s.to_string()).collect()))
}
pub struct GameData {
    pub bundles: IndexMap<i32, Bundle>,
    pub fish: IndexMap<String, Fish>,
    pub objects: IndexMap<String, ObjectData>,
    pub characters: IndexMap<String, CharacterData>,
    pub npc_gift_tastes: IndexMap<String, NpcGiftTastes>,
    object_name_map: HashMap<String, String>,
}

impl GameData {
    pub fn load<P: AsRef<Path>>(game_content_dir: P) -> Result<GameData> {
        let game_content_dir = game_content_dir.as_ref().to_path_buf();
        let mut data_dir = game_content_dir.clone();
        data_dir.push("Data");

        let mut character_file = data_dir.clone();
        character_file.push("Characters.xnb");
        let characters = character::load_characters(&character_file)?;

        let mut bundle_file = data_dir.clone();
        bundle_file.push("Bundles.xnb");
        let bundles = Bundle::load(&bundle_file)?;

        let mut fish_file = data_dir.clone();
        fish_file.push("Fish.xnb");
        let fish = Fish::load(&fish_file)?;

        let mut object_file = data_dir.clone();
        object_file.push("Objects.xnb");
        let objects = object::load_objects(&object_file)?;

        let mut npc_gift_tastes_file = data_dir.clone();
        npc_gift_tastes_file.push("NPCGiftTastes.xnb");
        let npc_gift_tastes = NpcGiftTastes::load(&npc_gift_tastes_file)?;

        let object_name_map = objects
            .iter()
            .map(|(id, object)| (object.name.clone(), id.clone()))
            .collect();

        Ok(GameData {
            bundles,
            fish,
            objects,
            characters,
            object_name_map,
            npc_gift_tastes,
        })
    }

    pub fn get_object(&self, id: &str) -> Result<&ObjectData> {
        self.objects
            .get(id)
            .ok_or(anyhow!("Can't find game object {}", id))
    }

    pub fn get_object_by_name(&self, name: &str) -> Result<&ObjectData> {
        let id = self
            .object_name_map
            .get(name)
            .ok_or(anyhow!("Can't find game object {}", name))?;
        self.get_object(id)
    }

    // pub fn load_map<P: AsRef<Path>>(&self, path: P) -> Result<Map> {
    //     let mut map_path = self.game_content_dir.clone();
    //     map_path.push(path);

    //     Map::load(map_path)
    // }

    // pub fn load_texture<P: AsRef<Path>>(&self, path: P) -> Result<Texture> {
    //     let mut texture_path = self.game_content_dir.clone();
    //     texture_path.push(path);

    //     Texture::load(texture_path)
    // }

    pub fn lookup_npc_taste_for_object(
        &self,
        npc: &String,
        object: &ObjectData,
    ) -> Result<ObjectTaste> {
        let mut taste = ObjectTaste::Neutral;
        let mut has_universal_neutral_id = false;

        let npc_tastes = self
            .npc_gift_tastes
            .get(npc)
            .ok_or_else(|| anyhow!("can't find gift taste data for {npc}"))?;
        let universal_tastes = self
            .npc_gift_tastes
            .get("Universal")
            .ok_or_else(|| anyhow!("can't find universal gift taste data"))?;

        if universal_tastes.love.has_category(&object.category) {
            taste = ObjectTaste::Love;
        } else if universal_tastes.hate.has_category(&object.category) {
            taste = ObjectTaste::Hate;
        } else if universal_tastes.like.has_category(&object.category) {
            taste = ObjectTaste::Like;
        } else if universal_tastes.dislike.has_category(&object.category) {
            taste = ObjectTaste::Dislike;
        }

        if universal_tastes.love.has_item(&object.id) {
            taste = ObjectTaste::Love;
        } else if universal_tastes.hate.has_item(&object.id) {
            taste = ObjectTaste::Hate;
        } else if universal_tastes.like.has_item(&object.id) {
            taste = ObjectTaste::Like;
        } else if universal_tastes.dislike.has_item(&object.id) {
            taste = ObjectTaste::Dislike;
        } else if universal_tastes.neutral.has_item(&object.id) {
            taste = ObjectTaste::Neutral;
            has_universal_neutral_id = true;
        }

        if taste == ObjectTaste::Neutral && !has_universal_neutral_id {
            if object.edibility > -300 && object.edibility < 0 {
                taste = ObjectTaste::Hate;
            } else if object.price < 20 {
                taste = ObjectTaste::Dislike;
            }
        }

        if npc_tastes.love.has_item(&object.id) || npc_tastes.love.has_category(&object.category) {
            taste = ObjectTaste::Love;
        } else if npc_tastes.hate.has_item(&object.id)
            || npc_tastes.hate.has_category(&object.category)
        {
            taste = ObjectTaste::Hate;
        } else if npc_tastes.like.has_item(&object.id)
            || npc_tastes.like.has_category(&object.category)
        {
            taste = ObjectTaste::Like;
        } else if npc_tastes.dislike.has_item(&object.id)
            || npc_tastes.like.has_category(&object.category)
        {
            taste = ObjectTaste::Dislike;
        } else if npc_tastes.neutral.has_item(&object.id)
            || npc_tastes.neutral.has_category(&object.category)
        {
            taste = ObjectTaste::Neutral;
        }

        Ok(taste)
    }
}
