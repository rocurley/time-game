extern crate petgraph;
extern crate types;

use graphmap::UnGraphMap;
use petgraph::graphmap;
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
