#[macro_use]
extern crate conrod;
extern crate glium;
extern crate tree;
#[macro_use(array)]
extern crate ndarray;

use std::collections::{HashMap, HashSet};
use conrod::{color, widget, Positionable, Widget, Sizeable, Colorable};
use ndarray::{ShapeBuilder, Zip, Array2, ArrayView, ArrayViewMut};

pub struct ImageIds {
    pub jump_icon : conrod::image::Id,
    pub move_arrows : [conrod::image::Id;4],
}

pub struct GameFrame {
    pub players : Vec<Player>,
    pub constraints : HashMap<(usize, usize), Constraint>,
}

impl GameFrame {
    pub fn new() -> Self {
        GameFrame {
            players : Vec::new(),
            constraints : HashMap::new(),
        }
    }
}

pub enum Selection {
    Player(widget::Id),
    GridCell(Point),
}

pub struct GameState {
    id : widget::Id,
    pub current_frame : GameFrame,
    pub selected : Option<Selection>,
    pub current_plan : Plan,
}

impl GameState {
    pub fn new(id_generator : & mut widget::id::Generator) -> Self {
        GameState {
            id : id_generator.next(),
            current_frame : GameFrame::new(),
            selected : None,
            current_plan : Plan::new(),
        }
    }

    pub fn render(&mut self,
                  ui_cell : &mut conrod::UiCell,
                  image_ids : & ImageIds) -> bool {
        const COLS : usize = 6;
        const ROWS : usize = 6;
        let mut elements = widget::Matrix::new(COLS, ROWS)
            .w_h(ui_cell.win_w , ui_cell.win_h)
            .middle_of(ui_cell.window)
            .set(self.id, ui_cell);
        let mut should_update = false;
        //elements.next is in column major order for some reason
        let mut elements_vec = Vec::new();
        while let Some(elem) = elements.next(ui_cell) {
            elements_vec.push(elem);
        }
        let mut grid_cells = ndarray::Array2::from_shape_vec((ROWS,COLS).f(), elements_vec).unwrap();
        let mut buttons = Array2::from_shape_fn(grid_cells.raw_dim(),|_|
                                                widget::Button::new()//.color(color::TRANSPARENT)
                                                );
        if let Some(Selection::GridCell((r,c))) = self.selected {
            //let luminance = n as f32 / (COLS * ROWS) as f32;
            //let button = widget::Button::new().color(color::BLUE.with_luminance(luminance));
            buttons[(r,c)].style.color = Some(color::RED);
        }

        Zip::indexed(&mut grid_cells).and(&mut buttons).apply(|(r,c), elem, button| {
            assert_eq!((r,c), (elem.row, elem.col));
            for _click in elem.set(button.clone(), ui_cell) {
                self.selected = Some(Selection::GridCell((r,c)));
                should_update = true;
                //println!("Hey! {:?}", (r, c));
            }
        });

        for (& (x,y), constraint) in self.current_frame.constraints.iter_mut() {
            let parent_elem = grid_cells[[x,y]];
            let id = constraint.id.get_or_insert(ui_cell.widget_id_generator().next());
            widget::Circle::fill(40.)
                .color(color::BLUE)
                .middle_of(parent_elem.widget_id)
                .set(*id, ui_cell);
        }

        for player in self.current_frame.players.iter() {
            //buttons[player.position] = buttons[player.position].clone().color(color::GREEN).label("Player");
            let parent_elem = grid_cells[player.position];
            let mut circle = widget::Circle::fill(30.0)
                .color(color::GREEN)
                //.label("Player")
                //.parent(parent_elem.widget_id)
                //.middle();
                .middle_of(parent_elem.widget_id);
            if let Some(Selection::Player(selectedPlayerId)) = self.selected {
                if selectedPlayerId == player.get_id() {
                    circle = circle.clone().color(color::RED);
                }
            }
            circle.set(player.ids.player, ui_cell);
            if let Some(player_move) = self.current_plan.moves.get(& player.get_id()) {
                player_move.widget(image_ids)
                    .parent(player.ids.player)
                    .set(player.ids.planned_move, ui_cell)
            }
            for _click in ui_cell.widget_input(player.get_id()).clicks(){
                self.selected = Some(Selection::Player(player.get_id()));
                should_update = true;
            }
        }
        return should_update 
    }
}


#[derive(Clone)]
pub struct Constraint {
    id : Option<widget::Id>,
    pub timestamp : usize,
    pub player_position : Point,
}

impl Constraint {
    pub fn new(timestamp : usize, player_position : Point) -> Self {
        Constraint{id : None, timestamp, player_position}
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
    fn rotation(& self) -> Array2<f64> {
        match *self {
            Direction::Up => array![[1., 0.],
                         [0., 1.]],
            Direction::Down => array![[-1.,  0.],
                           [ 0., -1.]],
            Direction::Left => array![[ 0., -1.],
                           [1., 0.]],
            Direction::Right => array![[0., 1.],
                            [-1.,  0.]],
        }
    }
}

#[derive(Clone)]
pub enum Move {
    Direction(Direction),
    Jump
}


impl Move {
    pub fn widget(& self, image_ids : & ImageIds) -> widget::Image {
        match *self {
            Move::Direction(ref direction) => {
                let unrotated_points = vec![[0.0, 0.0], [50.0,0.0], [25.0, 25.0]];
                let mut points = vec![[0.,0.];3];
                for (x,y) in unrotated_points.iter().zip(points.iter_mut()) {
                    //y <- a A x + b y
                    ndarray::linalg::general_mat_vec_mul(1.,//a
                                                         & direction.rotation(),//A
                                                         & ArrayView::from(x),//x
                                                         1.,//b
                                                         & mut ArrayViewMut::from(y)//y
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
            },
            Move::Jump => {
                widget::Image::new(image_ids.jump_icon).middle()
            }
        }
    }
}

pub struct Plan {
    pub moves : HashMap<widget::Id, Move>,
    pub portals : HashSet<(usize, usize)>
}

impl Plan {
    pub fn new() -> Self {
        Plan {
            moves : HashMap::new(),
            portals : HashSet::new(),

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
    ids : PlayerIds,
    pub position : Point,
}

impl Player {
    pub fn new(id_generator : widget::id::Generator, position : Point) -> Self {
        Player{ids : PlayerIds::new(id_generator), position}
    }
    pub fn get_id(& self) -> widget::Id {
        self.ids.player
    }
}

type Point = (usize, usize);
