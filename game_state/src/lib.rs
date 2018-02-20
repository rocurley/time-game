#![feature(nll)]

extern crate ggez;
extern crate nalgebra;
#[cfg(test)]
#[macro_use]
pub extern crate proptest;
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

fn draw_grid(ctx: &mut ggez::Context, bounds: graphics::Rect) -> ggez::GameResult<()> {
    let mut x = bounds.x;
    let mut y = bounds.y;
    while x <= bounds.x + bounds.w {
        graphics::line(
            ctx,
            &[
                Point2::new(x, bounds.y - 2.5),
                Point2::new(x, bounds.y + bounds.h + 2.5),
            ],
            5.,
        )?;
        x += SCALE;
    }
    while y <= bounds.y + bounds.h {
        graphics::line(
            ctx,
            &[
                Point2::new(bounds.x - 2.5, y),
                Point2::new(bounds.x + bounds.w + 2.5, y),
            ],
            5.,
        )?;
        y += SCALE;
    }
    Ok(())
}

fn draw_map_grid(ctx: &mut ggez::Context) -> ggez::GameResult<()> {
    let bounds = graphics::get_screen_coordinates(ctx);
    draw_grid(ctx, bounds)
}

pub struct GameState {
    pub history: tree::Zipper<GameFrame, Plan>,
    pub selected: Selection,
    pub current_plan: CachablePlan,
    pub image_map: ImageMap,
}

