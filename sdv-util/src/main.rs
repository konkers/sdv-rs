use std::{
    collections::{HashMap, HashSet},
    convert::{TryFrom, TryInto},
    fmt::{Display, Write},
    fs::File,
    hash::Hash,
    io::{BufReader, Seek},
    iter::FromIterator,
    path::{Path, PathBuf},
};

use ::crossterm::style::Color::*;
use anyhow::{anyhow, Result};
use image::{codecs::png::PngEncoder, RgbaImage};
use itertools::Itertools;
use sdv::{
    analyzer::perfection::analyze_perfection,
    common::{DayOfWeek, ObjectCategory, Point},
    gamedata::{Fish, GameData, Locale, ObjectTaste},
    save::Object,
    SaveGame,
};
use structopt::clap::arg_enum;
use structopt::StructOpt;
use termimad::{rgb, Alignment, MadSkin};
use walkdir::{DirEntry, WalkDir};
use xnb::xna::Texture2D;

// Needs to be updated for serde.
// mod render_map;
// use render_map::cmd_render_map;

#[derive(Debug, StructOpt)]
#[cfg(any(windows, target_os = "macos"))]
struct GameContentLoc {
    /// Path to Stardew Valley's Content directory
    #[structopt(long, parse(from_os_str))]
    game_content: Option<PathBuf>,
}

#[cfg(all(not(windows), not(target_os = "macos")))]
#[derive(Debug, StructOpt)]
struct GameContentLoc {
    /// Path to Stardew Valley's Content directory
    #[structopt(long, parse(from_os_str))]
    game_content: PathBuf,
}

impl GameContentLoc {
    #[cfg(windows)]
    fn get(&self) -> Result<PathBuf> {
        use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

        if let Some(path) = &self.game_content {
            return Ok(path.clone());
        }
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let steam = hklm.open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam")?;
        let steam_path: String = steam.get_value("InstallPath")?;
        let path: PathBuf = [
            &steam_path,
            "steamapps",
            "common",
            "Stardew Valley",
            "Content",
        ]
        .iter()
        .collect();

        Ok(path)
    }

    #[cfg(target_os = "macos")]
    fn get(&self) -> Result<PathBuf> {
        if let Some(path) = &self.game_content {
            return Ok(path.clone());
        }

        let mut home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Can't find home directory"))?;
        home_dir.push("Library/Application Support/Steam/steamapps/common/Stardew Valley/Contents/Resources/Content");
        println!("{}", home_dir.display());
        Ok(home_dir)
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    fn get(&self) -> Result<PathBuf> {
        Ok(self.game_content.clone())
    }
}

arg_enum! {
    #[derive(Debug)]
    enum Format {
        Text,
        Json,
    }
}

impl Default for Format {
    fn default() -> Self {
        Self::Text
    }
}

#[derive(Debug, StructOpt)]
struct DumpOpts {
    #[structopt(flatten)]
    content: GameContentLoc,

    #[structopt(long, default_value = "text")]
    format: Format,
}

#[derive(Debug, StructOpt)]
struct PackageOpts {
    #[structopt(flatten)]
    content: GameContentLoc,

    #[structopt(long)]
    pretty: bool,

