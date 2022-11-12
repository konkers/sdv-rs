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
use std::path::{Path, PathBuf};

pub mod bundle;
pub mod fish;
pub mod object;

pub use bundle::Bundle;
pub use fish::Fish;
pub use object::{Object, ObjectType};

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
        |out: &str| i32::from_str_radix(&str::replace(&out, "_", ""), 10),
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
    let (i, value) = recognize(many0(is_not(" /")))(i)?;
    let (i, _) = opt(sub_field_seperator)(i)?;
    Ok((i, value))
}

pub fn sub_field_value<'a, O2, G>(mut f: G) -> impl FnMut(&'a str) -> IResult<&'a str, O2>
where
    G: FnMut(&'a str) -> IResult<&'a str, O2>,
{
    move |input: &str| {
        let (i, o1) = sub_field.parse(input)?;
        let (_, value) = f(o1)?;

        Ok((i, value))
    }
}

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
    pub fish: IndexMap<i32, Fish>,
    pub objects: IndexMap<i32, Object>,
}

impl GameData {
    pub fn load<P: AsRef<Path>>(game_content_dir: P) -> Result<GameData> {
        let mut data_dir: PathBuf = game_content_dir.as_ref().to_path_buf();
        data_dir.push("Data");

        let mut bundle_file = data_dir.clone();
        bundle_file.push("Bundles.xnb");
        let bundles = Bundle::load(&bundle_file)?;

        let mut fish_file = data_dir.clone();
        fish_file.push("Fish.xnb");
        let fish = Fish::load(&fish_file)?;

        let mut object_file = data_dir.clone();
        object_file.push("ObjectInformation.xnb");
        let objects = Object::load(&object_file)?;

        Ok(GameData {
            bundles,
            fish,
            objects,
        })
    }

    pub fn get_object(&self, id: i32) -> Result<&Object> {
        self.objects
            .get(&id)
            .ok_or(anyhow!("Can't find game object {}", id))
    }
}
