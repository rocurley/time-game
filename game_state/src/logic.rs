extern crate types;

extern crate nalgebra;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use self::types::{Direction, GameCell, GameFrame, Move, Plan, Player, Point, Portal,
                  PortalGraphNode};

pub fn apply_plan(initial_frame: &GameFrame, plan: &Plan) -> Result<GameFrame, &'static str> {
    let mut portals = initial_frame.portals.clone();
    let mut portal_graph = initial_frame.portal_graph.clone();
    let mut map: HashMap<Point, GameCell> = initial_frame
        .map
        .iter()
        .filter_map(|(k, old_cell)| {
            if old_cell.portal.is_none() && old_cell.item.is_none() {
                return None;
            }
            let mut cell = old_cell.clone();
            cell.player = None;
            Some((k.clone(), cell))
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
                if let Entry::Occupied(mut game_cell_entry) = map.entry(old_player.position) {
                    {
                        let game_cell = game_cell_entry.get_mut();
                        let portal_id = game_cell
                            .portal
                            .take()
                            .ok_or("Tried to close loop at wrong position")?;
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
                    }
                    if game_cell_entry.get().is_empty() {
                        game_cell_entry.remove();
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
        let game_cell = map.entry(*pos).or_insert_with(GameCell::new);
        if game_cell.portal.is_some() {
            return Err("Overlapping portals prohibited.");
        }
        let portal = Portal::new(0, *pos);
        let portal_id = portal.id;
        game_cell.portal = Some(portal_id);
        portals.insert(portal_id, portal);
        portal_graph.insert_node(
            PortalGraphNode::Portal(portal_id),
            Vec::new(),
            vec![(PortalGraphNode::End, player_id)],
        );
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
            (game_frame.players.len()..game_frame.players.len() + 1),
        ).prop_map(move |moves_vec| {
            game_frame
                .players
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
                    .insert_player(player)
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
        let mut players_count = 0;
        let mut portals_count = 0;
        for (pt, game_cell) in game_frame.map.iter() {
            for player_id in game_cell.player.iter() {
                players_count += 1;
                assert_eq!(game_frame.players.get(player_id).unwrap().position, *pt)
            }
            for portal_id in game_cell.portal.iter() {
                portals_count += 1;
                assert_eq!(
                    game_frame.portals.get(portal_id).unwrap().player_position,
                    *pt
                )
            }
        }
        assert_eq!(players_count, game_frame.players.len());
        assert_eq!(portals_count, game_frame.portals.len());
    }

    proptest!{
        #[test]
        fn test_insert_player(ref player in arbitrary_player()) {
            let mut frame = GameFrame::new();
            frame.insert_player(player.clone()).expect("Failed to insert player");
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
