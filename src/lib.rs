use nom::{
    branch::alt,
    bytes::complete::is_not,
    bytes::complete::tag,
    character::complete::{char, one_of},
    combinator::{map_res, opt, recognize, value},
    multi::{many0, many1},
    sequence::{pair, preceded, terminated, tuple},
    IResult,
};
use serde::Deserialize;

pub mod gamedata;
pub mod save;

pub use save::SaveGame;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Season {
    Spring,
    Summer,
    Fall,
    Winter,
}

impl Season {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            value(Season::Spring, tag("spring")),
            value(Season::Summer, tag("summer")),
            value(Season::Fall, tag("fall")),
            value(Season::Winter, tag("winter")),
        ))(i)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Weather {
    Sunny,
    Rainy,
    Both,
}

impl Weather {
    fn parse(i: &str) -> IResult<&str, Self> {
        alt((
            value(Weather::Sunny, tag("sunny")),
            value(Weather::Rainy, tag("rainy")),
            value(Weather::Both, tag("both")),
        ))(i)
    }
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

pub fn field(i: &str) -> IResult<&str, &str> {
    recognize(many0(is_not("/")))(i)
}

pub fn sub_field(i: &str) -> IResult<&str, &str> {
    recognize(many0(is_not(" /")))(i)
}
