#![feature(nll)]

extern crate ggez;
#[cfg(test)]
#[macro_use]
pub extern crate proptest;
extern crate game_frame;
extern crate petgraph;
extern crate rand;
extern crate render;
extern crate tree;
extern crate types;

use ggez::graphics::Point2;
use ggez::{event, graphics};
use graphics::Drawable;

use std::f32::consts::PI;

use ggez::nalgebra;
use ggez::nalgebra::{Similarity2, Vector2};
use petgraph::dot::{Config, Dot};

use game_frame::*;
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

    pub fn rotate_plan(&mut self) -> Result<(), GameError> {
        match self.history.focus.children.len() {
            0 => Err("No future recorded: can't cycle plans")?,
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
                if let Some(player_id) = self.history.get_focus_val().players.id_by_position(&pt) {
                    self.selected = Selection::Player(player_id);
                }
            }
            Selection::WishPicker(player_id, _)
            | Selection::Inventory(player_id, _)
            | Selection::Player(player_id) => {
                if !self.history.get_focus_val().players.contains_id(&player_id) {
                    self.selected = Selection::Top;
                }
            }
            Selection::WishPickerInventoryViewer(player_id, ix, target_player_id) => {
                let players = &self.history.get_focus_val().players;
                if !players.contains_id(&player_id) {
                    self.selected = Selection::Top;
                }
                if !players.contains_id(&target_player_id) {
                    self.selected = Selection::WishPicker(player_id, ix);
                }
            }
        }
    }
    fn left_click_event(&mut self, ctx: &mut ggez::Context, pt: Point2) -> Result<(), GameError> {
        match self.selected {
            Selection::Inventory(player_id, _) => {
                self.selected = inventory_selection(pt, ctx, player_id);
                Ok(())
            }
            Selection::WishPicker(player_id, ix) => match world_selection(pt, ctx, self) {
                Selection::GridCell(tile_pt) => {
                    let frame = self.history.get_focus_val_mut();
                    let selection = &mut self.selected;
                    if let Some(item_drop) = frame.items.get_by_position(&tile_pt) {
                        frame.players.mutate_by_id(
                            &player_id,
                            |mut player| -> Result<Player, GameError> {
                                match player.inventory {
                                    Inventory::Actual(_) => panic!("Wishing from actual inventory"),
                                    Inventory::Hypothetical(ref mut hypothetical) => {
                                        hypothetical.wish(item_drop.item.clone(), ix)?;
                                        *selection = Selection::Inventory(player_id, Some(ix));
                                        Ok(player)
                                    }
                                }
                            },
                        )
                    } else {
                        Ok(())
                    }
                }
                Selection::Player(target_player_id) => {
                    self.selected =
                        Selection::WishPickerInventoryViewer(player_id, ix, target_player_id);
                    Ok(())
                }
                _ => panic!("Invalid selection type returned from `world_selection`"),
            },
            Selection::WishPickerInventoryViewer(player_id, _ix, target_player_id) => {
                match inventory_selection(pt, ctx, target_player_id) {
                    Selection::Inventory(_, None) => Ok(()),
                    Selection::Inventory(_, Some(ix)) => {
                        let frame = self.history.get_focus_val_mut();
                        let selection = &mut self.selected;
                        let target_player = frame
                            .players
                            .get_by_id(&target_player_id)
                            .expect("Selection target player id invalid");
                        if let Some(cell) = target_player.inventory.cells()[ix].as_ref() {
                            let item = cell.item.clone();
                            frame.players.mutate_by_id(
                                &player_id,
                                |mut player| -> Result<Player, GameError> {
                                    match player.inventory {
                                        Inventory::Actual(_) => {
                                            panic!("Wishing from actual inventory")
                                        }
                                        Inventory::Hypothetical(ref mut hypothetical) => {
                                            hypothetical.wish(item, ix)?;
                                            *selection = Selection::Inventory(player_id, Some(ix));
                                            Ok(player)
                                        }
                                    }
                                },
                            )
                        } else {
                            Ok(())
                        }
                    }
                    _ => panic!("Invalid selection type returned from `inventory_selection`"),
                }
            }
            _ => {
                self.selected = world_selection(pt, ctx, self);
                Ok(())
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
        .id_by_position(&world_space_pt);
    match player {
        Some(id) => Selection::Player(id),
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
        let pt = Point2::new(x as f32, y as f32);
        let result = match button {
            event::MouseButton::Left => self.left_click_event(ctx, pt),
            _ => Ok(()),
        };
        if let Err(msg) = result {
            println!("{}", msg)
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
                    if self
                        .current_plan
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
                    let selection = &mut self.selected;
                    let wish_result = self.history.get_focus_val_mut().players.mutate_by_id(
                        &player_id,
                        |mut player| -> Result<Player, GameError> {
                            if let Inventory::Hypothetical(ref mut hypothetical) = player.inventory
                            {
                                match hypothetical.cells[ix] {
                                    Some(ref cell) => hypothetical.wish(cell.item.clone(), ix)?,
                                    None => *selection = Selection::WishPicker(player_id, ix),
                                }
                            }
                            Ok(player)
                        },
                    );
                    if let Err(err) = wish_result {
                        println!("{}", err)
                    }
                }
                Keycode::Minus => {
                    let wish_result = self.history.get_focus_val_mut().players.mutate_by_id(
                        &player_id,
                        |mut player| -> Result<Player, GameError> {
                            if let Inventory::Hypothetical(ref mut hypothetical) = player.inventory
                            {
                                hypothetical.unwish(ix)?
                            }
                            Ok(player)
                        },
                    );
                    if let Err(err) = wish_result {
                        println!("{}", err)
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
            Keycode::Tab => {
                if let Err(err) = self.rotate_plan() {
                    println!("{}", err);
                }
            }
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
                    for (item_type, item_portal_graph) in new_frame.item_portal_graphs.iter() {
                        println!("{:?}", item_type);
                        println!(
                            "{:?}",
                            Dot::with_config(&item_portal_graph, &[Config::EdgeNoLabel])
                        );
                    }
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
        for (_, player) in self.history.get_focus_val().players.iter() {
            self.image_map.player.draw(
                ctx,
                transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(player.position),
                0.,
            )?;
            for mv in self
                .current_plan
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
        for (_, portal) in self.history.get_focus_val().portals.iter() {
            self.image_map.portal.draw(
                ctx,
                transform
                    * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(portal.player_position),
                0.,
            )?;
        }
        for (_, item_drop) in self.history.get_focus_val().items.iter() {
            item_drop.item.image(&self.image_map).draw(
                ctx,
                transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(item_drop.position),
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
                let player = self
                    .history
                    .get_focus_val()
                    .players
                    .get_by_id(&player_id)
                    .expect("Invalid player selection");
                self.image_map.selection.draw(
                    ctx,
                    transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(player.position),
                    0.,
                )?;
            }
            Selection::Inventory(player_id, ref selected_item_option) => {
                let inventory = &self
                    .history
                    .get_focus_val()
                    .players
                    .get_by_id(&player_id)
                    .expect("Invalid inventory player")
                    .inventory;
                render_inventory(inventory, ctx, &self.image_map, selected_item_option)?;
            }
            Selection::WishPickerInventoryViewer(_player_id, _ix, target_player_id) => {
                let inventory = &self
                    .history
                    .get_focus_val()
                    .players
                    .get_by_id(&target_player_id)
                    .expect("Invalid inventory player")
                    .inventory;
                render_inventory(inventory, ctx, &self.image_map, &None)?;
            }
        }
        graphics::present(ctx);
        Ok(())
    }
}
