//! The wood panel on the wall with the hairbrushes attached to it.
//!
//! The player clicks a brush to pick it up, then clicks a cat to brush it. The
//! panel only knows about the brushes and how to draw them; the brushing
//! itself happens to the cat (see `Cat::brush_if_hit`).

use macroquad::prelude::*;

use crate::ui;
use crate::world;

/// The panel on the wall, in the gap between the window and the shelf.
pub const PANEL: Rect = Rect {
    x: 960.0,
    y: 52.0,
    w: 118.0,
    h: 234.0,
};
const PADDING: f32 = 8.0;
const SLOT_H: f32 = 70.0;
const SLOT_GAP: f32 = 4.0;

// The panel must hang on the wall, clear of the baseboard...
const _: () = assert!(PANEL.y + PANEL.h < world::FLOOR_TOP - 8.0);
// ...and of the shelf on the right.
const _: () = assert!(PANEL.x + PANEL.w < ui::SHELF_X - 8.0);
// Every slot fits on the panel.
const _: () = assert!(
    2.0 * PADDING + Brush::ALL.len() as f32 * SLOT_H + (Brush::ALL.len() - 1) as f32 * SLOT_GAP
        <= PANEL.h
);

const BOARD: Color = Color::new(0.62, 0.43, 0.26, 1.0);
pub const BOARD_DARK: Color = Color::new(0.4, 0.26, 0.15, 1.0);
const GRAIN: Color = Color::new(0.5, 0.34, 0.2, 0.55);
const STRAP: Color = Color::new(0.3, 0.19, 0.12, 1.0);
pub const METAL: Color = Color::new(0.8, 0.8, 0.84, 1.0);

/// The brushes on the panel, one per slot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Brush {
    /// A wooden brush with soft bristles.
    Soft,
    /// A big round brush with a blue handle.
    Paddle,
    /// A pink slicker brush with metal pins.
    Slicker,
}

/// What the head of a brush looks like from above.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Head {
    Oval,
    Block,
}

/// The colours and shape of a brush.
struct Look {
    handle: Color,
    /// The back and rim of the head.
    pad: Color,
    bristles: Color,
    /// The tips of the bristles (or the pins).
    tips: Color,
    head: Head,
}

impl Brush {
    pub const ALL: [Self; 3] = [Self::Soft, Self::Paddle, Self::Slicker];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Soft => "Soft brush",
            Self::Paddle => "Paddle brush",
            Self::Slicker => "Slicker brush",
        }
    }

    /// How much affection the cat gets per second while being brushed with
    /// this one. The better the brush, the happier the cat.
    pub const fn love_per_sec(self) -> f32 {
        match self {
            Self::Soft => 0.08,
            Self::Paddle => 0.11,
            Self::Slicker => 0.15,
        }
    }

    const fn look(self) -> Look {
        match self {
            Self::Soft => Look {
                handle: Color::new(0.85, 0.65, 0.4, 1.0),
                pad: Color::new(0.72, 0.5, 0.3, 1.0),
                bristles: Color::new(0.95, 0.88, 0.72, 1.0),
                tips: Color::new(0.55, 0.42, 0.3, 1.0),
                head: Head::Oval,
            },
            Self::Paddle => Look {
                handle: Color::new(0.3, 0.5, 0.85, 1.0),
                pad: Color::new(0.22, 0.38, 0.68, 1.0),
                bristles: Color::new(0.2, 0.2, 0.24, 1.0),
                tips: Color::new(0.5, 0.5, 0.58, 1.0),
                head: Head::Oval,
            },
            Self::Slicker => Look {
                handle: Color::new(0.95, 0.5, 0.65, 1.0),
                pad: Color::new(0.9, 0.38, 0.55, 1.0),
                bristles: Color::new(0.98, 0.72, 0.8, 1.0),
                tips: METAL,
                head: Head::Block,
            },
        }
    }
}

/// Where and how a brush is drawn.
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    /// The middle of the brush's head, where it touches the fur.
    pub head: Vec2,
    /// Which way the handle points away from the head, in radians; `0.0` is
    /// the handle to the left of the head.
    pub angle: f32,
    pub scale: f32,
    /// `1.0` is solid, less is see-through.
    pub alpha: f32,
}

