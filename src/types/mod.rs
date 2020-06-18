use super::ggez::{graphics, nalgebra};
use enum_map::EnumMap;
use enumset::EnumSet;
use slotmap::{HopSlotMap, SecondaryMap, SparseSecondaryMap};
use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::{
        hash_map::{self, Entry},
        HashMap, HashSet,
    },
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};

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
    Player(Entity),
    GridCell(Point),
    Inventory(Entity, Option<usize>),
    WishPicker(Entity, usize),
    WishPickerInventoryViewer(Entity, usize, Entity),
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
    pub player: DrawLayer,
    pub selection: DrawLayer,
    pub move_arrow: DrawLayer,
    pub jump_icon: DrawLayer,
    pub pick_up_icon: DrawLayer,
    pub drop_icon: DrawLayer,
    pub portal: DrawLayer,
    pub key: DrawLayer,
    pub wall: DrawLayer,
    pub open_door: DrawLayer,
    pub closed_door: DrawLayer,
    pub plate: DrawLayer,
    pub lights: [DrawLayer; 4],
}

fn load_image(ctx: &mut ggez::Context, layer: Layer, path: &str) -> ggez::GameResult<DrawLayer> {
    let image = graphics::Image::new(ctx, path)?;
    Ok(DrawLayer {
        layer,
        draw_ref: &*Box::leak(Box::new(image)),
    })
}

impl ImageMap {
    pub fn new(ctx: &mut ggez::Context) -> ggez::GameResult<Self> {
        use Layer::*;
        let player = load_image(ctx, Foreground, "/images/player.png")?;
        let selection = load_image(ctx, UI, "/images/selection.png")?;
        let move_arrow = load_image(ctx, UI, "/images/arrow.png")?;
        let jump_icon = load_image(ctx, UI, "/images/jump.png")?;
        let pick_up_icon = load_image(ctx, UI, "/images/pick_up.png")?;
        let drop_icon = load_image(ctx, UI, "/images/drop.png")?;
        let portal = load_image(ctx, UI, "/images/portal.png")?;
        let key = load_image(ctx, Foreground, "/images/key.png")?;
        let wall = load_image(ctx, Foreground, "/images/wall.png")?;
        let open_door = load_image(ctx, Foreground, "/images/open_door.png")?;
        let closed_door = load_image(ctx, Foreground, "/images/closed_door.png")?;
        let plate = load_image(ctx, Background, "/images/plate.png")?;
        let lights = [
            load_image(ctx, Foreground, "/images/lights0.png")?,
            load_image(ctx, Foreground, "/images/lights1.png")?,
            load_image(ctx, Foreground, "/images/lights2.png")?,
            load_image(ctx, Foreground, "/images/lights3.png")?,
        ];
        Ok(ImageMap {
            player,
            selection,
            move_arrow,
            jump_icon,
            pick_up_icon,
            drop_icon,
            portal,
            key,
            wall,
            open_door,
            closed_door,
            plate,
            lights,
        })
    }
    pub fn mock() -> Self {
        let empty_image = DrawLayer {
            layer: Layer::Background,
            draw_ref: &*Box::leak(Box::new(EmptyImage {})),
        };
        ImageMap {
            player: empty_image,
            selection: empty_image,
            move_arrow: empty_image,
            jump_icon: empty_image,
            pick_up_icon: empty_image,
            drop_icon: empty_image,
            portal: empty_image,
            key: empty_image,
            wall: empty_image,
            open_door: empty_image,
            closed_door: empty_image,
            plate: empty_image,
            lights: [empty_image; 4],
        }
    }
}

#[derive(Debug)]
struct EmptyImage {}

