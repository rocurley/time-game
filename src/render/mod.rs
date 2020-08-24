use std::default::Default;

use super::ggez::graphics;
use graphics::{Canvas, DrawParam, Drawable, Mesh};

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

type Point2 = ggez::nalgebra::Point2<f32>;
pub struct DrawBuffer<'a> {
    ctx: &'a mut ggez::Context,
    layers: EnumMap<Layer, Canvas>,
    world_transform: Similarity2<f32>,
    inventory_transform: Similarity2<f32>,
}
fn inventory_transform(ctx: &ggez::Context) -> Similarity2<f32> {
    let graphics::Rect { x: x0, y: y0, .. } = inventory_bbox(ctx);
    Similarity2::new(Vector2::new(x0, y0), 0., SCALE)
}
impl<'a> DrawBuffer<'a> {
    pub fn new(ctx: &'a mut ggez::Context) -> ggez::GameResult<Self> {
        let graphics::Rect { x: x0, y: y0, .. } = graphics::screen_coordinates(ctx);
        let world_transform: Similarity2<f32> = Similarity2::new(Vector2::new(x0, y0), 0., SCALE);
        let inventory_transform = inventory_transform(ctx);
        // We do this nonsense because enum_map! makes a closure under the hood, and that closure
        // doesn't return a Result. Similarly, we can't, say, use array indexing because the
        // closure is called multiple times (once per enum).
        let mut layers_vec = (0..4)
            .map(|_| Canvas::with_window_size(ctx))
            .collect::<ggez::GameResult<Vec<Canvas>>>()?;
        let layers = enum_map! {
                Layer::Background => layers_vec.pop().unwrap(),
                Layer::Foreground => layers_vec.pop().unwrap(),
                Layer::UI => layers_vec.pop().unwrap(),
                Layer::Inventory => layers_vec.pop().unwrap(),
        };
        Ok(DrawBuffer {
            ctx,
            layers,
            world_transform,
            inventory_transform,
        })
    }
    fn draw(&mut self, layer: Layer, draw: BufferedDraw) -> ggez::GameResult<()> {
        let BufferedDraw {
            draw_ref,
            position,
            param,
        } = draw;
        let transform = match layer {
            Layer::Background => self.world_transform,
            Layer::Foreground => self.world_transform,
            Layer::UI => Similarity2::identity(),
            Layer::Inventory => self.inventory_transform,
        };
        graphics::set_canvas(self.ctx, Some(&self.layers[layer]));
        draw_ref.draw(self.ctx, param.dest(transform * position))?;
        graphics::set_canvas(self.ctx, None);
        Ok(())
    }
    pub fn push_with_param<P: SubsetOf<Point2>>(
        &mut self,
        draw_layer: DrawLayer,
        param: DrawParam,
        pt: P,
    ) -> ggez::GameResult<()> {
        let DrawLayer { draw_ref, layer } = draw_layer;
        let position = nalgebra::convert::<P, Point2>(pt);
        let draw = BufferedDraw {
            draw_ref,
            position,
            param,
        };
        self.draw(layer, draw)
    }
    pub fn draw_grid(
        &mut self,
        layer: Layer,
        bounds: graphics::Rect,
        color: graphics::Color,
    ) -> ggez::GameResult<()> {
        graphics::set_canvas(self.ctx, Some(&self.layers[layer]));
        let mut x = bounds.x;
        let mut y = bounds.y;
        while x <= bounds.x + bounds.w {
            Mesh::new_line(
                self.ctx,
                &[[x, bounds.y - 2.5], [x, bounds.y + bounds.h + 2.5]],
                5.,
                color,
            )?
            .draw(self.ctx, DrawParam::default())?;
            x += SCALE;
        }
        while y <= bounds.y + bounds.h {
            Mesh::new_line(
                self.ctx,
                &[[bounds.x - 2.5, y], [bounds.x + bounds.w + 2.5, y]],
                5.,
                color,
            )?
            .draw(self.ctx, DrawParam::default())?;
            y += SCALE;
        }
        graphics::set_canvas(self.ctx, None);
        Ok(())
    }
    pub fn draw_map_grid(&mut self, color: graphics::Color) -> ggez::GameResult<()> {
        let bounds = graphics::screen_coordinates(self.ctx);
        self.draw_grid(Layer::Foreground, bounds, color)
    }
    pub fn push<P: SubsetOf<Point2>>(
        &mut self,
        draw_layer: DrawLayer,
        pt: P,
    ) -> ggez::GameResult<()> {
        let DrawLayer { draw_ref, layer } = draw_layer;
        let position = nalgebra::convert::<P, Point2>(pt);
        let draw = BufferedDraw {
            draw_ref,
            position,
            param: DrawParam::new(),
        };
        self.draw(layer, draw)
    }
    pub fn push_rotated<P: SubsetOf<Point2>>(
        &mut self,
        draw_layer: DrawLayer,
        pt: P,
        rotation: f32,
    ) -> ggez::GameResult<()> {
        let position = nalgebra::convert::<P, Point2>(pt) + Vector2::new(0.5, 0.5);
        let param = DrawParam::new().offset([0.5, 0.5]).rotation(rotation);
        self.push_with_param(draw_layer, param, position)?;
        Ok(())
    }
    pub fn execute(self) -> ggez::GameResult<()> {
        for (_layer, canvas) in self.layers {
            canvas.draw(self.ctx, DrawParam::new())?;
        }
        Ok(())
    }
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

pub fn render_inventory(
    inventory: &Inventory,
    buffer: &mut DrawBuffer,
    image_map: &ImageMap,
    selected_item_option: &Option<usize>,
) -> ggez::GameResult<()> {
    let background = match *inventory {
        Inventory::Actual(_) => graphics::Color::from_rgb(127, 127, 127),
        Inventory::Hypothetical(_) => graphics::Color::from_rgb(127, 127, 255),
    };
    let bounds = ggez::graphics::Rect {
        x: 0.,
        y: 0.,
        w: INVENTORY_WIDTH as f32 * SCALE,
        h: INVENTORY_HEIGHT as f32 * SCALE,
    };
    let inventory_bg =
        Mesh::new_rectangle(buffer.ctx, graphics::DrawMode::fill(), bounds, background)?;
    buffer.push(
        DrawLayer {
            layer: Layer::Inventory,
            draw_ref: inventory_bg.into(),
        },
        Point2::new(0., 0.),
    )?;
    buffer.draw_grid(
        Layer::Inventory,
        inventory_bbox(buffer.ctx),
        (0, 0, 0).into(),
    )?;
    for (i, inventory_cell_option) in inventory.cells().iter().enumerate() {
        for inventory_cell in inventory_cell_option.iter() {
            let tile_space_pt = Point::new(
                i as i32 % INVENTORY_WIDTH as i32,
                i as i32 / INVENTORY_WIDTH as i32,
            );
            let mut draw_ref = inventory_cell.item.image(image_map);
            draw_ref.layer = Layer::Inventory;
            buffer.push(draw_ref, tile_space_pt)?;
            let text = graphics::Text::new(inventory_cell.count.to_string());
            buffer.push(
                DrawLayer {
                    layer: Layer::Inventory,
                    draw_ref: text.into(),
                },
                nalgebra::convert::<_, Point2>(tile_space_pt) + Vector2::new(0.2, 0.2),
            )?;
        }
    }
    for &i in selected_item_option {
        let tile_space_pt = Point::new(
            i as i32 % INVENTORY_WIDTH as i32,
            i as i32 / INVENTORY_WIDTH as i32,
        );
        let mut image = image_map.selection.clone();
        image.layer = Layer::Inventory;
        buffer.push(image, tile_space_pt)?;
    }
    Ok(())
}

pub fn ecs(ecs: &ECS, buffer: &mut DrawBuffer) -> ggez::GameResult<()> {
    for (entity, draw_layer) in ecs.images.iter() {
        if !ecs.entities.contains_key(entity) {
            continue;
        }
        let pt = match ecs.positions.get(entity) {
            Some(pt) => *pt,
            None => continue,
        };
        buffer.push(draw_layer.clone(), pt)?;
    }
    Ok(())
}
