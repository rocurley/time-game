extern crate graph;
extern crate types;

use graph::Graph;
use types::{ActualInventory, Id, Player, Portal};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum ItemPortalGraphNode {
    Beginning,
    Dropped(Id<ActualInventory>),
    Portal(Id<Portal>),
    End,
}

pub type ItemPortalGraph = Graph<ItemPortalGraphNode, Id<Player>>;
