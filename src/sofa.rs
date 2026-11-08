//! The sofa under the window, with its cushions. The cats jump up onto the top
//! of its backrest to sleep in the sun.

use macroquad::prelude::*;

use crate::home;
use crate::world::Place;

/// The sofa including its armrests and feet.
pub const BODY: Rect = Rect {
    x: 366.0,
    y: 298.0,
    w: 320.0,
    h: 88.0,
};
/// Where a cat's paws are when it lies on top of the backrest: just below
/// [`BODY`]'s top edge.
const TOP_Y: f32 = BODY.y + 6.0;
/// The floor in front of the sofa, where the cat takes off and lands.
pub const TAKE_OFF_Y: f32 = 404.0;

/// Two places to nap on top of the backrest, side by side so that both cats
/// fit up there at once.
pub const LEFT_NAP: Place = Place::sofa(vec2(452.0, TOP_Y), vec2(452.0, TOP_Y));
pub const RIGHT_NAP: Place = Place::sofa(vec2(600.0, TOP_Y), vec2(600.0, TOP_Y));

/// How high above the straight line between the floor and the sofa's top the
/// cat goes at the peak of its jump.
const JUMP_ARC: f32 = 34.0;

/// Where the cat is `progress` (`0.0..=1.0`) of the way through jumping
/// between the floor and the top of the sofa, at horizontal position `x`.
/// `up` says which way it is going.
pub fn jump_position(x: f32, progress: f32, up: bool) -> Vec2 {
    // 0.0 on the floor, 1.0 on the sofa.
    let height = if up { progress } else { 1.0 - progress };
    let y = TAKE_OFF_Y + (TOP_Y - TAKE_OFF_Y) * height - JUMP_ARC * 4.0 * height * (1.0 - height);
    vec2(x, y)
}

// The cat takes off from the floor in front of the sofa's feet.
const _: () = assert!(TAKE_OFF_Y > BODY.y + BODY.h);
// The sofa stands clear of the home on its right.
const _: () = assert!(BODY.x + BODY.w < home::KEEP_CLEAR.x);

const SHADOW: Color = Color::new(0.0, 0.0, 0.0, 0.2);
const FABRIC: Color = Color::new(0.36, 0.55, 0.5, 1.0);
const FABRIC_DARK: Color = Color::new(0.27, 0.43, 0.4, 1.0);
const FABRIC_LIGHT: Color = Color::new(0.45, 0.65, 0.58, 1.0);
const FOOT: Color = Color::new(0.4, 0.27, 0.18, 1.0);

/// A rectangle with rounded corners.
fn rounded_rect(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
    draw_rectangle(x + radius, y, w - 2.0 * radius, h, color);
    draw_rectangle(x, y + radius, w, h - 2.0 * radius, color);
    for (cx, cy) in [
        (x + radius, y + radius),
        (x + w - radius, y + radius),
        (x + radius, y + h - radius),
        (x + w - radius, y + h - radius),
    ] {
        draw_circle(cx, cy, radius, color);
    }
}

/// A plump throw cushion with a button in the middle.
fn draw_cushion(x: f32, y: f32, w: f32, h: f32, color: Color) {
    let edge = Color::new(color.r * 0.8, color.g * 0.8, color.b * 0.8, 1.0);
    rounded_rect(x - 2.0, y - 2.0, w + 4.0, h + 4.0, 13.0, edge);
    rounded_rect(x, y, w, h, 11.0, color);
    // A soft highlight on the upper left, and the button with its creases.
    draw_circle(
        x + w * 0.3,
        y + h * 0.3,
        5.0,
        Color::new(1.0, 1.0, 1.0, 0.25),
    );
    let middle = vec2(x + w / 2.0, y + h / 2.0);
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        draw_line(
            middle.x,
            middle.y,
            middle.x + dx * w * 0.3,
            middle.y + dy * h * 0.3,
            1.2,
            Color { a: 0.35, ..edge },
        );
    }
    draw_circle(middle.x, middle.y, 3.0, edge);
}

pub fn draw() {
    let Rect { x, y, w, h } = BODY;
    let (right, bottom) = (x + w, y + h);

    draw_ellipse(x + w / 2.0, bottom + 2.0, w / 2.0 + 8.0, 11.0, 0.0, SHADOW);

    // Feet.
    for foot_x in [x + 14.0, right - 24.0] {
        draw_rectangle(foot_x, bottom - 12.0, 10.0, 12.0, FOOT);
    }

    // Backrest, with the seam between its two halves.
    rounded_rect(x + 12.0, y, w - 24.0, 64.0, 16.0, FABRIC);
    draw_line(
        x + w / 2.0,
        y + 10.0,
        x + w / 2.0,
        y + 52.0,
        2.0,
        FABRIC_DARK,
    );

    // Seat: the base, then the two seat cushions on it.
    rounded_rect(x + 20.0, y + 56.0, w - 40.0, 32.0, 6.0, FABRIC_DARK);
    let cushion_w = (w - 76.0) / 2.0;
    for seat_x in [x + 38.0, x + 38.0 + cushion_w + 2.0] {
        rounded_rect(seat_x, y + 42.0, cushion_w - 2.0, 32.0, 10.0, FABRIC_LIGHT);
    }

    // The throw cushions, leaning on the backrest.
    draw_cushion(
        x + 42.0,
        y + 18.0,
        50.0,
        46.0,
        Color::new(0.92, 0.55, 0.4, 1.0),
    );
    draw_cushion(
        x + 132.0,
        y + 26.0,
        46.0,
        40.0,
        Color::new(0.96, 0.8, 0.35, 1.0),
    );
    draw_cushion(
        x + 226.0,
        y + 16.0,
        52.0,
        48.0,
        Color::new(0.82, 0.86, 0.96, 1.0),
    );

    // Armrests, in front of the cushions' ends.
    rounded_rect(x, y + 26.0, 36.0, 60.0, 16.0, FABRIC);
    rounded_rect(right - 36.0, y + 26.0, 36.0, 60.0, 16.0, FABRIC);
    for arm_x in [x + 5.0, right - 31.0] {
        draw_line(arm_x, y + 44.0, arm_x, y + 72.0, 2.0, FABRIC_LIGHT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_jump_starts_on_the_floor_and_ends_on_the_sofa() {
        assert_eq!(jump_position(450.0, 0.0, true), vec2(450.0, TAKE_OFF_Y));
        assert_eq!(jump_position(450.0, 1.0, true), vec2(450.0, TOP_Y));
        // Jumping down is the same jump backwards.
        assert_eq!(jump_position(450.0, 0.0, false), vec2(450.0, TOP_Y));
        assert_eq!(jump_position(450.0, 1.0, false), vec2(450.0, TAKE_OFF_Y));
    }

    #[test]
    fn the_cat_leaps_higher_than_the_sofa_in_the_middle_of_the_jump() {
        let middle = jump_position(450.0, 0.5, true).y;
        let straight_line = f32::midpoint(TAKE_OFF_Y, TOP_Y);
        assert!((straight_line - middle - JUMP_ARC).abs() < 1e-3, "{middle}");
    }

    #[test]
    fn the_naps_are_on_top_of_the_backrest_and_apart() {
        for nap in [LEFT_NAP, RIGHT_NAP] {
            assert!(BODY.x < nap.spot.x && nap.spot.x < BODY.x + BODY.w);
            assert!((nap.spot.y - TOP_Y).abs() < f32::EPSILON);
        }
    }

    // Two sleeping cats are about 80 wide each.
    const _: () = assert!(RIGHT_NAP.spot.x - LEFT_NAP.spot.x > 100.0);
}
