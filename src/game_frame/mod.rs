use petgraph::graphmap::GraphMap;
use portal_graph;
use portal_graph::{
    ItemPortalGraph, ItemPortalGraphNode, PlayerPortalGraph, PlayerPortalGraphNode,
};
use std::collections::HashMap;
use std::fmt;
use types::{DoubleMap, GameError, Id, Inventory, Item, ItemDrop, Player, Portal};

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

impl Default for GameFrame {
    fn default() -> Self {
        Self::new()
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
    pub fn insert_item_drop(
        &mut self,
        drop: ItemDrop,
        prior_item_count: usize,
    ) -> Result<(), GameError> {
        let item_portal_graph = self
            .item_portal_graphs
            .entry(drop.item.clone())
            .or_insert_with(GraphMap::new);
        item_portal_graph.add_edge(
            ItemPortalGraphNode::Beginning,
            ItemPortalGraphNode::Dropped(drop.id),
            prior_item_count,
        );
        self.items.insert(drop)?;
        Ok(())
    }
    pub fn insert_player(&mut self, player: Player) -> Result<(), GameError> {
        self.player_portal_graph.add_edge(
            PlayerPortalGraphNode::Beginning,
            PlayerPortalGraphNode::End,
            player.id,
        );
        self.players.insert(player)?;
        Ok(())
    }
    pub fn wish(
        &mut self,
        player_id: Id<Player>,
        ix: usize,
        clicked_item: Option<Item>,
    ) -> FrameWishResult {
        let players = &mut self.players;
        let item_portal_graphs = &mut self.item_portal_graphs;
        let player_portal_graph = &self.player_portal_graph;
        let mut player = players
            .get_mut_by_id(player_id)
            .expect("Couldn't find player in players");
        if let Inventory::Hypothetical(ref mut hypothetical) = player.inventory {
            let item = match (clicked_item, &hypothetical.cells[ix]) {
                (None, None) => {
                    return FrameWishResult::NoItem;
                }
                (Some(item), None) => item,
                (None, Some(ref cell)) => cell.item.clone(),
                (Some(_), Some(_)) => panic!("Selected item for wish into occupied cell"),
            };
            let item_portal_graph = item_portal_graphs.entry(item.clone()).or_default();
            hypothetical
                .wish(item, ix)
                .expect("Couldn't find player in players");
            portal_graph::wish(item_portal_graph, player_portal_graph, player_id, 1);
        }
        FrameWishResult::Success
    }
    pub fn unwish(
        &mut self,
        player_id: Id<Player>,
        ix: usize,
    ) -> Result<FrameWishResult, GameError> {
        let players = &mut self.players;
        let item_portal_graphs = &mut self.item_portal_graphs;
        let player_portal_graph = &self.player_portal_graph;
        let mut player = players
            .get_mut_by_id(player_id)
            .expect("Couldn't find player in players");
        if let Inventory::Hypothetical(ref mut hypothetical) = player.inventory {
            let item = match hypothetical.cells[ix] {
                None => return Ok(FrameWishResult::NoItem),
                Some(ref cell) => cell.item.clone(),
            };
            let item_portal_graph = item_portal_graphs.entry(item.clone()).or_default();
            hypothetical.unwish(ix)?;
            portal_graph::unwish(item_portal_graph, player_portal_graph, player_id, 1);
        }
        Ok(FrameWishResult::Success)
    }
}

pub enum FrameWishResult {
    Success,
    NoItem,
}
