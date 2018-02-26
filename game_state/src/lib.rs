#![feature(nll)]

extern crate ggez;
extern crate nalgebra;
#[cfg(test)]
#[macro_use]
pub extern crate proptest;
extern crate rand;
extern crate render;
extern crate tree;
extern crate types;

use ggez::{event, graphics};
use ggez::graphics::Point2;
use graphics::Drawable;

use std::f32::consts::PI;

use nalgebra::{Similarity2, Vector2};

use types::*;

use render::{draw_map_grid, inventory_bbox, pixel_space_to_tile_space, render_inventory};

mod logic;

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
            Selection::WishPicker(player_id, _)
            | Selection::Inventory(player_id, _)
            | Selection::Player(player_id) => {
                if !self.history
                    .get_focus_val()
                    .players
                    .by_id
                    .contains_key(&player_id)
                {
                    self.selected = Selection::Top;
                }
            }
            Selection::WishPickerInventoryViewer(player_id, ix, target_player_id) => {
                let players = &self.history.get_focus_val().players;
                if !players.by_id.contains_key(&player_id) {
                    self.selected = Selection::Top;
                }
                if !players.by_id.contains_key(&target_player_id) {
                    self.selected = Selection::WishPicker(player_id, ix);
                }
            }
        }
    }
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
    let ix = inventory_space_pt.map(|pt| pt.x as usize + pt.y as usize * INVENTORY_WIDTH);
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
            match self.selected {
                Selection::Inventory(player_id, _) => {
                    self.selected = inventory_selection(pt, ctx, player_id);
                }
                Selection::WishPicker(player_id, ix) => match world_selection(pt, ctx, self) {
                    Selection::GridCell(tile_pt) => {
                        let frame = self.history.get_focus_val_mut();
                        for item in frame.items.get(&tile_pt) {
                            let player = frame
                                .players
                                .by_id
                                .get_mut(&player_id)
                                .expect("Selection player id invalid");
                            match player.inventory {
                                Inventory::Actual(_) => panic!("Wishing from actual inventory"),
                                Inventory::Hypothetical(ref mut hypothetical) => match hypothetical
                                    .wish(item.clone(), ix)
                                {
                                    Ok(()) => {
                                        self.selected = Selection::Inventory(player_id, Some(ix))
                                    }
                                    Err(err) => println!("{}", err),
                                },
                            }
                        }
                    }
                    Selection::Player(target_player_id) => {
                        self.selected =
                            Selection::WishPickerInventoryViewer(player_id, ix, target_player_id);
                    }
                    _ => panic!("Invalid selection type returned from `world_selection`"),
                },
                Selection::WishPickerInventoryViewer(player_id, _ix, target_player_id) => {
                    match inventory_selection(pt, ctx, target_player_id) {
                        Selection::Inventory(_, ix_option) => for ix in ix_option {
                            let frame = self.history.get_focus_val_mut();
                            let target_player = frame
                                .players
                                .by_id
                                .get(&target_player_id)
                                .expect("Selection target player id invalid");
                            if let Some(cell) = target_player.inventory.cells()[ix].as_ref() {
                                let item = cell.item.clone();
                                let player = frame
                                    .players
                                    .by_id
                                    .get_mut(&player_id)
                                    .expect("Selection player id invalid");
                                match player.inventory {
                                    Inventory::Actual(_) => panic!("Wishing from actual inventory"),
                                    Inventory::Hypothetical(ref mut hypothetical) => {
                                        match hypothetical.wish(item, ix) {
                                            Ok(()) => {
                                                self.selected =
                                                    Selection::Inventory(player_id, Some(ix))
                                            }
                                            Err(err) => println!("{}", err),
                                        }
                                    }
                                }
                            }
                        },
                        _ => panic!("Invalid selection type returned from `inventory_selection`"),
                    }
                }
                _ => {
                    self.selected = world_selection(pt, ctx, self);
                }
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
            Selection::Inventory(player_id, Some(ix)) => match key {
                Keycode::T => {
                    self.current_plan
                        .cow(&self.history.focus.children)
                        .moves
                        .insert(player_id, Move::Drop(ix));
                }
                Keycode::Equals => {
                    let player = self.history
                        .get_focus_val_mut()
                        .players
                        .by_id
                        .get_mut(&player_id)
                        .expect("Invalid player selection");
                    if let Inventory::Hypothetical(ref mut hypothetical) = player.inventory {
                        match hypothetical.cells[ix] {
                            Some(ref cell) => hypothetical
                                .wish(cell.item.clone(), ix)
                                .unwrap_or_else(|err| println!("{}", err)),
                            None => self.selected = Selection::WishPicker(player_id, ix),
                        }
                    }
                }
                Keycode::Minus => {
                    let player = self.history
                        .get_focus_val_mut()
                        .players
                        .by_id
                        .get_mut(&player_id)
                        .expect("Invalid player selection");
                    if let Inventory::Hypothetical(ref mut hypothetical) = player.inventory {
                        hypothetical
                            .unwish(ix)
                            .unwrap_or_else(|err| println!("{}", err));
                    }
                }
                _ => {}
            },
            Selection::Inventory(_, None) => {}
            Selection::Top
            | Selection::WishPicker(_, _)
            | Selection::WishPickerInventoryViewer(_, _, _) => {}
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
            Selection::Player(player_id) | Selection::WishPicker(player_id, _) => {
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
                let inventory = &self.history
                    .get_focus_val()
                    .players
                    .by_id
                    .get(&player_id)
                    .expect("Invalid inventory player")
                    .inventory;
                render_inventory(inventory, ctx, &self.image_map, selected_item_option)?;
            }
            Selection::WishPickerInventoryViewer(_player_id, _ix, target_player_id) => {
                let inventory = &self.history
                    .get_focus_val()
                    .players
                    .by_id
                    .get(&target_player_id)
                    .expect("Invalid inventory player")
                    .inventory;
                render_inventory(inventory, ctx, &self.image_map, &None)?;
            }
        }
        graphics::present(ctx);
        Ok(())
    }
}
