use petgraph::{
    graphmap::GraphMap,
    visit::{self, IntoNeighbors},
};

use crate::{
    game_frame::GameFrame,
    portal_graph::{
        find_latest_held, find_latest_held_index, render_item_graph, signed_wish,
        ItemPortalGraphNode, PlayerPortalGraphNode,
    },
    types::{
        player_at, Action, Direction, Entity, EventTrigger, EventTriggerModifier, GameError,
        HypotheticalInventory, ImageMap, Inventory, ItemDrop, Move, Plan, Portal,
    },
};
use enum_map::EnumMap;
use enumset::EnumSet;
use ggez::nalgebra;
use std::{cmp::min, iter};

pub fn apply_plan(
    image_map: &ImageMap,
    initial_frame: &GameFrame,
    plan: &Plan,
) -> Result<GameFrame, GameError> {
    let mut out = initial_frame.clone();
    let mut jumpers: Vec<Entity> = Vec::new();
    for (&entity, mv) in plan.moves.iter() {
        if !out.ecs.entities.contains_key(entity) {
            panic!(
                "Plan for non-existant entity {:?} : ECS: {:#?}, Plan: {:#?}",
                entity, &out.ecs, plan
            );
        }
        match mv {
            Move::Direction(ref direction) => {
                let position = out
                    .ecs
                    .positions
                    .get_mut(entity)
                    .expect("Attempted to move entity with no position");
                let delta: nalgebra::Vector2<i32> = match *direction {
                    Direction::Up => -nalgebra::Vector2::y(),
                    Direction::Down => nalgebra::Vector2::y(),
                    Direction::Left => -nalgebra::Vector2::x(),
                    Direction::Right => nalgebra::Vector2::x(),
                };
                *position += delta;
            }
            Move::Jump => {
                // TODO: this is out of date: inline jumpers with the rest of the code here.
                // We can't do everything right now, because we need to wait for all the players to
                // exist in the new game frame. To make thing simple, we'll wait to do anything at
                // all.
                jumpers.push(entity);
            }
            Move::PickUp => {
                let inventory = out
                    .ecs
                    .players
                    .get_mut(entity)
                    .expect("Entity with no inventory attempted to pick up");
                let position = out
                    .ecs
                    .positions
                    .get(entity)
                    .expect("Entity with no position attempted to pick up");
                let item_drop = out
                    .items
                    .remove_by_position(position)
                    .ok_or("Couln't pick up: no item")?;
                let item = item_drop.item;
                let prior_item_count = inventory.count_items().get(&item).map_or(0, |x| *x);
                let item_portal_graph = out
                    .item_portal_graphs
                    .entry(item.clone())
                    .or_insert_with(GraphMap::new);
                let old_held_ix = find_latest_held_index(item_portal_graph, entity).unwrap_or(0);
                let new_held_ix = old_held_ix + 1;
                item_portal_graph.add_edge(
                    ItemPortalGraphNode::Dropped(item_drop.id),
                    ItemPortalGraphNode::Held(entity, new_held_ix),
                    1,
                );
                item_portal_graph.add_edge(
                    ItemPortalGraphNode::Held(entity, old_held_ix),
                    ItemPortalGraphNode::Held(entity, new_held_ix),
                    prior_item_count,
                );
                inventory.insert(&item)?;
            }
            Move::Drop(item_ix) => {
                let inventory = out
                    .ecs
                    .players
                    .get_mut(entity)
                    .expect("Entity with no inventory attempted to drop");
                let position = *out
                    .ecs
                    .positions
                    .get(entity)
                    .expect("Entity with no position attempted to drop");
                let item = inventory.drop(*item_ix)?;
                let remaining_item_count = inventory.count_items().get(&item).map_or(0, |x| *x);
                let item_drop = ItemDrop::new(item.clone(), position);
                let item_drop_id = item_drop.id;
                out.items.insert(item_drop)?;
                let item_portal_graph = out
                    .item_portal_graphs
                    .entry(item.clone())
                    .or_insert_with(GraphMap::new);
                let latest_held_index =
                    find_latest_held_index(item_portal_graph, entity).unwrap_or(0);
                let latest_held = ItemPortalGraphNode::Held(entity, latest_held_index);
                item_portal_graph.add_edge(
                    latest_held,
                    ItemPortalGraphNode::Dropped(item_drop_id),
                    1,
                );
                let next_held = ItemPortalGraphNode::Held(entity, latest_held_index + 1);
                item_portal_graph.add_edge(latest_held, next_held, remaining_item_count);
            }
        }
    }
    for &pos in plan.portals.iter() {
        let inventory = Inventory::Hypothetical(HypotheticalInventory::new());
        let player = out.ecs.insert_player(image_map, pos, inventory);
        let portal = Portal::new(0, pos);
        let portal_id = portal.id;
        out.portals.insert(portal)?;
        out.player_portal_graph.add_edge(
            PlayerPortalGraphNode::Portal(portal_id),
            PlayerPortalGraphNode::End,
            player,
        );
    }

    // This is pretty stupid. We want to have out.ecs.event_listeners borrowed mutably while
    // manipulating the rest of the ecs member variables. To convince rust that this is safe, we
    // swap the event listeners into a separate variable, and swap it back when we're done.
    let mut event_listeners = Default::default();
    std::mem::swap(&mut event_listeners, &mut out.ecs.event_listeners);
    let mut event_listeners_sorted = event_listeners
        .iter_mut()
        .flat_map(|(entity, listeners)| listeners.iter_mut().map(move |event| (entity, event)))
        .collect::<Vec<_>>();
    event_listeners_sorted.sort_by_key(|(_, listener)| listener.priority);
    for (entity, event_listener) in event_listeners_sorted {
        let disabled = out
            .ecs
            .disabled_event_groups
            .get(entity)
            .map_or(false, |disabled| disabled.contains(event_listener.group));
        if disabled {
            continue;
        }
        let base_triggered = match &event_listener.trigger {
            EventTrigger::PlayerIntersect => out
                .ecs
                .positions
                .get(entity)
                .and_then(|&pos| player_at(&out.ecs, pos))
                .is_some(),
            EventTrigger::PlayerNotIntersect => out
                .ecs
                .positions
                .get(entity)
                .map(|&pos| player_at(&out.ecs, pos).is_none())
                .unwrap_or(false),
            EventTrigger::PlayerIntersectHasItems(item, required_count) => out
                .ecs
                .positions
                .get(entity)
                .and_then(|&pos| player_at(&out.ecs, pos))
                .and_then(|player| out.ecs.players.get(player))
                .and_then(|inventory| inventory.count_items().get(&item).copied())
                .filter(|c| c >= required_count)
                .is_some(),
            EventTrigger::ItemIntersect(item) => out
                .ecs
                .positions
                .get(entity)
                .and_then(|pos| out.items.get_by_position(pos))
                .filter(|drop| drop.item == *item)
                .is_some(),
            EventTrigger::CounterPredicate(counter, p) => {
                let count = out
                    .ecs
                    .counters
                    .get(entity)
                    .map_or(0, |counters| counters[*counter]);
                p(count)
            }
        };
        let triggered = match &mut event_listener.modifier {
            EventTriggerModifier::Unmodified => base_triggered,
            EventTriggerModifier::Negated => !base_triggered,
            EventTriggerModifier::Rising(triggered_prior) => {
                let triggered = !*triggered_prior && base_triggered;
                *triggered_prior = base_triggered;
                triggered
            }
            EventTriggerModifier::Falling(triggered_prior) => {
                let triggered = *triggered_prior && !base_triggered;
                *triggered_prior = base_triggered;
                triggered
            }
        };
        if !triggered {
            continue;
        }
        // We avoid explicit recursion to avoid the borrow checker thinking we might mutate
        // ecs.event_listeners
        let mut actions = vec![&event_listener.action];
        while let Some(action) = actions.pop() {
            match action {
                Action::AlterCounter(target, counter, f) => {
                    if !out.ecs.counters.contains_key(*target) {
                        out.ecs.counters.insert(*target, EnumMap::new());
                    }
                    let counters = out
                        .ecs
                        .counters
                        .get_mut(*target)
                        .expect("Should have ensured that the counters existed");
                    counters[*counter] = f(counters[*counter]);
                }
                Action::PlayerMarkUsed(item, count) => {
                    let player_option = out
                        .ecs
                        .positions
                        .get(entity)
                        .and_then(|&pos| player_at(&out.ecs, pos));
                    if let Some(player) = player_option {
                        let inventory = out
                            .ecs
                            .players
                            .get_mut(player)
                            .expect("Player has no inventory");
                        let item_count = inventory.count_items().get(item).copied().unwrap_or(0);
                        if item_count < *count {
                            panic!("Not enough items for PlayerMarkUsed")
                        }
                        if let Inventory::Hypothetical(ref mut inventory) = inventory {
                            let minimum = inventory.minima.entry(item.clone()).or_insert(0);
                            *minimum = min(count - 1, *minimum);
                        }
                    };
                }
                Action::SetImage { target, img } => {
                    out.ecs.images.insert(*target, img.clone());
                }
                Action::EnableGroup(target, group) => {
                    if let Some(disabled_groups) = out.ecs.disabled_event_groups.get_mut(*target) {
                        disabled_groups.remove(*group);
                    }
                }
                Action::DisableGroup(target, group) => {
                    match out.ecs.disabled_event_groups.get_mut(*target) {
                        Some(disabled_groups) => {
                            disabled_groups.insert(*group);
                        }
                        None => {
                            out.ecs
                                .disabled_event_groups
                                .insert(*target, EnumSet::only(*group));
                        }
                    }
                }
                Action::Reject(msg) => Err(*msg)?,
                Action::All(new_actions) => {
                    actions.extend(new_actions.iter());
                }
            }
        }
    }
    std::mem::swap(&mut event_listeners, &mut out.ecs.event_listeners);

    while let Some(prior_player) = jumpers.pop() {
        // First, we remove the portal.
        let prior_player_position = out
            .ecs
            .positions
            .get(prior_player)
            .expect("Player without position");
        let portal = out
            .portals
            .remove_by_position(&prior_player_position)
            .ok_or("Tried to close loop at wrong position")?;
        // Next, we find the player we're merging into: "post_player"
        let mut last_edge = None;
        visit::depth_first_search(
            &out.player_portal_graph,
            iter::once(PlayerPortalGraphNode::Portal(portal.id)),
            |e| {
                if let visit::DfsEvent::TreeEdge(n1, n2) = e {
                    last_edge = out.player_portal_graph.edge_weight(n1, n2);
                }
            },
        );
        let post_player = *last_edge.expect("No outgoing edges for closed portal");
        if post_player == prior_player {
            Err("Attempted to jump into self")?;
        }
        // Merge the inventories
        let prior_inventory = out
            .ecs
            .players
            .get(prior_player)
            .expect("prior_player is not a player")
            .clone();
        let post_inventory = out
            .ecs
            .players
            .get_mut(post_player)
            .expect("post_player is not a player");
        let post_inventory_hypothetical = match post_inventory {
            Inventory::Actual(_) => panic!("Merged into an actual inventory"),
            Inventory::Hypothetical(ref inventory) => inventory,
        };
        let mut item_counts = prior_inventory.count_items();
        let (new_post_inventory, wishes) = post_inventory_hypothetical.merge_in(prior_inventory)?;
        *post_inventory = new_post_inventory;
        // Propegate the merge-implied wishes to the item graph. We need to do this before
        // modifying the players portal graph, or before adding the new edge to the item portal
        // graphs. Conceptually, wishing and unwishing happens _before_ the portal closes, to make
        // it valid to close the portal.
        for wish in wishes {
            let item_count = item_counts.entry(wish.item.clone()).or_insert(0);
            *item_count = (*item_count as i32 + wish.prior_count) as usize;
            let item_portal_graph = out
                .item_portal_graphs
                .get_mut(&wish.item)
                .expect("no item portal graph for existant item");
            println!("Before wishing");
            render_item_graph(item_portal_graph);
            signed_wish(
                item_portal_graph,
                &out.player_portal_graph,
                prior_player,
                wish.prior_count,
            );
            println!("After prior wishing");
            render_item_graph(item_portal_graph);
            signed_wish(
                item_portal_graph,
                &out.player_portal_graph,
                post_player,
                wish.post_count,
            );
            println!("After post wishing");
            render_item_graph(item_portal_graph);
        }
        // Disconnect the player edge from end and connect it to the portal jumped into.
        let (player_origin, _, _) = out
            .player_portal_graph
            .all_edges()
            .find(|(_, _, &edge)| edge == prior_player)
            .expect("Couldn't find player in portal graph");
        out.player_portal_graph
            .remove_edge(player_origin, PlayerPortalGraphNode::End)
            .expect("Tried to close portal when edge unconnected to End");
        out.player_portal_graph.add_edge(
            player_origin,
            PlayerPortalGraphNode::Portal(portal.id),
            prior_player,
        );
        // Check that the player can still reach end (no loops)
        if !petgraph::algo::has_path_connecting(
            &out.player_portal_graph,
            PlayerPortalGraphNode::Portal(portal.id),
            PlayerPortalGraphNode::End,
            None,
        ) {
            Err("Created infinite loop")?;
        }
        // Add the edge linking prior and post players to the item portal graph
        for (item, item_portal_graph) in out.item_portal_graphs.iter_mut() {
            if let Some(origin_node) = find_latest_held(item_portal_graph, prior_player) {
                item_portal_graph.add_edge(
                    origin_node,
                    ItemPortalGraphNode::Held(post_player, 0),
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

        for (item, item_portal_graph) in out.item_portal_graphs.iter_mut() {
            if let Some(origin_node) = find_latest_held(item_portal_graph, prior_player) {
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
        // Finally, remove prior_player from the ecs.
        out.ecs.entities.remove(prior_player);
    }
    Ok(out)
}
#[cfg(test)]
mod test;
