use petgraph::graphmap::DiGraphMap;
use petgraph::Direction::Incoming;
use types::{GameError, Id, ItemDrop, Player, Portal};

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
pub type ItemPortalGraph = DiGraphMap<ItemPortalGraphNode, ()>;

#[allow(dead_code)]
pub fn find_trail_from_origin(
    graph: &PlayerPortalGraph,
    id: Id<Player>,
) -> Option<Vec<PlayerPortalGraphNode>> {
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
    let mut nodes: Vec<PlayerPortalGraphNode> = Vec::new();
    loop {
        match *graph
            .neighbors_directed(node, Incoming)
            .collect::<Vec<PlayerPortalGraphNode>>()
            .as_slice()
        {
            [] => break,
            [mut new_node] => {
                std::mem::swap(&mut node, &mut new_node);
                nodes.push(new_node);
            }
            _ => panic!("Multiple incomming edges"),
        }
    }
    nodes.push(node);
    Some(nodes)
}

/*
pub fn wish(graph: &mut PlayerPortalGraph, id: Id<Player>) -> Result<(), GameError> {
    let trail = find_trail_from_origin(graph, id).ok_or("Couldn't find player origin.")?;
}
*/

pub fn find_latest_held(
    graph: &ItemPortalGraph,
    player_id: Id<Player>,
) -> Option<ItemPortalGraphNode> {
    if !graph.contains_node(ItemPortalGraphNode::Held(player_id, 0)) {
        return None;
    }
    let mut last = 0;
    while graph.contains_node(ItemPortalGraphNode::Held(player_id, last + 1)) {
        last += 1;
    }
    Some(ItemPortalGraphNode::Held(player_id, last))
}
