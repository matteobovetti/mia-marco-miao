//! The wood panel on the wall with the cat toys hanging on it.
//!
//! The player clicks a toy to pick it up and moves the mouse around with it;
//! while Space is held down the toy comes alive and the cats chase it (see
//! `Cat::play_with`). The panel only knows about the toys and how to draw
//! them; the playing itself happens to the cat.

use macroquad::prelude::*;

use crate::brushes::{BOARD_DARK, METAL, draw_board};
use crate::ui;
use crate::view;
use crate::window;

/// The panel on the left of the wall, under the cats' need bars and clear of
/// the window.
pub const PANEL: Rect = Rect {
    x: 16.0,
    y: 168.0,
    w: 304.0,
    h: 112.0,
};
const PADDING: f32 = 8.0;
const SLOT_GAP: f32 = 4.0;
const SLOT_W: f32 =
    (PANEL.w - 2.0 * PADDING - (Toy::ALL.len() - 1) as f32 * SLOT_GAP) / Toy::ALL.len() as f32;
const SLOT_H: f32 = PANEL.h - 2.0 * PADDING;

// The panel must hang on the wall, clear of the baseboard...
const _: () = assert!(PANEL.y + PANEL.h < crate::world::FLOOR_TOP - 8.0);
// ...below the need bars...
const _: () = assert!(PANEL.y > ui::NEEDS_BOTTOM + 8.0);
// ...and left of the window.
const _: () = assert!(PANEL.x + PANEL.w + 8.0 < window::LEFT_EDGE);

const INK: Color = Color::new(0.25, 0.18, 0.12, 1.0);
const RED: Color = Color::new(0.92, 0.15, 0.12, 1.0);

/// The toys on the panel, one per slot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Toy {
    /// A laser pointer: the cat chases the red dot.
    Laser,
    /// A small red ball that bounces.
    RedBall,
    /// A ball of yarn with a loose end.
    Yarn,
    /// A small toy mouse.
    Mouse,
}

impl Toy {
    pub const ALL: [Self; 4] = [Self::Laser, Self::RedBall, Self::Yarn, Self::Mouse];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Laser => "Laser pointer",
            Self::RedBall => "Small red ball",
            Self::Yarn => "Ball of yarn",
            Self::Mouse => "Toy mouse",
        }
    }

    /// What a cat chasing this toy is doing, for the status line.
    pub const fn chasing(self) -> &'static str {
        match self {
            Self::Laser => "Chasing the red dot!",
            Self::RedBall => "Chasing the red ball!",
            Self::Yarn => "Pouncing on the yarn!",
            Self::Mouse => "Hunting the mouse!",
        }
    }

    /// How much fun the cat gets per second while playing with this toy.
    /// The laser can never be caught, so it is the least satisfying.
    pub const fn fun_per_sec(self) -> f32 {
        match self {
            Self::Laser => 0.12,
            Self::RedBall => 0.15,
            Self::Yarn => 0.17,
            Self::Mouse => 0.2,
        }
    }
}

fn fade(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

/// Where and how a toy is drawn.
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    /// The middle of the toy (for the laser, of the pen).
    pub at: Vec2,
    /// Which way the toy points, in radians; `0.0` is to the right. Only the
    /// laser and the mouse care.
    pub angle: f32,
    pub scale: f32,
    /// `1.0` is solid, less is see-through.
    pub alpha: f32,
    /// Whether the player is playing with it right now: the toy bounces,
    /// rolls or wriggles.
    pub active: bool,
    /// Seconds, for the animation.
    pub time: f32,
}

/// Draws a toy. Each is about 50 units across at scale `1.0`.
pub fn draw_toy(toy: Toy, pose: Pose) {
    match toy {
        Toy::Laser => draw_laser(pose),
        Toy::RedBall => draw_red_ball(pose),
        Toy::Yarn => draw_yarn(pose),
        Toy::Mouse => draw_mouse(pose),
    }
}

