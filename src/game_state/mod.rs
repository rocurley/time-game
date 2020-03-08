use ggez::{
    event,
    graphics::{self, Color, DrawParam, Drawable},
};
use std::f32::consts::PI;

use ggez::nalgebra::{self, Similarity2, Vector2};

use crate::{game_frame::*, types::*};

use super::tree;
use crate::{
    portal_graph::render_item_graph,
    render::{self, draw_map_grid, inventory_bbox, pixel_space_to_tile_space, render_inventory},
};

mod planning;
type Point2 = ggez::nalgebra::Point2<f32>;

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
                        let wished_item = item_drop.item.clone();
                        frame.wish(player_id, ix, Some(wished_item));
                        *selection = Selection::Inventory(player_id, Some(ix));
                    }
                    Ok(())
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
                            let mut player = frame
                                .players
                                .get_mut_by_id(player_id)
                                .expect("Couldn't find player by id");
                            match player.inventory {
                                Inventory::Actual(_) => panic!("Wishing from actual inventory"),
                                Inventory::Hypothetical(ref mut hypothetical) => {
                                    hypothetical.wish(item, ix)?;
                                    *selection = Selection::Inventory(player_id, Some(ix));
                                }
                            }
                        }
                        Ok(())
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
    let world_space_pt: Point = pixel_space_to_tile_space(pt, graphics::screen_coordinates(ctx))
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
        x: f32,
        y: f32,
    ) {
        let pt = [x, y].into();
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
        key: event::KeyCode,
        _keymods: event::KeyMods,
        _repeat: bool,
    ) {
        use self::event::KeyCode;
        match self.selected {
            Selection::Player(player_id) => {
                enum Update {
                    Move(Move),
                    Other(KeyCode),
                }
                let update = match key {
                    KeyCode::W => Update::Move(Move::Direction(Direction::Up)),
                    KeyCode::A => Update::Move(Move::Direction(Direction::Left)),
                    KeyCode::S => Update::Move(Move::Direction(Direction::Down)),
                    KeyCode::D => Update::Move(Move::Direction(Direction::Right)),
                    KeyCode::Q => Update::Move(Move::Jump),
                    KeyCode::G => Update::Move(Move::PickUp),
                    keycode => Update::Other(keycode),
                };
                match update {
                    Update::Move(new_move) => {
                        self.current_plan
                            .cow(&self.history.focus.children)
                            .moves
                            .insert(player_id, new_move);
                    }
                    Update::Other(KeyCode::Space) => {
                        self.current_plan
                            .cow(&self.history.focus.children)
                            .moves
                            .remove(&player_id);
                    }
                    Update::Other(KeyCode::I) => {
                        self.selected = Selection::Inventory(player_id, None);
                    }
                    _ => {}
                }
            }
            Selection::GridCell(pt) => {
                if let KeyCode::Q = key {
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
                KeyCode::T => {
                    self.current_plan
                        .cow(&self.history.focus.children)
                        .moves
                        .insert(player_id, Move::Drop(ix));
                }
                KeyCode::Equals => {
                    let game_frame = self.history.get_focus_val_mut();
                    let wish_result = game_frame.wish(player_id, ix, None);
                    if let FrameWishResult::NoItem = wish_result {
                        self.selected = Selection::WishPicker(player_id, ix);
                    }
                }
                KeyCode::Minus => {
                    let game_frame = self.history.get_focus_val_mut();
                    if let Err(err) = game_frame.unwish(player_id, ix) {
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
            KeyCode::Tab => {
                if let Err(err) = self.rotate_plan() {
                    println!("{}", err);
                }
            }
            KeyCode::Back => match self.history.up() {
                Ok(ix) => {
                    self.current_plan = CachablePlan::Old(ix);
                }
                Err(err) => println!("{}", err),
            },
            KeyCode::Return => match planning::apply_plan(
                &self.history.get_focus_val(),
                &self.current_plan.get(&self.history.focus.children),
            ) {
                Err(err) => println!("{}", err),
                Ok(new_frame) => {
                    for (item_type, item_portal_graph) in new_frame.item_portal_graphs.iter() {
                        println!("{:?}", item_type);
                        render_item_graph(&item_portal_graph);
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
            KeyCode::Escape => self.selected.pop(),
            _ => {}
        }
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult<()> {
        let black: Color = (0, 0, 0).into();
        let white: Color = (255, 255, 255).into();
        let graphics::Rect { x: x0, y: y0, .. } = graphics::screen_coordinates(ctx);
        let transform: Similarity2<f32> = Similarity2::new(Vector2::new(x0, y0), 0., SCALE);
        graphics::clear(ctx, white);
        let frame = self.history.get_focus_val();
        render::ecs(ctx, &frame.ecs)?;
        draw_map_grid(ctx, black)?;
        for (_, player) in frame.players.iter() {
            self.image_map.player.draw(
                ctx,
                DrawParam::new().dest(
                    transform
                        * nalgebra::convert::<nalgebra::Point2<i32>, nalgebra::Point2<f32>>(
                            player.position,
                        ),
                ),
            )?;
            if let Some(mv) = self
                .current_plan
                .get(&self.history.focus.children)
                .moves
                .get(&player.id)
            {
                let (image, rotation) = match *mv {
                    Move::Direction(ref dir) => {
                        let angle = match *dir {
                            Direction::Up => 0.,
                            Direction::Left => 1.5 * PI,
                            Direction::Down => PI,
                            Direction::Right => 0.5 * PI,
                        };
                        (&self.image_map.move_arrow, angle)
                    }
                    Move::Jump => (&self.image_map.jump_icon, 0.),
                    Move::PickUp => (&self.image_map.pick_up_icon, 0.),
                    Move::Drop(_) => (&self.image_map.drop_icon, 0.),
                };
                let dest = transform
                    * (nalgebra::convert::<nalgebra::Point2<i32>, nalgebra::Point2<f32>>(
                        player.position,
                    ) + Vector2::new(0.5, 0.5));
                image.draw(
                    ctx,
                    DrawParam::new()
                        .dest(dest)
                        .offset([0.5, 0.5])
                        .rotation(rotation),
                )?;
            }
            for pt in &self.current_plan.get(&self.history.focus.children).portals {
                self.image_map.jump_icon.draw(
                    ctx,
                    DrawParam::new().dest(
                        transform
                            * nalgebra::convert::<nalgebra::Point2<i32>, nalgebra::Point2<f32>>(
                                *pt,
                            ),
                    ),
                )?;
            }
        }
        for (_, portal) in self.history.get_focus_val().portals.iter() {
            self.image_map.portal.draw(
                ctx,
                DrawParam::new().dest(
                    transform
                        * nalgebra::convert::<nalgebra::Point2<i32>, nalgebra::Point2<f32>>(
                            portal.player_position,
                        ),
                ),
            )?;
        }
        for (_, item_drop) in self.history.get_focus_val().items.iter() {
            item_drop.item.image(&self.image_map).draw(
                ctx,
                DrawParam::new().dest(
                    transform
                        * nalgebra::convert::<nalgebra::Point2<i32>, nalgebra::Point2<f32>>(
                            item_drop.position,
                        ),
                ),
            )?;
        }
        match self.selected {
            Selection::Top => {}
            Selection::GridCell(pt) => {
                self.image_map.selection.draw(
                    ctx,
                    DrawParam::new().dest(
                        transform
                            * nalgebra::convert::<nalgebra::Point2<i32>, nalgebra::Point2<f32>>(pt),
                    ),
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
                    DrawParam::new().dest(
                        transform
                            * nalgebra::convert::<nalgebra::Point2<i32>, nalgebra::Point2<f32>>(
                                player.position,
                            ),
                    ),
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
        graphics::present(ctx)
    }
}
