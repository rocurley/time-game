use petgraph::graphmap::GraphMap;
use petgraph::visit;
use petgraph::visit::IntoNeighbors;
use petgraph::visit::Walker;

use game_frame::GameFrame;
use ggez::nalgebra;
use portal_graph::{
    find_latest_held, find_latest_held_index, render_item_graph, signed_wish, ItemPortalGraphNode,
    PlayerPortalGraphNode,
};
use std::cmp::min;
use std::iter;
use types::{
    entities_at, Direction, DoubleMap, Entity, GameError, HypotheticalInventory, Inventory,
    ItemDrop, Move, Plan, Player, Portal,
};

pub fn apply_plan(initial_frame: &GameFrame, plan: &Plan) -> Result<GameFrame, GameError> {
    let mut portals = initial_frame.portals.clone();
    let mut player_portal_graph = initial_frame.player_portal_graph.clone();
    let mut item_portal_graphs = initial_frame.item_portal_graphs.clone();
    let mut items = initial_frame.items.clone();
    let mut players = DoubleMap::new();
    let mut ecs = initial_frame.ecs.clone();
    let mut jumpers: Vec<&Player> = Vec::new();
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
                let mut to_unlock = Vec::<Entity>::new();
                for (e, lock) in ecs.locks.iter() {
                    if !ecs.entities.contains_key(e) {
                        continue;
                    }
                    if ecs.positions.get(e) != Some(&player.position) {
                        continue;
                    }
                    let unlocking_item_count = player
                        .inventory
                        .count_items()
                        .get(&lock.unlocked_by)
                        .copied()
                        .unwrap_or(0);
                    if unlocking_item_count == 0 {
                        println!("Need a {:?}", &lock.unlocked_by);
                        continue;
                    }
                    to_unlock.push(e);
                    if let Inventory::Hypothetical(ref mut inventory) = player.inventory {
                        let minimum = inventory
                            .minima
                            .entry(lock.unlocked_by.clone())
                            .or_insert(0);
                        *minimum = min(unlocking_item_count - 1, *minimum);
                    }
                }
                for e in to_unlock {
                    let lock = ecs.locks.remove(e).expect("Can't find lock to unlock");
                    ecs.entities
                        .remove(e)
                        .expect("Can't find locked entity to remove");
                    ecs.positions.insert(lock.when_unlocked, player.position);
                }
                for e in entities_at(&ecs, player.position) {
                    if ecs.impassible.contains_key(e) {
                        Err("tried to move into somewhere impassible")?;
                    }
                }
                players.insert(player)?;
            }
            Some(&Move::Jump) => {
                // We can't do everything right now, because we need to wait for all the players to
                // exist in the new game frame. To make thing simple, we'll wait to do anything at
                // all.
                jumpers.push(old_player);
            }
            Some(&Move::PickUp) => {
                let mut player: Player = old_player.clone();
                let item_drop = items
                    .remove_by_position(&player.position)
                    .ok_or("Couln't pick up: no item")?;
                let item = item_drop.item;
                let prior_item_count = player.inventory.count_items().get(&item).map_or(0, |x| *x);
                let item_portal_graph = item_portal_graphs
                    .entry(item.clone())
                    .or_insert_with(GraphMap::new);
                let old_held_ix = find_latest_held_index(item_portal_graph, player.id).unwrap_or(0);
                let new_held_ix = old_held_ix + 1;
                item_portal_graph.add_edge(
                    ItemPortalGraphNode::Dropped(item_drop.id),
                    ItemPortalGraphNode::Held(player.id, new_held_ix),
                    1,
                );
                item_portal_graph.add_edge(
                    ItemPortalGraphNode::Held(player.id, old_held_ix),
                    ItemPortalGraphNode::Held(player.id, new_held_ix),
                    prior_item_count,
                );
                player.inventory.insert(&item)?;
                players.insert(player)?;
            }
            Some(&Move::Drop(item_ix)) => {
                let mut player: Player = old_player.clone();
                let item = player.inventory.drop(item_ix)?;
                let remaining_item_count =
                    player.inventory.count_items().get(&item).map_or(0, |x| *x);
                let item_drop = ItemDrop::new(item.clone(), player.position);
                let player_id = player.id;
                let item_drop_id = item_drop.id;
                items.insert(item_drop)?;
                players.insert(player)?;
                let item_portal_graph = item_portal_graphs
                    .entry(item.clone())
                    .or_insert_with(GraphMap::new);
                let latest_held_index =
                    find_latest_held_index(item_portal_graph, player_id).unwrap_or(0);
                let latest_held = ItemPortalGraphNode::Held(player_id, latest_held_index);
                item_portal_graph.add_edge(
                    latest_held,
                    ItemPortalGraphNode::Dropped(item_drop_id),
                    1,
                );
                let next_held = ItemPortalGraphNode::Held(player_id, latest_held_index + 1);
                item_portal_graph.add_edge(latest_held, next_held, remaining_item_count);
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
    for prior_player in jumpers {
        // Note that prior_player has not been inserted into players, nor will it be.
        // First, we remove the portal.
        let portal = portals
            .remove_by_position(&prior_player.position)
            .ok_or("Tried to close loop at wrong position")?;
        // Next, we find the player we're merging into: "post_player"
        let mut last_edge = None;
        visit::depth_first_search(
            &player_portal_graph,
            iter::once(PlayerPortalGraphNode::Portal(portal.id)),
            |e| {
                if let visit::DfsEvent::TreeEdge(n1, n2) = e {
                    last_edge = player_portal_graph.edge_weight(n1, n2);
                }
            },
        );
        let post_player_id = *last_edge.expect("No outgoing edges for closed portal");
        // TODO:There's a bug where this crashes if post_player themselves jumped, because they're
        // not in players. There are 3 possibilities:
        //
        // * The post_player didn't jump: they're in players.
        // * The post_player did jump, and they've already been processed (possibly many frames
        // ago). In that case, we want to follow the player_portal_graph, and figure out what
        // they're called now. DONE.
        // * The post_player did jump, and they haven't been processed. In that case, they're
        // somewhere in the rest of jumpers.
        //
        // By doing a topological sort, we could eliminate the last possibility.
        let mut post_player = players
            .get_mut_by_id(post_player_id)
            .expect("Couldn't get post_player by id");
        // Merge the inventories
        let post_inventory = match post_player.inventory {
            Inventory::Actual(_) => panic!("Merged into an actual inventory"),
            Inventory::Hypothetical(ref inventory) => inventory,
        };
        let (new_post_inventory, wishes) =
            post_inventory.merge_in(prior_player.inventory.clone())?;
        (*post_player).inventory = new_post_inventory;
        dbg!(&wishes);
        // Propegate the merge-implied wishes to the item graph. We need to do this before
        // modifying the players portal graph, or before adding the new edge to the item portal
        // graphs. Conceptually, wishing and unwishing happens _before_ the portal closes, to make
        // it valid to close the portal.
        let mut item_counts = prior_player.inventory.count_items();
        for wish in wishes {
            let item_count = item_counts.entry(wish.item.clone()).or_insert(0);
            *item_count = (*item_count as i32 + wish.prior_count) as usize;
            let item_portal_graph = item_portal_graphs
                .get_mut(&wish.item)
                .expect("no item portal graph for existant item");
            println!("Before wishing");
            render_item_graph(item_portal_graph);
            signed_wish(
                item_portal_graph,
                &player_portal_graph,
                prior_player.id,
                wish.prior_count,
            );
            println!("After prior wishing");
            render_item_graph(item_portal_graph);
            signed_wish(
                item_portal_graph,
                &player_portal_graph,
                post_player_id,
                wish.post_count,
            );
            println!("After post wishing");
            render_item_graph(item_portal_graph);
        }
        // Disconnect the player edge from end and connect it to the portal jumped into.
        let (player_origin, _, _) = player_portal_graph
            .all_edges()
            .find(|(_, _, &edge)| edge == prior_player.id)
            .expect("Couldn't find player in portal graph");
        player_portal_graph
            .remove_edge(player_origin, PlayerPortalGraphNode::End)
            .expect("Tried to close portal when edge unconnected to End");
        player_portal_graph.add_edge(
            player_origin,
            PlayerPortalGraphNode::Portal(portal.id),
            prior_player.id,
        );
        // Check that the player can still reach end (no loops)
        if !petgraph::algo::has_path_connecting(
            &player_portal_graph,
            PlayerPortalGraphNode::Portal(portal.id),
            PlayerPortalGraphNode::End,
            None,
        ) {
            Err("Created infinite loop")?;
        }
        // Add the edge linking prior and post players to the item portal graph
        dbg!(&post_player.inventory);
        for (item, item_portal_graph) in item_portal_graphs.iter_mut() {
            if let Some(origin_node) = find_latest_held(item_portal_graph, prior_player.id) {
                item_portal_graph.add_edge(
                    origin_node,
                    ItemPortalGraphNode::Held(post_player_id, 0),
                    item_counts.get(item).copied().unwrap_or(0),
                );
            }
        }
        // Define a "source" as a node with items flowing out of it but none going in.
        // This corresponds either to wishing or beginning. Similarly, define a sink as
        // having items going in but not out. This corresponds to a drop, or a current
        // hold.
        //
        // For an item graph to be valid, a source must connect to every node. This is
        // the same as saying every node must connect to a sink. By completing a jump,
        // we created an edge out of one node (origin_node) to another. The
        // only way we could have invalidated a graph is if we made the jump node
        // unable to reach a sink. This means that if we can connect the jump node to a
        // sink, we're good to go.

        for (item, item_portal_graph) in item_portal_graphs.iter_mut() {
            if let Some(origin_node) = find_latest_held(item_portal_graph, prior_player.id) {
                let filtered =
                    visit::EdgeFiltered::from_fn(&*item_portal_graph, |(_, _, &w)| w != 0);
                let mut dfs = visit::Dfs::new(&filtered, origin_node);
                let mut found_sink = false;
                while let Some(node) = dfs.next(&filtered) {
                    if filtered.neighbors(node).next().is_none() {
                        found_sink = true;
                        break;
                    }
                }
                if !found_sink {
                    render_item_graph(item_portal_graph);
                    Err(format!("Created infinite loop for {:?}", item))?;
                }
            }
        }
    }
    Ok(GameFrame {
        players,
        portals,
        items,
        player_portal_graph,
        item_portal_graphs,
        ecs,
    })
}
#[cfg(test)]
mod test;
