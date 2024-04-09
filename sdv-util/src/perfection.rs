use std::{fs::File, io::BufReader};

use sdv::{analyzer::perfection::analyze_perfection, GameData, SaveGame};

use crate::{GameAndSaveOpt, Result};

pub fn cmd_perfection(opt: &GameAndSaveOpt) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;
    let f = File::open(&opt.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;

    let analysis = analyze_perfection(&data, &save);
    println!("{analysis:#?}");
    Ok(())
}
