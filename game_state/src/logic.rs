extern crate types;

extern crate nalgebra;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use self::types::{insert_into_inventory, Direction, DoubleMap, GameFrame, Move, Plan, Player,
                  Portal, PortalGraphNode};

pub fn apply_plan(initial_frame: &GameFrame, plan: &Plan) -> Result<GameFrame, &'static str> {
    let mut portals = initial_frame.portals.clone();
    let mut portal_graph = initial_frame.portal_graph.clone();
    let mut items = initial_frame.items.clone();
    let mut players_by_id = HashMap::new();
    for old_player in initial_frame.players.by_id.values() {
        match plan.moves.get(&old_player.id) {
            None => {
                players_by_id.insert(old_player.id, old_player.clone());
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
                players_by_id.insert(player.id, player);
            }
            Some(&Move::Jump) => {
                let portal_id = portals
                    .by_position
                    .remove(&old_player.position)
                    .ok_or("Tried to close loop at wrong position")?;
                portals.by_id.remove(&portal_id);
                portal_graph
                    .edges
                    .insert(old_player.id, PortalGraphNode::Portal(portal_id));
                if !portal_graph
                    .get_node(PortalGraphNode::Portal(portal_id))
                    .connected_to(PortalGraphNode::End)
                {
                    return Err("Created infinite loop");
                }
            }
            Some(&Move::PickUp) => {
                let mut player: Player = old_player.clone();
                let item = items
                    .remove(&player.position)
                    .ok_or("Couln't pick up: no item")?;
                insert_into_inventory(&mut player.inventory, item)?;
                players_by_id.insert(player.id, player);
            }
            Some(&Move::Drop(item_ix)) => {
                let mut player: Player = old_player.clone();
                let mut inventory_cell = player.inventory[item_ix as usize]
                    .as_mut()
                    .ok_or("Tried to drop from empty inventory slot")?;
                inventory_cell.count -= 1;
                let item = inventory_cell.item.clone();
                if inventory_cell.count == 0 {
                    player.inventory[item_ix as usize] = None;
                }
                items.insert(player.position, item);
                players_by_id.insert(player.id, player);
            }
        }
    }
    for pos in plan.portals.iter() {
        let player = Player::new(*pos);
        let player_id = player.id;
        players_by_id.insert(player_id, player);
        let portal = Portal::new(0, *pos);
        let portal_id = portal.id;
        portals.insert(*pos, portal)?;
        portal_graph.insert_node(
            PortalGraphNode::Portal(portal_id),
            Vec::new(),
            vec![(PortalGraphNode::End, player_id)],
        );
    }
    let mut players = DoubleMap {
        by_id: players_by_id,
        by_position: HashMap::new(),
    };
    for player in players.by_id.values() {
        if let Entry::Vacant(entry) = players.by_position.entry(player.position) {
            entry.insert(player.id);
        } else {
            return Err("Player collision");
        }
    }
    Ok(GameFrame {
        players,
        portals,
        items,
        portal_graph,
    })
}
#[cfg(test)]
mod tests {
    use logic::apply_plan;
    use std::collections::HashSet;
    use super::super::proptest;
    use proptest::prelude::*;
    use super::types::{Direction, GameFrame, Move, Plan, Player};
    use nalgebra::Point2;

    static POSSIBLE_MOVES: [Move; 5] = [
        Move::Jump,
        Move::Direction(Direction::Up),
        Move::Direction(Direction::Down),
        Move::Direction(Direction::Left),
        Move::Direction(Direction::Right),
    ];

    fn arbitrary_player() -> BoxedStrategy<Player> {
        (proptest::num::i32::ANY, proptest::num::i32::ANY)
            .prop_map(|(x, y)| Player::new(Point2::new(x, y)))
            .boxed()
    }

    fn valid_plan(game_frame: GameFrame) -> BoxedStrategy<Plan> {
        let moves = proptest::collection::vec(
            proptest::sample::select(POSSIBLE_MOVES.as_ref()),
            (game_frame.players.by_id.len()..game_frame.players.by_id.len() + 1),
        ).prop_map(move |moves_vec| {
            game_frame
                .players
                .by_id
                .keys()
                .map(|id| id.clone())
                .zip(moves_vec.into_iter())
                .collect()
        });
        let portals = prop_oneof![
            4 => Just(HashSet::new()),
            1 => 
        (proptest::num::i32::ANY, proptest::num::i32::ANY)
            .prop_map(|(x, y)| {
                let mut portals = HashSet::new();
                let portal = Point2::new(x, y);
                portals.insert(portal);
                portals
            })
        ];
        (moves, portals)
            .prop_map(|(moves, portals)| Plan { moves, portals })
            .boxed()
    }

    fn unfold_arbitrary_plans(depth: u32) -> BoxedStrategy<Vec<GameFrame>> {
        arbitrary_player()
            .prop_map(|player| {
                let mut frame = GameFrame::new();
                frame
                    .players
                    .insert(player)
                    .expect("Failed to insert player");
                vec![frame]
            })
            .prop_recursive(depth, depth, 1, |prop_prior_frames| {
                prop_prior_frames
                    .prop_flat_map(|prior_frames: Vec<GameFrame>| {
                        let prior_frame: GameFrame =
                            prior_frames.last().expect("Empty frames vec").clone();
                        valid_plan(prior_frame.clone())
                            .prop_map(move |plan| apply_plan(&prior_frame, &plan.clone()))
                            .prop_filter("plan wasn't allowed", |frame| frame.is_ok())
                            .prop_map(move |frame| {
                                let new_frame = frame.expect("Should have been filtered");
                                let mut new_frames = prior_frames.clone();
                                new_frames.push(new_frame);
                                new_frames
                            })
                    })
                    .boxed()
            })
            .boxed()
    }

    fn check_frame_consistency(game_frame: &GameFrame) {
        for (pt, player_id) in game_frame.players.by_position.iter() {
            assert_eq!(
                game_frame.players.by_id.get(player_id).unwrap().position,
                *pt
            )
        }
        assert_eq!(
            game_frame.players.by_id.len(),
            game_frame.players.by_position.len()
        );
        assert_eq!(
            game_frame.portals.by_id.len(),
            game_frame.portals.by_position.len()
        );
    }

    proptest!{
        #[test]
        fn test_insert_player(ref player in arbitrary_player()) {
            let mut frame = GameFrame::new();
            frame.players.insert(player.clone()).expect("Failed to insert player");
            check_frame_consistency(& frame);
        }
        #[test]
        fn test_apply_plan(ref game_frames in unfold_arbitrary_plans(10)) {
            for frame in game_frames.iter() {
                check_frame_consistency(frame)
            }
        }
    }
}
