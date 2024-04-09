pub mod analyzer;
pub mod common;
pub mod gamedata;
pub mod predictor;
pub mod rng;
pub mod save;

pub use gamedata::{GameData, Locale};
pub use save::SaveGame;

pub trait FromJsonReader
where
    Self: Sized,
{
    fn from_json_reader<R: std::io::Read>(reader: R) -> anyhow::Result<Self>;
}
