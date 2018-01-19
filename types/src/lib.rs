extern crate ggez;
extern crate graph;
extern crate nalgebra;
extern crate rand;
extern crate tree;

use std::collections::{HashMap, HashSet};
use ggez::graphics;
use std::marker::PhantomData;
use graph::Graph;

pub const SCALE: f32 = 100.;

//Why `Id<T>`s instead of some sort of reference? The fundamental problem, I think, is that a given
//`Id<Player>` referes to multiple different `Player`s, since each `GameFrame` has a different
//`Player` with the same `Id<Player>`. Were it not for this, I think an `Rc<Cell<Player>>` or
//something would work.
pub struct Id<T>(u64, PhantomData<T>);
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Id<T>) -> bool {
        self.0 == other.0
    }
}
impl<T> Eq for Id<T> {}

impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Id(self.0, PhantomData)
    }
}

impl<T> Copy for Id<T> {}

impl<T> rand::Rand for Id<T> {
    fn rand<R: rand::Rng>(rng: &mut R) -> Self {
        Id(rand::Rand::rand(rng), PhantomData)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum PortalGraphNode {
    Beginning,
    Portal(Id<Portal>),
    End,
}

type PortalGraph = Graph<PortalGraphNode, Id<Player>>;

pub struct GameCell {
    pub player: Option<Id<Player>>,
    pub portal: Option<Id<Portal>>,
}

impl GameCell {
    pub fn new() -> Self {
        GameCell {
            player: None,
            portal: None,
        }
    }
}

type IdMap<T> = HashMap<Id<T>, T>;

pub struct GameFrame {
    pub players: IdMap<Player>,
    pub portals: IdMap<Portal>,
    pub map: HashMap<Point, GameCell>,
    pub portal_graph: PortalGraph,
}

impl GameFrame {
    pub fn new() -> Self {
        GameFrame {
            players: HashMap::new(),
            portals: HashMap::new(),
            map: HashMap::new(),
            portal_graph: Graph::new(),
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Selection {
    Player(Id<Player>),
    GridCell(Point),
}

pub struct ImageMap {
    pub player: graphics::Image,
    pub selection: graphics::Image,
    pub move_arrow: graphics::Image,
    pub jump_icon: graphics::Image,
    pub portal: graphics::Image,
}

impl ImageMap {
    pub fn new(ctx: &mut ggez::Context) -> ggez::GameResult<Self> {
        let player = graphics::Image::new(ctx, "/images/player.png")?;
        let selection = graphics::Image::new(ctx, "/images/selection.png")?;
        let move_arrow = graphics::Image::new(ctx, "/images/arrow.png")?;
        let jump_icon = graphics::Image::new(ctx, "/images/jump.png")?;
        let portal = graphics::Image::new(ctx, "/images/portal.png")?;
        Ok(ImageMap {
            player,
            selection,
            move_arrow,
            jump_icon,
            portal,
        })
    }
}

#[derive(Clone)]
pub struct Portal {
    pub timestamp: usize,
    pub id: Id<Portal>,
    pub player_position: Point,
}

impl Portal {
    pub fn new(timestamp: usize, player_position: Point) -> Self {
        Portal {
            timestamp,
            id: rand::random(),
            player_position,
        }
    }
}

#[derive(Clone)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /*
    fn rotation(&self) -> Array2<f64> {
        match *self {
            Direction::Up => array![[1., 0.], [0., 1.]],
            Direction::Down => array![[-1., 0.], [0., -1.]],
            Direction::Left => array![[0., -1.], [1., 0.]],
            Direction::Right => array![[0., 1.], [-1., 0.]],
        }
    }
    */
}

#[derive(Clone)]
pub enum Move {
    Direction(Direction),
    Jump,
}

#[derive(Clone)]
pub struct Plan {
    pub moves: HashMap<Id<Player>, Move>,
    pub portals: HashSet<Point>,
}

impl Plan {
    pub fn new() -> Self {
        Plan {
            moves: HashMap::new(),
            portals: HashSet::new(),
        }
    }
}

pub enum CachablePlan {
    Novel(Plan),
    Old(usize),
}

impl CachablePlan {
    pub fn new() -> Self {
        CachablePlan::Novel(Plan::new())
    }
    pub fn get<'a, T>(&'a self, history_children: &'a Vec<(Plan, T)>) -> &'a Plan {
        match self {
            &CachablePlan::Novel(ref p) => &p,
            &CachablePlan::Old(ix) => &history_children[ix].0,
        }
    }
    pub fn cow<'a, T>(&'a mut self, history_children: &'a Vec<(Plan, T)>) -> &'a mut Plan {
        if let &mut CachablePlan::Old(ix) = self {
            *self = CachablePlan::Novel(history_children[ix].0.clone());
        }
        match self {
            &mut CachablePlan::Novel(ref mut plan) => plan,
            _ => panic!("Just set the plan to be novel"),
        }
    }
}

#[derive(Clone)]
pub struct Player {
    pub id: Id<Player>,
    pub position: Point,
}

impl Player {
    pub fn new(position: Point) -> Self {
        Player {
            id: rand::random(),
            position,
        }
    }
}

pub type Point = nalgebra::Point2<i32>;

// Inventory system
//
// We don't just want a flat pile of items: We want to be able to look them up quickly.
//
// We don't want a trait object (probably?) because we want to be able to recover the original
// item. It's imaginable that we could have a trait that is good for everything (Maia say: have a
// "use' method, which isn't at all out of the question given that it's a game).
//
// A huge struct (one per item type) seems clunky, but might work.
//
// Do your items have structure? If not, you could just have a map from an Item sum type to a
// count. That's probably good enough for Minecraft's inventory system, for example.
//
// Hell, minecraft's system could be done with an array.
//
// I've been assuming that lookup is important, but what if it isn't? Maybe a Vec or array is the
// way to go here. It does make consolidating items into stacks harder.
//
// What would an ECS do? Dumb entities with smart components. How do you select all the entities
// with a given set of components?
//
// Hell, maybe "use()" is the way to go. Select something from your inventory, click on something
// else. This calls use() with what you clicked as an argument?
