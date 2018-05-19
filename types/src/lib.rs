#![feature(nll)]

extern crate ggez;
extern crate nalgebra;
extern crate rand;
extern crate tree;

use ggez::graphics;
use std::collections::hash_map::Entry;
use std::collections::{hash_map, HashMap, HashSet};
use std::fmt;
use std::marker::PhantomData;

pub const SCALE: f32 = 100.;
pub const INVENTORY_WIDTH: usize = 8;
pub const INVENTORY_HEIGHT: usize = 4;

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
impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Id::new({})", self.0)
    }
}

type IdMap<T> = HashMap<Id<T>, T>;

#[derive(Clone, Debug)]
pub struct DoubleMap<T> {
    by_id: IdMap<T>,
    by_position: HashMap<Point, Id<T>>,
}
impl<T> DoubleMap<T> {
    pub fn new() -> Self {
        DoubleMap {
            by_id: HashMap::new(),
            by_position: HashMap::new(),
        }
    }
    pub fn iter<'a, 'b: 'a>(&'b self) -> hash_map::Iter<'a, Id<T>, T> {
        self.by_id.iter()
    }
    pub fn contains_id(&self, id: &Id<T>) -> bool {
        self.by_id.contains_key(id)
    }
    pub fn get_by_id<'a, 'b: 'a>(&'b self, id: &Id<T>) -> Option<&'a T> {
        self.by_id.get(id)
    }
    pub fn get_by_position<'a, 'b: 'a>(&'b self, pos: &Point) -> Option<&'a T> {
        self.by_position
            .get(pos)
            .map(|id| self.by_id.get(id).expect("DoubleMap inconsistent"))
    }
    pub fn remove_by_position(&mut self, pos: &Point) -> Option<T> {
        self.by_position
            .remove(pos)
            .map(|id| self.by_id.remove(&id).expect("DoubleMap inconsistent"))
    }
    pub fn id_by_position(&self, pos: &Point) -> Option<Id<T>> {
        self.by_position.get(pos).map(|id| id.clone())
    }
}

impl<T> DoubleMap<T>
where
    T: DoubleMappable,
{
    pub fn insert(&mut self, t: T) -> Result<(), &'static str> {
        match self.by_position.entry(t.position()) {
            Entry::Occupied(_) => return Err("Position occupied"),
            Entry::Vacant(position_entry) => match self.by_id.entry(t.id()) {
                Entry::Occupied(_) => return Err("Id already exists"),
                Entry::Vacant(mut id_entry) => {
                    position_entry.insert(t.id());
                    id_entry.insert(t);
                    Ok(())
                }
            },
        }
    }
    pub fn remove_by_id(&mut self, id: &Id<T>) -> Option<T> {
        self.by_id.remove(id).map(|t| {
            self.by_position
                .remove(&t.position())
                .expect("DoubleMap inconsistent");
            t
        })
    }
    pub fn mutate_by_id<F>(&mut self, id: &Id<T>, f: F) -> Result<(), &'static str>
    where
        F: FnOnce(T) -> Result<T, &'static str>,
    {
        let t_orig = self.remove_by_id(id).ok_or("Id missing")?;
        let t_final = f(t_orig)?;
        self.insert(t_final)
    }
}

pub trait DoubleMappable: Sized {
    fn position(&self) -> Point;
    fn id(&self) -> Id<Self>;
}

impl DoubleMappable for Portal {
    fn position(&self) -> Point {
        self.player_position
    }
    fn id(&self) -> Id<Portal> {
        self.id
    }
}

impl DoubleMappable for Player {
    fn position(&self) -> Point {
        self.position
    }
    fn id(&self) -> Id<Player> {
        self.id
    }
}

impl DoubleMappable for ItemDrop {
    fn position(&self) -> Point {
        self.position
    }
    fn id(&self) -> Id<ItemDrop> {
        self.id
    }
}

