extern crate game_state;
extern crate types;

use types::{GameCell, Player, PortalGraphNode};
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
    {
        let game_frame = game_state.history.get_focus_val_mut();
        let player = Player::new(Point2::new(0, 4));
        let player_id = player.id;
        game_frame.map.insert(
            player.position,
            GameCell {
                player: Some(player_id),
                portal: None,
            },
        );
        game_frame.players.insert(player_id, player);
        game_frame
            .portal_graph
            .insert_node(PortalGraphNode::Beginning, Vec::new(), Vec::new());
        game_frame.portal_graph.insert_node(
            PortalGraphNode::End,
            vec![(PortalGraphNode::Beginning, player_id)],
            Vec::new(),
        );
    }
    event::run(ctx, game_state).unwrap();
}
