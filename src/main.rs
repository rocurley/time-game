extern crate game_state;
extern crate types;

//use types::{CachablePlan, Direction, GameState, Move, Plan, Player, Selection};
use types::Player;
use game_state::GameState;

//mod support;
//mod logic;

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
    let game_state = &mut GameState::new(ctx).unwrap();
    game_state
        .history
        .get_focus_val_mut()
        .players
        .push(Player::new(Point2::new(0, 4)));
    event::run(ctx, game_state).unwrap();
}