#[derive(PartialEq, Eq)]
pub enum Selection {
    Top,
    Player(Id<Player>),
    GridCell(Point),
    Inventory(Id<Player>, Option<usize>),
    WishPicker(Id<Player>, usize),
    WishPickerInventoryViewer(Id<Player>, usize, Id<Player>),
}

impl Selection {
    pub fn pop(&mut self) {
        match self {
            &mut Selection::Top => {}
            &mut Selection::Player(_) => *self = Selection::Top,
            &mut Selection::GridCell(_) => *self = Selection::Top,
            &mut Selection::Inventory(id, None) => *self = Selection::Player(id),
            &mut Selection::Inventory(id, Some(_)) => *self = Selection::Inventory(id, None),
            &mut Selection::WishPicker(id, ix) => *self = Selection::Inventory(id, Some(ix)),
            &mut Selection::WishPickerInventoryViewer(id, ix, _) => {
                *self = Selection::WishPicker(id, ix)
            }
        }
    }
}

pub struct ImageMap {
    pub player: graphics::Image,
    pub selection: graphics::Image,
    pub move_arrow: graphics::Image,
    pub jump_icon: graphics::Image,
    pub pick_up_icon: graphics::Image,
    pub drop_icon: graphics::Image,
    pub portal: graphics::Image,
    pub key: graphics::Image,
}