/// A laser pen. Its tip points along `pose.angle` and glows while it is on.
fn draw_laser(pose: Pose) {
    let Pose {
        at,
        angle,
        scale,
        alpha,
        active,
        ..
    } = pose;
    let dir = Vec2::from_angle(angle);
    let side = dir.perp();
    let along = |x: f32| at + dir * x * scale;
    let colour = |c: Color| fade(c, alpha);

    let (tail, tip) = (along(-24.0), along(14.0));
    // Body, a grip band, the cap and the lens.
    draw_line(
        tail.x,
        tail.y,
        tip.x,
        tip.y,
        10.0 * scale,
        colour(Color::new(0.25, 0.3, 0.45, 1.0)),
    );
    let (band_a, band_b) = (along(-14.0), along(-8.0));
    draw_line(
        band_a.x,
        band_a.y,
        band_b.x,
        band_b.y,
        11.0 * scale,
        colour(METAL),
    );
    draw_circle(tail.x, tail.y, 5.0 * scale, colour(METAL));
    let button = along(2.0) + side * 5.0 * scale;
    draw_circle(button.x, button.y, 2.2 * scale, colour(RED));
    draw_circle(
        tip.x,
        tip.y,
        4.0 * scale,
        colour(if active { RED } else { INK }),
    );
    if active {
        draw_circle(tip.x, tip.y, 8.0 * scale, fade(RED, 0.3 * alpha));
    }
}

/// A shiny red ball. It bounces on the spot while played with.
fn draw_red_ball(pose: Pose) {
    let Pose {
        at,
        scale,
        alpha,
        active,
        time,
        ..
    } = pose;
    let radius = 13.0 * scale;
    let hop = if active {
        (time * 9.0).sin().abs() * 14.0 * scale
    } else {
        0.0
    };
    // The shadow stays on the ground and shrinks as the ball goes up.
    let ground = at + vec2(0.0, radius + 4.0 * scale);
    let squash = 1.0 - hop / (40.0 * scale);
    draw_ellipse(
        ground.x,
        ground.y,
        radius * 0.9 * squash,
        3.0 * scale,
        0.0,
        Color::new(0.0, 0.0, 0.0, 0.2 * alpha),
    );

    let centre = at - vec2(0.0, hop);
    draw_circle(
        centre.x,
        centre.y,
        radius,
        fade(Color::new(0.6, 0.05, 0.08, 1.0), alpha),
    );
    draw_circle(
        centre.x - 1.0 * scale,
        centre.y - 1.0 * scale,
        radius - 2.5 * scale,
        fade(RED, alpha),
    );
    draw_circle(
        centre.x - 4.5 * scale,
        centre.y - 5.0 * scale,
        3.2 * scale,
        fade(Color::new(1.0, 0.85, 0.85, 1.0), alpha),
    );
}

/// A ball of yarn with a loose end. It rolls and the end swishes while played
/// with.
fn draw_yarn(pose: Pose) {
    let Pose {
        at,
        scale,
        alpha,
        active,
        time,
        ..
    } = pose;
    let radius = 14.0 * scale;
    let yarn = Color::new(0.6, 0.35, 0.85, 1.0);
    let yarn_dark = Color::new(0.4, 0.2, 0.62, 1.0);
    let roll = if active { time * 5.0 } else { 0.5 };

    // The loose end trails off to the right.
    let start = at + vec2(radius * 0.8, radius * 0.5);
    let swish = if active { 1.0 } else { 0.25 };
    let mut previous = start;
    for k in 1..=8 {
        let t = k as f32;
        let next = start
            + vec2(
                t * 3.8 * scale,
                (t * t * 0.3 + (time * 7.0 + t * 0.7).sin() * 2.5 * swish) * scale,
            );
        draw_line(
            previous.x,
            previous.y,
            next.x,
            next.y,
            2.4 * scale,
            fade(yarn, alpha),
        );
        previous = next;
    }

    draw_circle(at.x, at.y, radius, fade(yarn_dark, alpha));
    draw_circle(at.x, at.y, radius - 1.5 * scale, fade(yarn, alpha));
    // The turns of wool: chords across the ball.
    for i in 0..5 {
        let from = Vec2::from_angle(roll + i as f32 * 0.7) * (radius - 1.0 * scale);
        let to = Vec2::from_angle(roll + i as f32 * 0.7 + 2.3) * (radius - 1.0 * scale);
        draw_line(
            at.x + from.x,
            at.y + from.y,
            at.x + to.x,
            at.y + to.y,
            1.4 * scale,
            fade(yarn_dark, alpha),
        );
    }
}

