use anyhow::{anyhow, bail, Context, Result};
use std::{collections::HashMap, fs::File, io::BufReader, path::Path};
use xnb::Xnb;

use crate::common::Size;

fn xnb_props_to_hash_map(props: &[xnb::value::map::Property]) -> Result<HashMap<String, String>> {
    props
        .iter()
        .map(|prop| {
            let xnb::Value::String(value) = &prop.val else {
                return Err(anyhow!(
                    "Encountered non-string property value {:?}",
                    prop.val
                ));
            };

            Ok((prop.key.clone(), value.clone()))
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct TileSheet {
    pub id: String,
    pub description: String,
    pub image_src: String,
    pub sheet_size: Size<usize>,
    pub tile_size: Size<usize>,
    pub margin: Size<usize>,
    pub spacing: Size<usize>,
    pub properties: HashMap<String, String>,
}

impl TileSheet {
    fn from_xnb_data(sheet: &xnb::value::map::TileSheet) -> Result<Self> {
        Ok(TileSheet {
            id: sheet.id.clone(),
            description: sheet.desc.clone(),
            image_src: sheet.image_src.clone(),
            sheet_size: sheet.sheet_size.clone().into(),
            tile_size: sheet.tile_size.clone().into(),
            margin: sheet.margin.clone().into(),
            spacing: sheet.spacing.clone().into(),
            properties: xnb_props_to_hash_map(&sheet.props)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StaticTile {
    pub index: i32,
    pub tile_sheet: usize,
    pub blend_mode: u32,
}

impl StaticTile {
    fn from_xnb_data(
        tile: &xnb::value::map::StaticTile,
        tile_sheet_map: &HashMap<String, usize>,
    ) -> Result<Self> {
        let tile_sheet = tile_sheet_map
            .get(&tile.tile_sheet)
            .ok_or_else(|| anyhow!("Unkown tile sheet: {}", &tile.tile_sheet))?;

        Ok(StaticTile {
            index: tile.index,
            tile_sheet: *tile_sheet,
            blend_mode: tile.blend_mode as u32,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tile {
    Null,
    StaticTile(StaticTile),
    AnimatedTile {
        interval: i32,
        count: i32,
        frames: Vec<StaticTile>,
    },
}

impl Tile {
    fn from_xnb_data(
        tile: &xnb::value::map::Tile,
        tile_sheet_map: &HashMap<String, usize>,
    ) -> Result<Self> {
        match tile {
            xnb::value::map::Tile::Null => Ok(Tile::Null),
            xnb::value::map::Tile::Static(tile) => Ok(Tile::StaticTile(StaticTile::from_xnb_data(
                tile,
                tile_sheet_map,
            )?)),
            xnb::value::map::Tile::Animated(tile) => Ok(Tile::AnimatedTile {
                interval: tile.interval,
                count: tile.count,
                frames: tile
                    .frames
                    .iter()
                    .map(|tile| StaticTile::from_xnb_data(tile, tile_sheet_map))
                    .collect::<Result<Vec<StaticTile>>>()?,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub id: String,
    pub visible: bool,
    pub description: String,
    pub size: Size<usize>,
    pub tile_size: Size<usize>,
    pub properties: HashMap<String, String>,
    pub tiles: Vec<Tile>,
}

impl Layer {
    fn from_xnb_data(
        layer: &xnb::value::map::Layer,
        tile_sheet_map: &HashMap<String, usize>,
    ) -> Result<Self> {
        Ok(Layer {
            id: layer.id.clone(),
            visible: layer.visible,
            description: layer.desc.clone(),
            size: layer.size.clone().into(),
            tile_size: layer.tile_size.clone().into(),
            properties: xnb_props_to_hash_map(&layer.props)?,
            tiles: layer
                .tiles
                .iter()
                .map(|tile| Tile::from_xnb_data(tile, tile_sheet_map))
                .collect::<Result<Vec<Tile>>>()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub tile_sheets: Vec<TileSheet>,
    pub properties: HashMap<String, String>,
    pub layers: Vec<Layer>,
}

impl Map {
    pub fn load<P: AsRef<Path>>(file: P) -> Result<Self> {
        let f = File::open(&file).context("Can't open object file")?;
        let mut r = BufReader::new(f);
        let xnb = Xnb::new(&mut r).context("Can't parse object xnb file")?;

        let xnb::Value::Map(map) = xnb.content else {
            bail!("{} is not a map", file.as_ref().to_string_lossy());
        };

        let properties = xnb_props_to_hash_map(&map.properties)?;
        let tile_sheets = map
            .tile_sheets
            .iter()
            .map(TileSheet::from_xnb_data)
            .collect::<Result<Vec<TileSheet>>>()?;

        let tile_sheet_map: HashMap<_, _> = tile_sheets
            .iter()
            .enumerate()
            .map(|(index, value)| (value.id.clone(), index))
            .collect();

        let layers = map
            .layers
            .iter()
            .map(|layer| Layer::from_xnb_data(layer, &tile_sheet_map))
            .collect::<Result<Vec<Layer>>>()?;

        Ok(Self {
            properties,
            tile_sheets,
            layers,
        })
    }
}
