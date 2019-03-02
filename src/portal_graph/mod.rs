use petgraph::graphmap::DiGraphMap;
use petgraph::Direction::Incoming;
use types::{Id, ItemDrop, Player, Portal};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PlayerPortalGraphNode {
    Beginning,
    Portal(Id<Portal>),
    End,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ItemPortalGraphNode {
    Beginning,
    Dropped(Id<ItemDrop>),
    Portal(Id<Portal>),
    Held(Id<Player>, usize), //Index. Lets you figure out the last node the player was at.
}

pub type PlayerPortalGraph = DiGraphMap<PlayerPortalGraphNode, Id<Player>>;
pub type ItemPortalGraph = DiGraphMap<ItemPortalGraphNode, usize>;

pub fn find_trail_from_origin(
    graph: &PlayerPortalGraph,
    id: Id<Player>,
) -> Option<Vec<Id<Player>>> {
    let nodes: Vec<PlayerPortalGraphNode> = graph
        .neighbors_directed(PlayerPortalGraphNode::End, Incoming)
        .filter(|n| {
            let e = graph
                .edge_weight(*n, PlayerPortalGraphNode::End)
                .expect("Edge listed in neighbors not found");
            id == *e
        })
        .collect();
    let mut node = match *nodes.as_slice() {
        [] => None,
        [node] => Some(node),
        _ => panic!("Multiple edges with same player id"),
    }?;
    let mut edges = vec![id];
    loop {
        match *graph
            .neighbors_directed(node, Incoming)
            .collect::<Vec<PlayerPortalGraphNode>>()
            .as_slice()
        {
            [] => break,
            [new_node] => {
                let e = graph
                    .edge_weight(new_node, node)
                    .expect("Edge listed in neighbors not found");
                edges.push(*e);
                node = new_node;
            }
            _ => panic!("Multiple incomming edges"),
        }
    }
    Some(edges)
}

fn player_held_nodes(
    graph: &ItemPortalGraph,
    player_graph: &PlayerPortalGraph,
    id: Id<Player>,
) -> Option<Vec<ItemPortalGraphNode>> {
    let player_ids = find_trail_from_origin(player_graph, id)?;
    println!("player_held_nodes");
    println!("{:?}", player_ids);
    let mut held_nodes = Vec::new();
    for player_id in player_ids {
        held_nodes.push(ItemPortalGraphNode::Held(player_id, 0));
        held_nodes.extend(
            (1..)
                .map(|i| ItemPortalGraphNode::Held(player_id, i))
                .take_while(|n| graph.contains_node(*n)),
        )
    }
    Some(held_nodes)
}

pub fn wish(
    graph: &mut ItemPortalGraph,
    player_graph: &PlayerPortalGraph,
    id: Id<Player>,
    count: usize,
) {
    println!("portal_graph::wish");
    let held_nodes =
        player_held_nodes(graph, player_graph, id).expect("Couldn't find player in portal graph");
    if let Some((mut last_node, tail)) = held_nodes.split_first() {
        for node in tail {
            match graph.edge_weight_mut(*last_node, *node) {
                //The "existing edge" case doesn't seem to work
                Some(existing_edge) => *existing_edge += count,
                None => {
                    graph.add_edge(*last_node, *node, count);
                }
            }
            last_node = node;
        }
    }
}

pub fn unwish(
    graph: &mut ItemPortalGraph,
    player_graph: &PlayerPortalGraph,
    id: Id<Player>,
    count: usize,
) {
    let held_nodes =
        player_held_nodes(graph, player_graph, id).expect("Couldn't find player in portal graph");
    if let Some((mut last_node, tail)) = held_nodes.split_first() {
        for node in tail {
            let existing_edge = graph
                .edge_weight_mut(*last_node, *node)
                .expect("unwished but edge was empty");
            use std::cmp::Ordering;
            match (*existing_edge).cmp(&count) {
                Ordering::Less => panic!("Unwished but edge was too small"),
                Ordering::Equal => {
                    graph.remove_edge(*last_node, *node);
                }
                Ordering::Greater => *existing_edge -= count,
            }
            last_node = node;
        }
    }
}

pub fn find_latest_held_index(graph: &ItemPortalGraph, player_id: Id<Player>) -> Option<usize> {
    if !graph.contains_node(ItemPortalGraphNode::Held(player_id, 0)) {
        return None;
    }
    let mut last = 0;
    while graph.contains_node(ItemPortalGraphNode::Held(player_id, last + 1)) {
        last += 1;
    }
    Some(last)
}

pub fn find_latest_held(
    graph: &ItemPortalGraph,
    player_id: Id<Player>,
) -> Option<ItemPortalGraphNode> {
    find_latest_held_index(graph, player_id).map(|i| ItemPortalGraphNode::Held(player_id, i))
}