/// How far from the middle of the head the handle's end is, at scale `1.0`.
const HANDLE_END: f32 = 78.0;
/// How far the head reaches from its middle.
const HEAD_REACH: f32 = 22.0;

fn fade(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

fn lighten(color: Color) -> Color {
    Color::new(
        (color.r + 0.25).min(1.0),
        (color.g + 0.25).min(1.0),
        (color.b + 0.25).min(1.0),
        color.a,
    )
}

/// Draws a brush with the handle on the left and the head on the right,
/// turned by `pose.angle`.
pub fn draw_brush(brush: Brush, pose: Pose) {
    let Pose {
        head: origin,
        angle,
        scale,
        alpha,
    } = pose;
    let look = brush.look();
    let turn = Vec2::from_angle(angle);
    // `(x, y)` is along and across the brush: `x` runs from the head's middle
    // towards the end of the handle, `y` across it.
    let at = |x: f32, y: f32| origin + turn.rotate(vec2(-x, y) * scale);
    let colour = |c: Color| fade(c, alpha);

    // Handle, with a neck where it meets the head, and a shine.
    let (end, neck) = (at(HANDLE_END, 0.0), at(HEAD_REACH - 2.0, 0.0));
    draw_line(
        end.x,
        end.y,
        neck.x,
        neck.y,
        9.0 * scale,
        colour(look.handle),
    );
    draw_circle(end.x, end.y, 4.5 * scale, colour(look.handle));
    let (shine_from, shine_to) = (at(HANDLE_END - 4.0, 2.5), at(HEAD_REACH + 6.0, 2.5));
    draw_line(
        shine_from.x,
        shine_from.y,
        shine_to.x,
        shine_to.y,
        2.0 * scale,
        colour(lighten(look.handle)),
    );

    // Head: the rim, then the bristles inside it.
    match look.head {
        Head::Oval => {
            draw_ellipse(
                origin.x,
                origin.y,
                24.0 * scale,
                16.0 * scale,
                angle,
                colour(look.pad),
            );
            draw_ellipse(
                origin.x,
                origin.y,
                20.0 * scale,
                12.0 * scale,
                angle,
                colour(look.bristles),
            );
        }
        Head::Block => {
            let (a, b) = (at(HEAD_REACH, 0.0), at(-HEAD_REACH, 0.0));
            draw_line(a.x, a.y, b.x, b.y, 30.0 * scale, colour(look.pad));
            let (a, b) = (at(HEAD_REACH - 4.0, 0.0), at(-HEAD_REACH + 4.0, 0.0));
            draw_line(a.x, a.y, b.x, b.y, 22.0 * scale, colour(look.bristles));
        }
    }

    // Rows of bristle tips (or pins).
    for column in -2..=2 {
        for row in -1..=1 {
            let (x, y) = (column as f32 * 7.0, row as f32 * 6.0);
            if look.head == Head::Oval && (x / 18.0).powi(2) + (y / 10.0).powi(2) > 1.0 {
                continue;
            }
            let tip = at(x, y);
            draw_circle(tip.x, tip.y, 1.7 * scale, colour(look.tips));
        }
    }
}

fn slot(index: usize) -> Rect {
    Rect::new(
        PANEL.x + PADDING,
        PANEL.y + PADDING + index as f32 * (SLOT_H + SLOT_GAP),
        PANEL.w - 2.0 * PADDING,
        SLOT_H,
    )
}

/// Which brush, if any, hangs under `point`.
pub fn brush_at(point: Vec2) -> Option<Brush> {
    Brush::ALL
        .into_iter()
        .enumerate()
        .find(|&(i, _)| slot(i).contains(point))
        .map(|(_, brush)| brush)
}

/// The size the brushes are drawn at on the panel.
const PANEL_SCALE: f32 = 0.9;

/// Draws a wooden board hanging on the wall, with a shadow behind it, the
/// grain running along its length and a screw in each corner. The brush panel
/// and the toy panel are both made of it.
pub fn draw_board(board: Rect) {
    draw_rectangle(
        board.x + 4.0,
        board.y + 5.0,
        board.w,
        board.h,
        Color::new(0.0, 0.0, 0.0, 0.2),
    );
    draw_rectangle(board.x, board.y, board.w, board.h, BOARD);
    // The grain of the wood runs along the board.
    for i in 1..8 {
        let y = board.y + i as f32 * board.h / 8.0 + (i % 3) as f32 * 3.0;
        draw_line(board.x + 3.0, y, board.x + board.w - 3.0, y, 1.5, GRAIN);
    }
    draw_rectangle_lines(board.x, board.y, board.w, board.h, 5.0, BOARD_DARK);
    for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        let screw = vec2(
            board.x + 8.0 + dx * (board.w - 16.0),
            board.y + 8.0 + dy * (board.h - 16.0),
        );
        draw_circle(screw.x, screw.y, 3.0, METAL);
        draw_line(
            screw.x - 2.0,
            screw.y - 1.0,
            screw.x + 2.0,
            screw.y + 1.0,
            1.0,
            BOARD_DARK,
        );
    }
}

