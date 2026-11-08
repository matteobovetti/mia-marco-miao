//! Fitting the game's fixed-size canvas to the window, windowed or full screen.
//!
//! All the game's drawing and logic use *canvas* coordinates: `0..WIDTH` by
//! `0..HEIGHT`, with the origin at the top-left. Every frame [`begin`] finds
//! the biggest centred rectangle of the window that has the canvas's shape and
//! points a camera at it, so the same picture is shown at any window size.
//! Where the window has a different shape, black bars fill the gap.
//!
//! Two things don't follow the camera by themselves, so use these instead of
//! the Macroquad versions: [`mouse`] for the pointer, and [`draw_text`] and
//! [`text_width`] for text (which stays sharp instead of being stretched).

use std::cell::Cell;

use macroquad::prelude::*;

use crate::world::{HEIGHT, WIDTH};

const CANVAS: Vec2 = vec2(WIDTH, HEIGHT);

/// How the canvas sits in the window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    window: Vec2,
    /// Window units per canvas unit.
    scale: f32,
    /// Where the canvas's top-left corner is, in window units.
    origin: Vec2,
}

impl View {
    /// The largest centred fit of the canvas in a window of the given size.
    pub fn fit(window: Vec2) -> Self {
        // A minimised window can report a size of zero.
        let window = window.max(Vec2::ONE);
        let scale = (window.x / WIDTH).min(window.y / HEIGHT);
        Self {
            window,
            scale,
            origin: (window - CANVAS * scale) / 2.0,
        }
    }

    /// Converts a position in the window (like the mouse) to the canvas.
    pub fn to_canvas(self, window_point: Vec2) -> Vec2 {
        (window_point - self.origin) / self.scale
    }

    /// A camera that draws the canvas into its place in the window.
    fn camera(self) -> Camera2D {
        Camera2D {
            target: CANVAS / 2.0,
            zoom: 2.0 * self.scale / self.window,
            ..Camera2D::default()
        }
    }
}

thread_local! {
    static CURRENT: Cell<View> = Cell::new(View::fit(CANVAS));
    static FULLSCREEN: Cell<bool> = const { Cell::new(false) };
}

fn current() -> View {
    CURRENT.with(Cell::get)
}

/// Whether the window is full screen (as far as the game knows: Macroquad
/// can't be asked, so this tracks the requests the game itself has made).
pub fn is_fullscreen() -> bool {
    FULLSCREEN.with(Cell::get)
}

/// Records that the window was created full screen (see `Conf::fullscreen`).
pub fn started_fullscreen(fullscreen: bool) {
    FULLSCREEN.with(|state| state.set(fullscreen));
}

/// What the full-screen state should be after this frame's key presses:
/// `toggle` flips it and `escape` leaves full screen (and does nothing
/// otherwise).
fn next_fullscreen(now: bool, toggle: bool, escape: bool) -> bool {
    if toggle { !now } else { now && !escape }
}

/// Switches full screen on or off with the keys: F11 toggles it anywhere,
/// the letter F toggles it too when `letter_allowed` (not while the player is
/// typing a name), and Escape leaves it.
pub fn handle_fullscreen_keys(letter_allowed: bool) {
    let toggle = is_key_pressed(KeyCode::F11) || (letter_allowed && is_key_pressed(KeyCode::F));
    let wanted = next_fullscreen(is_fullscreen(), toggle, is_key_pressed(KeyCode::Escape));
    if wanted != is_fullscreen() {
        FULLSCREEN.with(|state| state.set(wanted));
        set_fullscreen(wanted);
    }
}

/// Starts a frame: clears the whole window to black and switches drawing to
/// canvas coordinates. Call it before reading the mouse or drawing anything.
pub fn begin() {
    let view = View::fit(vec2(screen_width(), screen_height()));
    CURRENT.with(|current| current.set(view));

    set_default_camera();
    clear_background(BLACK);
    set_camera(&view.camera());
}

