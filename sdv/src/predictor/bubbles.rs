use anyhow::Result;
use log::debug;
use xnb::xtile::Map;

use crate::{
    common::{Point, Rect, TimeSpan},
    rng::{Rng, SeedGenerator},
};

#[derive(Clone, Debug)]
pub struct Bubbles {
    pub location: Point<i32>,
    pub span: TimeSpan,
}

fn time_to_minutes(time: i32) -> i32 {
    time / 100 * 60 + time % 100
}

fn minutes_between_times(start: i32, end: i32) -> i32 {
    time_to_minutes(end) - time_to_minutes(start)
}

fn is_water_tile(map: &Map, x: i32, y: i32) -> bool {
    let prop = map.get_tile_property(x, y, "Water", "Back");

    prop.is_some()
}

fn is_open_water(map: &Map, x: i32, y: i32) -> bool {
    if !is_water_tile(map, x, y) {
        return false;
    }

    let Some(layer) = map.get_layer("Buildings") else {
        return true;
    };

    let Some(tile) = layer.get_tile(x, y) else {
        return true;
    };

    let Some((index, sheet)) = tile.sheet_and_index() else {
        return true;
    };

    sheet == "outdoors" && [628, 629, 734, 759].contains(&index)

    // At this point the game checks for objects existing (like crab pots?)
}

fn pt_in_bounds(map: &Map, x: i32, y: i32) -> bool {
    (0..map.layers[0].size.w).contains(&x) && (0..map.layers[0].size.h).contains(&y)
}

fn distance_to_land(map: &Map, x: i32, y: i32) -> i32 {
    let mut bounding_rect = Rect::from_xywh(x - 1, y - 1, 3, 3);
    let mut found_land = false;
    let mut distance = 1;
    while !found_land && bounding_rect.width() <= 11 {
        for p in bounding_rect.border_points() {
            if !pt_in_bounds(map, p.x, p.y) || is_water_tile(map, p.x, p.y) {
                continue;
            }
            found_land = true;
            distance = bounding_rect.width() / 2;
            break;
        }
        bounding_rect.inflate(1, 1);
    }

    if bounding_rect.width() > 11 {
        6
    } else {
        distance - 1
    }
}

pub fn calculate_bubbles<G: SeedGenerator>(
    map: &Map,
    days_played: u32,
    game_id: u32,
) -> Result<Vec<Bubbles>> {
    let mut fish_splash_point_time = 0;
    let mut fish_splash_point: Option<Point<i32>> = None;
    let mut bubbles = Vec::new();

    let map_size = &map.layers[0].size;

    for time_of_day in (610..2600).step_by(10) {
        if time_of_day % 100 >= 60 {
            continue;
        }

        let mut r = Rng::new(G::generate_day_save_seed(
            days_played,
            game_id,
            time_of_day as f64,
            map_size.w as f64,
            0 as f64,
        ));

        let splash_point_druation_so_far =
            minutes_between_times(fish_splash_point_time, time_of_day);
        // frenzy = fishFrenzyFish.Value != null && !fishFrenzyFish.Value.Equals("");
        debug!("{time_of_day}");
        if fish_splash_point.is_none() && r.next_bool() {
            for _tries in 0..2 {
                let p = Point {
                    x: r.next_range(0, map_size.w)?,
                    y: r.next_range(0, map_size.h)?,
                };
                debug!(
                    "{p:?} {} {:?}",
                    is_open_water(map, p.x, p.y),
                    map.get_tile_property(p.x, p.y, "NoFishing", "Back")
                );
                if !is_open_water(map, p.x, p.y)
                    || map
                        .get_tile_property(p.x, p.y, "NoFishing", "Back")
                        .is_some()
                {
                    continue;
                }

                let to_land = distance_to_land(map, p.x, p.y);
                debug!(" to land {to_land}");
                if to_land <= 1 || to_land >= 5 {
                    continue;
                }

                // TODO frienzy calc
                let _ = r.next_double();
                fish_splash_point = Some(p);
                fish_splash_point_time = time_of_day;
                break;
            }
        } else if fish_splash_point.is_some()
            && r.next_double() < 0.1 + ((splash_point_druation_so_far as f32 / 1800f32) as f64)
            && splash_point_druation_so_far > (if false /* frenzy */ { 120 } else { 60})
        {
            bubbles.push(Bubbles {
                location: fish_splash_point.expect("garuntted to exist"),
                span: TimeSpan {
                    start: fish_splash_point_time,
                    end: time_of_day,
                },
            });

            fish_splash_point = None;
            fish_splash_point_time = 0;
        }
    }

    Ok(bubbles)
}
