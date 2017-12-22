#![feature(i128_type)]

#[macro_use]
extern crate conrod;
extern crate glium;
#[macro_use(array)]
extern crate ndarray;
extern crate rand;
extern crate tree;

use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use conrod::{color, widget, Colorable, Positionable, Sizeable, Widget};
use ndarray::{Array2, ArrayView, ArrayViewMut, ShapeBuilder, Zip};

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

pub struct ImageIds {
    pub jump_icon: conrod::image::Id,
    pub move_arrows: [conrod::image::Id; 4],
}

pub struct GameFrame {
    pub players: Vec<Player>,
    pub constraints: HashMap<(usize, usize), Constraint>,
}

impl GameFrame {
    pub fn new() -> Self {
        GameFrame {
            players: Vec::new(),
            constraints: HashMap::new(),
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Selection {
    Player(Id<Player>),
    GridCell(Point),
}

widget_ids! {
    #[derive(Clone)]
    struct GameStateIds {
        grid,
        planned_portals[],
    }
}

pub struct GameState {
    ids: GameStateIds,
    pub history: tree::Zipper<GameFrame, Plan>,
    pub selected: Option<Selection>,
    pub current_plan: CachablePlan,
}

impl GameState {
    pub fn new(id_generator: widget::id::Generator) -> Self {
        GameState {
            ids: GameStateIds::new(id_generator),
            history: tree::Zipper::new(tree::RoseTree::singleton(GameFrame::new())),
            selected: None,
            current_plan: CachablePlan::new(),
        }
    }

    pub fn rotate_plan(&mut self) -> Result<(), &str> {
        match self.history.focus.children.len() {
            0 => Err("No future recorded: can't cycle plans"),
            l => {
                self.current_plan = match self.current_plan {
                    CachablePlan::Old(i) => CachablePlan::Old(i.checked_sub(1).unwrap_or(l - 1)),
                    CachablePlan::Novel(_) => CachablePlan::Old(l - 1),
                };
                Ok(())
            }
        }
    }

    pub fn render(&mut self, ui_cell: &mut conrod::UiCell, image_ids: &ImageIds) -> bool {
        const COLS: usize = 6;
        const ROWS: usize = 6;
        let mut elements = widget::Matrix::new(COLS, ROWS)
            .w_h(ui_cell.win_w, ui_cell.win_h)
            .middle_of(ui_cell.window)
            .set(self.ids.grid, ui_cell);
        let mut should_update = false;
        //elements.next is in column major order for some reason
        let mut elements_vec = Vec::new();
        while let Some(elem) = elements.next(ui_cell) {
            elements_vec.push(elem);
        }
        let mut grid_cells =
            ndarray::Array2::from_shape_vec((ROWS, COLS).f(), elements_vec).unwrap();
        let mut buttons = Array2::from_shape_fn(
            grid_cells.raw_dim(),
            |_| widget::Button::new(), //.color(color::TRANSPARENT)
        );
        if let Some(Selection::GridCell((r, c))) = self.selected {
            //let luminance = n as f32 / (COLS * ROWS) as f32;
            //let button = widget::Button::new().color(color::BLUE.with_luminance(luminance));
            buttons[(r, c)].style.color = Some(color::RED);
        }

        Zip::indexed(&mut grid_cells)
            .and(&mut buttons)
            .apply(|(r, c), elem, button| {
                assert_eq!((r, c), (elem.row, elem.col));
                for _click in elem.set(button.clone(), ui_cell) {
                    self.selected = Some(Selection::GridCell((r, c)));
                    should_update = true;
                    //println!("Hey! {:?}", (r, c));
                }
            });

        for (&(x, y), constraint) in self.history.get_focus_val_mut().constraints.iter_mut() {
            let parent_elem = grid_cells[[x, y]];
            let id = constraint
                .id
                .get_or_insert(ui_cell.widget_id_generator().next());
            widget::Circle::fill(40.)
                .color(color::BLUE)
                .middle_of(parent_elem.widget_id)
                .set(*id, ui_cell);
        }

        for player in self.history.focus.val.players.iter_mut() {
            //buttons[player.position] = buttons[player.position].clone().color(color::GREEN).label("Player");
            let parent_elem = grid_cells[player.position];
            let widget_ids = player
                .widget_ids
                .get_or_insert(PlayerIds::new(ui_cell.widget_id_generator()));
            let mut circle = widget::Circle::fill(30.0)
                .color(color::GREEN)
                //.label("Player")
                //.parent(parent_elem.widget_id)
                //.middle();
                .middle_of(parent_elem.widget_id);
            if Some(Selection::Player(player.id)) == self.selected {
                circle = circle.clone().color(color::RED);
            }
            circle.set(widget_ids.player, ui_cell);
            if let Some(player_move) = self.current_plan
                .get(&self.history.focus.children)
                .moves
                .get(&player.id)
            {
                player_move
                    .widget(image_ids)
                    .parent(widget_ids.player)
                    .set(widget_ids.planned_move, ui_cell)
            }
            for _click in ui_cell.widget_input(widget_ids.player).clicks() {
                self.selected = Some(Selection::Player(player.id));
                should_update = true;
            }
        }
        let mut portals_ids = self.ids.planned_portals.walk();
        for &(x, y) in self.current_plan
            .get(&self.history.focus.children)
            .portals
            .iter()
        {
            let parent_elem = grid_cells[[x, y]];
            let id = portals_ids.next(
                &mut self.ids.planned_portals,
                &mut ui_cell.widget_id_generator(),
            );
            let color = color::Color::Rgba(0., 0., 0.7, 0.5);
            widget::Circle::fill(40.)
                .color(color)
                .middle_of(parent_elem.widget_id)
                .set(id, ui_cell);
        }
        return should_update;
    }
}

#[derive(Clone)]
pub struct Constraint {
    id: Option<widget::Id>,
    pub timestamp: usize,
    pub player_position: Point,
}

impl Constraint {
    pub fn new(timestamp: usize, player_position: Point) -> Self {
        Constraint {
            id: None,
            timestamp,
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
    fn rotation(&self) -> Array2<f64> {
        match *self {
            Direction::Up => array![[1., 0.], [0., 1.]],
            Direction::Down => array![[-1., 0.], [0., -1.]],
            Direction::Left => array![[0., -1.], [1., 0.]],
            Direction::Right => array![[0., 1.], [-1., 0.]],
        }
    }
}

#[derive(Clone)]
pub enum Move {
    Direction(Direction),
    Jump,
}

impl Move {
    pub fn widget(&self, image_ids: &ImageIds) -> widget::Image {
        match *self {
            Move::Direction(ref direction) => {
                let unrotated_points = vec![[0.0, 0.0], [50.0, 0.0], [25.0, 25.0]];
                let mut points = vec![[0., 0.]; 3];
                for (x, y) in unrotated_points.iter().zip(points.iter_mut()) {
                    //y <- a A x + b y
                    ndarray::linalg::general_mat_vec_mul(
                        1.,                         //a
                        &direction.rotation(),      //A
                        &ArrayView::from(x),        //x
                        1.,                         //b
                        &mut ArrayViewMut::from(y), //y
                    );
                }
                let triangle = match *direction {
                    Direction::Up => widget::Image::new(image_ids.move_arrows[0]),
                    Direction::Left => widget::Image::new(image_ids.move_arrows[1]),
                    Direction::Down => widget::Image::new(image_ids.move_arrows[2]),
                    Direction::Right => widget::Image::new(image_ids.move_arrows[3]),
                };
                match *direction {
                    Direction::Up => triangle.up(0.).align_middle_x(),
                    Direction::Down => triangle.down(0.).align_middle_x(),
                    Direction::Left => triangle.left(0.).align_middle_y(),
                    Direction::Right => triangle.right(0.).align_middle_y(),
                }
            }
            Move::Jump => widget::Image::new(image_ids.jump_icon).middle(),
        }
    }
}

#[derive(Clone)]
pub struct Plan {
    pub moves: HashMap<Id<Player>, Move>,
    pub portals: HashSet<(usize, usize)>,
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

widget_ids! {
    #[derive(Clone)]
    struct PlayerIds {
        player,
        planned_move,
    }
}

#[derive(Clone)]
pub struct Player {
    widget_ids: Option<PlayerIds>,
    pub id: Id<Player>,
    pub position: Point,
}

impl Player {
    pub fn new(position: Point) -> Self {
        Player {
            widget_ids: None,
            id: rand::random(),
            position,
        }
    }
}

type Point = (usize, usize);

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
