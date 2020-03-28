extern crate time_game_lib;

use time_game_lib::{
    game_state::GameState,
    types::{Action, Counter, Group, Item, ItemDrop, Key, MapElement, Player},
};

extern crate ggez;
use crate::nalgebra::Point2;
use ggez::*;

use std::{env, path};

pub fn main() {
    let mut cb = ContextBuilder::new("time game", "Roger")
        .window_setup(conf::WindowSetup::default().title("Time Game"))
        .window_mode(conf::WindowMode::default().dimensions(1000., 1000.));

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

    let (ctx, event_loop) = &mut cb.build().unwrap();
    let game_state = &mut GameState::new(ctx).unwrap();
    let game_frame = game_state.history.get_focus_val_mut();
    game_frame
        .insert_player(&game_state.image_map, Point2::new(0, 4))
        .expect("Could not insert player");
    game_frame
        .insert_item_drop(ItemDrop::new(Item::Key(Key {}), Point2::new(1, 1)), 1)
        .expect("Could not insert item");
    // TODO: Some possible puzzles:
    // Key in a locked room (below)
    // Multiple blocks required, only 1 block exists.
    // Get something from the end of a hallway with death walls coming towards you, IE:
    // ----------------------
    // x    x    x    x    x
    // ----------------------
    // ...
    // ----------------------
    //  x    x    x    x    x
    // ----------------------
    // ...
    // ----------------------
    //   x    x    x    x
    // ----------------------
    let map = [
        (
            MapElement::Wall,
            vec![
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (0, 1),
                (3, 1),
                (0, 2),
                (3, 2),
                (0, 3),
                (2, 3),
                (3, 3),
            ],
        ),
        (MapElement::ClosedDoor, vec![(1, 3)]),
    ];
    for (el, pts) in map.iter() {
        for &(x, y) in pts {
            el.add(
                &game_state.image_map,
                Point2::new(x, y),
                &mut game_frame.ecs,
            );
        }
    }
    let light_door = MapElement::RemoteDoor.add(
        &game_state.image_map,
        Point2::new(5, 6),
        &mut game_frame.ecs,
    );
    let light = MapElement::Light {
        counter: Counter::Unlock,
        rising: Action::All(vec![
            Action::SetImage {
                target: light_door,
                img: game_state.image_map.open_door.clone(),
            },
            Action::DisableGroup(light_door, Group::Locked),
        ]),
        falling: Action::All(vec![
            Action::SetImage {
                target: light_door,
                img: game_state.image_map.closed_door.clone(),
            },
            Action::EnableGroup(light_door, Group::Locked),
        ]),
    }
    .add(
        &game_state.image_map,
        Point2::new(5, 5),
        &mut game_frame.ecs,
    );
    MapElement::Plate(Counter::Unlock, light).add(
        &game_state.image_map,
        Point2::new(4, 5),
        &mut game_frame.ecs,
    );
    MapElement::Plate(Counter::Unlock, light).add(
        &game_state.image_map,
        Point2::new(3, 5),
        &mut game_frame.ecs,
    );
    MapElement::Plate(Counter::Unlock, light).add(
        &game_state.image_map,
        Point2::new(2, 5),
        &mut game_frame.ecs,
    );
    event::run(ctx, event_loop, game_state).unwrap();
}
