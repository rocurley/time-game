extern crate graph;
extern crate portal_graph;
extern crate types;

use graph::Graph;
use portal_graph::{PlayerPortalGraph, ItemPortalGraph};
use std::fmt;
use types::{DoubleMap, ItemDrop, Player, Portal, Item};
use std::collections::{HashMap};

pub struct GameFrame {
    pub players: DoubleMap<Player>,
    pub portals: DoubleMap<Portal>,
    pub items: DoubleMap<ItemDrop>,
    pub player_portal_graph: PlayerPortalGraph,
    pub item_portal_graphs: HashMap<Item, ItemPortalGraph>,
}
impl fmt::Debug for GameFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "GameFrame{{ players:{:?}, portals:{:?}, items:{:?}, portal_graph:???}}",
            self.players, self.portals, self.items
        )
    }
}

impl GameFrame {
    pub fn new() -> Self {
        GameFrame {
            players: DoubleMap::new(),
            portals: DoubleMap::new(),
            items: DoubleMap::new(),
            player_portal_graph: Graph::new(),
            item_portal_graphs: HashMap::new(),
        }
    }
}