impl ggez::graphics::Drawable for EmptyImage {
    fn draw(
        &self,
        _ctx: &mut ggez::Context,
        _param: ggez::graphics::DrawParam,
    ) -> ggez::GameResult<()> {
        Ok(())
    }
    fn dimensions(&self, _ctx: &mut ggez::Context) -> Option<ggez::graphics::Rect> {
        None
    }
    fn set_blend_mode(&mut self, _mode: Option<ggez::graphics::BlendMode>) {}
    fn blend_mode(&self) -> Option<ggez::graphics::BlendMode> {
        None
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

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
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
    pub moves: HashMap<Entity, Move>,
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
        *self.constraints.entry(item).or_insert(0) += 1;
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

    pub fn merge_in(&self, other: Inventory) -> Result<(Inventory, Vec<MergeWish>), String> {
        // self = post, other = prior
        let mut changes = Vec::<MergeWish>::new();
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
                            changes.push(MergeWish {
                                item: item.clone(),
                                post_count: -(count as i32),
                                prior_count: 0,
                            });
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
                            changes.push(MergeWish {
                                item: item.clone(),
                                post_count: (count as i32),
                                prior_count: 0,
                            });
                        }
                    }
                }
                Ok((Inventory::Actual(ActualInventory { cells }), changes))
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
                let mut other_constraints = hypothetical_other.constraints;
                let mut self_constraints = self.constraints.clone();
                let mut minima = self.minima.clone();
                let mut cells = self.cells.clone();
                //We want to match up self_constraints with other_counts.
                //After this, what's left of extras will be what the self inventory needs to
                //wish for.
                for (item, &mut needed) in self_constraints.iter_mut() {
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
                                changes.push(MergeWish {
                                    item: item.clone(),
                                    post_count: 0,
                                    prior_count: (needed - other_count) as i32,
                                });
                            }
                        }
                    }
                }
                //Wish for extras:
                for (item, extra) in extras {
                    add_to_cells(&mut cells, &item, extra).map_err(|overflow| {
                        format!("Too many {:?} : can't find space for {}", item, overflow)
                    })?;
                    let minimum = minima.entry(item.clone()).or_insert(0);
                    *minimum += extra;
                    changes.push(MergeWish {
                        item: item.clone(),
                        post_count: (extra as i32),
                        prior_count: 0,
                    });
                }
                //AFIACT this can't be done in place, which is a bit distressing.
                let merged_minima = minima
                    .into_iter()
                    .filter_map(|(item, minimum)| {
                        let other_minimum = other_minima.get(&item)?;
                        Some((item, std::cmp::min(minimum, *other_minimum)))
                    })
                    .collect();
                Ok((
                    Inventory::Hypothetical(HypotheticalInventory {
                        cells,
                        minima: merged_minima,
                        constraints: other_constraints,
                    }),
                    changes,
                ))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct MergeWish {
    pub item: Item,
    pub post_count: i32,
    pub prior_count: i32,
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Key(Key),
}