/// A grey toy mouse, nose along `pose.angle`. It scurries and its tail swishes
/// while played with.
fn draw_mouse(pose: Pose) {
    let Pose {
        at,
        angle,
        scale,
        alpha,
        active,
        time,
    } = pose;
    let fur = Color::new(0.66, 0.66, 0.72, 1.0);
    let ear = Color::new(0.95, 0.65, 0.7, 1.0);
    // While played with it jitters about as if scurrying.
    let jitter = if active {
        vec2((time * 40.0).sin(), (time * 33.0).cos()) * 1.2 * scale
    } else {
        Vec2::ZERO
    };
    let turn = Vec2::from_angle(angle);
    // `(x, y)` is along and across the mouse, with the nose at positive `x`.
    let place = |x: f32, y: f32| at + jitter + turn.rotate(vec2(x, y) * scale);
    let colour = |c: Color| fade(c, alpha);

    // Tail, swishing.
    let swish = if active { 5.0 } else { 1.5 };
    let mut previous = place(-14.0, 0.0);
    for k in 1..=6 {
        let t = k as f32;
        let next = place(
            -14.0 - t * 4.5,
            (time * 12.0 + t * 0.9).sin() * swish * t / 6.0,
        );
        draw_line(
            previous.x,
            previous.y,
            next.x,
            next.y,
            1.8 * scale,
            colour(ear),
        );
        previous = next;
    }

    // Ears, body, nose, eye and whiskers.
    for y in [-7.0, 7.0] {
        let e = place(2.0, y);
        draw_circle(e.x, e.y, 5.5 * scale, colour(fur));
        draw_circle(e.x, e.y, 3.2 * scale, colour(ear));
    }
    let (back, front) = (place(-9.0, 0.0), place(9.0, 0.0));
    draw_line(back.x, back.y, front.x, front.y, 15.0 * scale, colour(fur));
    let nose = place(18.0, 0.0);
    draw_triangle(place(8.0, -6.0), place(8.0, 6.0), nose, colour(fur));
    draw_circle(
        nose.x,
        nose.y,
        2.2 * scale,
        colour(Color::new(0.9, 0.35, 0.45, 1.0)),
    );
    let eye = place(11.0, -3.0);
    draw_circle(eye.x, eye.y, 1.5 * scale, colour(INK));
    for y in [-1.0, 1.0] {
        let (a, b) = (place(14.0, y * 2.0), place(22.0, y * 7.0));
        draw_line(a.x, a.y, b.x, b.y, 1.0 * scale, colour(INK));
    }
}

fn slot(index: usize) -> Rect {
    Rect::new(
        PANEL.x + PADDING + index as f32 * (SLOT_W + SLOT_GAP),
        PANEL.y + PADDING,
        SLOT_W,
        SLOT_H,
    )
}

/// Which toy, if any, hangs under `point`.
pub fn toy_at(point: Vec2) -> Option<Toy> {
    Toy::ALL
        .into_iter()
        .enumerate()
        .find(|&(i, _)| slot(i).contains(point))
        .map(|(_, toy)| toy)
}

