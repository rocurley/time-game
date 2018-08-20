extern crate petgraph;
extern crate types;

use graphmap::UnGraphMap;
use petgraph::graphmap;
use petgraph::Direction::{Incoming};
use types::{Id, ItemDrop, Player, Portal};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlayerPortalGraphNode {
    Beginning,
    Portal(Id<Portal>),
    End,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemPortalGraphNode {
    Beginning,
    Dropped(Id<ItemDrop>),
    Held(Id<Player>, usize),
    End,
}

pub type PlayerPortalGraph = UnGraphMap<PlayerPortalGraphNode, Id<Player>>;
pub type ItemPortalGraph = UnGraphMap<ItemPortalGraphNode, ()>;

fn find_origin(graph : PlayerPortalGraph, id : Id<Player>) -> Option<PlayerPortalGraphNode> {
    let nodes : Vec<PlayerPortalGraphNode>= graph.neighbors_directed(PlayerPortalGraphNode::End, Incoming)
        .filter_map(|n| {
            let e = graph.edge_weight(n, PlayerPortalGraphNode::End).expect("Edge listed in neighbors not found");
            if id == *e {
                Some(n)
            } else {
                None
            }
        }).collect();
    let mut node = match nodes.as_slice() {
        [] => None,
        [node] => Some(*node),
        _ => panic!("Multiple edges with same player id")
    }?;
    loop{
        match graph.neighbors_directed(node, Incoming).collect::<Vec<PlayerPortalGraphNode>>().as_slice() {
            [] => return Some(node),
            [new_node] => node = *new_node,
            _ => panic!("Multiple incomming edges"),
        }

    }
}