impl GameState {
    pub fn new(ctx: &mut ggez::Context) -> ggez::GameResult<Self> {
        let image_map = ImageMap::new(ctx)?;
        Ok(GameState {
            history: tree::Zipper::new(tree::RoseTree::singleton(GameFrame::new())),
            selected: Selection::Top,
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
    pub fn validate_selection(&mut self) {
        match self.selected {
            Selection::Top => {}
            Selection::GridCell(pt) => {
                if let Some(player_id) = self.history.get_focus_val().players.by_position.get(&pt) {
                    self.selected = Selection::Player(player_id.clone());
                }
            }
            Selection::Inventory(player_id, _) | Selection::Player(player_id) => {
                if !self.history
                    .get_focus_val()
                    .players
                    .by_id
                    .contains_key(&player_id)
                {
                    self.selected = Selection::Top;
                }
            }
        }
    }
}

fn inventory_bbox(ctx: &ggez::Context) -> graphics::Rect {
    let screen_bounds = graphics::get_screen_coordinates(ctx);
    let w = INVENTORY_WIDTH as f32 * SCALE;
    let h = INVENTORY_HEIGHT as f32 * SCALE;
    graphics::Rect {
        x: screen_bounds.x + (screen_bounds.w - w) / 2.,
        y: screen_bounds.y + (screen_bounds.h - h) / 2.,
        w,
        h,
    }
}

fn pixel_space_to_tile_space(pt: Point2, bbox: ggez::graphics::Rect) -> Option<Point> {
    if !bbox.contains(pt) {
        return None;
    }
    let graphics::Rect { x: x0, y: y0, .. } = bbox;
    let inv_transform: Similarity2<f32> =
        Similarity2::new(Vector2::new(x0, y0), 0., SCALE).inverse();
    Some(nalgebra::try_convert::<Point2, nalgebra::Point2<i32>>(inv_transform * pt).unwrap())
}

fn tile_space_to_pixel_space(pt: Point, bbox: graphics::Rect) -> Point2 {
    let graphics::Rect { x: x0, y: y0, .. } = bbox;
    let transform: Similarity2<f32> = Similarity2::new(Vector2::new(x0, y0), 0., SCALE);
    transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(pt)
}

fn world_selection(pt: Point2, ctx: &ggez::Context, game_state: &GameState) -> Selection {
    let world_space_pt: Point =
        pixel_space_to_tile_space(pt, graphics::get_screen_coordinates(ctx))
            .expect("Somehow clicked outside window");
    let player = game_state
        .history
        .get_focus_val()
        .players
        .by_position
        .get(&world_space_pt);
    match player {
        Some(id) => Selection::Player(id.clone()),
        None => Selection::GridCell(world_space_pt),
    }
}

fn inventory_selection(pt: Point2, ctx: &ggez::Context, player_id: Id<Player>) -> Selection {
    let bbox = inventory_bbox(ctx);
    let inventory_space_pt = pixel_space_to_tile_space(pt, bbox);
    let row_length = (bbox.w / SCALE).trunc() as u8;
    let ix = inventory_space_pt.map(|pt| pt.x as u8 + pt.y as u8 * row_length);
    Selection::Inventory(player_id, ix)
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
        if let event::MouseButton::Left = button {
            let pt = Point2::new(x as f32, y as f32);
            self.selected = match self.selected {
                Selection::Inventory(player_id, _) => inventory_selection(pt, ctx, player_id),
                _ => world_selection(pt, ctx, self),
            }
        }
    }
    fn key_down_event(
        &mut self,
        _ctx: &mut ggez::Context,
        key: event::Keycode,
        _keymod: event::Mod,
        _repeat: bool,
    ) {
        use event::Keycode;
        match self.selected {
            Selection::Player(player_id) => {
                enum Update {
                    Move(Move),
                    Other(Keycode),
                }
                let update = match key {
                    Keycode::W => Update::Move(Move::Direction(Direction::Up)),
                    Keycode::A => Update::Move(Move::Direction(Direction::Left)),
                    Keycode::S => Update::Move(Move::Direction(Direction::Down)),
                    Keycode::D => Update::Move(Move::Direction(Direction::Right)),
                    Keycode::Q => Update::Move(Move::Jump),
                    Keycode::G => Update::Move(Move::PickUp),
                    keycode => Update::Other(keycode),
                };
                match update {
                    Update::Move(new_move) => {
                        self.current_plan
                            .cow(&self.history.focus.children)
                            .moves
                            .insert(player_id, new_move);
                    }
                    Update::Other(Keycode::Space) => {
                        self.current_plan
                            .cow(&self.history.focus.children)
                            .moves
                            .remove(&player_id);
                    }
                    Update::Other(Keycode::I) => {
                        self.selected = Selection::Inventory(player_id, None);
                    }
                    _ => {}
                }
            }
            Selection::GridCell(pt) => {
                if let Keycode::Q = key {
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
            Selection::Inventory(player_id, Some(ix)) => {
                if let Keycode::T = key {
                    self.current_plan
                        .cow(&self.history.focus.children)
                        .moves
                        .insert(player_id, Move::Drop(ix));
                }
            }
            Selection::Inventory(_, None) => {}
            Selection::Top => {}
        }
        match key {
            Keycode::Tab => if let Err(err) = self.rotate_plan() {
                println!("{}", err);
            },
            Keycode::Backspace => match self.history.up() {
                Ok(ix) => {
                    self.current_plan = CachablePlan::Old(ix);
                }
                Err(err) => println!("{}", err),
            },
            Keycode::Return => match logic::apply_plan(
                &self.history.get_focus_val(),
                &self.current_plan.get(&self.history.focus.children),
            ) {
                Err(err) => println!("{}", err),
                Ok(new_frame) => {
                    match self.current_plan {
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
                    }
                    self.validate_selection();
                }
            },
            Keycode::Escape => self.selected.pop(),
            _ => {}
        }
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult<()> {
        let graphics::Rect { x: x0, y: y0, .. } = graphics::get_screen_coordinates(ctx);
        let transform: Similarity2<f32> = Similarity2::new(Vector2::new(x0, y0), 0., SCALE);
        graphics::clear(ctx);
        graphics::set_background_color(ctx, graphics::Color::from_rgb(255, 255, 255));
        graphics::set_color(ctx, graphics::Color::from_rgb(0, 0, 0))?;
        draw_map_grid(ctx)?;
        graphics::set_color(ctx, graphics::Color::from_rgb(255, 255, 255))?;
        for player in self.history.get_focus_val().players.by_id.values() {
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
                    &Move::PickUp => (&self.image_map.pick_up_icon, 0.),
                    &Move::Drop(_) => (&self.image_map.drop_icon, 0.),
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
            for pt in &self.current_plan.get(&self.history.focus.children).portals {
                self.image_map.jump_icon.draw(
                    ctx,
                    transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(*pt),
                    0.,
                )?;
            }
        }
        for portal in self.history.get_focus_val().portals.by_id.values() {
            self.image_map.portal.draw(
                ctx,
                transform
                    * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(portal.player_position),
                0.,
            )?;
        }
        for (pt, item) in self.history.get_focus_val().items.iter() {
            item.image(&self.image_map).draw(
                ctx,
                transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(*pt),
                0.,
            )?;
        }
        match self.selected {
            Selection::Top => {}
            Selection::GridCell(pt) => {
                self.image_map.selection.draw(
                    ctx,
                    transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(pt),
                    0.,
                )?;
            }
            Selection::Player(player_id) => {
                let player = self.history
                    .get_focus_val()
                    .players
                    .by_id
                    .get(&player_id)
                    .expect("Invalid player selection");
                self.image_map.selection.draw(
                    ctx,
                    transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(player.position),
                    0.,
                )?;
            }
            Selection::Inventory(player_id, ref selected_item_option) => {
                let bounds = inventory_bbox(ctx);
                graphics::set_color(ctx, graphics::Color::from_rgb(127, 127, 127))?;
                graphics::rectangle(ctx, graphics::DrawMode::Fill, bounds)?;
                graphics::set_color(ctx, graphics::Color::from_rgb(0, 0, 0))?;
                draw_grid(ctx, bounds)?;
                graphics::set_color(ctx, graphics::Color::from_rgb(255, 255, 255))?;
                let inventory = &self.history
                    .get_focus_val()
                    .players
                    .by_id
                    .get(&player_id)
                    .expect("Invalid inventory player")
                    .inventory;
                for (i, inventory_cell_option) in inventory.iter().enumerate() {
                    for inventory_cell in inventory_cell_option.iter() {
                        let tile_space_pt = Point::new(
                            i as i32 % INVENTORY_WIDTH as i32,
                            i as i32 / INVENTORY_WIDTH as i32,
                        );
                        let pixel_space_pt = tile_space_to_pixel_space(tile_space_pt, bounds);
                        inventory_cell
                            .item
                            .image(&self.image_map)
                            .draw(ctx, pixel_space_pt, 0.)?;
                        let text = graphics::Text::new(
                            ctx,
                            &inventory_cell.count.to_string(),
                            &ctx.default_font.clone(),
                        )?;
                        graphics::set_color(ctx, graphics::Color::from_rgb(0, 0, 0))?;
                        text.draw(ctx, pixel_space_pt + Vector2::new(5., 5.), 0.0)?;
                    }
                }
                for &i in selected_item_option {
                    let tile_space_pt = Point::new(
                        i as i32 % INVENTORY_WIDTH as i32,
                        i as i32 / INVENTORY_WIDTH as i32,
                    );
                    let pixel_space_pt = tile_space_to_pixel_space(tile_space_pt, bounds);
                    self.image_map.selection.draw(ctx, pixel_space_pt, 0.)?;
                }
            }
        }
        graphics::present(ctx);
        Ok(())
    }
}
