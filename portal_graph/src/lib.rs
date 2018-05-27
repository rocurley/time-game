extern crate graph;
extern crate types;

use graph::Graph;
use types::{Id, ItemDrop, Player, Portal};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum PlayerPortalGraphNode {
    Beginning,
    Portal(Id<Portal>),
    End,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum ItemPortalGraphNode {
    Beginning,
    Dropped(Id<ItemDrop>),
    Portal(Id<Portal>),
    End,
}

pub type PlayerPortalGraph = Graph<PlayerPortalGraphNode, Id<Player>>;
pub type ItemPortalGraph = Graph<ItemPortalGraphNode, Id<Player>>;
