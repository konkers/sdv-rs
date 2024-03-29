use ::crossterm::style::Color::*;
use anyhow::{anyhow, Result};
use itertools::Itertools;
use sdv::{
    common::{DayOfWeek, ObjectCategory, Point, Season},
    gamedata::{GameData, ObjectTaste},
    //predictor::{Geode, GeodeType},
    save::Object,
    SaveGame,
};
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    fmt::Write,
    fs::File,
    io::BufReader,
    iter::FromIterator,
    path::PathBuf,
};
use structopt::StructOpt;
use termimad::{rgb, Alignment, MadSkin};

// Needs to be updated for serde.
// mod render_map;
// use render_map::cmd_render_map;

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
    Characters(GameContentLoc),
    Fish(GameContentLoc),
    Objects(GameContentLoc),
    NpcGiftTastes(GameContentLoc),
    Save(SaveFileLoc),
}

#[derive(Debug, StructOpt)]
struct ItemsOpt {
    #[structopt(flatten)]
    loc: GameAndSaveOpt,

    #[structopt(long)]
    csv: bool,

    #[structopt(long)]
    all: bool,
}

#[derive(Debug, StructOpt)]
#[allow(unused)]
struct RenderMapOpt {
    #[structopt(flatten)]
    content: GameContentLoc,

    map_name: String,
}

#[derive(Debug, StructOpt)]
enum Opt {
    Bundles(GameAndSaveOpt),
    Dump(DumpOpt),
    Fish(GameAndSaveOpt),
    Food(GameAndSaveOpt),
    //  Geodes(GameAndSaveOpt),
    Items(ItemsOpt),
    //RenderMap(RenderMapOpt),
    Todo(GameAndSaveOpt),
}

fn mad_skin() -> MadSkin {
    let mut skin = MadSkin::default();
    skin.set_headers_fg(rgb(255, 187, 0));
    skin.bold.set_fg(Yellow);
    skin.italic.set_fgbg(Magenta, rgb(30, 30, 40));
    //skin.paragraph.align = Alignment::Center;
    skin.table.align = Alignment::Center;

    skin
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
        .filter(|(id, _fish)| !save.player.fish_caught.contains_key(&format!("(O){id}")))
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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

#[derive(Debug)]
struct Item<'a> {
    object: &'a Object,
    location: ItemLocation,
}