/// Ends the frame by covering whatever was drawn outside the canvas (a
/// speech bubble at the edge, say) with the black bars.
pub fn end() {
    let View { window, origin, .. } = current();
    set_default_camera();
    draw_rectangle(0.0, 0.0, origin.x, window.y, BLACK);
    draw_rectangle(window.x - origin.x, 0.0, origin.x, window.y, BLACK);
    draw_rectangle(0.0, 0.0, window.x, origin.y, BLACK);
    draw_rectangle(0.0, window.y - origin.y, window.x, origin.y, BLACK);
}

/// The mouse position on the canvas. It can be outside the canvas, over the
/// black bars.
pub fn mouse() -> Vec2 {
    current().to_canvas(Vec2::from(mouse_position()))
}

/// The font size to rasterise `size` at so the text is sharp on screen, and
/// the scale that brings it back to `size` canvas units.
fn sharp(size: f32) -> (u16, f32) {
    let pixels = (size * current().scale).round().max(1.0);
    (pixels as u16, size / pixels)
}

/// Draws text with its baseline at `y`, `size` canvas units tall. Unlike
/// Macroquad's `draw_text`, it stays sharp when the canvas is scaled up.
pub fn draw_text(text: &str, x: f32, y: f32, size: f32, color: Color) {
    let (font_size, font_scale) = sharp(size);
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font_size,
            font_scale,
            color,
            ..TextParams::default()
        },
    );
}

