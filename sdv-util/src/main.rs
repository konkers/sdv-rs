use anyhow::{anyhow, Result};
use crossterm::style::Color::*;
use sdv::{
    common::{ObjectCategory, Point},
    gamedata::GameData,
    save::Object,
    SaveGame,
};
use std::{collections::HashSet, fs::File, io::BufReader, iter::FromIterator, path::PathBuf};
use structopt::StructOpt;
use termimad::*;

#[derive(Debug, StructOpt)]
struct GameContentLoc {
    /// Path to Stardew Valley's Content directory
    #[structopt(long, parse(from_os_str))]
    game_content: PathBuf,
}

#[derive(Debug, StructOpt)]
struct GameAndSaveOpt {
    #[structopt(flatten)]
    content: GameContentLoc,

    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

#[derive(Debug, StructOpt)]
struct SaveFileLoc {
    file: PathBuf,
}

#[derive(Debug, StructOpt)]
enum DumpOpt {
    Bundles(GameContentLoc),
    Fish(GameContentLoc),
    Objects(GameContentLoc),
    Save(SaveFileLoc),
}

#[derive(Debug, StructOpt)]
struct ItemsOpt {
    #[structopt(flatten)]
    loc: GameAndSaveOpt,

    #[structopt(long)]
    all: bool,
}

#[derive(Debug, StructOpt)]
enum Opt {
    Bundles(GameAndSaveOpt),
    Dump(DumpOpt),
    Fish(GameAndSaveOpt),
    Items(ItemsOpt),
    Todo(GameAndSaveOpt),
}

fn cmd_fish(opt: &GameAndSaveOpt) -> Result<()> {
    let data = GameData::load(&opt.content.game_content)?;
    let f = File::open(&opt.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;

    println!(
        "Today is {:?} {} year {}.  Available, uncaught fish:",
        &save.current_season, &save.day_of_month, &save.year
    );

    for (_id, fish) in data
        .fish
        .iter()
        .filter(|(id, _fish)| !save.player.fish_caught.contains_key(*id))
        .filter(|(_id, fish)| fish.in_season(&save.current_season))
    {
        println!("  {}", &fish.name());
    }

    println!("\nunavailable, uncaught:");
    for (_id, fish) in data
        .fish
        .iter()
        .filter(|(id, _fish)| !save.player.fish_caught.contains_key(*id))
        .filter(|(_id, fish)| !fish.in_season(&save.current_season))
    {
        println!("  {}", &fish.name());
    }
    Ok(())
}

#[derive(Debug)]
enum ItemLocation {
    Player,
    Map(String, Point<i32>),
    Chest(String, Point<i32>),
}
impl std::fmt::Display for ItemLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Player => write!(f, "Player"),
            Self::Map(name, loc) => write!(f, "{}: {}, {}", name, loc.x, loc.y),
            Self::Chest(name, loc) => write!(f, "{} Chest: {}, {}", name, loc.x, loc.y),
        }
    }
}

struct Item<'a> {
    object: &'a Object,
    location: ItemLocation,
}