fn get_all_items(save: &SaveGame, all: bool) -> Vec<Item> {
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
            } else if all {
                items.push(Item {
                    object,
                    location: ItemLocation::Map(name.clone(), *pos),
                });
            }
        }
    }

    items
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ItemQuantityAndLocations {
    quantity: usize,
    locations: HashSet<ItemLocation>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ItemInfo {
    id: String,
    normal: ItemQuantityAndLocations,
    iron: ItemQuantityAndLocations,
    gold: ItemQuantityAndLocations,
    irridium: ItemQuantityAndLocations,
}

fn aggregate_items(items: Vec<Item>) -> HashMap<String, ItemInfo> {
    items.iter().fold(HashMap::new(), |mut acc, item| {
        let info = acc.entry(item.object.name.clone()).or_default();
        let quantity_and_locations = match item.object.quality {
            Some(1) => &mut info.iron,
            Some(2) => &mut info.gold,
            Some(4) => &mut info.irridium,
            _ => &mut info.normal,
        };
        quantity_and_locations.quantity += item.object.stack as usize;
        quantity_and_locations
            .locations
            .insert(item.location.clone());
        info.id = item.object.id.clone();
        acc
    })
}

fn cmd_food(opt: &GameAndSaveOpt) -> Result<()> {
    let _data = GameData::load(&opt.content.game_content)?;
    let f = File::open(&opt.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;

    let items = get_all_items(&save, false);

    let mut items: Vec<_> = items
        .iter()
        .filter(|item| item.object.edibility.unwrap_or(0) > 0)
        .fold(HashMap::new(), |mut acc, item| {
            let entry = acc
                .entry((item.object.name.clone(), item.object.quality))
                .or_insert((
                    item.object.energy() as f32
                        / item.object.adjusted_price(&save.player.professions) as f32,
                    Vec::new(),
                    item.object.clone(),
                    0,
                ));
            //entry.0 = item.object.adjusted_price(&save.player.professions);
            entry.1.push(item.location.clone());
            entry.3 += item.object.stack;

            acc
        })
        .into_iter()
        .collect();

    items.sort_by(|a, b| a.1 .0.partial_cmp(&b.1 .0).unwrap());

    let mut text = "|:-:|:-:|:-:|\n".to_string();
    text.push_str("|**Name**|**Qty**|**Energy**|**Price**|**Ratio**|**Location**|\n");
    text.push_str("|:-|:-:|:-:|-\n");

    for item in items {
        let quality = item.1 .2.quality.unwrap_or(0);
        let quality_txt = match quality {
            1 => " (s)",
            2 => " (**g**)",
            4 => " (*i*)",
            _ => "",
        };
        #[allow(unstable_name_collisions)]
        let locations: String = item
            .1
             .1
            .iter()
            .map(|loc| format!("{}", loc))
            .intersperse(", ".to_string())
            .collect();

        let ratio = item.1 .0;
        text.push_str(&format!(
            "|{}{} |{} | {} | {} | {:0.02} |{} |\n",
            item.1 .2.name,
            quality_txt,
            item.1 .3,
            item.1 .2.energy(),
            item.1 .2.adjusted_price(&save.player.professions),
            ratio,
            locations,
        ));
    }
    text.push_str("|-\n");
    println!("{}", mad_skin().term_text(&text));
    Ok(())
}

fn cmd_items(opt: &ItemsOpt) -> Result<()> {
    let _data = GameData::load(&opt.loc.content.game_content)?;
    let f = File::open(&opt.loc.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;

    let items = get_all_items(&save, opt.all);

    let mut items: Vec<_> = items
        .iter()
        .fold(HashMap::new(), |mut acc, item| {
            let entry = acc
                .entry((item.object.name.clone(), item.object.quality))
                .or_insert((0, Vec::new(), item.object.clone(), 0));
            entry.0 += item.object.stack_price(&save.player.professions);
            entry.1.push(item.location.clone());
            entry.3 += item.object.stack;

            acc
        })
        .into_iter()
        .collect();

    items.sort_by(|a, b| a.1 .0.partial_cmp(&b.1 .0).unwrap());

    if opt.csv {
        for item in items {
            let quality = item.1 .2.quality.unwrap_or(0);
            let name = item.1 .2.name;
            let quantity = item.1 .3;
            let stack_price = item.1 .0;

            println!("{}, {}, {}, {}", name, quality, quantity, stack_price);
        }
    } else {
        let total: i64 = items.iter().map(|item| item.1 .0 as i64).sum();

        let mut skin = MadSkin::default();
        skin.set_headers_fg(rgb(255, 187, 0));
        skin.bold.set_fg(Yellow);
        skin.italic.set_fgbg(Magenta, rgb(30, 30, 40));
        skin.paragraph.align = Alignment::Center;
        skin.table.align = Alignment::Center;

        let mut text = "|:-:|:-:|:-:|\n".to_string();
        text.push_str("|**Name**|**Qty**|**Price**|**Location**|\n");
        text.push_str("|:-|:-:|:-:|-\n");

        for item in items {
            let quality = item.1 .2.quality.unwrap_or(0);
            let quality_txt = match quality {
                1 => " (s)",
                2 => " (**g**)",
                4 => " (*i*)",
                _ => "",
            };
            #[allow(unstable_name_collisions)]
            let locations: String = item
                .1
                 .1
                .iter()
                .map(|loc| format!("{}", loc))
                .intersperse(", ".to_string())
                .collect();

            let stack_price = item.1 .0;
            if item.1 .2.price_multiplier(&save.player.professions) > 1.0 {
                text.push_str(&format!(
                    "|**{}**{} |{} | **{}** |{} |\n",
                    item.1 .2.name, quality_txt, item.1 .3, stack_price, locations,
                ));
            } else {
                text.push_str(&format!(
                    "|{}{} |{} | {} |{} |\n",
                    item.1 .2.name, quality_txt, item.1 .3, stack_price, locations,
                ));
            }
        }
        text.push_str("|:-|:-:|:-:|-\n");
        text.push_str(&format!("|**Total**||{}||\n", total));
        text.push_str("|-\n");
        println!("{}", skin.term_text(&text));
    }
    Ok(())
}

// fn optimize_geodes(
//     geode_count: &mut Vec<(GeodeType, i32)>,
//     predictions: &HashMap<GeodeType, Vec<save::Object>>,
//     professions: &IndexSet<Profession>,
//     level: usize,
//     num_predictions: usize,
//     memo: &mut HashMap<Vec<(GeodeType, i32)>, (i32, Vec<(GeodeType, i32, i32)>)>,
// ) -> (i32, Vec<(GeodeType, i32, i32)>) {
//     if level >= num_predictions {
//         return (0, Vec::with_capacity(num_predictions));
//     }

//     if let Some(result) = memo.get(geode_count) {
//         return (result.0, result.1.clone());
//     }

//     let chain = GeodeType::iter()
//         .enumerate()
//         .filter_map(|(i, ty)| {
//             if geode_count[i].1 <= 0 {
//                 None
//             } else {
//                 geode_count[i].1 -= 1;
//                 let reward = &predictions.get(&ty).unwrap()[level];
//                 let value = reward.stack_price(professions)
//                     - 25
//                     - match ty {
//                         GeodeType::Geode => 50,
//                         GeodeType::FrozenGeode => 100,
//                         GeodeType::MagmaGeode => 150,
//                         GeodeType::OmniGeode => 0,
//                         GeodeType::ArtifactTrove => 0,
//                         GeodeType::GoldenCoconut => 0,
//                     };
//                 let mut new_chain = optimize_geodes(
//                     geode_count,
//                     predictions,
//                     professions,
//                     level + 1,
//                     num_predictions,
//                     memo,
//                 );
//                 geode_count[i].1 += 1;

//                 new_chain.0 += value;
//                 new_chain.1.push((ty, reward.id(), value));
//                 Some(new_chain)
//             }
//         })
//         .max_by(|a, b| a.0.cmp(&b.0))
//         .unwrap();

//     memo.insert(geode_count.clone(), chain.clone());

//     chain
// }

// fn cmd_geodes(opt: &GameAndSaveOpt) -> Result<()> {
//     let data = GameData::load(&opt.content.game_content)?;
//     let f = File::open(&opt.file)?;
//     let mut r = BufReader::new(f);
//     let save = SaveGame::from_reader(&mut r)?;

//     let geodes_map = get_all_items(&save, false)
//         .iter()
//         .filter(|i| i.object.is_geode())
//         .fold(HashMap::new(), |mut map, item| {
//             *map.entry(GeodeType::from_i32(item.object.id()).unwrap())
//                 .or_insert(0) += item.object.stack;
//             map
//         });

//     let mut geodes: Vec<(GeodeType, i32)> = GeodeType::iter()
//         .map(|t| (t, *geodes_map.get(&t).unwrap_or(&0)))
//         .collect();

//     let num_geodes = geodes.iter().fold(0, |count, (_id, num)| count + num);

//     let predictions = Geode::predict(num_geodes, 1, &data, &save)?;
//     let chain = optimize_geodes(
//         &mut geodes,
//         &predictions,
//         &save.player.professions,
//         0,
//         num_geodes as usize,
//         &mut HashMap::new(),
//     );

//     println!("chain value: {}", chain.0);
//     for (i, entry) in chain.1.iter().rev().enumerate() {
//         let object = data.get_object(entry.1).unwrap();
//         println!(" {} {:?} -> {} {}", i + 1, entry.0, object.name, entry.2);
//     }

//     let prediction = Geode::predict(10, 1, &data, &save)?;
//     let mut skin = MadSkin::default();
//     skin.set_headers_fg(rgb(255, 187, 0));
//     skin.bold.set_fg(Yellow);
//     skin.italic.set_fgbg(Magenta, rgb(30, 30, 40));
//     skin.paragraph.align = Alignment::Center;
//     skin.table.align = Alignment::Center;
//     let text_template = TextTemplate::from(
//         r#"
//     |:-:|:-|:-|:-|:-|:-|:-|
//     |**N**|**Geode**|**Frozen Geode**|**Magma Geode**|**Omni Geode**|**Artifact Trove**|**Golden Coconut**|
//     |:-:|:-|:-|:-|:-|:-|:-|
//     ${rows
//     |**${i}**|${geode}|${frozen_geode}|${magma_geode}|${omni_geode}|${artifact_trove}|${golden_coconut}|
//     }
//     |:-:|:-|:-|:-|:-|:-|:-|
//     "#,
//     );

//     let mut expander = text_template.expander();
//     let indexes: Vec<String> = (0..10).map(|i| format!("{}", i + 1)).collect();
//     for i in 0..10 {
//         expander
//             .sub("rows")
//             .set("i", &indexes[i])
//             .set("geode", &prediction.get(&GeodeType::Geode).unwrap()[i].name)
//             .set(
//                 "frozen_geode",
//                 &prediction.get(&GeodeType::FrozenGeode).unwrap()[i].name,
//             )
//             .set(
//                 "magma_geode",
//                 &prediction.get(&GeodeType::MagmaGeode).unwrap()[i].name,
//             )
//             .set(
//                 "omni_geode",
//                 &prediction.get(&GeodeType::OmniGeode).unwrap()[i].name,
//             )
//             .set(
//                 "artifact_trove",
//                 &prediction.get(&GeodeType::ArtifactTrove).unwrap()[i].name,
//             )
//             .set(
//                 "golden_coconut",
//                 &prediction.get(&GeodeType::GoldenCoconut).unwrap()[i].name,
//             );
//     }

//     skin.print_expander(expander);

//     Ok(())
// }

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
            if data.get_object(&format!("{}", item.id)).is_ok() && bundle_state[index] {
                completed += 1;
            }
        }

        println!("{}: {}/{}", bundle.name, completed, bundle.num_items_needed);

        for (index, item) in bundle.requirements.iter().enumerate() {
            if let Ok(object) = data.get_object(&format!("{}", item.id)) {
                let found = bundle_state[index];
                println!("  {}: {}", object.name, found);
            }
        }
    }
    Ok(())
}