/// Draws the panel. The brush being held is missing from it: only a faint
/// outline is left where it goes back.
pub fn draw_panel(held: Option<Brush>, pointer: Vec2) {
    draw_board(PANEL);

    for (i, brush) in Brush::ALL.into_iter().enumerate() {
        let rect = slot(i);
        let centre = vec2(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        let taken = held == Some(brush);
        let hovered = !taken && rect.contains(pointer);

        // A ledge for the brush to rest on.
        let ledge_y = rect.y + rect.h - 6.0;
        draw_rectangle(rect.x, ledge_y, rect.w, 5.0, BOARD_DARK);
        if hovered {
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

        // The brush lifts a little when the pointer is over it.
        let lift = if hovered { 3.0 } else { 0.0 };
        let head_at = vec2(
            centre.x + (HANDLE_END - HEAD_REACH) / 2.0 * PANEL_SCALE,
            centre.y - 5.0 - lift,
        );
        let pose = Pose {
            head: head_at,
            angle: 0.0,
            scale: PANEL_SCALE,
            alpha: if taken { 0.18 } else { 1.0 },
        };
        draw_brush(brush, pose);

        if !taken {
            // A strap around the handle holds it to the board.
            let strap = head_at.x - 40.0 * PANEL_SCALE;
            draw_rectangle(strap - 3.0, head_at.y - 9.0, 6.0, 18.0, STRAP);
            draw_circle(strap, head_at.y - 5.0, 1.2, METAL);
            draw_circle(strap, head_at.y + 5.0, 1.2, METAL);
        }
    }
}

/// The handle points up and to the right, the way you would hold it.
const HELD_ANGLE: f32 = std::f32::consts::PI - 0.5;

/// Draws the brush in the player's hand, with its head at the pointer.
pub fn draw_held(brush: Brush, pointer: Vec2) {
    draw_brush(
        brush,
        Pose {
            head: pointer,
            angle: HELD_ANGLE,
            scale: 1.0,
            alpha: 1.0,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_brush_hangs_in_its_own_slot_on_the_panel() {
        for (i, brush) in Brush::ALL.into_iter().enumerate() {
            let rect = slot(i);
            assert!(PANEL.contains(rect.point()));
            assert!(PANEL.contains(rect.point() + rect.size() - Vec2::ONE));
            assert_eq!(brush_at(rect.point() + rect.size() / 2.0), Some(brush));
            if i > 0 {
                assert!(slot(i - 1).y + slot(i - 1).h <= rect.y, "slots overlap");
            }
        }
    }

    #[test]
    fn clicking_the_wall_beside_the_panel_picks_no_brush() {
        assert_eq!(brush_at(vec2(PANEL.x - 5.0, PANEL.y + 30.0)), None);
        assert_eq!(
            brush_at(vec2(PANEL.x + 30.0, PANEL.y + PANEL.h + 5.0)),
            None
        );
        // The gap between two slots is not a brush either.
        assert_eq!(
            brush_at(vec2(PANEL.x + 30.0, slot(0).y + SLOT_H + 2.0)),
            None
        );
    }

    #[test]
    fn a_brush_fits_in_its_slot() {
        let reach = (HANDLE_END + HEAD_REACH) * PANEL_SCALE;
        assert!(reach <= slot(0).w, "{reach} is wider than the slot");
    }

    #[test]
    fn better_brushes_make_the_cat_happier() {
        let rates = Brush::ALL.map(Brush::love_per_sec);
        assert!(rates.iter().all(|&rate| rate > 0.0));
        assert!(rates.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
