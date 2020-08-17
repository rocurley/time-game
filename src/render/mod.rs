use std::default::Default;

use super::ggez::graphics;
use graphics::{DrawParam, Drawable, Mesh};

use self::nalgebra::{Similarity2, Vector2};
use super::ggez::nalgebra;
extern crate alga;
use alga::general::SubsetOf;

use super::types::*;

use enum_map::EnumMap;

struct BufferedDraw {
    draw_ref: TGDrawable,
    position: Point2,
    param: DrawParam,
}

impl BufferedDraw {
    fn draw(self, ctx: &mut ggez::Context, transform: Similarity2<f32>) -> ggez::GameResult<()> {
        let dest = transform * self.position;
        self.draw_ref.draw(ctx, self.param.dest(dest))
    }
}

type Point2 = ggez::nalgebra::Point2<f32>;
pub struct DrawBuffer {
    buffer: EnumMap<Layer, Vec<BufferedDraw>>,
    world_transform: Similarity2<f32>,
}
impl DrawBuffer {
    pub fn new(ctx: &ggez::Context) -> Self {
        let graphics::Rect { x: x0, y: y0, .. } = graphics::screen_coordinates(ctx);
        let world_transform: Similarity2<f32> = Similarity2::new(Vector2::new(x0, y0), 0., SCALE);
        DrawBuffer {
            buffer: EnumMap::new(),
            world_transform,
        }
    }
    pub fn push_with_param<P: SubsetOf<Point2>>(
        &mut self,
        draw_layer: DrawLayer,
        param: DrawParam,
        pt: P,
    ) {
        let DrawLayer { draw_ref, layer } = draw_layer;
        let position = nalgebra::convert::<P, Point2>(pt);
        let draw = BufferedDraw {
            draw_ref,
            position,
            param,
        };
        self.buffer[layer].push(draw);
    }
    pub fn push<P: SubsetOf<Point2>>(&mut self, draw_layer: DrawLayer, pt: P) {
        let DrawLayer { draw_ref, layer } = draw_layer;
        let position = nalgebra::convert::<P, Point2>(pt);
        let draw = BufferedDraw {
            draw_ref,
            position,
            param: DrawParam::new(),
        };
        self.buffer[layer].push(draw);
    }
    pub fn push_rotated<P: SubsetOf<Point2>>(
        &mut self,
        draw_layer: DrawLayer,
        pt: P,
        rotation: f32,
    ) {
        let position = nalgebra::convert::<P, Point2>(pt) + Vector2::new(0.5, 0.5);
        let param = DrawParam::new().offset([0.5, 0.5]).rotation(rotation);
        self.push_with_param(draw_layer, param, position);
    }
    pub fn draw(self, ctx: &mut ggez::Context) -> ggez::GameResult<()> {
        for (layer, images) in self.buffer {
            let transform = match layer {
                Background => self.world_transform,
                Foreground => self.world_transform,
                UI => Similarity2::identity(),
            };
            for BufferedDraw {
                draw_ref,
                position,
                param,
            } in images
            {
                draw_ref.draw(ctx, param.dest(transform * position))?;
            }
        }
        Ok(())
    }
    pub fn tile_space_to_pixel_space(&self, layer: Layer, pt: Point) -> Point2 {
        let transform = match layer {
            Background => self.world_transform,
            Foreground => self.world_transform,
            UI => Similarity2::identity(),
        };
        transform * nalgebra::convert::<nalgebra::Point2<i32>, Point2>(pt)
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

pub fn tile_space_to_pixel_space<P: SubsetOf<Point2>>(pt: P, bbox: graphics::Rect) -> Point2 {
    let graphics::Rect { x: x0, y: y0, .. } = bbox;
    let world_transform: Similarity2<f32> = Similarity2::new(Vector2::new(x0, y0), 0., SCALE);
    world_transform * nalgebra::convert::<P, Point2>(pt)
}

pub fn render_inventory(
    inventory: &Inventory,
    buffer: &mut DrawBuffer,
    image_map: &ImageMap,
    selected_item_option: &Option<usize>,
) -> ggez::GameResult<()> {
    /*
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
    */
    for (i, inventory_cell_option) in inventory.cells().iter().enumerate() {
        for inventory_cell in inventory_cell_option.iter() {
            let tile_space_pt = Point::new(
                i as i32 % INVENTORY_WIDTH as i32,
                i as i32 / INVENTORY_WIDTH as i32,
            );
            let mut draw_ref = inventory_cell.item.image(image_map);
            draw_ref.layer = Layer::Inventory;
            buffer.push(draw_ref, tile_space_pt);
            let text = graphics::Text::new(inventory_cell.count.to_string());
            buffer.push(
                DrawLayer {
                    layer: Layer::Inventory,
                    draw_ref: text,
                },
                tile_space_pt + Vector2::new(0.2, 0.2),
            );
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