/// How wide `text` is when drawn with [`draw_text`], in canvas units.
pub fn text_width(text: &str, size: f32) -> f32 {
    let (font_size, font_scale) = sharp(size);
    measure_text(text, None, font_size, font_scale).width
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW_SIZES: [(f32, f32); 7] = [
        (1280.0, 720.0),  // exactly the canvas
        (1920.0, 1080.0), // full HD, same shape
        (2560.0, 1440.0),
        (1440.0, 900.0),  // a 16:10 laptop: bars top and bottom
        (1024.0, 768.0),  // 4:3
        (800.0, 1200.0),  // portrait
        (2560.0, 1080.0), // ultra-wide: bars left and right
    ];

    /// Where the camera puts a canvas point, in window units, worked out from
    /// the camera's own matrix (the same one the GPU gets).
    fn through_camera(view: View, canvas_point: Vec2) -> Vec2 {
        let ndc = view
            .camera()
            .matrix()
            .project_point3(vec3(canvas_point.x, canvas_point.y, 0.0));
        // Clip space is -1..1 with y pointing up; the window has y pointing down.
        vec2(
            f32::midpoint(ndc.x, 1.0) * view.window.x,
            f32::midpoint(-ndc.y, 1.0) * view.window.y,
        )
    }

    fn assert_close(a: Vec2, b: Vec2, what: &str) {
        assert!(a.distance(b) < 0.01, "{what}: {a} vs {b}");
    }

    #[test]
    fn the_canvas_fills_a_window_of_the_same_shape() {
        let view = View::fit(vec2(1920.0, 1080.0));
        assert!((view.scale - 1.5).abs() < 1e-6);
        assert_close(view.origin, Vec2::ZERO, "no bars");
    }

    #[test]
    fn the_canvas_is_centred_and_never_cropped() {
        for (w, h) in WINDOW_SIZES {
            let view = View::fit(vec2(w, h));
            let (top_left, bottom_right) = (view.origin, view.origin + CANVAS * view.scale);

            // Inside the window, with equal bars on both sides...
            assert!(top_left.x >= -0.01 && top_left.y >= -0.01, "{w}x{h}");
            assert!(
                bottom_right.x <= w + 0.01 && bottom_right.y <= h + 0.01,
                "{w}x{h}"
            );
            assert_close(top_left, vec2(w, h) - bottom_right, "centred");
            // ...and touching the window on at least one axis (as big as it gets).
            let touches_x = top_left.x.abs() < 0.01;
            let touches_y = top_left.y.abs() < 0.01;
            assert!(touches_x || touches_y, "{w}x{h} could be bigger");
        }
    }

    #[test]
    fn the_camera_draws_the_canvas_where_the_view_says() {
        for (w, h) in WINDOW_SIZES {
            let view = View::fit(vec2(w, h));
            for corner in [
                Vec2::ZERO,
                vec2(WIDTH, 0.0),
                vec2(0.0, HEIGHT),
                CANVAS,
                CANVAS / 2.0,
            ] {
                let expected = view.origin + corner * view.scale;
                assert_close(
                    through_camera(view, corner),
                    expected,
                    &format!("{w}x{h} {corner}"),
                );
            }
        }
    }

    #[test]
    fn canvas_y_grows_downwards_like_the_default_camera() {
        let view = View::fit(vec2(1280.0, 720.0));
        let top = through_camera(view, vec2(100.0, 100.0));
        let lower = through_camera(view, vec2(100.0, 400.0));
        assert!(lower.y > top.y, "larger y must be lower on screen");
        let left = through_camera(view, vec2(100.0, 100.0));
        let right = through_camera(view, vec2(600.0, 100.0));
        assert!(right.x > left.x, "larger x must be further right");
    }

    #[test]
    fn the_mouse_maps_back_to_the_canvas() {
        for (w, h) in WINDOW_SIZES {
            let view = View::fit(vec2(w, h));
            for point in [Vec2::ZERO, vec2(321.0, 654.0), CANVAS, CANVAS / 2.0] {
                let on_screen = view.origin + point * view.scale;
                assert_close(view.to_canvas(on_screen), point, &format!("{w}x{h}"));
            }
        }
    }

    #[test]
    fn the_middle_of_the_window_is_the_middle_of_the_canvas() {
        for (w, h) in WINDOW_SIZES {
            let view = View::fit(vec2(w, h));
            assert_close(view.to_canvas(vec2(w, h) / 2.0), CANVAS / 2.0, "centre");
        }
    }

    #[test]
    fn points_over_the_bars_are_outside_the_canvas() {
        // A wide window has bars at the left and right.
        let view = View::fit(vec2(2560.0, 1080.0));
        let in_left_bar = view.to_canvas(vec2(10.0, 500.0));
        assert!(in_left_bar.x < 0.0, "{in_left_bar}");
    }

    #[test]
    fn a_zero_sized_window_does_not_produce_nan() {
        // Happens while the window is minimised.
        let view = View::fit(Vec2::ZERO);
        assert!(view.scale.is_finite() && view.scale > 0.0);
        assert!(view.to_canvas(Vec2::ZERO).is_finite());
        assert!(view.camera().matrix().is_finite());
    }

    #[test]
    fn the_toggle_key_switches_full_screen_on_and_off() {
        assert!(next_fullscreen(false, true, false));
        assert!(!next_fullscreen(true, true, false));
    }

    #[test]
    fn escape_leaves_full_screen_but_never_enters_it() {
        assert!(!next_fullscreen(true, false, true));
        assert!(!next_fullscreen(false, false, true));
    }

    #[test]
    fn full_screen_stays_as_it_is_when_no_key_is_pressed() {
        assert!(next_fullscreen(true, false, false));
        assert!(!next_fullscreen(false, false, false));
    }

    #[test]
    fn pressing_the_toggle_and_escape_together_obeys_the_toggle() {
        assert!(next_fullscreen(false, true, true));
    }

    #[test]
    fn starting_full_screen_is_remembered() {
        started_fullscreen(true);
        assert!(is_fullscreen());
        started_fullscreen(false);
        assert!(!is_fullscreen());
    }

    #[test]
    fn sharp_text_is_rasterised_at_the_screen_size_and_drawn_at_the_canvas_size() {
        CURRENT.with(|current| current.set(View::fit(vec2(1920.0, 1080.0))));
        let (font_size, font_scale) = sharp(22.0);
        assert_eq!(font_size, 33, "22 canvas units are 33 pixels at 1.5x");
        assert!((f32::from(font_size) * font_scale - 22.0).abs() < 1e-4);

        // Tiny windows never ask for a zero-sized font.
        CURRENT.with(|current| current.set(View::fit(vec2(10.0, 10.0))));
        assert!(sharp(22.0).0 >= 1);
    }
}
