extern crate ggez;
extern crate nalgebra;
extern crate rand;
extern crate tree;
extern crate types;

use ggez::{event, graphics};
use ggez::graphics::Point2;
use graphics::Drawable;

use std::f32::consts::PI;

use nalgebra::{Similarity2, Vector2};

use types::*;

mod logic;

fn draw_grid(ctx: &mut ggez::Context) -> ggez::GameResult<()> {
    let graphics::Rect { x: x0, y: y0, w, h } = graphics::get_screen_coordinates(ctx);
    let mut x = x0;
    let mut y = y0;
    while x <= x0 + w {
        graphics::line(ctx, &[Point2::new(x, y0), Point2::new(x, y0 + h)], 5.)?;
        x += SCALE;
    }
    while y <= y0 + h {
        graphics::line(ctx, &[Point2::new(x0, y), Point2::new(x0 + w, y)], 5.)?;
        y += SCALE;
    }
    Ok(())
}

pub struct GameState {
    pub history: tree::Zipper<GameFrame, Plan>,
    pub selected: Option<Selection>,
    pub current_plan: CachablePlan,
    pub image_map: ImageMap,
}

impl GameState {
    pub fn new(ctx: &mut ggez::Context) -> ggez::GameResult<Self> {
        let image_map = ImageMap::new(ctx)?;
        Ok(GameState {
            history: tree::Zipper::new(tree::RoseTree::singleton(GameFrame::new())),
            selected: None,
            current_plan: CachablePlan::new(),
            image_map,
        })
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

    /*
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
    */
}

impl event::EventHandler for GameState {
    fn update(&mut self, _ctx: &mut ggez::Context) -> ggez::GameResult<()> {
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        ctx: &mut ggez::Context,
        button: event::MouseButton,
        x: i32,
        y: i32,
    ) {
        let graphics::Rect { x: x0, y: y0, .. } = graphics::get_screen_coordinates(ctx);
        let inv_transform: Similarity2<f32> =
            Similarity2::new(Vector2::new(x0, y0), 0., SCALE).inverse();
        let world_space_pt: Point = nalgebra::try_convert::<Point2, nalgebra::Point2<i32>>(
            inv_transform * Point2::new(x as f32, y as f32),
        ).unwrap();
        match button {
            event::MouseButton::Left => {
                for player in self.history.get_focus_val().players.iter() {
                    if player.position == world_space_pt {
                        self.selected = Some(Selection::Player(player.id));
                        return;
                    }
                }
                self.selected = Some(Selection::GridCell(world_space_pt));
            }
            _ => {}
        }
    }
    fn key_down_event(
        &mut self,
        _ctx: &mut ggez::Context,
        key: event::Keycode,
        _keymod: event::Mod,
        _repeat: bool,
    ) {
        match self.selected {
            Some(Selection::Player(player_id)) => {
                enum Update {
                    SetMove(Move),
                    ClearMove,
                }
                let update_option = match key {
                    event::Keycode::W => Some(Update::SetMove(Move::Direction(Direction::Up))),
                    event::Keycode::A => Some(Update::SetMove(Move::Direction(Direction::Left))),
                    event::Keycode::S => Some(Update::SetMove(Move::Direction(Direction::Down))),
                    event::Keycode::D => Some(Update::SetMove(Move::Direction(Direction::Right))),
                    event::Keycode::Q => Some(Update::SetMove(Move::Jump)),
                    event::Keycode::Space => Some(Update::ClearMove),
                    _ => None,
                };
                for update in update_option {
                    match update {
                        Update::SetMove(new_move) => self.current_plan
                            .cow(&self.history.focus.children)
                            .moves
                            .insert(player_id, new_move),
                        Update::ClearMove => self.current_plan
                            .cow(&self.history.focus.children)
                            .moves
                            .remove(&player_id),
                    };
                }
            }
            Some(Selection::GridCell(pt)) => {
                if let event::Keycode::Q = key {
                    if self.current_plan
                        .get(&self.history.focus.children)
                        .portals
                        .contains(&pt)
                    {
                        self.current_plan
                            .cow(&self.history.focus.children)
                            .portals
                            .remove(&pt);
                    } else {
                        self.current_plan
                            .cow(&self.history.focus.children)
                            .portals
                            .insert(pt);
                    }
                }
            }
            None => {}
        }
        match key {
            event::Keycode::Tab => if let Err(err) = self.rotate_plan() {
                println!("{}", err);
            },
            event::Keycode::Backspace => match self.history.up() {
                Ok(ix) => {
                    self.current_plan = CachablePlan::Old(ix);
                }
                Err(err) => println!("{}", err),
            },
            event::Keycode::Return => match logic::apply_plan(
                &self.history.get_focus_val(),
                &self.current_plan.get(&self.history.focus.children),
            ) {
                Err(err) => println!("{}", err),
                Ok(new_frame) => match self.current_plan {
                    CachablePlan::Novel(ref mut plan) => {
                        let old_plan = std::mem::replace(plan, Plan::new());
                        self.history.push(new_frame, old_plan);
                    }
                    CachablePlan::Old(ix) => {
                        self.history.down(ix).expect("Cached plan wasn't there!");
                        self.current_plan = match self.history.focus.children.len() {
                            0 => CachablePlan::new(),
                            l => CachablePlan::Old(l - 1),
                        }
                    }
                },
            },
            _ => {}
        }
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult<()> {
        let graphics::Rect { x: x0, y: y0, .. } = graphics::get_screen_coordinates(ctx);
        let transform: Similarity2<f32> = Similarity2::new(Vector2::new(x0, y0), 0., SCALE);
        graphics::clear(ctx);
        graphics::set_background_color(ctx, graphics::Color::from_rgb(255, 255, 255));
        graphics::set_color(ctx, graphics::Color::from_rgb(0, 0, 0))?;
        draw_grid(ctx)?;
        graphics::set_color(ctx, graphics::Color::from_rgb(255, 255, 255))?;
        for player in self.history.get_focus_val().players.iter() {
            self.image_map.player.draw(
                ctx,
                transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(player.position),
                0.,
            )?;
            for mv in self.current_plan
                .get(&self.history.focus.children)
                .moves
                .get(&player.id)
            {
                let (image, rotation) = match mv {
                    &Move::Direction(ref dir) => {
                        let angle = match dir {
                            &Direction::Up => 0.,
                            &Direction::Left => 1.5 * PI,
                            &Direction::Down => PI,
                            &Direction::Right => 0.5 * PI,
                        };
                        (&self.image_map.move_arrow, angle)
                    }
                    &Move::Jump => (&self.image_map.jump_icon, 0.),
                };
                let dest = transform
                    * (nalgebra::convert::<nalgebra::Point2<i32>, Point2>(player.position)
                        + Vector2::new(0.5, 0.5));
                image.draw_ex(
                    ctx,
                    graphics::DrawParam {
                        dest,
                        offset: Point2::new(0.5, 0.5),
                        rotation,
                        ..Default::default()
                    },
                )?;
            }
            for pt in self.history.get_focus_val().constraints.keys() {
                self.image_map.portal.draw(
                    ctx,
                    transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(*pt),
                    0.,
                )?;
            }
            for pt in &self.current_plan.get(&self.history.focus.children).portals {
                self.image_map.jump_icon.draw(
                    ctx,
                    transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(*pt),
                    0.,
                )?;
            }
            if Some(Selection::Player(player.id)) == self.selected {
                self.image_map.selection.draw(
                    ctx,
                    transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(player.position),
                    0.,
                )?;
            }
        }
        if let Some(Selection::GridCell(pt)) = self.selected {
            self.image_map.selection.draw(
                ctx,
                transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(pt),
                0.,
            )?;
        }
        graphics::present(ctx);
        Ok(())
    }
}