fn cmd_todo(opt: &GameAndSaveOpt) -> Result<()> {
    let data = GameData::load(&opt.content.game_content)?;
    let f = File::open(&opt.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;

    let season = &Season::Fall; //&save.current_season;
    let day = &13; //&save.day_of_month;

    let mut text = String::new();

    let items = get_all_items(&save, false);
    let aggregate_items = aggregate_items(items);

    writeln!(
        &mut text,
        "Today is {:?} {} year {}.",
        &save.current_season, &save.day_of_month, &save.year
    )?;

    let birthday = data.characters.iter().find(|(_name, character)| {
        let Some(birth_season) = &character.birth_season else {
            return false;
        };
        birth_season == season && character.birthday == *day
    });

    let day_of_week = DayOfWeek::try_from(*day)?;
    if day_of_week == DayOfWeek::Wednesday || day_of_week == DayOfWeek::Sunday {
        writeln!(&mut text, "*Queen of Sauce is airing today!*")?;
    }
    if let Some((name, _character)) = birthday {
        writeln!(&mut text, "*It's {name}'s birthday today!*")?;
        let loved: Vec<_> = aggregate_items
            .iter()
            .filter_map(|(_, info)| {
                let Ok(object) = data.get_object(&info.id) else {
                    return None;
                };
                let Ok(taste) = data.lookup_npc_taste_for_object(name, object) else {
                    return None;
                };
                if taste == ObjectTaste::Love {
                    Some(object.name.clone())
                } else {
                    None
                }
            })
            .collect();
        let liked: Vec<_> = aggregate_items
            .iter()
            .filter_map(|(_, info)| {
                let Ok(object) = data.get_object(&info.id) else {
                    return None;
                };
                let Ok(taste) = data.lookup_npc_taste_for_object(name, object) else {
                    return None;
                };
                if taste == ObjectTaste::Like {
                    Some(object.name.clone())
                } else {
                    None
                }
            })
            .collect();
        writeln!(&mut text, "Love items you own: {loved:?}")?;
        writeln!(&mut text, "Liked items you own: {liked:?}")?;
    }

    let luck_text = if save.daily_luck > 0.5 {
        format!("*{}*", &save.daily_luck)
    } else if save.daily_luck > 0.0 {
        format!("**{}**", &save.daily_luck)
    } else {
        format!("{}", &save.daily_luck)
    };
    writeln!(&mut text, "Today's Luck: {}", luck_text)?;

    let default_weather = save.get_weather("Default");
    let island_weather = save.get_weather("Island");
    writeln!(&mut text, "Today's weather:")?;
    writeln!(&mut text, "  Farm: {:?}", default_weather.today())?;
    writeln!(&mut text, "  Island: {:?}", island_weather.today())?;
    writeln!(&mut text, "Tomorrow's weather:")?;
    writeln!(&mut text, "  Farm: {:?}", default_weather.tomorrow())?;
    writeln!(&mut text, "  Island: {:?}", island_weather.tomorrow())?;

    writeln!(&mut text, "\nLevels:")?;
    for (skill, (level, xp_to_go)) in save.player.levels() {
        writeln!(
            &mut text,
            "  {}: {} ({} to next level)",
            skill, level, xp_to_go
        )?;
    }

    println!("{}", mad_skin().term_text(&text));
    Ok(())
}

fn cmd_dump_bundles(opt: &GameContentLoc) -> Result<()> {
    let data = GameData::load(&opt.game_content)?;

    for bundle in &data.bundles {
        println!("{:?}", &bundle);
    }

    Ok(())
}

fn cmd_dump_characters(opt: &GameContentLoc) -> Result<()> {
    let data = GameData::load(&opt.game_content)?;

    for character in &data.characters {
        println!("{:?}", &character);
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

    let cat_set: HashSet<ObjectCategory> =
        HashSet::from_iter(data.objects.iter().map(|o| o.1.category.clone()));
    let cats: Vec<ObjectCategory> = cat_set.iter().cloned().collect();

    println!("types: {:?}", &cats);

    Ok(())
}

fn cmd_dump_npc_gift_tastes(opt: &GameContentLoc) -> Result<()> {
    let data = GameData::load(&opt.game_content)?;

    for (id, tastes) in &data.npc_gift_tastes {
        println!("{}: {:?}", id, &tastes);
    }

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
        DumpOpt::Bundles(o) => cmd_dump_bundles(o),
        DumpOpt::Characters(o) => cmd_dump_characters(o),
        DumpOpt::Fish(o) => cmd_dump_fish(o),
        DumpOpt::Objects(o) => cmd_dump_objects(o),
        DumpOpt::NpcGiftTastes(o) => cmd_dump_npc_gift_tastes(o),
        DumpOpt::Save(o) => cmd_dump_save(o),
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let opt = Opt::from_args();

    match opt {
        Opt::Dump(o) => cmd_dump(&o)?,
        Opt::Bundles(o) => cmd_bundles(&o)?,
        Opt::Fish(o) => cmd_fish(&o)?,
        Opt::Food(o) => cmd_food(&o)?,
        //Opt::Geodes(o) => cmd_geodes(&o)?,
        Opt::Items(o) => cmd_items(&o)?,
        //Opt::RenderMap(o) => cmd_render_map(&o)?,
        Opt::Todo(o) => cmd_todo(&o)?,
    }

    Ok(())
}