    #[structopt(long, parse(from_os_str))]
    output: PathBuf,
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
    BigCraftables(DumpOpts),
    Bundles(DumpOpts),
    Characters(DumpOpts),
    Fish(DumpOpts),
    Locale(DumpOpts),
    Locations(DumpOpts),
    NpcGiftTastes(DumpOpts),
    Objects(DumpOpts),
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
enum PackageOpt {
    GameData(PackageOpts),
    Locale(PackageOpts),
    Textures(PackageOpts),
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
    Package(PackageOpt),
    Perfection(GameAndSaveOpt),
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

fn print_list<T: Display, L: IntoIterator<Item = T>>(list: L) {
    for (n, item) in list.into_iter().enumerate() {
        if n > 0 {
            print!(", {item}");
        } else {
            print!("{item}");
        }
    }
}

fn print_fish(id: &str, fish: &Fish, fish_locations: &HashMap<String, Vec<String>>) {
    println!("* {}", &fish.name());

    if let Fish::Line { times, .. } = fish {
        print!("  - times: ");
        print_list(times);
        println!();
    }

    if let Some(locations) = fish_locations.get(&format!("(O){id}")) {
        print!("  - locations: ");
        print_list(locations);
        println!();
    }
    if let Fish::Trap { location, .. } = fish {
        println!("  - crab pot location: {location}");
    }
}

fn cmd_fish(opt: &GameAndSaveOpt) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;
    let f = File::open(&opt.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;

    let fish_locations = calculate_fish_locations(&data)?;
    println!(
        "Today is {:?} {} year {}.\n",
        &save.current_season, &save.day_of_month, &save.year
    );

    println!("Available, uncaught line fish:");
    for (id, fish) in data
        .fish
        .iter()
        .filter(|(id, _fish)| !save.player.fish_caught.contains_key(&format!("(O){id}")))
        .filter(|(_id, fish)| fish.in_season(&save.current_season))
        .filter(|(_id, fish)| fish.is_line_fish())
    {
        print_fish(id, fish, &fish_locations);
    }

    println!("\nAvailable, uncaught pot fish:");
    for (id, fish) in data
        .fish
        .iter()
        .filter(|(id, _fish)| !save.player.fish_caught.contains_key(&format!("(O){id}")))
        .filter(|(_id, fish)| fish.in_season(&save.current_season))
        .filter(|(_id, fish)| fish.is_pot_fish())
    {
        print_fish(id, fish, &fish_locations);
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

fn aggregate_items(items: Vec<Item>, data: &GameData) -> Result<HashMap<String, ItemInfo>> {
    items.iter().try_fold(HashMap::new(), |mut acc, item| {
        let name = item.object.lookup_name(data)?.to_string();
        let info: &mut ItemInfo = acc.entry(name).or_default();
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
        Ok(acc)
    })
}

fn calculate_fish_locations(data: &GameData) -> Result<HashMap<String, Vec<String>>> {
    let mut fish_locations = HashMap::<String, Vec<String>>::new();
    for (location_name, location) in &data.locations {
        let Some(fishes) = &location.fish else {
            continue;
        };

        for fish in fishes {
            let tmp = vec![fish.parent.parent.id.clone()];
            let ids = fish.parent.parent.random_item_id.as_ref().unwrap_or(&tmp);

            for id in ids {
                match fish_locations.get_mut(id) {
                    Some(locations) => locations.push(location_name.clone()),
                    None => {
                        fish_locations.insert(id.clone(), vec![location_name.clone()]);
                    }
                }
            }
        }
    }
    Ok(fish_locations)
}

fn cmd_food(opt: &GameAndSaveOpt) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;
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
            item.1 .2.lookup_name(&data)?,
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
    let data = GameData::from_content_dir(opt.loc.content.get()?)?;
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
            let name = item.1 .2.lookup_name(&data)?;
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
                    item.1 .2.lookup_name(&data)?,
                    quality_txt,
                    item.1 .3,
                    stack_price,
                    locations,
                ));
            } else {
                text.push_str(&format!(
                    "|{}{} |{} | {} |{} |\n",
                    item.1 .2.lookup_name(&data)?,
                    quality_txt,
                    item.1 .3,
                    stack_price,
                    locations,
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
    let data = GameData::from_content_dir(opt.content.get()?)?;

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
    let data = GameData::from_content_dir(opt.content.get()?)?;
    let f = File::open(&opt.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;

    let season = &save.current_season;
    let day = &save.day_of_month;

    let mut text = String::new();

    let items = get_all_items(&save, false);
    let aggregate_items = aggregate_items(items, &data)?;

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
    if day_of_week == DayOfWeek::Friday || day_of_week == DayOfWeek::Sunday {
        writeln!(&mut text, "*Traveling cart is here!*")?;
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

fn cmd_dump_big_craftables(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    for (id, val) in &data.big_craftables {
        println!("{}: {:?}", id, &val);
    }

    Ok(())
}

fn cmd_dump_bundles(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    for bundle in &data.bundles {
        println!("{:?}", &bundle);
    }

    Ok(())
}

fn cmd_dump_characters(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    for character in &data.characters {
        println!("{:?}", &character);
    }

    Ok(())
}

fn cmd_dump_fish(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    for (id, fish) in &data.fish {
        println!("{}: {:?}", id, &fish);
    }

    for fish in calculate_fish_locations(&data)? {
        println!("{:?}", fish);
    }

    Ok(())
}

fn cmd_dump_locale(opt: &DumpOpts) -> Result<()> {
    let locale = Locale::from_content_dir(opt.content.get()?, "en-EN")?;
    match opt.format {
        Format::Text => {
            for (key, value) in &locale.strings {
                println!("{key}: {value}");
            }
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&locale.strings)?);
        }
    }

    Ok(())
}

fn cmd_dump_locations(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    match opt.format {
        Format::Text => {
            for location in &data.locations {
                println!("{:?}", location);
            }
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&data.locations)?);
        }
    }

    Ok(())
}

fn cmd_dump_objects(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    for (id, object) in &data.objects {
        println!("{}: {:?}", id, &object);
    }

    let cat_set: HashSet<ObjectCategory> =
        HashSet::from_iter(data.objects.iter().map(|o| o.1.category.clone()));
    let cats: Vec<ObjectCategory> = cat_set.iter().cloned().collect();

    println!("types: {:?}", &cats);

    Ok(())
}

fn cmd_dump_npc_gift_tastes(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

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
        DumpOpt::BigCraftables(o) => cmd_dump_big_craftables(o),
        DumpOpt::Bundles(o) => cmd_dump_bundles(o),
        DumpOpt::Characters(o) => cmd_dump_characters(o),
        DumpOpt::Fish(o) => cmd_dump_fish(o),
        DumpOpt::Locale(o) => cmd_dump_locale(o),
        DumpOpt::Locations(o) => cmd_dump_locations(o),
        DumpOpt::Objects(o) => cmd_dump_objects(o),
        DumpOpt::NpcGiftTastes(o) => cmd_dump_npc_gift_tastes(o),
        DumpOpt::Save(o) => cmd_dump_save(o),
    }
}

fn cmd_package_game_data(opt: &PackageOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;
    let mut output = File::create(&opt.output)?;
    if opt.pretty {
        data.to_pretty_json_writer(&mut output)?;
    } else {
        data.to_json_writer(&mut output)?;
    }

    Ok(())
}

fn cmd_package_locale(opt: &PackageOpts) -> Result<()> {
    let data = Locale::from_content_dir(opt.content.get()?, "en-EN")?;
    let mut output = File::create(&opt.output)?;
    if opt.pretty {
        data.to_pretty_json_writer(&mut output)?;
    } else {
        data.to_json_writer(&mut output)?;
    }

    Ok(())
}

// Canonicalize the path representations across OSs.
fn path_to_string(path: &std::path::Path) -> String {
    let mut path_str = String::new();
    for component in path.components() {
        if let std::path::Component::Normal(os_str) = component {
            if !path_str.is_empty() {
                path_str.push('/');
            }
            path_str.push_str(&os_str.to_string_lossy());
        }
    }
    path_str
}

fn handle_zip_entry<W: std::io::Write + Seek>(
    zip: &mut zip::ZipWriter<W>,
    content_path: &Path,
    entry: DirEntry,
) -> Result<()> {
    let path = entry.into_path();
    let relative_path = path.strip_prefix(content_path)?;
    if relative_path.as_os_str().is_empty() {
        return Err(anyhow!("relative path empty"));
    }

    if path.is_dir() {
        zip.add_directory(
            path_to_string(relative_path),
            zip::write::FileOptions::default(),
        )?;
        return Ok(());
    }

    if path.extension().ok_or_else(|| anyhow!("no extension"))? != "xnb" {
        return Err(anyhow!("Not an xnb file"));
    }

    let png_path = relative_path.with_extension("png");

    let data = std::fs::read(&path)?;

    let texture = xnb::from_bytes::<Texture2D>(&data)?;

    println!("Processing {}", png_path.display());

    let image: RgbaImage = texture.try_into()?;
    zip.start_file(
        path_to_string(&png_path),
        zip::write::FileOptions::default(),
    )?;
    let encoder = PngEncoder::new(zip);
    image.write_with_encoder(encoder)?;

    Ok(())
}

fn cmd_package_textures(opt: &PackageOpts) -> Result<()> {
    let content_path = opt.content.get()?;

    let mut zip_file = File::create(&opt.output)?;
    let mut zip = zip::ZipWriter::new(&mut zip_file);

    for entry in WalkDir::new(&content_path) {
        let Ok(entry) = entry else {
            continue;
        };
        let _ = handle_zip_entry(&mut zip, &content_path, entry);
    }

    zip.finish()?;
    Ok(())
}

fn cmd_package(opt: &PackageOpt) -> Result<()> {
    match opt {
        PackageOpt::GameData(o) => cmd_package_game_data(o),
        PackageOpt::Locale(o) => cmd_package_locale(o),
        PackageOpt::Textures(o) => cmd_package_textures(o),
    }
}

fn cmd_perfection(opt: &GameAndSaveOpt) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;
    let f = File::open(&opt.file)?;
    let mut r = BufReader::new(f);
    let save = SaveGame::from_reader(&mut r)?;

    let analysis = analyze_perfection(&data, &save);
    println!("{analysis:#?}");
    Ok(())
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
        Opt::Package(o) => cmd_package(&o)?,
        Opt::Perfection(o) => cmd_perfection(&o)?,
        //Opt::RenderMap(o) => cmd_render_map(&o)?,
        Opt::Todo(o) => cmd_todo(&o)?,
    }

    Ok(())
}
