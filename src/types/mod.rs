use super::ggez::graphics;
use super::ggez::nalgebra;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{hash_map, HashMap, HashSet};
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

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
impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Id<T>) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Id<T>) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

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

#[derive(Clone, Debug, Default)]
pub struct DoubleMap<T> {
    by_id: IdMap<T>,
    by_position: HashMap<Point, Id<T>>,
}
#[allow(clippy::trivially_copy_pass_by_ref)]
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
        self.by_position.get(pos).copied()
    }
    pub fn len(&self) -> usize {
        self.by_id.len()
    }
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

pub type GameError = Cow<'static, str>;

pub struct DoubleMapRef<'a, T: DoubleMappable> {
    value: Option<T>,
    map: &'a mut DoubleMap<T>,
}

impl<'a, T: DoubleMappable> Deref for DoubleMapRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.value
            .as_ref()
            .expect("DoubleMapRef value missing before drop")
    }
}

impl<'a, T: DoubleMappable> DerefMut for DoubleMapRef<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value
            .as_mut()
            .expect("DoubleMapRef value missing before drop")
    }
}

impl<'a, T: DoubleMappable> Drop for DoubleMapRef<'a, T> {
    fn drop(&mut self) {
        let v = self
            .value
            .take()
            .expect("DoubleMapRef value missing at drop");
        self.map
            .insert(v)
            .unwrap_or_else(|err| panic!("Error inserting back into map: {}", &err))
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
impl<T> DoubleMap<T>
where
    T: DoubleMappable,
{
    pub fn insert(&mut self, t: T) -> Result<(), GameError> {
        match self.by_position.entry(t.position()) {
            Entry::Occupied(_) => {
                Err("Position occupied")?;
            }
            Entry::Vacant(position_entry) => match self.by_id.entry(t.id()) {
                Entry::Occupied(_) => {
                    Err("Id already exists")?;
                }
                Entry::Vacant(id_entry) => {
                    position_entry.insert(t.id());
                    id_entry.insert(t);
                }
            },
        }
        Ok(())
    }
    pub fn remove_by_id(&mut self, id: &Id<T>) -> Option<T> {
        self.by_id.remove(id).map(|t| {
            self.by_position
                .remove(&t.position())
                .expect("DoubleMap inconsistent");
            t
        })
    }
    pub fn get_mut_by_id<'a, 'b: 'a>(&'b mut self, id: Id<T>) -> Option<DoubleMapRef<'a, T>> {
        let value = Some(self.remove_by_id(&id)?);
        Some(DoubleMapRef { value, map: self })
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
        match *self {
            Selection::Top => {}
            Selection::Player(_) => *self = Selection::Top,
            Selection::GridCell(_) => *self = Selection::Top,
            Selection::Inventory(id, None) => *self = Selection::Player(id),
            Selection::Inventory(id, Some(_)) => *self = Selection::Inventory(id, None),
            Selection::WishPicker(id, ix) => *self = Selection::Inventory(id, Some(ix)),
            Selection::WishPickerInventoryViewer(id, ix, _) => {
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

#[derive(Clone, Debug, Default)]
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

impl Default for CachablePlan {
    fn default() -> Self {
        Self::new()
    }
}

impl CachablePlan {
    pub fn new() -> Self {
        CachablePlan::Novel(Plan::new())
    }
    pub fn get<'a, T>(&'a self, history_children: &'a [(Plan, T)]) -> &'a Plan {
        match *self {
            CachablePlan::Novel(ref p) => &p,
            CachablePlan::Old(ix) => &history_children[ix].0,
        }
    }
    pub fn cow<'a, T>(&'a mut self, history_children: &'a [(Plan, T)]) -> &'a mut Plan {
        if let CachablePlan::Old(ix) = *self {
            *self = CachablePlan::Novel(history_children[ix].0.clone());
        }
        match *self {
            CachablePlan::Novel(ref mut plan) => plan,
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

fn count_cells_items(cells: &[Option<InventoryCell>]) -> HashMap<Item, usize> {
    let mut counts = HashMap::new();
    for option_cell in cells {
        if let Some(cell) = option_cell {
            let count = counts.entry(cell.item.clone()).or_insert(0);
            *count += cell.count as usize;
        }
    }
    counts
}

#[derive(Clone, Debug, Default)]
pub struct HypotheticalInventory {
    pub cells: [Option<InventoryCell>; 32],
    //What the player has "wished for".
    pub constraints: HashMap<Item, usize>,
    //The minimum number of a given type the player ever had. Assume 0.
    //Will be subtracted from constraints when attempting to resolve.
    //Note that this will become more subtle if you can use items within
    //your inventory: it's really a count of the number of never-used
    //instances of the item.
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

    pub fn drop(&mut self, item_ix: usize) -> Result<Item, &'static str> {
        let item = drop_from_cells(&mut self.cells, item_ix)?;
        let mut count = 0;
        for option_cell in self.cells.iter() {
            for cell in option_cell {
                if cell.item == item {
                    count += cell.count as usize
                }
            }
        }
        let item_min = self.minima.entry(item.clone()).or_insert(0);
        *item_min = std::cmp::min(count, *item_min);
        Ok(item)
    }

    pub fn count_items(&self) -> HashMap<Item, usize> {
        count_cells_items(&self.cells)
    }

    //TODO: We need to be able to update the item graph when you merge inventories, since this
    //involves wishing and unwishing.
    pub fn merge_in(&self, other: Inventory) -> Result<Inventory, String> {
        match other {
            Inventory::Actual(actual_other) => {
                let mut constraints: HashMap<Item, isize> = self
                    .constraints
                    .iter()
                    .map(|(i, &c)| (i.clone(), c as isize))
                    .collect();
                for cell in actual_other.cells.iter().flat_map(Option::iter) {
                    let constraint = constraints.entry(cell.item.clone()).or_insert(0);
                    *constraint -= cell.count as isize;
                }
                let mut cells = self.cells.clone();
                for (item, &count) in constraints.iter() {
                    match count.cmp(&0) {
                        Ordering::Less => {
                            if let Err(extra) = add_to_cells(&mut cells, item, (-count) as usize) {
                                panic!("Too many {:?}: can't find space for {:?}", item, extra);
                            }
                        }
                        Ordering::Equal => {}
                        Ordering::Greater => {
                            let minimum = self.minima.get(item).map_or(0, |x| *x as isize);
                            if count > minimum {
                                return Err(format!(
                                    "Not enough {:?} : {} short",
                                    item,
                                    count - minimum
                                ));
                            } else if let Err(short) =
                                remove_from_cells(&mut cells, item, count as usize)
                            {
                                panic!(
                                    "Should have had enough {:?}, but fell {:?} short",
                                    item, short
                                );
                            }
                        }
                    }
                }
                Ok(Inventory::Actual(ActualInventory { cells }))
            }
            Inventory::Hypothetical(hypothetical_other) => {
                //This can't actually fail. We basically want to adjust the other inventory
                //until its item counts match our constraints. Sometimes this won't be possible:
                //the other inventory's minima prevent unwishing far enough. In that case,
                //we can wish up the current inventory. Once the inventories match up,
                //we merge the minima and we're done.
                //
                let mut extras = count_cells_items(&hypothetical_other.cells);
                let mut other_minima = hypothetical_other.minima.clone();
                let mut other_constraints = hypothetical_other.constraints.clone();
                let mut self_constraints = self.constraints.clone();
                let mut minima = self.minima.clone();
                let mut cells = self.cells.clone();
                //We want to match up self_constraints with other_counts.
                //After this, what's left of extras will be what the self inventory needs to
                //wish for.
                for (item, &mut needed) in self_constraints.iter_mut() {
                    use std::collections::hash_map::Entry;
                    if let Entry::Occupied(mut extra) = extras.entry(item.clone()) {
                        match needed.cmp(extra.get()) {
                            Ordering::Less => {
                                *extra.get_mut() -= needed;
                            }
                            Ordering::Equal => {
                                extra.remove();
                            }
                            Ordering::Greater => {
                                //Other has to wish to make up the difference
                                let other_count = extra.remove();
                                let other_constraint =
                                    other_constraints.entry(item.clone()).or_insert(0);
                                *other_constraint += needed - other_count;
                                let other_minimum = other_minima.entry(item.clone()).or_insert(0);
                                *other_minimum += needed - other_count;
                            }
                        }
                    }
                }
                //Wish for extras:
                for (item, extra) in extras {
                    add_to_cells(&mut cells, &item, extra).map_err(|overflow| {
                        format!("Too many {:?} : can't find space for {}", item, overflow)
                    })?;
                    let minimum = minima.entry(item).or_insert(0);
                    *minimum += extra;
                }
                //AFIACT this can't be done in place, which is a bit distressing.
                let merged_minima = minima
                    .into_iter()
                    .filter_map(|(item, minimum)| {
                        let other_minimum = other_minima.get(&item)?;
                        Some((item, std::cmp::min(minimum, *other_minimum)))
                    })
                    .collect();
                Ok(Inventory::Hypothetical(HypotheticalInventory {
                    cells,
                    minima: merged_minima,
                    constraints: other_constraints,
                }))
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActualInventory {
    pub cells: [Option<InventoryCell>; 32],
}
impl ActualInventory {
    pub fn new() -> Self {
        ActualInventory {
            cells: Default::default(),
        }
    }
    pub fn drop(&mut self, item_ix: usize) -> Result<Item, &'static str> {
        drop_from_cells(&mut self.cells, item_ix)
    }
    pub fn count_items(&self) -> HashMap<Item, usize> {
        count_cells_items(&self.cells)
    }
}

fn insert_into_cells(
    cells: &mut [Option<InventoryCell>; 32],
    item: &Item,
) -> Result<(), GameError> {
    add_to_cells(cells, item, 1).map_err(|_| format!("Can't find space for {:?}", item).into())
}

fn add_to_cells(
    cells: &mut [Option<InventoryCell>],
    item: &Item,
    mut count: usize,
) -> Result<(), usize> {
    for cell_option in cells.iter_mut() {
        match cell_option {
            Some(cell) if cell.item == *item => {
                match count.cmp(&((u8::max_value() - cell.count) as usize)) {
                    Ordering::Less => {
                        cell.count += count as u8;
                        return Ok(());
                    }
                    Ordering::Equal => {
                        cell.count = u8::max_value();
                        return Ok(());
                    }
                    Ordering::Greater => {
                        count -= (u8::max_value() - cell.count) as usize;
                        cell.count = u8::max_value();
                    }
                }
            }
            _ => {}
        }
    }
    for cell_option in cells.iter_mut() {
        if cell_option.is_none() {
            let cell = cell_option.get_or_insert(InventoryCell {
                item: item.clone(),
                count: 0,
            });
            match count.cmp(&(u8::max_value() as usize)) {
                Ordering::Less => {
                    cell.count = count as u8;
                    return Ok(());
                }
                Ordering::Equal => {
                    cell.count = u8::max_value();
                    return Ok(());
                }
                Ordering::Greater => {
                    count -= u8::max_value() as usize;
                    cell.count = u8::max_value();
                }
            }
        }
    }
    Err(count)
}

fn remove_from_cells(
    cells: &mut [Option<InventoryCell>],
    item: &Item,
    mut count: usize,
) -> Result<(), usize> {
    for cell_option in cells.iter_mut().rev() {
        match cell_option {
            Some(cell) if cell.item == *item => match count.cmp(&(cell.count as usize)) {
                Ordering::Less => {
                    cell.count -= count as u8;
                    return Ok(());
                }
                Ordering::Equal => {
                    *cell_option = None;
                    return Ok(());
                }
                Ordering::Greater => {
                    count -= cell.count as usize;
                    *cell_option = None;
                }
            },
            _ => {}
        }
    }
    Err(count)
}

fn drop_from_cells(
    cells: &mut [Option<InventoryCell>],
    item_ix: usize,
) -> Result<Item, &'static str> {
    let inventory_cell = cells[item_ix]
        .as_mut()
        .ok_or("Tried to drop from empty inventory slot")?;
    inventory_cell.count -= 1;
    let item = inventory_cell.item.clone();
    if inventory_cell.count == 0 {
        cells[item_ix as usize] = None;
    };
    Ok(item)
}

#[derive(Clone, Debug)]
pub enum Inventory {
    Actual(ActualInventory),
    Hypothetical(HypotheticalInventory),
}
impl Inventory {
    pub fn insert(&mut self, item: &Item) -> Result<(), GameError> {
        insert_into_cells(self.cells_mut(), item)
    }
    pub fn drop(&mut self, item_ix: usize) -> Result<Item, &'static str> {
        match self {
            Inventory::Actual(actual) => actual.drop(item_ix),
            Inventory::Hypothetical(hypothetical) => hypothetical.drop(item_ix),
        }
    }
    pub fn cells(&self) -> &[Option<InventoryCell>; 32] {
        match *self {
            Inventory::Actual(ref inventory) => &inventory.cells,
            Inventory::Hypothetical(ref inventory) => &inventory.cells,
        }
    }
    pub fn cells_mut(&mut self) -> &mut [Option<InventoryCell>; 32] {
        match *self {
            Inventory::Actual(ref mut inventory) => &mut inventory.cells,
            Inventory::Hypothetical(ref mut inventory) => &mut inventory.cells,
        }
    }
    pub fn count_items(&self) -> HashMap<Item, usize> {
        match self {
            Inventory::Actual(actual) => actual.count_items(),
            Inventory::Hypothetical(hypothetical) => hypothetical.count_items(),
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
        match *self {
            Item::Key(ref key) => key.image(image_map),
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

#[cfg(test)]
mod tests {
    use super::{
        add_to_cells, ActualInventory, HypotheticalInventory, Inventory, InventoryCell, Item, Key,
    };
    use proptest::arbitrary::{any, Arbitrary};
    use proptest::strategy::{BoxedStrategy, Strategy};

    impl Arbitrary for Item {
        type Parameters = ();
        type Strategy = BoxedStrategy<Item>;
        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            (0u8..1)
                .prop_map(|n| match n {
                    0 => Item::Key(Key {}),
                    _ => panic!("Generated impossible discriminant for item"),
                })
                .boxed()
        }
    }

    impl Arbitrary for InventoryCell {
        type Parameters = ();
        type Strategy = BoxedStrategy<InventoryCell>;
        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            (any::<Item>(), any::<u8>())
                .prop_map(|(item, count)| InventoryCell { item, count })
                .boxed()
        }
    }

    impl Arbitrary for ActualInventory {
        type Parameters = ();
        type Strategy = BoxedStrategy<ActualInventory>;
        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            any::<[Option<InventoryCell>; 32]>()
                .prop_map(|cells| ActualInventory { cells })
                .boxed()
        }
    }
    proptest! {
        #[test]
        fn test_merge_in_empty_hypothetical(actual in any::<ActualInventory>()) {
            let hypothetical = HypotheticalInventory::new();
            let initial_counts = actual.count_items();
            let merged = hypothetical.merge_in(Inventory::Actual(actual)).expect("Merge failed");
            let merged_counts = merged.count_items();
            assert_eq!(initial_counts, merged_counts);
        }
    }
    proptest! {
        #[test]
        fn test_merge_in_wished_hypothetical(actual in any::<ActualInventory>(),
                                             item in any::<Item>(),
                                             count in any::<u8>()) {
            let mut hypothetical = HypotheticalInventory::new();
            for _ in 0..count {
                hypothetical.wish(item.clone(), 0).expect("Wishing failed");
            }
            let initial_counts = actual.count_items();
            let merged = hypothetical.merge_in(Inventory::Actual(actual)).expect("Merge failed");
            let merged_counts = merged.count_items();
            assert_eq!(initial_counts, merged_counts);
        }
    }
    proptest! {
        #[test]
        fn test_merge_in_too_many_drops( item in any::<Item>(),
                                         mut numbers in any::<[u8; 3]>()) {
            numbers.sort_unstable();
            let [available, drop_count, wish_count] = numbers;
            if available == drop_count {
                return Ok(());
            }
            let mut hypothetical = HypotheticalInventory::new();
            for _ in 0..wish_count {
                hypothetical.wish(item.clone(), 0).expect("Wishing failed");
            }
            for _ in 0..drop_count {
                hypothetical.drop(0).expect("Dropping failed");
            }
            let mut actual = ActualInventory::new();
            add_to_cells(&mut actual.cells, &item, available as usize).expect("Adding items to actual failed");

            hypothetical.merge_in(Inventory::Actual(actual)).expect_err("Merge succeeded");
        }
    }
    proptest! {
        #[test]
        fn test_merge_in_enough_drops( item in any::<Item>(),
                                         mut numbers in any::<[u8; 3]>()) {
            numbers.sort_unstable();
            let [drop_count, available, wish_count] = numbers;
            let mut hypothetical = HypotheticalInventory::new();
            for _ in 0..wish_count {
                hypothetical.wish(item.clone(), 0).expect("Wishing failed");
            }
            for _ in 0..drop_count {
                hypothetical.drop(0).expect("Dropping failed");
            }
            let mut actual = ActualInventory::new();
            add_to_cells(&mut actual.cells, &item, available as usize).expect("Adding items to actual failed");
            use std::collections::HashMap;
            let expected_counts : HashMap<Item, usize> =
                if available > drop_count {
                    [(item, (available - drop_count) as usize)]
                    .iter()
                    .cloned()
                    .collect()
                } else {
                    HashMap::new()
                };
            let merged = hypothetical.merge_in(Inventory::Actual(actual)).expect("Merge failed");
            let merged_counts = merged.count_items();
            assert_eq!(expected_counts, merged_counts);
        }
    }
    //TODO: Hypothetical self merge should yield self
    //May need an actual unit test here.
}
