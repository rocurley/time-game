use crate::{
    portal_graph::{
        self, ItemPortalGraph, ItemPortalGraphNode, PlayerPortalGraph, PlayerPortalGraphNode,
    },
    types::{
        ActualInventory, DoubleMap, Entity, GameError, ImageMap, Inventory, Item, ItemDrop, Point,
        Portal, ECS,
    },
};
use petgraph::graphmap::GraphMap;
use std::{collections::HashMap, fmt};

#[derive(Clone)]
pub struct GameFrame {
    pub portals: DoubleMap<Portal>,
    pub items: DoubleMap<ItemDrop>,
    pub player_portal_graph: PlayerPortalGraph,
    pub item_portal_graphs: HashMap<Item, ItemPortalGraph>,
    pub ecs: ECS,
}
impl fmt::Debug for GameFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "GameFrame{{ portals:{:?}, items:{:?}, portal_graph:???}}",
            self.portals, self.items
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
            portals: DoubleMap::new(),
            items: DoubleMap::new(),
            player_portal_graph: GraphMap::new(),
            item_portal_graphs: HashMap::new(),
            ecs: ECS::default(),
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
    pub fn insert_player(
        &mut self,
        image_map: &ImageMap,
        position: Point,
    ) -> Result<Entity, GameError> {
        let inventory = Inventory::Actual(ActualInventory::new());
        let player = self.ecs.insert_player(image_map, position, inventory);
        self.player_portal_graph.add_edge(
            PlayerPortalGraphNode::Beginning,
            PlayerPortalGraphNode::End,
            player,
        );
        Ok(player)
    }
    pub fn wish(
        &mut self,
        player: Entity,
        ix: usize,
        clicked_item: Option<Item>,
    ) -> FrameWishResult {
        let item_portal_graphs = &mut self.item_portal_graphs;
        let player_portal_graph = &self.player_portal_graph;
        let inventory = self
            .ecs
            .players
            .get_mut(player)
            .expect("Player lacks an inventory");
        if let Inventory::Hypothetical(ref mut hypothetical) = inventory {
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
            portal_graph::wish(item_portal_graph, player_portal_graph, player, 1);
        }
        FrameWishResult::Success
    }
    pub fn unwish(&mut self, player: Entity, ix: usize) -> Result<FrameWishResult, GameError> {
        let item_portal_graphs = &mut self.item_portal_graphs;
        let player_portal_graph = &self.player_portal_graph;
        let inventory = self
            .ecs
            .players
            .get_mut(player)
            .expect("Player lacks an inventory");
        if let Inventory::Hypothetical(ref mut hypothetical) = inventory {
            let item = match hypothetical.cells[ix] {
                None => return Ok(FrameWishResult::NoItem),
                Some(ref cell) => cell.item.clone(),
            };
            let item_portal_graph = item_portal_graphs.entry(item).or_default();
            hypothetical.unwish(ix)?;
            portal_graph::unwish(item_portal_graph, player_portal_graph, player, 1);
        }
        Ok(FrameWishResult::Success)
    }
}

pub enum FrameWishResult {
    Success,
    NoItem,
}
