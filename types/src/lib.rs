#[macro_use]
extern crate conrod;
extern crate tree;
extern crate ndarray;

use std::collections::HashMap;
use conrod::{color, widget, Positionable, Widget, Sizeable, Colorable, Labelable};
use ndarray::{ShapeBuilder, Zip, Array2};

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

    pub fn render(&mut self, ui_cell : &mut conrod::UiCell) -> bool {
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

        for player in self.current_frame.players.iter() {
            //buttons[player.position] = buttons[player.position].clone().color(color::GREEN).label("Player");
            let parent_elem = grid_cells[player.position];
            let mut circle = widget::Circle::fill(30.0)
                .color(color::GREEN)
                //.label("Player")
                //.parent(parent_elem.widget_id)
                .middle_of(parent_elem.widget_id);
            if let Some(Selection::Player(selectedPlayerId)) = self.selected {
                if selectedPlayerId == player.get_id() {
                    circle = circle.clone().color(color::RED);
                }
            }
            circle.set(player.ids.player, ui_cell);
            if let Some(player_move) = self.current_plan.moves.get(& player.get_id()) {
                player_move.symbol()
                    .right_from(player.ids.player,0.0)
                    //.middle_of(parent_elem.widget_id)
                    .set(player.ids.planned_move, ui_cell)
            }
            for click in ui_cell.widget_input(player.get_id()).clicks(){
                self.selected = Some(Selection::Player(player.get_id()));
                should_update = true;
            }
        }
        return should_update 
    }
}


#[derive(Clone)]
pub struct Constraint {
    pub timestamp : usize,
    pub player_position : Point,
}

#[derive(Clone)]
pub enum Move {
    Up,
    Down,
    Left,
    Right,
    Jump
}

impl Move {
    pub fn symbol(& self) -> widget::Polygon<Vec<conrod::Point>> {
        widget::Polygon::centred_fill(vec![[0.0, 0.0], [0.0,50.0], [25.0, 25.0]])
            .color(color::BLUE)
    }
}

pub struct Plan {
    pub moves : HashMap<widget::Id, Move>,
}

impl Plan {
    pub fn new() -> Self {
        Plan {
            moves : HashMap::new()
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
