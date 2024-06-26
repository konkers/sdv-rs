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
use indexmap::IndexMap;
use itertools::Itertools;
use sdv::{
    analyzer::perfection::analyze_perfection,
    common::{DayOfWeek, ObjectCategory, Point},
    gamedata::{Fish, GameData, Locale, ObjectTaste},
    item_id,
    predictor::{
        self,
        garbage::{predict_garbage, GarbageCan, GarbageCanLocation},
        geode::{predict_single_geode, Geode, GeodeType},
        PredictionGameState,
    },
    rng::HashedSeedGenerator,
    save::Object,
    SaveGame,
};
use serde::Serialize;
use structopt::{clap::arg_enum, StructOpt};
use strum::IntoEnumIterator;
use termimad::{rgb, Alignment, MadSkin};
use walkdir::{DirEntry, WalkDir};
use xnb::xna::Texture2D;

mod render_map;
use render_map::cmd_render_map;

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
    fn get(&self) -> Result<PathBuf> {
        if let Some(path) = &self.game_content {
            return Ok(path.clone());
        }

        sdv::gamedata::get_game_content_path()
            .ok_or_else(|| anyhow!("Can't locate default game data path"))
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
struct DumpMapOpts {
    #[structopt(flatten)]
    dump: DumpOpts,

    map: String,
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
    CookingRecipes(DumpOpts),
    CraftingRecipes(DumpOpts),
    BigCraftables(DumpOpts),
    Bundles(DumpOpts),
    Characters(DumpOpts),
    Fish(DumpOpts),
    Garbage(DumpOpts),
    Locale(DumpOpts),
    Locations(DumpOpts),
    LocationContexts(DumpOpts),
    Map(DumpMapOpts),
    NpcGiftTastes(DumpOpts),
    Objects(DumpOpts),
    PassiveFestivals(DumpOpts),
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
struct RenderMapOpt {
    #[structopt(flatten)]
    content: GameContentLoc,

    map_name: String,
}

#[derive(Debug, StructOpt)]
struct BubblesOpt {
    #[structopt(flatten)]
    content: GameContentLoc,

    #[structopt(long)]
    map_name: String,

    #[structopt(long)]
    days_played: u32,

    #[structopt(long)]
    seed: i32,
}

#[derive(Debug, StructOpt)]
struct GeodesOpt {
    #[structopt(flatten)]
    content: GameContentLoc,

    #[structopt(long)]
    geode_type: GeodeType,

    #[structopt(long)]
    geodes_cracked: u32,

    #[structopt(long)]
    multiplayer_id: i64,

    #[structopt(long)]
    deepest_mine_level: u32,

    #[structopt(long)]
    seed: u32,
}

#[derive(Debug, StructOpt)]
enum PredictOpt {
    Bubbles(BubblesOpt),
    Garbage(GameContentLoc),
    Geode(GeodesOpt),
}

#[derive(Debug, StructOpt)]
enum GenerateOpt {
    Objects(GameContentLoc),
}

#[derive(Debug, StructOpt)]
enum Opt {
    Bundles(GameAndSaveOpt),
    Dump(DumpOpt),
    Fish(GameAndSaveOpt),
    Food(GameAndSaveOpt),
    Generate(GenerateOpt),
    //Geodes(GameAndSaveOpt),
    Items(ItemsOpt),
    RenderMap(RenderMapOpt),
    Package(PackageOpt),
    Perfection(GameAndSaveOpt),
    Predict(PredictOpt),
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

fn dump_data_map<T: std::fmt::Debug + Serialize>(
    opt: &DumpOpts,
    data: &IndexMap<String, T>,
) -> Result<()> {
    match opt.format {
        Format::Text => {
            for (id, val) in data {
                println!("{}: {:?}", id, &val);
            }
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
    }

    Ok(())
}

fn dump_data<T: std::fmt::Debug + Serialize>(opt: &DumpOpts, data: &T) -> Result<()> {
    match opt.format {
        Format::Text => {
            println!("{:?}", &data);
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
    }

    Ok(())
}

fn cmd_dump_cooking_recipes(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    dump_data_map(opt, &data.cooking_recipies)
}

fn cmd_dump_crafting_recipes(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    dump_data_map(opt, &data.crafting_recipies)
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

fn cmd_dump_garbage(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    dump_data(opt, &data.garbage_cans)
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

fn cmd_dump_location_contexts(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    dump_data(opt, &data.location_contexts)?;
    Ok(())
}

fn cmd_dump_objects(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    for (id, object) in &data.objects {
        println!("{}: {:?}", id, &object);
    }

    let cat_set: HashSet<ObjectCategory> =
        HashSet::from_iter(data.objects.iter().map(|o| o.1.category));
    let cats: Vec<ObjectCategory> = cat_set.iter().cloned().collect();

    println!("types: {:?}", &cats);

    Ok(())
}

fn cmd_dump_passive_festivals(opt: &DumpOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;

    dump_data(opt, &data.passive_festivals)?;
    Ok(())
}

fn cmd_dump_map(opt: &DumpMapOpts) -> Result<()> {
    let data = GameData::from_content_dir(opt.dump.content.get()?)?;
    let map = data.load_map(&opt.map)?;

    match opt.dump.format {
        Format::Text => {
            println!("{:#?}", map);
        }
        Format::Json => {
            //   println!("{}", serde_json::to_string_pretty(&map)?);
        }
    }

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
        DumpOpt::CookingRecipes(o) => cmd_dump_cooking_recipes(o),
        DumpOpt::CraftingRecipes(o) => cmd_dump_crafting_recipes(o),
        DumpOpt::BigCraftables(o) => cmd_dump_big_craftables(o),
        DumpOpt::Bundles(o) => cmd_dump_bundles(o),
        DumpOpt::Characters(o) => cmd_dump_characters(o),
        DumpOpt::Fish(o) => cmd_dump_fish(o),
        DumpOpt::Garbage(o) => cmd_dump_garbage(o),
        DumpOpt::Locale(o) => cmd_dump_locale(o),
        DumpOpt::Locations(o) => cmd_dump_locations(o),
        DumpOpt::LocationContexts(o) => cmd_dump_location_contexts(o),
        DumpOpt::Objects(o) => cmd_dump_objects(o),
        DumpOpt::PassiveFestivals(o) => cmd_dump_passive_festivals(o),
        DumpOpt::Map(o) => cmd_dump_map(o),
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

fn cmd_predict_bubbles(opt: &BubblesOpt) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;
    let map = data.load_map(&opt.map_name)?;
    let bubbles = predictor::bubbles::calculate_bubbles::<HashedSeedGenerator>(
        &map,
        opt.days_played,
        opt.seed as u32,
    )?;
    for bubble in bubbles {
        println!(
            "(x:{} y:{}) {}",
            bubble.location.x, bubble.location.y, bubble.span
        );
    }
    Ok(())
}

fn cmd_predict_garbage(opt: &GameContentLoc) -> Result<()> {
    let data = GameData::from_content_dir(opt.get()?)?;
    let state = PredictionGameState {
        game_id: 254546202,
        days_played: 1,
        daily_luck: 0.0999,
        ..Default::default()
    };
    let cans = GarbageCanLocation::iter()
        .map(|location| GarbageCan::new(location, &data.garbage_cans))
        .collect::<Result<Vec<_>>>()?;
    println!("{:?}", item_id!("RANDOM_BASE_SEASON_ITEM"));
    let special_items = HashMap::from([
        (item_id!("RANDOM_BASE_SEASON_ITEM"), "Random season item"),
        (item_id!("DISH_OF_THE_DAY"), "Dish of the Day"),
    ]);

    for can in &cans {
        if let Some((reward, min_luck)) = predict_garbage::<HashedSeedGenerator>(can, &state)? {
            if let Ok(item) = data.get_object_by_id(&reward.item) {
                println!(
                    "{}: {} {} (min luck {})",
                    can.location, reward.quantity, item.name, min_luck
                );
            } else if let Some(name) = special_items.get(&reward.item) {
                println!(
                    "{}: {} {} (min luck {})",
                    can.location, reward.quantity, name, min_luck
                );
            } else {
                println!(
                    "{}: {} {:?} (min luck {})",
                    can.location, reward.quantity, reward.item, min_luck
                );
            }
        }
    }

    Ok(())
}

fn cmd_predict_geode(opt: &GeodesOpt) -> Result<()> {
    let data = GameData::from_content_dir(opt.content.get()?)?;
    let geode = Geode::new(opt.geode_type, &data)?;

    for i in 0..10 {
        let state = PredictionGameState {
            game_id: opt.seed,
            multiplayer_id: opt.multiplayer_id,
            geodes_cracked: opt.geodes_cracked + i,
            deepest_mine_level: opt.deepest_mine_level,
            qi_beans_quest_active: false,
            ..Default::default()
        };
        let reward = predict_single_geode::<HashedSeedGenerator>(&geode, &state)?;
        let object = data.get_object_by_id(&reward.item)?;
        println!("{i}: {} {}", object.name, reward.quantity);
    }
    Ok(())
}

fn cmd_predict(opt: &PredictOpt) -> Result<()> {
    match opt {
        PredictOpt::Bubbles(o) => cmd_predict_bubbles(o),
        PredictOpt::Garbage(o) => cmd_predict_garbage(o),
        PredictOpt::Geode(o) => cmd_predict_geode(o),
    }
}

fn cmd_generate_objects(opt: &GameContentLoc) -> Result<()> {
    let data = GameData::from_content_dir(opt.get()?)?;
    let locale = Locale::from_content_dir(opt.get()?, "en-EN")?;
    for (id, object) in &data.objects {
        let display_name = object.display_name(&locale);
        let mut const_name = match id.as_str() {
            "180" => "EGG_180".to_string(),
            "182" => "LARGE_EGG_182".to_string(),
            "930" => "QUESTION_MARKS".to_string(),
            "BasicCoalNode0" => "BASIC_COAL_NODE_0".to_string(),
            "BasicCoalNode1" => "BASIC_COAL_NODE_1".to_string(),
            "CalicoEggStone_0" => "CALICO_EGG_STONE_0".to_string(),
            "CalicoEggStone_1" => "CALICO_EGG_STONE_1".to_string(),
            "CalicoEggStone_2" => "CALICO_EGG_STONE_2".to_string(),
            "GreenRainWeeds0" => "GREEN_RAIN_WEEDS_0".to_string(),
            "GreenRainWeeds1" => "GREEN_RAIN_WEEDS_1".to_string(),
            "GreenRainWeeds2" => "GREEN_RAIN_WEEDS_2".to_string(),
            "GreenRainWeeds3" => "GREEN_RAIN_WEEDS_3".to_string(),
            "GreenRainWeeds4" => "GREEN_RAIN_WEEDS_4".to_string(),
            "GreenRainWeeds5" => "GREEN_RAIN_WEEDS_5".to_string(),
            "GreenRainWeeds6" => "GREEN_RAIN_WEEDS_6".to_string(),
            "GreenRainWeeds7" => "GREEN_RAIN_WEEDS_7".to_string(),
            "PotOfGold" => "POT_OF_GOLD".to_string(),
            "SeedSpot" => "SEED_SPOT".to_string(),
            "SpecificBait" => "SPECIFIC_BAIT".to_string(),
            "VolcanoCoalNode0" => "VOLCANO_COAL_NODE_0".to_string(),
            "VolcanoCoalNode1" => "VOLCANO_COAL_NODE_1".to_string(),
            "VolcanoGoldNode" => "VOLCANO_GOLD_NODE".to_string(),
            _ => display_name
                .to_ascii_uppercase()
                .replace([' ', '-'], "_")
                .replace('ñ', "N")
                .replace(['.', ',', '\'', '(', ')', ':'], ""),
        };

        if const_name == "WEEDS" && id != "0" {
            const_name = format!("WEEDS_{id}");
        }

        if const_name == "STONE" && id != "390" {
            const_name = format!("STONE_{id}");
        }

        if const_name == "SNOWY_STONE" {
            const_name = format!("SNOWY_STONE_{id}");
        }

        if const_name == "STRANGE_DOLL" {
            const_name = format!("STRANGE_DOLL_{id}");
        }

        if const_name == "TWIG" {
            const_name = format!("TWIG_{id}");
        }

        if const_name == "ICE_CRYSTAL" {
            const_name = format!("ICE_CRYSTAL_{id}");
        }

        if const_name == "ROTTEN_PLANT" {
            const_name = format!("ROTTEN_PLANT_{id}");
        }

        if const_name == "FOSSIL_STONE" {
            const_name = format!("FOSSIL_STONE_{id}");
        }

        if const_name == "CINDER_SHARD_STONE" {
            const_name = format!("CINDER_SHARD_STONE_{id}");
        }

        if const_name == "COPPER_STONE" {
            const_name = format!("COPPER_STONE_{id}");
        }

        if const_name == "IRON_STONE" {
            const_name = format!("IRON_STONE_{id}");
        }

        if const_name == "SUPPLYCRATE" {
            const_name = format!("SUPPLYCRATE_{id}");
        }

        println!("pub const {const_name}: ItemId = item_id!(\"(O){id}\");");
    }

    Ok(())
}

fn cmd_generate(opt: &GenerateOpt) -> Result<()> {
    match opt {
        GenerateOpt::Objects(o) => cmd_generate_objects(o),
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
        Opt::Generate(o) => cmd_generate(&o)?,
        //Opt::Geodes(o) => cmd_geodes(&o)?,
        Opt::Items(o) => cmd_items(&o)?,
        Opt::Package(o) => cmd_package(&o)?,
        Opt::Perfection(o) => cmd_perfection(&o)?,
        Opt::Predict(o) => cmd_predict(&o)?,
        Opt::RenderMap(o) => cmd_render_map(&o)?,
        Opt::Todo(o) => cmd_todo(&o)?,
    }

    Ok(())
}
