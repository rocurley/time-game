use std::default::Default;

use super::ggez::graphics;
use graphics::{DrawParam, Drawable, Mesh};

use self::nalgebra::{Similarity2, Vector2};
use super::ggez::nalgebra;

use super::types::*;

use enum_map::EnumMap;

type Point2 = ggez::nalgebra::Point2<f32>;
pub struct DrawBuffer {
    buffer: EnumMap<Layer, Vec<(DrawRef, DrawParam)>>,
    transform: Similarity2<f32>,
}
impl DrawBuffer {
    pub fn new(ctx: &ggez::Context) -> Self {
        let graphics::Rect { x: x0, y: y0, .. } = graphics::screen_coordinates(ctx);
        let transform: Similarity2<f32> = Similarity2::new(Vector2::new(x0, y0), 0., SCALE);
        DrawBuffer {
            buffer: EnumMap::new(),
            transform,
        }
    }
    pub fn push_with_param(&mut self, draw_layer: DrawLayer, param: DrawParam) {
        self.buffer[draw_layer.layer].push((draw_layer.draw_ref, param));
    }
    pub fn push(&mut self, draw_layer: DrawLayer, pt: Point) {
        let dest = self.tile_space_to_pixel_space(pt);
        self.push_with_param(draw_layer, DrawParam::new().dest(dest));
    }
    pub fn push_rotated(&mut self, draw_layer: DrawLayer, pt: Point, rotation: f32) {
        let dest = self.transform
            * (nalgebra::convert::<nalgebra::Point2<i32>, Point2>(pt) + Vector2::new(0.5, 0.5));
        let param = DrawParam::new()
            .dest(dest)
            .offset([0.5, 0.5])
            .rotation(rotation);
        self.push_with_param(draw_layer, param);
    }
    pub fn draw(self, ctx: &mut ggez::Context) -> ggez::GameResult<()> {
        for (_, layer) in self.buffer {
            for (image, param) in layer {
                image.draw(ctx, param)?;
            }
        }
        Ok(())
    }
    pub fn tile_space_to_pixel_space(&self, pt: Point) -> Point2 {
        self.transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(pt)
    }
}

pub fn draw_grid(
    ctx: &mut ggez::Context,
    bounds: graphics::Rect,
    color: graphics::Color,
) -> ggez::GameResult<()> {
    let mut x = bounds.x;
    let mut y = bounds.y;
    while x <= bounds.x + bounds.w {
        Mesh::new_line(
            ctx,
            &[[x, bounds.y - 2.5], [x, bounds.y + bounds.h + 2.5]],
            5.,
            color,
        )?
        .draw(ctx, DrawParam::default())?;
        x += SCALE;
    }
    while y <= bounds.y + bounds.h {
        Mesh::new_line(
            ctx,
            &[[bounds.x - 2.5, y], [bounds.x + bounds.w + 2.5, y]],
            5.,
            color,
        )?
        .draw(ctx, DrawParam::default())?;
        y += SCALE;
    }
    Ok(())
}

pub fn draw_map_grid(ctx: &mut ggez::Context, color: graphics::Color) -> ggez::GameResult<()> {
    let bounds = graphics::screen_coordinates(ctx);
    draw_grid(ctx, bounds, color)
}

pub fn inventory_bbox(ctx: &ggez::Context) -> graphics::Rect {
    let screen_bounds = graphics::screen_coordinates(ctx);
    let w = INVENTORY_WIDTH as f32 * SCALE;
    let h = INVENTORY_HEIGHT as f32 * SCALE;
    graphics::Rect {
        x: screen_bounds.x + (screen_bounds.w - w) / 2.,
        y: screen_bounds.y + (screen_bounds.h - h) / 2.,
        w,
        h,
    }
}

pub fn pixel_space_to_tile_space(pt: Point2, bbox: ggez::graphics::Rect) -> Option<Point> {
    if !bbox.contains(pt) {
        return None;
    }
    let graphics::Rect { x: x0, y: y0, .. } = bbox;
    let inv_transform: Similarity2<f32> =
        Similarity2::new(Vector2::new(x0, y0), 0., SCALE).inverse();
    Some(nalgebra::try_convert::<Point2, nalgebra::Point2<i32>>(inv_transform * pt).unwrap())
}

pub fn tile_space_to_pixel_space(pt: Point, bbox: graphics::Rect) -> Point2 {
    let graphics::Rect { x: x0, y: y0, .. } = bbox;
    let transform: Similarity2<f32> = Similarity2::new(Vector2::new(x0, y0), 0., SCALE);
    transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(pt)
}

pub fn render_inventory(
    inventory: &Inventory,
    buffer: &mut DrawBuffer,
    image_map: &ImageMap,
    selected_item_option: &Option<usize>,
) -> ggez::GameResult<()> {
    let bounds = inventory_bbox(ctx);
    let background = match *inventory {
        Inventory::Actual(_) => graphics::Color::from_rgb(127, 127, 127),
        Inventory::Hypothetical(_) => graphics::Color::from_rgb(127, 127, 255),
    };
    Mesh::new_rectangle(
        ctx,
        graphics::DrawMode::Fill(Default::default()),
        bounds,
        background,
    )?
    .draw(ctx, DrawParam::new())?;
    draw_grid(ctx, bounds, (0, 0, 0).into())?;
    for (i, inventory_cell_option) in inventory.cells().iter().enumerate() {
        for inventory_cell in inventory_cell_option.iter() {
            let tile_space_pt = Point::new(
                i as i32 % INVENTORY_WIDTH as i32,
                i as i32 / INVENTORY_WIDTH as i32,
            );
            let pixel_space_pt = tile_space_to_pixel_space(tile_space_pt, bounds);
            inventory_cell
                .item
                .image(image_map)
                .draw(ctx, DrawParam::new().dest(pixel_space_pt))?;
            let text = graphics::Text::new(inventory_cell.count.to_string());
            text.draw(
                ctx,
                DrawParam::new().dest(pixel_space_pt + Vector2::new(5., 5.)),
            )?;
        }
    }
    for &i in selected_item_option {
        let tile_space_pt = Point::new(
            i as i32 % INVENTORY_WIDTH as i32,
            i as i32 / INVENTORY_WIDTH as i32,
        );
        let pixel_space_pt = tile_space_to_pixel_space(tile_space_pt, bounds);
        image_map
            .selection
            .draw(ctx, DrawParam::new().dest(pixel_space_pt))?;
    }
    Ok(())
}

pub fn ecs(ecs: &ECS, buffer: &mut DrawBuffer) {
    for (entity, (layer, image)) in ecs.images.iter() {
        if !ecs.entities.contains_key(entity) {
            continue;
        }
        let pt = match ecs.positions.get(entity) {
            Some(pt) => *pt,
            None => continue,
        };
        buffer.push(image, pt);
    }
}
