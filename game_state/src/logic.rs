extern crate types;

extern crate nalgebra;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use self::types::{Direction, GameCell, GameFrame, Move, Plan, Player, Point, Portal,
                  PortalGraphNode};

pub fn apply_plan(initial_frame: &GameFrame, plan: &Plan) -> Result<GameFrame, &'static str> {
    let mut portals = initial_frame.portals.clone();
    let mut portal_graph = initial_frame.portal_graph.clone();
    let mut map: HashMap<Point, GameCell> = portals
        .values()
        .map(|portal| {
            (
                portal.player_position,
                GameCell {
                    player: None,
                    portal: Some(portal.id),
                },
            )
        })
        .collect();
    let mut players = HashMap::new();
    for old_player in initial_frame.players.values() {
        match plan.moves.get(&old_player.id) {
            None => {
                players.insert(old_player.id, old_player.clone());
            }
            Some(&Move::Direction(ref direction)) => {
                let mut player: Player = old_player.clone();
                let delta: nalgebra::Vector2<i32> = match *direction {
                    Direction::Up => -nalgebra::Vector2::y(),
                    Direction::Down => nalgebra::Vector2::y(),
                    Direction::Left => -nalgebra::Vector2::x(),
                    Direction::Right => nalgebra::Vector2::x(),
                };
                player.position += delta;
                players.insert(player.id, player);
            }
            Some(&Move::Jump) => {
                //Since map was only populated with portals thus far, this is fine.
                if let Entry::Occupied(game_cell_entry) = map.entry(old_player.position) {
                    let (_, game_cell) = game_cell_entry.remove_entry();
                    let portal_id = game_cell
                        .portal
                        .expect("Portal missing when game cell was not.");
                    portals.remove(&portal_id);
                    portal_graph
                        .edges
                        .insert(old_player.id, PortalGraphNode::Portal(portal_id));
                    if !portal_graph
                        .get_node(PortalGraphNode::Portal(portal_id))
                        .connected_to(PortalGraphNode::End)
                    {
                        return Err("Created infinite loop");
                    }
                } else {
                    return Err("Tried to close loop at wrong positon");
                }
            }
        }
    }
    for pos in plan.portals.iter() {
        let player = Player::new(*pos);
        let player_id = player.id;
        players.insert(player_id, player);
        match map.entry(*pos) {
            //Again, the map is only populated with portals so far
            Entry::Occupied(_) => return Err("Overlapping portals prohibited."),
            Entry::Vacant(vacant_entry) => {
                let portal = Portal::new(0, *pos);
                let portal_id = portal.id;
                vacant_entry.insert(GameCell {
                    portal: Some(portal_id),
                    player: None,
                });
                portals.insert(portal_id, portal);
                portal_graph.insert_node(
                    PortalGraphNode::Portal(portal_id),
                    Vec::new(),
                    vec![(PortalGraphNode::End, player_id)],
                );
            }
        };
    }
    for player in players.values() {
        let game_cell = map.entry(player.position).or_insert_with(GameCell::new);
        if !game_cell.player.is_none() {
            return Err("Player collision");
        }
        game_cell.player = Some(player.id)
    }
    Ok(GameFrame {
        players,
        portals,
        map,
        portal_graph,
    })
}
