use anyhow::Result;
use indexmap::IndexMap;
use std::path::{Path, PathBuf};

pub mod bundle;
pub mod fish;
pub mod object;

pub use bundle::Bundle;
pub use fish::Fish;
pub use object::{Object, ObjectType};

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
}