impl ImageMap {
    pub fn new(ctx: &mut ggez::Context) -> ggez::GameResult<Self> {
        let player = graphics::Image::new(ctx, "/images/player.png")?;
        let selection = graphics::Image::new(ctx, "/images/selection.png")?;
        let move_arrow = graphics::Image::new(ctx, "/images/arrow.png")?;
        let jump_icon = graphics::Image::new(ctx, "/images/jump.png")?;
        let pick_up_icon = graphics::Image::new(ctx, "/images/pick_up.png")?;
        let drop_icon = graphics::Image::new(ctx, "/images/drop.png")?;
        let portal = graphics::Image::new(ctx, "/images/portal.png")?;
        let key = graphics::Image::new(ctx, "/images/key.png")?;
        Ok(ImageMap {
            player,
            selection,
            move_arrow,
            jump_icon,
            pick_up_icon,
            drop_icon,
            portal,
            key,
        })
    }
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub enum Move {
    Direction(Direction),
    Jump,
    PickUp,
    Drop(usize),
}

#[derive(Clone, Debug)]
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

pub type Point = nalgebra::Point2<i32>;

#[derive(Clone, Debug)]
pub struct InventoryCell {
    pub item: Item,
    pub count: u8,
}

#[derive(Clone, Debug)]
pub struct HypotheticalInventory {
    pub cells: [Option<InventoryCell>; 32],
    //What the player has "wished for".
    pub constraints: HashMap<Item, usize>,
    //The minimum number of a given type the player ever had. Assume 0.
    //Will be subtracted from constraints when attempting to resolve.
    pub minima: HashMap<Item, usize>,
}

impl HypotheticalInventory {
    pub fn new() -> Self {
        HypotheticalInventory {
            cells: Default::default(),
            constraints: HashMap::new(),
            minima: HashMap::new(),
        }
    }
    pub fn wish(&mut self, item: Item, ix: usize) -> Result<(), &'static str> {
        match &mut self.cells[ix] {
            cell @ &mut None => {
                *cell = Some(InventoryCell {
                    item: item.clone(),
                    count: 1,
                })
            }
            &mut Some(ref mut cell) => {
                if cell.item != item {
                    return Err("Tried to insert conflicting item");
                }
                cell.count += 1;
            }
        }
        *self.minima.entry(item.clone()).or_insert(0) += 1;
        *self.constraints.entry(item.clone()).or_insert(0) += 1;
        Ok(())
    }
    pub fn unwish(&mut self, ix: usize) -> Result<(), &'static str> {
        let cell = self.cells[ix]
            .as_mut()
            .ok_or("Can't un-wish: empty inventory cell")?;
        let item = &cell.item;
        let min = self.minima.entry(item.clone()).or_insert(0);
        if *min == 0 {
            return Err("Can't un-wish: minimum value 0");
        }
        let constraint = self.constraints.entry(item.clone()).or_insert(0);
        if *constraint == 0 {
            return Err("Can't un-wish: never wished in the first place");
        }
        cell.count -= 1;
        *min -= 1;
        *constraint -= 1;
        if cell.count == 0 {
            self.cells[ix] = None
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ActualInventory {
    pub cells: [Option<InventoryCell>; 32],
}
impl ActualInventory {
    pub fn new() -> Self {
        ActualInventory {
            cells: Default::default(),
        }
    }
}

fn insert_into_cells(
    cells: &mut [Option<InventoryCell>; 32],
    item: Item,
) -> Result<(), &'static str> {
    let mut fallback: Option<&mut Option<InventoryCell>> = None;
    for slot in cells {
        match slot {
            &mut None => if fallback.is_none() {
                fallback = Some(slot)
            },
            &mut Some(ref mut inventory_cell) => {
                if inventory_cell.item == item {
                    inventory_cell.count += 1;
                    return Ok(());
                }
            }
        }
    }
    match fallback {
        None => Err("Nowhere to insert into inventory"),
        Some(slot) => {
            *slot = Some(InventoryCell { item, count: 1 });
            Ok(())
        }
    }
}

#[derive(Clone, Debug)]
pub enum Inventory {
    Actual(ActualInventory),
    Hypothetical(HypotheticalInventory),
}
impl Inventory {
    pub fn insert(&mut self, item: Item) -> Result<(), &'static str> {
        insert_into_cells(self.cells_mut(), item)
    }
    pub fn drop(&mut self, item_ix: usize) -> Result<Item, &'static str> {
        let inventory_cell = self.cells_mut()[item_ix]
            .as_mut()
            .ok_or("Tried to drop from empty inventory slot")?;
        inventory_cell.count -= 1;
        let item = inventory_cell.item.clone();
        if inventory_cell.count == 0 {
            self.cells_mut()[item_ix as usize] = None;
        };
        if let &mut Inventory::Hypothetical(ref mut hypothetical) = self {
            let mut count = 0;
            for option_cell in hypothetical.cells.iter() {
                for cell in option_cell {
                    if cell.item == item {
                        count += cell.count as usize
                    }
                }
            }
            let item_min = hypothetical.minima.entry(item.clone()).or_insert(0);
            *item_min = std::cmp::min(count, *item_min);
        }
        Ok(item)
    }
    pub fn cells(&self) -> &[Option<InventoryCell>; 32] {
        match self {
            &Inventory::Actual(ref inventory) => &inventory.cells,
            &Inventory::Hypothetical(ref inventory) => &inventory.cells,
        }
    }
    pub fn cells_mut(&mut self) -> &mut [Option<InventoryCell>; 32] {
        match self {
            &mut Inventory::Actual(ref mut inventory) => &mut inventory.cells,
            &mut Inventory::Hypothetical(ref mut inventory) => &mut inventory.cells,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub id: Id<Player>,
    pub position: Point,
    pub inventory: Inventory,
}

impl Player {
    pub fn new(position: Point) -> Self {
        Player {
            id: rand::random(),
            position,
            inventory: Inventory::Actual(ActualInventory::new()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Key(Key),
}

impl Item {
    pub fn image<'a>(&self, image_map: &'a ImageMap) -> &'a graphics::Image {
        match self {
            &Item::Key(ref key) => key.image(image_map),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {}

impl Key {
    pub fn image<'a>(&self, image_map: &'a ImageMap) -> &'a graphics::Image {
        &image_map.key
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemDrop {
    pub id: Id<ItemDrop>,
    pub position: Point,
    pub item: Item,
}
impl ItemDrop {
    pub fn new(item: Item, position: Point) -> Self {
        ItemDrop {
            id: rand::random(),
            item,
            position,
        }
    }
}

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
