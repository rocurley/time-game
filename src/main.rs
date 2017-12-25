extern crate types;

//use types::{CachablePlan, Direction, GameState, Move, Plan, Player, Selection};
use types::{GameState, Player};

//mod support;
//mod logic;

extern crate ggez;
use ggez::*;
use nalgebra::Point2;

use std::{env, path};

pub fn main() {
    let mut cb = ContextBuilder::new("time game", "Roger")
        .window_setup(conf::WindowSetup::default().title("Time Game"))
        .window_mode(conf::WindowMode::default().dimensions(1000, 1000));

    // We add the CARGO_MANIFEST_DIR/resources to the filesystems paths so
    // we we look in the cargo project for files.
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("assets");
        println!("Adding path {:?}", path);
        // We need this re-assignment alas, see
        // https://aturon.github.io/ownership/builders.html
        // under "Consuming builders"
        cb = cb.add_resource_path(path);
    } else {
        println!("Not building from cargo?  Ok.");
    }

    //let ctx = &mut Context::load_from_conf("astroblasto", "ggez", c).unwrap();
    let ctx = &mut cb.build().unwrap();
    let game_state = &mut GameState::new(ctx).unwrap();
    game_state
        .history
        .get_focus_val_mut()
        .players
        .push(Player::new(Point2::new(0, 4)));
    event::run(ctx, game_state).unwrap();
}
/*
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

    // Add a `Font` to the `Ui`'s `font::Map` from file.
    const FONT_PATH: &'static str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/fonts/NotoSans/NotoSans-Regular.ttf"
    );
    ui.fonts.insert_from_file(FONT_PATH).expect(FONT_PATH);

    // A type used for converting `conrod::render::Primitives` into `Command`s that can be used
    // for drawing to the glium `Surface`.
    let mut renderer = conrod::backend::glium::Renderer::new(&display).unwrap();

    const JUMP_PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/images/jump.png");
    const ARROW_PATH: &'static str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/images/arrow.png");

    // The image map describing each of our widget->image mappings (in our case, none).
    let mut image_map = conrod::image::Map::<glium::texture::SrgbTexture2d>::new();
    let image_ids = ImageIds {
        jump_icon: image_map.insert(load_image(&display, JUMP_PATH, None)),
        move_arrows: [
            image_map.insert(load_image(&display, ARROW_PATH, Some(Direction::Up))),
            image_map.insert(load_image(&display, ARROW_PATH, Some(Direction::Left))),
            image_map.insert(load_image(&display, ARROW_PATH, Some(Direction::Down))),
            image_map.insert(load_image(&display, ARROW_PATH, Some(Direction::Right))),
        ],
    };

    let mut game_state = GameState::new(ui.widget_id_generator());
    game_state
        .history
        .get_focus_val_mut()
        .players
        .push(Player::new((0, 4)));

    let mut main_loop = support::EventLoop::new();
    'main: loop {
        for event in main_loop.next(&mut events_loop) {
            // Break from the loop upon `Escape` or closed window.
            match &event {
                &glium::glutin::Event::WindowEvent { ref event, .. } => match event {
                    &glium::glutin::WindowEvent::Closed
                    | &glium::glutin::WindowEvent::KeyboardInput {
                        input:
                            glium::glutin::KeyboardInput {
                                virtual_keycode: Some(glium::glutin::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => break 'main,
                    _ => (),
                },
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
                use conrod::input::Key;
                if let &event::Ui::Press(
                    _,
                    event::Press {
                        button: event::Button::Keyboard(key),
                        ..
                    },
                ) = event
                {
                    match game_state.selected {
                        Some(Selection::Player(player_id)) => {
                            enum Update {
                                SetMove(Move),
                                ClearMove,
                            }
                            let update_option = match key {
                                Key::W => Some(Update::SetMove(Move::Direction(Direction::Up))),
                                Key::A => Some(Update::SetMove(Move::Direction(Direction::Left))),
                                Key::S => Some(Update::SetMove(Move::Direction(Direction::Down))),
                                Key::D => Some(Update::SetMove(Move::Direction(Direction::Right))),
                                Key::Q => Some(Update::SetMove(Move::Jump)),
                                Key::Space => Some(Update::ClearMove),
                                _ => None,
                            };
                            for update in update_option {
                                match update {
                                    Update::SetMove(new_move) => game_state
                                        .current_plan
                                        .cow(&game_state.history.focus.children)
                                        .moves
                                        .insert(player_id, new_move),
                                    Update::ClearMove => game_state
                                        .current_plan
                                        .cow(&game_state.history.focus.children)
                                        .moves
                                        .remove(&player_id),
                                };
                            }
                        }
                        Some(Selection::GridCell(pt)) => {
                            if let Key::Q = key {
                                if game_state
                                    .current_plan
                                    .get(&game_state.history.focus.children)
                                    .portals
                                    .contains(&pt)
                                {
                                    game_state
                                        .current_plan
                                        .cow(&game_state.history.focus.children)
                                        .portals
                                        .remove(&pt);
                                } else {
                                    game_state
                                        .current_plan
                                        .cow(&game_state.history.focus.children)
                                        .portals
                                        .insert(pt);
                                }
                            }
                        }
                        None => {}
                    }
                    match key {
                        Key::Tab => if let Err(err) = game_state.rotate_plan() {
                            println!("{}", err);
                        },
                        Key::Backspace => match game_state.history.up() {
                            Ok(ix) => {
                                game_state.current_plan = CachablePlan::Old(ix);
                            }
                            Err(err) => println!("{}", err),
                        },
                        Key::Return => match logic::apply_plan(
                            &game_state.history.get_focus_val(),
                            &game_state
                                .current_plan
                                .get(&game_state.history.focus.children),
                        ) {
                            Err(err) => println!("{}", err),
                            Ok(new_frame) => match game_state.current_plan {
                                CachablePlan::Novel(ref mut plan) => {
                                    let old_plan = std::mem::replace(plan, Plan::new());
                                    game_state.history.push(new_frame, old_plan);
                                }
                                CachablePlan::Old(ix) => {
                                    game_state
                                        .history
                                        .down(ix)
                                        .expect("Cached plan wasn't there!");
                                    game_state.current_plan =
                                        match game_state.history.focus.children.len() {
                                            0 => CachablePlan::new(),
                                            l => CachablePlan::Old(l - 1),
                                        }
                                }
                            },
                        },
                        _ => {}
                    }
                }
            }
            if game_state.render(ui_cell, &image_ids) {
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
*/
