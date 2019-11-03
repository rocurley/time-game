extern crate time_game_lib;

use time_game_lib::game_state::GameState;
use time_game_lib::types::{Item, ItemDrop, Key, Map, MapElement, Player};

extern crate ggez;
use ggez::*;
use nalgebra::Point2;

use std::{env, path};

pub fn main() {
    let mut cb = ContextBuilder::new("time game", "Roger")
        .window_setup(conf::WindowSetup::default().title("Time Game"))
        .window_mode(conf::WindowMode::default().dimensions(1000, 1000));

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("assets");
        println!("Adding path {:?}", path);
        // We need this re-assignment alas, see
        // https://aturon.github.io/ownership/builders.html
        // under "Consuming builders"
        cb = cb.add_resource_path(path);
    } else {
        println!("Not building from cargo?  Ok.");
    }

    let ctx = &mut cb.build().unwrap();
    let mut map = Map::new(10, 10);
    map[(0, 0)] = MapElement::Wall;
    let game_state = &mut GameState::new(ctx, map).unwrap();
    {
        let game_frame = game_state.history.get_focus_val_mut();
        game_frame
            .insert_player(Player::new(Point2::new(0, 4)))
            .expect("Could not insert player");
        game_frame
            .insert_item_drop(ItemDrop::new(Item::Key(Key {}), Point2::new(3, 3)), 1)
            .expect("Could not insert item");
    }
    event::run(ctx, game_state).unwrap();
}
