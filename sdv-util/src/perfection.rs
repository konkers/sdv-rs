use sdv::GameData;

use crate::{GameAndSaveOpt, Result};

pub fn cmd_perfection(opt: &GameAndSaveOpt) -> Result<()> {
    let data = GameData::load(opt.content.get()?)?;
    let locale = sdv::gamedata::locale::Locale::load(opt.content.get()?, "en-EN")?;

    for (_, object) in &data.objects {
        if object.is_potential_basic_shipped() {
            println!("{}", object.display_name(&locale));
        }
    }
    Ok(())
}
