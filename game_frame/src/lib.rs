extern crate petgraph;
extern crate portal_graph;
extern crate types;

use self::portal_graph::{ItemPortalGraphNode, PlayerPortalGraphNode};
use portal_graph::{ItemPortalGraph, PlayerPortalGraph};
use std::collections::HashMap;
use std::fmt;
use types::{DoubleMap, Item, ItemDrop, Player, Portal};

use graphmap::GraphMap;
use petgraph::graphmap;

#[derive(Clone)]
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
            player_portal_graph: GraphMap::new(),
            item_portal_graphs: HashMap::new(),
        }
    }
    pub fn insert_item_drop(&mut self, drop: ItemDrop) -> Result<(), &'static str> {
        let item_portal_graph = self.item_portal_graphs
            .entry(drop.item.clone())
            .or_insert_with(GraphMap::new);
        item_portal_graph.add_edge(
                ItemPortalGraphNode::Beginning,
                ItemPortalGraphNode::Dropped(drop.id),
                (),
            );
        self.items.insert(drop)?;
        Ok(())
    }
    pub fn insert_player(&mut self, player: Player) -> Result<(), &'static str> {
        self.player_portal_graph.add_edge(
            PlayerPortalGraphNode::Beginning,
            PlayerPortalGraphNode::End,
            player.id,
        );
        self.players.insert(player)?;
        Ok(())
    }
}
