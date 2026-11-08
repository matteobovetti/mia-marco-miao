mod audio;
mod brushes;
mod cat;
mod dispenser;
mod game;
mod home;
mod naming;
mod sofa;
mod supplies;
mod synth;
mod toys;
mod ui;
mod view;
mod window;
mod world;

use macroquad::prelude::*;

use audio::Audio;
use game::Game;
use naming::Naming;

/// Pass `--fullscreen` to start the game full screen.
fn wants_fullscreen() -> bool {
    std::env::args().any(|arg| arg == "--fullscreen")
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Cat Life".to_owned(),
        window_width: world::WIDTH as i32,
        window_height: world::HEIGHT as i32,
        // Resizable also lets macOS take the window full screen.
        window_resizable: true,
        fullscreen: wants_fullscreen(),
        high_dpi: true,
        ..Default::default()
    }
}

enum Screen {
    /// Choosing the cat's name.
    Naming(Naming),
    Playing(Game),
}

#[macroquad::main(window_conf)]
async fn main() {
    macroquad::rand::srand(macroquad::miniquad::date::now() as u64);
    view::started_fullscreen(wants_fullscreen());

    let mut screen = Screen::Naming(Naming::new());

    loop {
        // Cap dt so dragging the window or a hitch doesn't teleport the cat.
        let dt = get_frame_time().min(0.1);

        // While naming the cat, "F" is just a letter.
        view::handle_fullscreen_keys(matches!(screen, Screen::Playing(_)));
        view::begin();

        screen = match screen {
            Screen::Naming(mut naming) => {
                let chosen = naming.update(dt);
                naming.draw();
                match chosen {
                    // Sounds are loaded once, when the game starts.
                    Some(names) => Screen::Playing(Game::new(names, Audio::load().await)),
                    None => Screen::Naming(naming),
                }
            }
            Screen::Playing(mut game) => {
                game.update(dt);
                game.draw();
                Screen::Playing(game)
            }
        };

        view::end();
        next_frame().await;
    }
}
