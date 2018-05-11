#![feature(nll)]

extern crate ggez;
extern crate nalgebra;
extern crate types;

use ggez::graphics;
use graphics::Drawable;
use graphics::Point2;

use nalgebra::{Similarity2, Vector2};

use types::*;

pub fn draw_grid(ctx: &mut ggez::Context, bounds: graphics::Rect) -> ggez::GameResult<()> {
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

pub fn draw_map_grid(ctx: &mut ggez::Context) -> ggez::GameResult<()> {
    let bounds = graphics::get_screen_coordinates(ctx);
    draw_grid(ctx, bounds)
}

pub fn inventory_bbox(ctx: &ggez::Context) -> graphics::Rect {
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
    ctx: &mut ggez::Context,
    image_map: &ImageMap,
    selected_item_option: &Option<usize>,
) -> ggez::GameResult<()> {
    let bounds = inventory_bbox(ctx);
    let background = match inventory {
        &Inventory::Actual(_) => graphics::Color::from_rgb(127, 127, 127),
        &Inventory::Hypothetical(_) => graphics::Color::from_rgb(127, 127, 255),
    };
    graphics::set_color(ctx, background)?;
    graphics::rectangle(ctx, graphics::DrawMode::Fill, bounds)?;
    graphics::set_color(ctx, graphics::Color::from_rgb(0, 0, 0))?;
    draw_grid(ctx, bounds)?;
    graphics::set_color(ctx, graphics::Color::from_rgb(255, 255, 255))?;
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
                .draw(ctx, pixel_space_pt, 0.)?;
            let font = ctx.default_font.clone();
            let text = graphics::Text::new(ctx, &inventory_cell.count.to_string(), &font)?;
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
        image_map.selection.draw(ctx, pixel_space_pt, 0.)?;
    }
    Ok(())
}
