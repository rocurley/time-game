//! A simple example that demonstrates using conrod within a basic `winit` window loop, using
//! `glium` to render the `conrod::render::Primitives` to screen.

#[macro_use]
extern crate conrod;
extern crate image;
extern crate types;

use conrod::backend::glium::glium::{self, Surface};
use types::{GameState, Player, Move, Direction, Selection, Plan, ImageIds};
use image::imageops;

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

    const JUMP_PATH: &'static str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/images/jump.png");
    const ARROW_PATH: &'static str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/images/arrow.png");

    // The image map describing each of our widget->image mappings (in our case, none).
    let mut image_map = conrod::image::Map::<glium::texture::SrgbTexture2d>::new();
    let image_ids = ImageIds {
        jump_icon : image_map.insert(load_image(&display, JUMP_PATH, None)),
        move_arrows : [ image_map.insert(load_image(&display, ARROW_PATH, Some(Direction::Up))),
                        image_map.insert(load_image(&display, ARROW_PATH, Some(Direction::Left))),
                        image_map.insert(load_image(&display, ARROW_PATH, Some(Direction::Down))),
                        image_map.insert(load_image(&display, ARROW_PATH, Some(Direction::Right))),
                      ]
                        
    };

    let mut game_state = GameState::new(& mut ui.widget_id_generator());
    game_state.current_frame.players.push(Player::new(ui.widget_id_generator(), (0,4)));

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
            for event in ui_cell.global_input().events().ui() {
                use conrod::event;
                if let & event::Ui::Press(_, event::Press{button : event::Button::Keyboard(key), ..}) = event {
                    match game_state.selected {
                        Some(Selection::Player(player_id)) => {
                            enum Update {
                                SetMove(Move),
                                ClearMove,
                            }
                            let update_option = match key {
                                conrod::input::Key::W => Some(Update::SetMove(Move::Direction(Direction::Up))),
                                conrod::input::Key::A => Some(Update::SetMove(Move::Direction(Direction::Left))),
                                conrod::input::Key::S => Some(Update::SetMove(Move::Direction(Direction::Down))),
                                conrod::input::Key::D => Some(Update::SetMove(Move::Direction(Direction::Right))),
                                conrod::input::Key::Q => Some(Update::SetMove(Move::Jump)),
                                conrod::input::Key::Space => Some(Update::ClearMove),
                                _ => None,
                            };
                            for update in update_option {
                                match update {
                                    Update::SetMove(new_move) =>
                                        game_state.current_plan.moves.insert(player_id, new_move),
                                    Update::ClearMove => game_state.current_plan.moves.remove(& player_id),
                                };
                            }
                        },
                        Some(Selection::GridCell(pt)) => {
                            if let conrod::input::Key::Q = key {
                                if game_state.current_plan.portals.contains(& pt) {
                                    game_state.current_plan.portals.remove(& pt);
                                } else {
                                    game_state.current_plan.portals.insert(pt);
                                }
                            }
                        },
                        None => {},
                    }
                    if let conrod::input::Key::Return = key {
                        match logic::apply_plan(& game_state.current_frame, & game_state.current_plan) {
                            Err(err) => println!("{}", err),
                            Ok(new_frame) => {
                                game_state.current_frame = new_frame;
                                game_state.current_plan = Plan::new();
                            }
                        }
                    }
                }
            }
            if game_state.render(ui_cell, & image_ids){
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

fn load_image<P>(display: &glium::Display, path: P, angle : Option<Direction>) -> glium::texture::SrgbTexture2d
    where P: AsRef<std::path::Path>,
{
    let path = path.as_ref();
    let mut rgba_image = image::open(&std::path::Path::new(&path)).unwrap().to_rgba();
    rgba_image = match angle {
        None => rgba_image,
        Some(Direction::Up) => rgba_image,
        Some(Direction::Right) => imageops::rotate90(& rgba_image),
        Some(Direction::Down) => imageops::rotate180(& rgba_image),
        Some(Direction::Left) => imageops::rotate270(& rgba_image),
    };
    let image_dimensions = rgba_image.dimensions();
    let raw_image = glium::texture::RawImage2d::from_raw_rgba_reversed(&rgba_image.into_raw(), image_dimensions);
    let texture = glium::texture::SrgbTexture2d::new(display, raw_image).unwrap();
    texture
}

