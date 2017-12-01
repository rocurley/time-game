//! A simple example that demonstrates using conrod within a basic `winit` window loop, using
//! `glium` to render the `conrod::render::Primitives` to screen.

#[macro_use]
extern crate conrod;
extern crate types;

use conrod::backend::glium::glium::{self, Surface};
use types::{GameState, Player, Move};

mod support;
mod logic;


fn main() {
    const WIDTH: u32 = 600;
    const HEIGHT: u32 = 600;

    // Build the window.
    let mut events_loop = glium::glutin::EventsLoop::new();
    let window = glium::glutin::WindowBuilder::new()
        .with_title("Hello Conrod!")
        .with_dimensions(WIDTH, HEIGHT);
    let context = glium::glutin::ContextBuilder::new()
        .with_vsync(true)
        .with_multisampling(4);
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    // construct our `Ui`.
    let mut ui = conrod::UiBuilder::new([WIDTH as f64, HEIGHT as f64]).build();

    // Generate the widget identifiers.
    widget_ids!(struct Ids { button, button_matrix });
    let ids = Ids::new(ui.widget_id_generator());

    // Add a `Font` to the `Ui`'s `font::Map` from file.
    const FONT_PATH: &'static str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/NotoSans/NotoSans-Regular.ttf");
    ui.fonts.insert_from_file(FONT_PATH).expect(FONT_PATH);

    // A type used for converting `conrod::render::Primitives` into `Command`s that can be used
    // for drawing to the glium `Surface`.
    let mut renderer = conrod::backend::glium::Renderer::new(&display).unwrap();

    // The image map describing each of our widget->image mappings (in our case, none).
    let image_map = conrod::image::Map::<glium::texture::Texture2d>::new();

    let mut game_state = GameState::new(& mut ui.widget_id_generator());
    game_state.current_frame.players.push(Player::new(ui.widget_id_generator(), (0,4)));
    game_state.current_plan.moves.insert(game_state.current_frame.players[0].get_id(), Move::Down);

    let mut main_loop = support::EventLoop::new();
    'main: loop {
        for event in main_loop.next(&mut events_loop) {
            // Break from the loop upon `Escape` or closed window.
            match & event {
                & glium::glutin::Event::WindowEvent { ref event, .. } => {
                    match event {
                        & glium::glutin::WindowEvent::Closed |
                        & glium::glutin::WindowEvent::KeyboardInput {
                            input: glium::glutin::KeyboardInput {
                                virtual_keycode: Some(glium::glutin::VirtualKeyCode::Escape),
                                ..
                            },
                            ..
                        } => break 'main,
                        _ => (),
                    }
                }
                _ => (),
            };

            // Use the `winit` backend feature to convert the winit event to a conrod input.
            let input = match conrod::backend::winit::convert_event(event, &display) {
                None => continue,
                Some(input) => input,
            };

            // Handle the input with the `Ui`.
            ui.handle_event(input);
        }
        {
            // Set the widgets.
            let ui_cell = &mut ui.set_widgets();
            if game_state.render(ui_cell){
                main_loop.update();
            }
        }

        // Draw the `Ui` if it has changed.
        if let Some(primitives) = ui.draw_if_changed() {
            renderer.fill(&display, primitives, &image_map);
            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 1.0);
            renderer.draw(&display, &mut target, &image_map).unwrap();
            target.finish().unwrap();
        };
    }
}

