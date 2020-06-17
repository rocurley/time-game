extern crate time_game_lib;

use time_game_lib::{
    game_state::GameState,
    types::{Action, Counter, Direction, Group, Item, ItemDrop, Key, MapElement},
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
                img: game_state.image_map.open_door,
            },
            Action::DisableGroup(light_door, Group::Locked),
        ]),
        falling: Action::All(vec![
            Action::SetImage {
                target: light_door,
                img: game_state.image_map.closed_door,
            },
            Action::EnableGroup(light_door, Group::Locked),
        ]),
    }
    .add(
        &game_state.image_map,
        Point2::new(5, 5),
        &mut game_frame.ecs,
    );
    let map = [
        (
            MapElement::Wall,
            vec![
                // Key Box
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
                // Death hallway
                (9, 0),
                (9, 1),
                (9, 3),
                (9, 4),
                (9, 5),
                (9, 6),
                (9, 7),
                (9, 8),
                (9, 9),
                (7, 0),
                (7, 1),
                (7, 2),
                (7, 3),
                (7, 4),
                (7, 5),
                (7, 6),
                (7, 8),
                (7, 9),
            ],
        ),
        (MapElement::ClosedDoor, vec![(1, 3)]),
        (
            MapElement::Plate(Counter::Unlock, light),
            vec![(2, 5), (3, 5), (4, 5)],
        ),
        (
            MapElement::MovingWall {
                reset: Some((Point2::new(8, 0), Point2::new(8, 12))),
                direction: Direction::Down,
            },
            vec![(8, 0), (8, 4), (8, 8)],
        ),
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
    event::run(ctx, event_loop, game_state).unwrap();
}
