extern crate game_frame;
extern crate petgraph;
extern crate portal_graph;
extern crate types;

extern crate ggez;

use self::graphmap::GraphMap;
use self::petgraph::graphmap;

use self::game_frame::GameFrame;
use self::portal_graph::{find_latest_held, ItemPortalGraphNode, PlayerPortalGraphNode};
use self::types::{
    Direction, DoubleMap, HypotheticalInventory, Inventory, ItemDrop, Move, Plan, Player, Portal, GameError
};
use ggez::nalgebra;

pub fn apply_plan(initial_frame: &GameFrame, plan: &Plan) -> Result<GameFrame, GameError> {
    let mut portals = initial_frame.portals.clone();
    let mut player_portal_graph = initial_frame.player_portal_graph.clone();
    let mut item_portal_graphs = initial_frame.item_portal_graphs.clone();
    let mut items = initial_frame.items.clone();
    let mut players = DoubleMap::new();
    for (_, old_player) in initial_frame.players.iter() {
        match plan.moves.get(&old_player.id) {
            None => {
                players.insert(old_player.clone())?;
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
                players.insert(player)?;
            }
            Some(&Move::Jump) => {
                let portal = portals
                    .remove_by_position(&old_player.position)
                    .ok_or("Tried to close loop at wrong position")?;
                let (player_origin, _, _) = player_portal_graph
                    .all_edges()
                    .find(|(_, _, &edge)| edge == old_player.id)
                    .expect("Couldn't find player in portal graph");
                player_portal_graph
                    .remove_edge(player_origin, PlayerPortalGraphNode::End)
                    .expect("Tried to close portal when edge unconnected to End");
                player_portal_graph.add_edge(
                    player_origin,
                    PlayerPortalGraphNode::Portal(portal.id),
                    old_player.id,
                );
                let new_player_id = match player_portal_graph
                    .edges(PlayerPortalGraphNode::Portal(portal.id))
                    .collect::<Vec<_>>()
                    .as_slice()
                {
                    [(_, _, &new_player_id)] => new_player_id,
                    slice => panic!("New node didn't have exactly one outgoing edge: {:?}", slice),
                };
                if !petgraph::algo::has_path_connecting(
                    &player_portal_graph,
                    PlayerPortalGraphNode::Portal(portal.id),
                    PlayerPortalGraphNode::End,
                    None,
                ) {
                    Err("Created infinite loop")?;
                }
                for (_, item_portal_graph) in item_portal_graphs.iter_mut() {
                    for origin_node in find_latest_held(item_portal_graph, old_player.id) {
                        item_portal_graph.add_edge(
                            origin_node,
                            ItemPortalGraphNode::Held(new_player_id, 0),
                            (),
                        );
                    }
                }
            }
            Some(&Move::PickUp) => {
                let mut player: Player = old_player.clone();
                let item_drop = items
                    .remove_by_position(&player.position)
                    .ok_or("Couln't pick up: no item")?;
                let item = item_drop.item;
                let item_portal_graph = item_portal_graphs
                    .entry(item.clone())
                    .or_insert_with(GraphMap::new);
                let new_held_ix = (0usize..)
                    .find(|&i| {
                        !item_portal_graph.contains_node(ItemPortalGraphNode::Held(player.id, i))
                    }).expect("Exhausted usize looking for unused held index");
                item_portal_graph.add_edge(
                    ItemPortalGraphNode::Dropped(item_drop.id),
                    ItemPortalGraphNode::Held(player.id, new_held_ix),
                    (),
                );
                player.inventory.insert(&item)?;
                players.insert(player)?;
            }
            Some(&Move::Drop(item_ix)) => {
                //TODO: We need to be able to update the item graph live as you wish and unwish for
                //things.
                let mut player: Player = old_player.clone();
                let item = player.inventory.drop(item_ix)?;
                let still_holding = player
                    .inventory
                    .cells()
                    .iter()
                    .any(|o_cell| o_cell.as_ref().map_or(false, |cell| cell.item == item));
                let item_drop = ItemDrop::new(item.clone(), player.position);
                let player_id = player.id;
                let item_drop_id = item_drop.id;
                items.insert(item_drop)?;
                players.insert(player)?;
                let item_portal_graph = item_portal_graphs
                    .entry(item.clone())
                    .or_insert_with(GraphMap::new);
                let new_held_ix = (0usize..)
                    .find(|&i| {
                        !item_portal_graph.contains_node(ItemPortalGraphNode::Held(player_id, i))
                    }).expect("Exhausted usize looking for unused held index");
                item_portal_graph.add_edge(
                    ItemPortalGraphNode::Dropped(item_drop_id),
                    ItemPortalGraphNode::Held(player_id, new_held_ix),
                    (),
                );
                if still_holding {
                    let held_ix = new_held_ix - 1;
                    item_portal_graph.add_edge(
                        ItemPortalGraphNode::Dropped(item_drop_id),
                        ItemPortalGraphNode::Held(player_id, held_ix),
                        (),
                    );
                }
            }
        }
    }
    for pos in plan.portals.iter() {
        let mut player = Player::new(*pos);
        player.inventory = Inventory::Hypothetical(HypotheticalInventory::new());
        let player_id = player.id;
        players.insert(player)?;
        let portal = Portal::new(0, *pos);
        let portal_id = portal.id;
        portals.insert(portal)?;
        player_portal_graph.add_edge(
            PlayerPortalGraphNode::Portal(portal_id),
            PlayerPortalGraphNode::End,
            player_id,
        );
    }
    Ok(GameFrame {
        players,
        portals,
        items,
        player_portal_graph,
        item_portal_graphs,
    })
}
#[cfg(test)]
mod tests {
    use super::super::proptest;
    use super::game_frame::GameFrame;
    use super::types::{Direction, Move, Plan, Player};
    use logic::apply_plan;
    use nalgebra::Point2;
    use proptest::prelude::*;
    use std::collections::HashSet;

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
            game_frame.players.len()..game_frame.players.len() + 1,
        )
        .prop_map(move |moves_vec| {
            game_frame
                .players
                .iter()
                .map(|(id, _)| id.clone())
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
            }).prop_recursive(depth, depth, 1, |prop_prior_frames| {
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
                    }).boxed()
            }).boxed()
    }

    proptest! {
        #[test]
        fn test_apply_plan(ref game_frames in unfold_arbitrary_plans(10)) {
            for frame in game_frames.iter() {
                prop_assert_eq!(frame.players.len() - frame.portals.len(), 1)
            }
        }
    }
    #[test]
    fn test_loop() {
        let game_frame_0 = GameFrame::new();
        let mut plan_0 = Plan::new();
        plan_0.portals.insert(Point2::new(0, 0));
        let game_frame_1 = apply_plan(&game_frame_0, &plan_0).expect("Couldn't create a portal");
        let player_id = game_frame_1
            .players
            .id_by_position(&Point2::new(0, 0))
            .expect("Couldn't find a player at (0,0)");
        let mut plan_1 = Plan::new();
        plan_1.moves.insert(player_id, Move::Jump);
        apply_plan(&game_frame_1, &plan_1).expect_err("Completed infinite loop");
    }
}