/// Draws the panel. The toy being held is missing from it: only a faint
/// outline is left where it goes back.
pub fn draw_panel(held: Option<Toy>, pointer: Vec2) {
    draw_board(PANEL);

    let mut hovered_toy = None;
    for (i, toy) in Toy::ALL.into_iter().enumerate() {
        let rect = slot(i);
        let taken = held == Some(toy);
        let hovered = !taken && rect.contains(pointer);

        // A ledge for the toy to rest on.
        draw_rectangle(rect.x, rect.y + rect.h - 6.0, rect.w, 5.0, BOARD_DARK);
        if hovered {
            hovered_toy = Some(toy);
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::new(1.0, 1.0, 1.0, 0.18),
            );
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                2.0,
                Color::new(1.0, 0.95, 0.7, 1.0),
            );
        }

        // The toy lifts a little when the pointer is over it.
        let lift = if hovered { 3.0 } else { 0.0 };
        draw_toy(
            toy,
            Pose {
                at: vec2(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0 - 4.0 - lift),
                angle: if toy == Toy::Laser { -0.45 } else { 0.0 },
                scale: 1.0,
                alpha: if taken { 0.18 } else { 1.0 },
                active: false,
                time: 0.0,
            },
        );
    }

    // The name of the toy under the pointer, just under the panel.
    if let Some(toy) = hovered_toy {
        view::draw_text(toy.name(), PANEL.x, PANEL.y + PANEL.h + 20.0, 18.0, INK);
    }
}

/// How far from the pointer the laser pen is held, down and to the right.
const PEN_REACH: f32 = 30.0;

/// Draws the toy in the player's hand: at the pointer, or for the laser the
/// red dot is at the pointer and the pen is held beside it. `active` is
/// whether Space is down.
pub fn draw_held(toy: Toy, pointer: Vec2, active: bool, time: f32) {
    let pose = Pose {
        at: pointer,
        angle: 0.0,
        scale: 1.0,
        alpha: 1.0,
        active,
        time,
    };
    if toy != Toy::Laser {
        draw_toy(toy, pose);
        return;
    }

    // The pen points up and to the left, at the dot.
    let dir = vec2(-1.0, -1.0).normalize();
    let pen = Pose {
        at: pointer - dir * (PEN_REACH + 12.0),
        angle: dir.to_angle(),
        ..pose
    };
    draw_toy(toy, pen);
    if active {
        let tip = pen.at + dir * 14.0;
        draw_line(tip.x, tip.y, pointer.x, pointer.y, 1.5, fade(RED, 0.55));
        // The dot, pulsing, with a glow.
        let pulse = 1.0 + (time * 14.0).sin() * 0.15;
        draw_circle(pointer.x, pointer.y, 20.0 * pulse, fade(RED, 0.12));
        draw_circle(pointer.x, pointer.y, 12.0 * pulse, fade(RED, 0.25));
        draw_circle(pointer.x, pointer.y, 6.0 * pulse, RED);
        draw_circle(pointer.x, pointer.y, 2.5, Color::new(1.0, 0.8, 0.75, 1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_toy_hangs_in_its_own_slot_on_the_panel() {
        for (i, toy) in Toy::ALL.into_iter().enumerate() {
            let rect = slot(i);
            assert!(PANEL.contains(rect.point()));
            assert!(PANEL.contains(rect.point() + rect.size() - Vec2::ONE));
            assert_eq!(toy_at(rect.point() + rect.size() / 2.0), Some(toy));
            if i > 0 {
                assert!(slot(i - 1).x + slot(i - 1).w <= rect.x, "slots overlap");
            }
        }
    }

    #[test]
    fn clicking_the_wall_beside_the_panel_picks_no_toy() {
        assert_eq!(toy_at(vec2(PANEL.x - 5.0, PANEL.y + 30.0)), None);
        assert_eq!(toy_at(vec2(PANEL.x + 30.0, PANEL.y + PANEL.h + 5.0)), None);
        // The gap between two slots is not a toy either.
        assert_eq!(
            toy_at(vec2(slot(0).x + SLOT_W + SLOT_GAP / 2.0, PANEL.y + 30.0)),
            None
        );
    }

    #[test]
    fn the_panel_does_not_cover_the_brush_panel() {
        assert!(!PANEL.overlaps(&crate::brushes::PANEL));
    }

    #[test]
    fn every_toy_is_fun_and_has_its_own_name() {
        assert!(Toy::ALL.iter().all(|toy| toy.fun_per_sec() > 0.0));
        for (i, a) in Toy::ALL.into_iter().enumerate() {
            for b in Toy::ALL.into_iter().skip(i + 1) {
                assert_ne!(a.name(), b.name());
                assert_ne!(a.chasing(), b.chasing());
            }
        }
    }
}