impl Item {
    pub fn image(&self, image_map: &ImageMap) -> DrawLayer {
        match *self {
            Item::Key(ref key) => key.image(image_map),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {}

impl Key {
    pub fn image(&self, image_map: &ImageMap) -> DrawLayer {
        image_map.key
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

#[derive(Clone, Debug)]
pub enum MapElement {
    Empty,
    Wall,
    ClosedDoor,
    RemoteDoor,
    OpenDoor,
    Plate(Counter, Entity),
    Light {
        counter: Counter,
        rising: Action,
        falling: Action,
    },
    MovingWall {
        direction: Direction,
        reset: Option<(Point, Point)>,
    },
}
impl MapElement {
    pub fn image(&self, image_map: &ImageMap) -> Option<DrawLayer> {
        match self {
            MapElement::Empty => None,
            MapElement::Wall => Some(image_map.wall),
            MapElement::ClosedDoor | MapElement::RemoteDoor => Some(image_map.closed_door),
            MapElement::OpenDoor => Some(image_map.open_door),
            MapElement::Plate(_, _) => Some(image_map.plate),
            MapElement::Light { .. } => Some(image_map.lights[0]),
            MapElement::MovingWall { .. } => Some(image_map.wall),
        }
    }
    pub fn passable(&self) -> bool {
        match self {
            MapElement::Empty
            | MapElement::OpenDoor
            | MapElement::ClosedDoor // Dealt with later
            | MapElement::RemoteDoor // Dealt with later
            | MapElement::Plate(_, _)
            | MapElement::Light{..} => true,
            MapElement::Wall
            | MapElement::MovingWall {..}=> false,
        }
    }
    pub fn add(&self, image_map: &ImageMap, pt: Point, ecs: &mut ECS) -> Entity {
        let e = ecs.entities.insert(());
        if let Some(image) = self.image(image_map) {
            ecs.images.insert(e, image);
        }
        ecs.positions.insert(e, pt);
        let mut event_listeners = Vec::new();
        match self {
            MapElement::ClosedDoor => {
                event_listeners.push(
                    EventListener::new(
                        EventTrigger::PlayerIntersectHasItems(Item::Key(Key {}), 1),
                        Action::All(vec![
                            Action::PlayerMarkUsed(Item::Key(Key {}), 1),
                            Action::SetImage {
                                target: e,
                                img: image_map.open_door,
                            },
                            Action::DisableGroup(e, Group::Locked),
                        ]),
                    )
                    .with_group(Group::Locked),
                );
                event_listeners.push(
                    EventListener::new(
                        EventTrigger::PlayerIntersect,
                        Action::Reject("Door locked"),
                    )
                    .with_group(Group::Locked),
                );
            }
            MapElement::RemoteDoor => {
                event_listeners.push(
                    EventListener::new(
                        EventTrigger::PlayerIntersect,
                        Action::Reject("Door locked remotely"),
                    )
                    .with_group(Group::Locked),
                );
            }
            MapElement::Light {
                counter,
                rising,
                falling,
            } => {
                event_listeners.extend((0..4).map(|i| {
                    EventListener::new(
                        EventTrigger::CounterPredicate(
                            *counter,
                            Rc::new(Box::new(move |c| c == i)),
                        ),
                        Action::SetImage {
                            target: e,
                            img: image_map.lights[i as usize],
                        },
                    )
                    .with_priority(Priority::Cleanup)
                }));
                event_listeners.extend_from_slice(&[
                    EventListener::new(
                        EventTrigger::CounterPredicate(*counter, Rc::new(Box::new(|c| c == 3))),
                        rising.clone(),
                    )
                    .with_priority(Priority::Cleanup)
                    .with_modifier(EventTriggerModifier::Rising(false)),
                    EventListener::new(
                        EventTrigger::CounterPredicate(*counter, Rc::new(Box::new(|c| c == 3))),
                        falling.clone(),
                    )
                    .with_priority(Priority::Cleanup)
                    .with_modifier(EventTriggerModifier::Falling(false)),
                ]);
            }
            MapElement::Plate(counter, target) => {
                event_listeners.extend_from_slice(&[
                    EventListener::new(
                        EventTrigger::PlayerIntersect,
                        Action::AlterCounter(*target, *counter, Rc::new(Box::new(|c| c + 1))),
                    )
                    .with_modifier(EventTriggerModifier::Rising(false)),
                    EventListener::new(
                        EventTrigger::PlayerIntersect,
                        Action::AlterCounter(*target, *counter, Rc::new(Box::new(|c| c - 1))),
                    )
                    .with_modifier(EventTriggerModifier::Falling(false)),
                ]);
            }
            MapElement::Wall => {
                event_listeners.push(EventListener::new(
                    EventTrigger::PlayerIntersect,
                    Action::Reject("impassible"),
                ));
            }
            MapElement::MovingWall { direction, reset } => {
                event_listeners.push(EventListener::new(
                    EventTrigger::PlayerIntersect,
                    Action::Reject("impassible"),
                ));
                if let Some((start, end)) = *reset {
                    event_listeners.push(EventListener::new(
                        EventTrigger::PositionPredicate(Rc::new(Box::new(move |pt| pt == end))),
                        Action::SetPosition {
                            target: e,
                            position: start,
                        },
                    ));
                }
                ecs.movement.insert(
                    e,
                    Movement {
                        direction: Some(*direction),
                        movement_type: MovementType::Constant(*direction),
                    },
                );
            }
            _ => {}
        };
        if !event_listeners.is_empty() {
            ecs.event_listeners.insert(e, event_listeners);
        }
        e
    }
}

// See https://kyren.github.io/2018/09/14/rustconf-talk.html
// Fundamentally, we want to store a big set of structs. Each field of the struct is nullable, and
// each field corresponds to a different "component" that may or may not be present in a given
// "entity". This is pretty far from the standard ECS representation. To get there, we:
// * Swap out the set with an array.
// * Perform a array of structs to struct of arrays transform
// * Re-use deleted slots using a generational index. Identify the entity with the generational
// indices.
// For our use case, many things are non-standard:
// * Performance is much less of a concern. This game is in no way real-time: the game state can
// only evolve at the speed of human input, and the number of entities is likely less than 100 at
// any given time.
// * We'll have one ECS per frame, not one for the game as a while. Since almost everything happens
// local to a frame, it's more important to make single-frame manipulation easy than cross-frame
// manipulation. This means we'll be copying the ECS as a whole a lot: from a performance
// perspective, making that cheap may be important. Persistent collections could help here,
// although those fall back to more standard collections at small sizes, which we may be under.
// * The good news is that if we're yolo copying the ECS every frame, the IDs can persist without
// issue. In particular, this means an ECS ID can refer to the "same" object in multiple frames, so
// we can use the ECS IDs for references.
new_key_type! { pub struct Entity; }
pub type Components<T> = SecondaryMap<Entity, T>;
pub type SparseComponents<T> = SparseSecondaryMap<Entity, T>;
#[derive(Clone, Debug, Default)]
pub struct ECS {
    // TODO: leak ImageMap at launch, then stick a reference to it in every ECS so an ECS can
    // insert things itself.
    pub entities: HopSlotMap<Entity, ()>,
    pub images: Components<DrawLayer>,
    pub positions: Components<Point>,
    pub event_listeners: Components<Vec<EventListener>>,
    pub disabled_event_groups: Components<EnumSet<Group>>,
    pub counters: Components<EnumMap<Counter, i64>>,
    pub players: Components<Inventory>,
    pub movement: Components<Movement>,
}

impl ECS {
    pub fn insert_player(
        &mut self,
        image_map: &ImageMap,
        pos: Point,
        inventory: Inventory,
    ) -> Entity {
        let player = self.entities.insert(());
        self.players.insert(player, inventory);
        self.positions.insert(player, pos);
        self.images.insert(player, image_map.player);
        self.movement.insert(
            player,
            Movement {
                direction: None,
                movement_type: MovementType::PlayerControlled,
            },
        );
        self.event_listeners.insert(
            player,
            vec![EventListener::new(
                EventTrigger::PlayerIntersect,
                Action::Reject("impassible"),
            )],
        );
        player
    }
    pub fn verify(&self) {
        for (player, _inventory) in self.players.iter() {
            if !self.entities.contains_key(player) {
                continue;
            }
            if !self.positions.contains_key(player) {
                panic!("Player without position in ECS: {:#?}", self)
            }
        }
    }
}

// TODO: doing a mutable join is really tricky here: get_mut doesn't trust that all your entities
// are different. If you've only got one mutable component, that's fine: you use that as your base.
// If you've got more, you probably need to do it by hand.
pub fn inner_join<I: Iterator<Item = (Entity, T1)>, T1, T2>(
    iter: I,
    other: &Components<T2>,
) -> impl Iterator<Item = (Entity, (T1, &T2))> {
    iter.filter_map(move |(e, x1)| {
        let x2 = other.get(e)?;
        Some((e, (x1, x2)))
    })
}

pub trait DrawDebug: graphics::Drawable + std::fmt::Debug {}
impl<T> DrawDebug for T where T: graphics::Drawable + std::fmt::Debug {}

pub type DrawRef = &'static dyn DrawDebug;
#[derive(Clone, Copy, Debug)]
pub struct DrawLayer {
    pub layer: Layer,
    pub draw_ref: DrawRef,
}

pub trait CloneFn<A, B>: objekt::Clone + Fn(A) -> B {}
objekt::clone_trait_object!(<A,B>CloneFn<A,B>);

// TODO: consider type-level shenanigans to prevent composing an Action that requires an input the
// EventTrigger can't provide.
#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub enum EventTrigger {
    PlayerIntersect,
    PlayerNotIntersect,
    PlayerIntersectHasItems(Item, usize),
    ItemIntersect(Item),
    CounterPredicate(
        Counter,
        #[derivative(Debug = "ignore")] Rc<Box<dyn Fn(i64) -> bool>>,
    ),
    PositionPredicate(#[derivative(Debug = "ignore")] Rc<Box<dyn Fn(Point) -> bool>>),
}

#[derive(Copy, Clone, Debug, Enum)]
pub enum Counter {
    Unlock,
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub enum Action {
    AlterCounter(
        Entity,
        Counter,
        #[derivative(Debug = "ignore")] Rc<Box<dyn Fn(i64) -> i64>>,
    ),
    // Implicitly uses intersecting player; should maybe take an argument for how to find the player.
    PlayerMarkUsed(Item, usize),
    Reject(&'static str),
    SetImage {
        target: Entity,
        img: DrawLayer,
    },
    EnableGroup(Entity, Group),
    DisableGroup(Entity, Group),
    SetPosition {
        target: Entity,
        position: Point,
    },
    All(Vec<Action>),
}

#[derive(Clone, Debug)]
pub enum EventTriggerModifier {
    Unmodified,
    Rising(bool),
    Falling(bool),
    Negated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Main,
    Cleanup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Enum)]
pub enum Layer {
    Background,
    Foreground,
    UI,
}

#[derive(Debug, EnumSetType)]
pub enum Group {
    Default,
    Locked,
}

#[derive(Clone, Debug)]
pub struct EventListener {
    pub trigger: EventTrigger,
    pub modifier: EventTriggerModifier,
    pub action: Action,
    pub group: Group,
    pub priority: Priority,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum MovementType {
    PlayerControlled,
    Constant(Direction),
}

#[derive(Clone, Debug)]
pub struct Movement {
    pub direction: Option<Direction>,
    pub movement_type: MovementType,
}

impl EventListener {
    pub fn new(trigger: EventTrigger, action: Action) -> Self {
        EventListener {
            trigger,
            modifier: EventTriggerModifier::Unmodified,
            action,
            group: Group::Default,
            priority: Priority::Main,
        }
    }
    pub fn with_group(mut self, group: Group) -> Self {
        self.group = group;
        self
    }
    pub fn with_modifier(mut self, modifier: EventTriggerModifier) -> Self {
        self.modifier = modifier;
        self
    }
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
}

pub fn entities_at(ecs: &ECS, pt: Point) -> Vec<Entity> {
    ecs.positions
        .iter()
        .filter_map(|(e, &p)| {
            if ecs.entities.contains_key(e) && p == pt {
                Some(e)
            } else {
                None
            }
        })
        .collect()
}

pub fn player_at(ecs: &ECS, pt: Point) -> Option<Entity> {
    let entities = entities_at(ecs, pt);
    entities.into_iter().find(|e| ecs.players.contains_key(*e))
}

#[cfg(test)]
mod tests {
    use super::{
        add_to_cells, ActualInventory, HypotheticalInventory, Inventory, InventoryCell, Item, Key,
    };
    use proptest::{
        arbitrary::{any, Arbitrary},
        strategy::{BoxedStrategy, Strategy},
    };

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
            let (merged, _ )= hypothetical.merge_in(Inventory::Actual(actual)).expect("Merge failed");
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
            let (merged, _) = hypothetical.merge_in(Inventory::Actual(actual)).expect("Merge failed");
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
            let (merged, _) = hypothetical.merge_in(Inventory::Actual(actual)).expect("Merge failed");
            let merged_counts = merged.count_items();
            assert_eq!(expected_counts, merged_counts);
        }
    }
    //TODO: Hypothetical self merge should yield self
    //May need an actual unit test here.
}