fn cmd_items(opt: &ItemsOpt) -> Result<()> {
    let _data = GameData::load(&opt.loc.content.game_content)?;
    let f = File::open(&opt.loc.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;

    let mut items = Vec::new();

    for item in &save.player.items {
        items.push(Item {
            object: item,
            location: ItemLocation::Player,
        });
    }

    for (name, location) in &save.locations {
        for (pos, object) in &location.objects {
            if let Some(chest_items) = &object.items {
                for item in chest_items {
                    items.push(Item {
                        object: item,
                        location: ItemLocation::Chest(name.clone(), *pos),
                    });
                }
            } else if opt.all {
                items.push(Item {
                    object: object,
                    location: ItemLocation::Map(name.clone(), *pos),
                });
            }
        }
    }

    items.sort_by(|a, b| {
        a.object
            .stack_price()
            .partial_cmp(&b.object.stack_price())
            .unwrap()
    });

    let total: i64 = items
        .iter()
        .map(|item| item.object.stack_price() as i64)
        .sum();

    let mut skin = MadSkin::default();
    skin.set_headers_fg(rgb(255, 187, 0));
    skin.bold.set_fg(Yellow);
    skin.italic.set_fgbg(Magenta, rgb(30, 30, 40));
    skin.paragraph.align = Alignment::Center;
    skin.table.align = Alignment::Center;

    let mut text = "|:-:|:-:|:-:|:-:|\n".to_string();
    text.push_str("|**Name**|**Qty**|**Price**|**Location**|\n");
    text.push_str("|:-|:-|:-|-\n");

    for item in items {
        text.push_str(&format!(
            "|{} | {} | {} |{} |\n",
            item.object.name,
            item.object.stack,
            item.object.price.unwrap_or(-1) * item.object.stack,
            item.location,
        ));
    }
    text.push_str("|:-|:-|:-|-\n");
    text.push_str(&format!("|**Total**||{}||\n", total));
    text.push_str("|-\n");
    println!("{}", skin.term_text(&text));
    Ok(())
}

fn cmd_bundles(opt: &GameAndSaveOpt) -> Result<()> {
    let data = GameData::load(&opt.content.game_content)?;

    let f = File::open(&opt.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;
    let save_bundles = save.get_bundles()?;

    for (id, bundle) in &data.bundles {
        let bundle_state = save_bundles
            .get(id)
            .ok_or(anyhow!("Can't get bundle state for {}", &id))?;

        let mut completed = 0;
        for (index, item) in bundle.requirements.iter().enumerate() {
            if let Ok(_) = data.get_object(item.id) {
                if bundle_state[index] {
                    completed += 1;
                }
            }
        }

        println!("{}: {}/{}", bundle.name, completed, bundle.num_items_needed);

        for (index, item) in bundle.requirements.iter().enumerate() {
            if let Ok(object) = data.get_object(item.id) {
                let found = bundle_state[index];
                println!("  {}: {}", object.name, found);
            }
        }
    }
    Ok(())
}

fn cmd_todo(opt: &GameAndSaveOpt) -> Result<()> {
    let _data = GameData::load(&opt.content.game_content)?;
    let f = File::open(&opt.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;

    println!(
        "Today is {:?} {} year {}.",
        &save.current_season, &save.day_of_month, &save.year
    );
    let default_weather = save.get_weather("Default");
    let island_weather = save.get_weather("Island");
    println!("Today's weather:");
    println!("  Farm: {:?}", default_weather.today());
    println!("  Island: {:?}", island_weather.today());
    println!("Tomorrow's weather:");
    println!("  Farm: {:?}", default_weather.tomorrow());
    println!("  Island: {:?}", island_weather.tomorrow());
    Ok(())
}

fn cmd_dump_bundles(opt: &GameContentLoc) -> Result<()> {
    let data = GameData::load(&opt.game_content)?;

    for bundle in &data.bundles {
        println!("{:?}", &bundle);
    }

    Ok(())
}

fn cmd_dump_fish(opt: &GameContentLoc) -> Result<()> {
    let data = GameData::load(&opt.game_content)?;

    for (id, fish) in &data.fish {
        println!("{}: {:?}", id, &fish);
    }

    Ok(())
}

fn cmd_dump_objects(opt: &GameContentLoc) -> Result<()> {
    let data = GameData::load(&opt.game_content)?;

    for (id, object) in &data.objects {
        println!("{}: {:?}", id, &object);
    }

    let cat_set: HashSet<Option<ObjectCategory>> =
        HashSet::from_iter(data.objects.iter().map(|o| o.1.category.clone()));
    let cats: Vec<Option<ObjectCategory>> = cat_set.iter().cloned().collect();

    println!("types: {:?}", &cats);

    Ok(())
}

fn cmd_dump_save(opt: &SaveFileLoc) -> Result<()> {
    let f = File::open(&opt.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;

    println!("{:#?}", &save);

    Ok(())
}

fn cmd_dump(opt: &DumpOpt) -> Result<()> {
    match opt {
        DumpOpt::Bundles(o) => cmd_dump_bundles(&o),
        DumpOpt::Fish(o) => cmd_dump_fish(&o),
        DumpOpt::Objects(o) => cmd_dump_objects(&o),
        DumpOpt::Save(o) => cmd_dump_save(&o),
    }
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    match opt {
        Opt::Dump(o) => cmd_dump(&o)?,
        Opt::Bundles(o) => cmd_bundles(&o)?,
        Opt::Fish(o) => cmd_fish(&o)?,
        Opt::Items(o) => cmd_items(&o)?,
        Opt::Todo(o) => cmd_todo(&o)?,
    }

    Ok(())
}
