extern crate types;

extern crate nalgebra;

use std::collections::hash_map::Entry;
use self::types::{Direction, GameFrame, Move, Plan, Player, Portal, PortalGraphNode};

pub fn apply_plan(initial_frame: &GameFrame, plan: &Plan) -> Result<GameFrame, &'static str> {
    let mut portals = initial_frame.portals.clone();
    let mut portal_graph = initial_frame.portal_graph.clone();
    let mut players = initial_frame
        .players
        .iter()
        .filter_map(|old_player: &Player| match plan.moves.get(&old_player.id) {
            None => Some(Ok(old_player.clone())),
            Some(&Move::Direction(ref direction)) => {
                let mut player: Player = old_player.clone();
                let delta: nalgebra::Vector2<i32> = match *direction {
                    Direction::Up => -nalgebra::Vector2::y(),
                    Direction::Down => nalgebra::Vector2::y(),
                    Direction::Left => -nalgebra::Vector2::x(),
                    Direction::Right => nalgebra::Vector2::x(),
                };
                player.position += delta;
                Some(Ok(player))
            }
            Some(&Move::Jump) => {
                if let Entry::Occupied(portal_entry) = portals.entry(old_player.position) {
                    let (_, portal) = portal_entry.remove_entry();
                    portal_graph
                        .edges
                        .insert(old_player.id, PortalGraphNode::Portal(portal.id));
                    if portal_graph
                        .get_node(PortalGraphNode::Portal(portal.id))
                        .connected_to(PortalGraphNode::End)
                    {
                        None
                    } else {
                        Some(Err("Created infinite loop"))
                    }
                } else {
                    Some(Err("Tried to close loop at wrong positon"))
                }
            }
        })
        .collect::<Result<Vec<Player>, &str>>()?;
    for pos in plan.portals.iter() {
        let player = Player::new(*pos);
        let player_id = player.id;
        players.push(player);
        match portals.entry(*pos) {
            Entry::Occupied(_) => return Err("Overlapping portals prohibited."),
            Entry::Vacant(vacant_entry) => {
                let portal_id = vacant_entry.insert(Portal::new(0, *pos)).id;
                portal_graph.insert_node(
                    PortalGraphNode::Portal(portal_id),
                    Vec::new(),
                    vec![(PortalGraphNode::End, player_id)],
                );
            }
        };
    }
    Ok(GameFrame {
        players,
        portals,
        portal_graph,
    })
}
