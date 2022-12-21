use anyhow::Result;
use image::{imageops::overlay, GenericImageView, ImageBuffer, RgbaImage};
use sdv::{
    common::Size,
    gamedata::{Texture, Tile},
    GameData,
};

use super::RenderMapOpt;

pub(super) fn cmd_render_map(opt: &RenderMapOpt) -> Result<()> {
    let data = GameData::load(&opt.content.game_content)?;
    let map = data.load_map(&opt.map_name)?;
    let textures = map
        .tile_sheets
        .iter()
        .map(|sheet| data.load_texture(format!("Maps/{}.xnb", sheet.image_src)))
        .collect::<Result<Vec<Texture>>>()?;
    let texture_images: Vec<_> = textures
        .iter()
        .map(|texture| {
            ImageBuffer::from_raw(
                texture.texture.width as u32,
                texture.texture.height as u32,
                texture.texture.data.clone(),
            )
            .unwrap() as RgbaImage
        })
        .collect();

    let image_size = map
        .layers
        .iter()
        .fold(Size::<usize> { w: 0, h: 0 }, |acc, layer| Size {
            w: std::cmp::max(acc.w, layer.size.w * layer.tile_size.w),
            h: std::cmp::max(acc.h, layer.size.h * layer.tile_size.h),
        });
    let mut img: RgbaImage = ImageBuffer::new(image_size.w as u32, image_size.h as u32);

    for (i, layer) in map.layers.iter().enumerate() {
        let mut layer_img: RgbaImage = ImageBuffer::new(image_size.w as u32, image_size.h as u32);
        let layer_width = layer.size.w * layer.tile_size.w;
        let tile_width = layer.tile_size.w;
        let tile_height = layer.tile_size.h;
        for (i, tile) in layer.tiles.iter().enumerate() {
            match tile {
                Tile::StaticTile(tile) => {
                    let dest_x = (i * tile_width) % layer_width;
                    let dest_y = (i * tile_width) / layer_width * tile_height;

                    let texture = &texture_images[tile.tile_sheet];

                    let tile_x = (tile.index as usize * tile_width) as u32 % texture.width();
                    let tile_y = (tile.index as usize * tile_width) as u32 / texture.width()
                        * tile_height as u32;
                    let tile_img = texture
                        .view(tile_x, tile_y, tile_width as u32, tile_height as u32)
                        .to_image();

                    overlay(&mut layer_img, &tile_img, dest_x as i64, dest_y as i64)
                }
                Tile::AnimatedTile {
                    interval: _,
                    count: _,
                    frames,
                } => {
                    let tile = &frames[0];
                    let dest_x = (i * tile_width) % layer_width;
                    let dest_y = (i * tile_width) / layer_width * tile_height;

                    let texture = &texture_images[tile.tile_sheet];

                    let tile_x = (tile.index as usize * tile_width) as u32 % texture.width();
                    let tile_y = (tile.index as usize * tile_width) as u32 / texture.width()
                        * tile_height as u32;
                    let tile_img = texture
                        .view(tile_x, tile_y, tile_width as u32, tile_height as u32)
                        .to_image();

                    overlay(&mut layer_img, &tile_img, dest_x as i64, dest_y as i64)
                }
                _ => (),
            }
        }
        layer_img.save(format!("map-{i}-{}.png", layer.id))?;
        overlay(&mut img, &layer_img, 0, 0);
    }

    img.save("map.png")?;
    Ok(())
}
