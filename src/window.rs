//! The big window on the back wall: a view of a sunny garden that the player
//! can open and close by clicking on it.
//!
//! Everything outside is drawn first, then the frame and the two sashes (the
//! glass panels that swing open) on top. The drawing sticks to the opening
//! except for the drifting clouds, whose overflow is hidden by strips of wall
//! colour at the sides.

use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::brushes;
use crate::sofa;
use crate::world;

/// The window including its frame.
const OUTER: Rect = Rect {
    x: 350.0,
    y: 36.0,
    w: 580.0,
    h: 238.0,
};
const FRAME_THICKNESS: f32 = 14.0;
/// The glass: where the garden shows.
const OPENING: Rect = Rect {
    x: OUTER.x + FRAME_THICKNESS,
    y: OUTER.y + FRAME_THICKNESS,
    w: OUTER.w - 2.0 * FRAME_THICKNESS,
    h: OUTER.h - 2.0 * FRAME_THICKNESS,
};
const OPENING_BOTTOM: f32 = OPENING.y + OPENING.h;

/// The ledge under the window sticks out this far on each side...
const SILL_OVERHANG: f32 = 12.0;
/// ...and this far below the frame.
const SILL_DROP: f32 = 10.0;
/// Clicking anywhere on the window or its sill opens or closes it.
const CLICKABLE: Rect = Rect {
    x: OUTER.x - SILL_OVERHANG,
    y: OUTER.y,
    w: OUTER.w + 2.0 * SILL_OVERHANG,
    h: OUTER.h + SILL_DROP,
};

/// The left edge of the window and its sill; the toy panel hangs to the left.
pub const LEFT_EDGE: f32 = CLICKABLE.x;

// The window must fit on the wall, clear of the baseboard...
const _: () = assert!(CLICKABLE.y + CLICKABLE.h < world::FLOOR_TOP - 8.0);
// ...and of the need bars on the left and the shelf on the right.
const _: () = assert!(CLICKABLE.x > 230.0);
const _: () = assert!(CLICKABLE.x + CLICKABLE.w < world::WIDTH - 200.0);
// The panel of hairbrushes hangs on the wall to the right of the window.
const _: () = assert!(CLICKABLE.x + CLICKABLE.w < brushes::PANEL.x);
// The sofa stands under the window: clear of its sill, and no wider than it.
const _: () = assert!(sofa::BODY.y > CLICKABLE.y + CLICKABLE.h);
const _: () = assert!(sofa::BODY.x > OUTER.x);
const _: () = assert!(sofa::BODY.x + sofa::BODY.w < OUTER.x + OUTER.w);

/// How long the window takes to swing fully open or shut, in seconds.
const SWING_SECONDS: f32 = 0.7;
/// How much of its width a sash loses when fully open: it swings towards us,
/// so it is seen almost edge-on.
const SASH_FORESHORTENING: f32 = 0.85;
/// How much taller the free edge of a fully open sash looks, because it is
/// nearer to us.
const SASH_GROWTH: f32 = 16.0;

const HORIZON: f32 = OPENING.y + OPENING.h * 0.55;
/// Clouds drift this far beyond each side of the opening before wrapping
/// round; the side strips of wall colour hide them there.
const CLOUD_MARGIN: f32 = 110.0;

const WOOD: Color = Color::new(0.97, 0.95, 0.9, 1.0);
const GLASS: Color = Color::new(0.78, 0.9, 1.0, 0.16);
const SKY_TOP: Color = Color::new(0.4, 0.68, 0.95, 1.0);
const SKY_HORIZON: Color = Color::new(0.82, 0.93, 1.0, 1.0);

/// Whether the window is open, and how far its sashes have swung.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Window {
    open: bool,
    /// `0.0` = shut, `1.0` = wide open; moves towards the wanted state.
    progress: f32,
}

impl Window {
    /// Whether a click at `point` (in canvas coordinates) is on the window.
    pub fn contains(point: Vec2) -> bool {
        CLICKABLE.contains(point)
    }

    pub const fn is_open(self) -> bool {
        self.open
    }

    /// Opens the window if it is shut and shuts it if it is open. It carries
    /// on from wherever the sashes are, even if they are still moving.
    pub const fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Swings the sashes towards the open or shut position.
    pub fn update(&mut self, dt: f32) {
        let target = if self.open { 1.0 } else { 0.0 };
        let step = dt / SWING_SECONDS;
        self.progress = if self.progress < target {
            (self.progress + step).min(target)
        } else {
            (self.progress - step).max(target)
        };
    }

    /// How open the window looks, `0.0..=1.0`. It starts and ends gently.
    pub fn openness(self) -> f32 {
        smoothstep(self.progress)
    }

    /// Draws the window, the garden outside it and the sunbeam it lets in.
    pub fn draw(self) {
        let time = get_time() as f32;
        draw_sky();
        draw_sun(time);
        draw_clouds(time);
        draw_hills();
        draw_tree();
        draw_fence();
        draw_lawn(time);
        hide_cloud_overflow();
        draw_frame();
        let openness = self.openness();
        draw_sash(Sash::new(Side::Left, openness));
        draw_sash(Sash::new(Side::Right, openness));
        draw_sunbeam(openness);
    }
}

/// Eases `t` (`0.0..=1.0`) so that it starts and ends slowly.
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    Color::new(
        from.r + (to.r - from.r) * t,
        from.g + (to.g - from.g) * t,
        from.b + (to.b - from.b) * t,
        from.a + (to.a - from.a) * t,
    )
}

/// Fills the four-cornered shape, whose corners are given in order around it.
fn fill_quad(corners: [Vec2; 4], color: Color) {
    draw_triangle(corners[0], corners[1], corners[2], color);
    draw_triangle(corners[0], corners[2], corners[3], color);
}

// --- The sashes ---------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Side {
    Left,
    Right,
}

/// One of the two glass panels, seen from the room. Its hinged edge stays on
/// the frame; its free edge swings towards us as the window opens, so the
/// panel looks narrower and a little taller.
#[derive(Clone, Copy, Debug)]
struct Sash {
    hinge_x: f32,
    free_x: f32,
    /// How much taller the free edge looks than the opening.
    growth: f32,
}

impl Sash {
    fn new(side: Side, openness: f32) -> Self {
        let reach = OPENING.w / 2.0 * (1.0 - SASH_FORESHORTENING * openness);
        let growth = SASH_GROWTH * openness;
        match side {
            Side::Left => Self {
                hinge_x: OPENING.x,
                free_x: OPENING.x + reach,
                growth,
            },
            Side::Right => Self {
                hinge_x: OPENING.x + OPENING.w,
                free_x: OPENING.x + OPENING.w - reach,
                growth,
            },
        }
    }

    /// A point on the panel: `u` goes from the hinge (`0.0`) to the free edge
    /// (`1.0`) and `v` from the top (`0.0`) to the bottom (`1.0`).
    fn point(self, u: f32, v: f32) -> Vec2 {
        let top = OPENING.y - self.growth * u;
        let bottom = OPENING_BOTTOM + self.growth * u;
        vec2(
            self.hinge_x + (self.free_x - self.hinge_x) * u,
            top + (bottom - top) * v,
        )
    }

    fn width(self) -> f32 {
        (self.free_x - self.hinge_x).abs()
    }
}

/// Draws a sash: wooden frame, tinted glass with a glint, cross bars and a
/// handle by the free edge.
fn draw_sash(sash: Sash) {
    let quad = |u0: f32, u1: f32, v0: f32, v1: f32| {
        [
            sash.point(u0, v0),
            sash.point(u1, v0),
            sash.point(u1, v1),
            sash.point(u0, v1),
        ]
    };
    fill_quad(quad(0.0, 1.0, 0.0, 1.0), WOOD);

    // The glass is inset by about 7 units on every side.
    let inset_u = (7.0 / sash.width().max(1.0)).min(0.45);
    let inset_v = 7.0 / OPENING.h;
    fill_quad(quad(inset_u, 1.0 - inset_u, inset_v, 1.0 - inset_v), GLASS);

    let glass_width = 1.0 - 2.0 * inset_u;
    let across = |fraction: f32| inset_u + glass_width * fraction;
    fill_quad(
        [
            sash.point(across(0.08), 0.08),
            sash.point(across(0.28), 0.08),
            sash.point(across(0.62), 0.5),
            sash.point(across(0.42), 0.5),
        ],
        Color::new(1.0, 1.0, 1.0, 0.28),
    );

    let bar = |from: Vec2, to: Vec2| draw_line(from.x, from.y, to.x, to.y, 4.0, WOOD);
    bar(sash.point(0.5, inset_v), sash.point(0.5, 1.0 - inset_v));
    bar(sash.point(inset_u, 0.45), sash.point(1.0 - inset_u, 0.45));

    let handle = sash.point(0.9, 0.55);
    draw_circle(handle.x, handle.y, 4.5, Color::new(0.85, 0.7, 0.25, 1.0));
}

// --- The frame ----------------------------------------------------------

fn draw_frame() {
    let Rect { x, y, w, h } = OUTER;
    let t = FRAME_THICKNESS;
    draw_rectangle(x + 6.0, y + 8.0, w, h, Color::new(0.0, 0.0, 0.0, 0.13));
    for (rx, ry, rw, rh) in [
        (x, y, w, t),
        (x, y + h - t, w, t),
        (x, y, t, h),
        (x + w - t, y, t, h),
    ] {
        draw_rectangle(rx, ry, rw, rh, WOOD);
    }

    // The ledge underneath.
    let sill = Rect::new(
        x - SILL_OVERHANG,
        y + h - 6.0,
        w + 2.0 * SILL_OVERHANG,
        16.0,
    );
    draw_rectangle(
        sill.x,
        sill.y + 4.0,
        sill.w,
        sill.h,
        Color::new(0.0, 0.0, 0.0, 0.12),
    );
    draw_rectangle(
        sill.x,
        sill.y,
        sill.w,
        sill.h,
        Color::new(0.93, 0.9, 0.82, 1.0),
    );
}

/// Paints the wall colour over the left and right of the window so clouds
/// that have drifted past the edge of the glass can't be seen there.
fn hide_cloud_overflow() {
    let wall_bottom = world::FLOOR_TOP - 8.0;
    let right = OPENING.x + OPENING.w;
    draw_rectangle(0.0, 0.0, OPENING.x, wall_bottom, world::WALL);
    draw_rectangle(right, 0.0, world::WIDTH - right, wall_bottom, world::WALL);
}

// --- The sunbeam --------------------------------------------------------

/// How strong the patch of sunlight on the floor is: stronger with the
/// window open.
fn sunbeam_alpha(openness: f32) -> f32 {
    0.12 + 0.10 * openness
}

/// The patch of sun that falls from the bottom of the window onto the floor.
fn draw_sunbeam(openness: f32) {
    let colour = Color::new(1.0, 0.95, 0.6, sunbeam_alpha(openness));
    let edge = OUTER.y + OUTER.h;
    let (a, b) = (
        vec2(OPENING.x + 0.2 * OPENING.w, edge),
        vec2(OPENING.x + 0.6 * OPENING.w, edge),
    );
    let (c, d) = (vec2(b.x + 240.0, 600.0), vec2(a.x + 240.0, 600.0));
    fill_quad([a, b, c, d], colour);
}

// --- The garden outside -------------------------------------------------

fn draw_sky() {
    const BANDS: usize = 16;
    let band_height = (HORIZON - OPENING.y) / BANDS as f32;
    for band in 0..BANDS {
        let t = band as f32 / (BANDS - 1) as f32;
        draw_rectangle(
            OPENING.x,
            OPENING.y + band as f32 * band_height,
            OPENING.w,
            band_height + 1.0,
            lerp_color(SKY_TOP, SKY_HORIZON, t),
        );
    }
    // Under the horizon, so no gaps show between the hills.
    draw_rectangle(
        OPENING.x,
        HORIZON,
        OPENING.w,
        OPENING_BOTTOM - HORIZON,
        SKY_HORIZON,
    );
}

fn draw_sun(time: f32) {
    let centre = vec2(OPENING.x + 95.0, OPENING.y + 52.0);
    draw_circle(centre.x, centre.y, 62.0, Color::new(1.0, 0.95, 0.6, 0.16));
    draw_circle(centre.x, centre.y, 46.0, Color::new(1.0, 0.93, 0.5, 0.25));

    // Twelve rays that turn slowly and flicker.
    for ray in 0..12 {
        let angle = time * 0.2 + ray as f32 * TAU / 12.0;
        let (sin, cos) = angle.sin_cos();
        let reach = 40.0 + 7.0 * (time * 2.0 + ray as f32).sin();
        draw_line(
            centre.x + cos * 32.0,
            centre.y + sin * 32.0,
            centre.x + cos * reach,
            centre.y + sin * reach,
            3.5,
            Color::new(1.0, 0.82, 0.2, 0.9),
        );
    }
    draw_circle(centre.x, centre.y, 27.0, Color::new(1.0, 0.85, 0.25, 1.0));
    draw_circle(centre.x, centre.y, 20.0, Color::new(1.0, 0.94, 0.5, 1.0));
}

/// A cloud: how high it floats (down from the top of the glass), how big it
/// is, how fast it drifts to the right and where it starts.
struct Cloud {
    height: f32,
    scale: f32,
    speed: f32,
    start: f32,
}

const CLOUDS: [Cloud; 3] = [
    Cloud {
        height: 76.0,
        scale: 1.0,
        speed: 9.0,
        start: 40.0,
    },
    Cloud {
        height: 30.0,
        scale: 0.8,
        speed: 5.0,
        start: 330.0,
    },
    Cloud {
        height: 84.0,
        scale: 1.1,
        speed: 13.0,
        start: 560.0,
    },
];

/// Where a cloud is across the window `time` seconds in. It drifts to the
/// right, and comes round again at the left after leaving.
fn cloud_x(cloud: &Cloud, time: f32) -> f32 {
    let span = OPENING.w + 2.0 * CLOUD_MARGIN;
    OPENING.x - CLOUD_MARGIN + (cloud.start + time * cloud.speed).rem_euclid(span)
}

fn draw_clouds(time: f32) {
    // Puffs as (across, up, radius), relative to the cloud's middle.
    const PUFFS: [(f32, f32, f32); 4] = [
        (-30.0, 4.0, 18.0),
        (-8.0, -8.0, 26.0),
        (18.0, -3.0, 22.0),
        (40.0, 5.0, 16.0),
    ];
    for cloud in &CLOUDS {
        let (x, y) = (cloud_x(cloud, time), OPENING.y + cloud.height);
        let s = cloud.scale;
        for (dx, dy, radius) in PUFFS {
            // A soft shadow under each puff, then the puff itself.
            let shade = Color::new(0.8, 0.86, 0.95, 0.7);
            draw_circle(x + dx * s, y + (dy + 5.0) * s, radius * s, shade);
        }
        for (dx, dy, radius) in PUFFS {
            draw_circle(
                x + dx * s,
                y + dy * s,
                radius * s,
                Color::new(1.0, 1.0, 1.0, 0.96),
            );
        }
    }
}

/// Rolling hills, drawn as thin columns so they never stray out of the glass.
fn draw_hills() {
    const STEP: f32 = 6.0;
    let far = Color::new(0.62, 0.8, 0.58, 1.0);
    let near = Color::new(0.5, 0.76, 0.38, 1.0);
    let mut x = 0.0;
    while x < OPENING.w {
        let far_height = 26.0 + 14.0 * (x * 0.011 + 0.8).sin() + 9.0 * (x * 0.027 + 2.0).sin();
        let far_top = HORIZON + 8.0 - far_height;
        draw_rectangle(
            OPENING.x + x,
            far_top,
            STEP + 1.0,
            OPENING_BOTTOM - far_top,
            far,
        );

        let near_top = HORIZON + 8.0 + 3.0 * (x * 0.02).sin();
        draw_rectangle(
            OPENING.x + x,
            near_top,
            STEP + 1.0,
            OPENING_BOTTOM - near_top,
            near,
        );
        x += STEP;
    }
}

/// Where the fence's posts stand on the grass.
const FENCE_BASE: f32 = HORIZON + 52.0;

fn draw_tree() {
    let x = OPENING.x + 420.0;
    let base = FENCE_BASE - 6.0;
    let leaf = Color::new(0.25, 0.55, 0.28, 1.0);

    draw_rectangle(
        x - 7.0,
        base - 90.0,
        14.0,
        90.0,
        Color::new(0.45, 0.3, 0.18, 1.0),
    );
    for (dx, dy, radius) in [
        (0.0, -88.0, 40.0),
        (-30.0, -72.0, 28.0),
        (30.0, -74.0, 29.0),
        (0.0, -112.0, 26.0),
    ] {
        draw_circle(x + dx, base + dy, radius, leaf);
    }
    // Apples.
    for (dx, dy) in [
        (-18.0, -92.0),
        (14.0, -80.0),
        (26.0, -100.0),
        (-34.0, -70.0),
        (2.0, -118.0),
    ] {
        draw_circle(x + dx, base + dy, 4.0, Color::new(0.9, 0.2, 0.2, 1.0));
    }

    // A bush at each side of the garden.
    let bush = Color::new(0.28, 0.55, 0.28, 1.0);
    for bush_x in [OPENING.x + 70.0, OPENING.x + 505.0] {
        draw_ellipse(bush_x, FENCE_BASE - 8.0, 46.0, 24.0, 0.0, bush);
    }
}

/// A white picket fence all along the garden.
fn draw_fence() {
    let board = Color::new(0.98, 0.96, 0.9, 1.0);
    let rail = Color::new(0.9, 0.87, 0.78, 1.0);
    let top = FENCE_BASE - 40.0;

    draw_rectangle(OPENING.x, FENCE_BASE - 28.0, OPENING.w, 5.0, rail);
    draw_rectangle(OPENING.x, FENCE_BASE - 12.0, OPENING.w, 5.0, rail);

    let mut x = OPENING.x + 10.0;
    while x < OPENING.x + OPENING.w {
        draw_rectangle(x - 5.0, top + 8.0, 10.0, 32.0, board);
        draw_triangle(
            vec2(x - 5.0, top + 8.0),
            vec2(x + 5.0, top + 8.0),
            vec2(x, top),
            board,
        );
        x += 20.0;
    }
}

/// Where each flower grows, as fractions across and down the lawn, and which
/// colour it is (an index into [`PETALS`]).
const FLOWERS: [(f32, f32, usize); 16] = [
    (0.04, 0.30, 0),
    (0.10, 0.70, 1),
    (0.17, 0.20, 3),
    (0.23, 0.85, 4),
    (0.30, 0.45, 2),
    (0.36, 0.10, 0),
    (0.42, 0.75, 1),
    (0.49, 0.35, 4),
    (0.55, 0.90, 3),
    (0.61, 0.15, 2),
    (0.68, 0.60, 0),
    (0.74, 0.30, 1),
    (0.81, 0.80, 4),
    (0.87, 0.20, 3),
    (0.93, 0.55, 2),
    (0.98, 0.85, 0),
];
const PETALS: [Color; 5] = [
    Color::new(0.92, 0.25, 0.3, 1.0),
    Color::new(1.0, 0.85, 0.2, 1.0),
    Color::new(0.98, 0.55, 0.75, 1.0),
    Color::new(0.98, 0.98, 0.98, 1.0),
    Color::new(0.65, 0.4, 0.85, 1.0),
];

/// The grass in front of the fence, with flowers swaying in the breeze.
fn draw_lawn(time: f32) {
    const STEP: f32 = 6.0;
    let grass = Color::new(0.38, 0.68, 0.3, 1.0);
    let lawn_top = FENCE_BASE + 4.0;
    let mut x = 0.0;
    while x < OPENING.w {
        let top = lawn_top + 4.0 * (x * 0.03 + 1.0).sin();
        draw_rectangle(OPENING.x + x, top, STEP + 1.0, OPENING_BOTTOM - top, grass);
        x += STEP;
    }

    let stem = Color::new(0.2, 0.5, 0.22, 1.0);
    for (across, down, kind) in FLOWERS {
        let base = vec2(
            OPENING.x + across * OPENING.w,
            lawn_top + 14.0 + down * (OPENING_BOTTOM - lawn_top - 24.0),
        );
        let sway = (time * 1.6 + across * 12.0).sin() * 2.5;
        let head = vec2(base.x + sway, base.y - 16.0);
        draw_line(base.x, base.y, head.x, head.y, 2.0, stem);
        for petal in 0..5 {
            let angle = petal as f32 * TAU / 5.0;
            draw_circle(
                head.x + angle.cos() * 4.5,
                head.y + angle.sin() * 4.5,
                3.6,
                PETALS[kind],
            );
        }
        draw_circle(head.x, head.y, 3.0, Color::new(1.0, 0.8, 0.1, 1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// How far a cloud's puffs and their shadows reach above and below its
    /// middle, at scale 1.0 (see `draw_clouds`).
    const CLOUD_REACH_UP: f32 = 34.0;
    const CLOUD_REACH_DOWN: f32 = 26.0;

    fn run(window: &mut Window, seconds: f32) {
        for _ in 0..(seconds / DT).round() as usize {
            window.update(DT);
        }
    }

    // --- Opening and closing ------------------------------------------

    #[test]
    fn a_new_window_is_shut() {
        let window = Window::default();
        assert!(!window.is_open());
        assert!(window.openness().abs() < f32::EPSILON);
    }

    #[test]
    fn clicking_opens_a_shut_window_and_shuts_an_open_one() {
        let mut window = Window::default();
        window.toggle();
        assert!(window.is_open());
        window.toggle();
        assert!(!window.is_open());
    }

    #[test]
    fn the_sashes_swing_open_over_a_moment_not_instantly() {
        let mut window = Window::default();
        window.toggle();

        window.update(DT);
        assert!(window.openness() > 0.0, "starts moving at once");
        assert!(window.openness() < 0.05, "but not jumping open");

        run(&mut window, SWING_SECONDS / 2.0);
        let half = window.openness();
        assert!((0.3..0.7).contains(&half), "half way, was {half}");

        run(&mut window, SWING_SECONDS);
        assert!((window.openness() - 1.0).abs() < 1e-6, "ends fully open");
    }

    #[test]
    fn it_stays_open_until_clicked_again_then_swings_shut() {
        let mut window = Window::default();
        window.toggle();
        run(&mut window, 3.0);
        assert!((window.openness() - 1.0).abs() < 1e-6);

        window.toggle();
        run(&mut window, SWING_SECONDS + 0.1);
        assert!(window.openness().abs() < 1e-6, "fully shut again");
    }

    #[test]
    fn clicking_part_way_reverses_from_where_the_sashes_are() {
        let mut window = Window::default();
        window.toggle();
        run(&mut window, SWING_SECONDS / 2.0);
        let before = window.openness();

        window.toggle();
        window.update(DT);
        let after = window.openness();

        assert!(after < before, "starts closing: {before} -> {after}");
        assert!(before - after < 0.1, "without a jump");
    }

    #[test]
    fn the_swing_is_smooth_and_stays_between_zero_and_one() {
        let mut window = Window::default();
        window.toggle();
        let mut last = 0.0;
        for _ in 0..(1.5 / DT) as usize {
            window.update(DT);
            let now = window.openness();
            assert!((0.0..=1.0).contains(&now));
            assert!(now >= last - 1e-6, "never goes backwards while opening");
            last = now;
        }
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-6);
        assert!(smoothstep(-1.0).abs() < 1e-6 && (smoothstep(2.0) - 1.0).abs() < 1e-6);
    }

    // --- Where to click -------------------------------------------------

    #[test]
    fn a_click_on_the_glass_frame_or_sill_hits_the_window() {
        let middle = vec2(OUTER.x + OUTER.w / 2.0, OUTER.y + OUTER.h / 2.0);
        assert!(Window::contains(middle));
        assert!(
            Window::contains(vec2(OUTER.x + 3.0, OUTER.y + 3.0)),
            "frame"
        );
        assert!(
            Window::contains(vec2(OUTER.x - 6.0, OUTER.y + OUTER.h + 5.0)),
            "sill"
        );
    }

    #[test]
    fn a_click_elsewhere_misses_it() {
        assert!(!Window::contains(vec2(100.0, 100.0)));
        assert!(!Window::contains(vec2(
            OUTER.x + OUTER.w + 40.0,
            OUTER.y + 50.0
        )));
        assert!(!Window::contains(vec2(
            OUTER.x + 100.0,
            world::FLOOR_TOP + 50.0
        )));
        assert!(!Window::contains(vec2(OUTER.x + 100.0, OUTER.y - 5.0)));
    }

    // --- The sashes -----------------------------------------------------

    fn assert_close(a: f32, b: f32, what: &str) {
        assert!((a - b).abs() < 0.01, "{what}: {a} vs {b}");
    }

    #[test]
    fn shut_sashes_meet_in_the_middle_and_fill_the_opening() {
        let (left, right) = (Sash::new(Side::Left, 0.0), Sash::new(Side::Right, 0.0));
        let middle = OPENING.x + OPENING.w / 2.0;

        assert_close(left.point(0.0, 0.0).x, OPENING.x, "left hinge");
        assert_close(
            right.point(0.0, 0.0).x,
            OPENING.x + OPENING.w,
            "right hinge",
        );
        assert_close(left.point(1.0, 0.5).x, middle, "left free edge");
        assert_close(right.point(1.0, 0.5).x, middle, "right free edge");
        // Exactly as tall as the opening.
        assert_close(left.point(1.0, 0.0).y, OPENING.y, "top");
        assert_close(left.point(1.0, 1.0).y, OPENING_BOTTOM, "bottom");
    }

    #[test]
    fn open_sashes_are_seen_almost_edge_on_leaving_the_garden_clear() {
        let (left, right) = (Sash::new(Side::Left, 1.0), Sash::new(Side::Right, 1.0));

        let each = OPENING.w / 2.0 * (1.0 - SASH_FORESHORTENING);
        assert_close(left.width(), each, "left width");
        assert_close(right.width(), each, "right width");

        let clear = right.point(1.0, 0.5).x - left.point(1.0, 0.5).x;
        assert!(
            clear > OPENING.w * 0.8,
            "clear gap {clear} of {}",
            OPENING.w
        );
    }

    #[test]
    fn opening_sashes_swing_towards_us_so_their_free_edge_grows() {
        let open = Sash::new(Side::Left, 1.0);
        assert_close(open.point(0.0, 0.0).y, OPENING.y, "hinge edge stays put");
        assert_close(
            open.point(1.0, 0.0).y,
            OPENING.y - SASH_GROWTH,
            "free edge top",
        );
        assert_close(
            open.point(1.0, 1.0).y,
            OPENING_BOTTOM + SASH_GROWTH,
            "free edge bottom",
        );
    }

    #[test]
    fn the_two_sashes_are_mirror_images() {
        for openness in [0.0, 0.3, 0.7, 1.0] {
            let (left, right) = (
                Sash::new(Side::Left, openness),
                Sash::new(Side::Right, openness),
            );
            for (u, v) in [(0.0, 0.0), (0.4, 0.3), (1.0, 0.9)] {
                let from_left = left.point(u, v).x - OPENING.x;
                let from_right = OPENING.x + OPENING.w - right.point(u, v).x;
                assert_close(from_left, from_right, "mirrored");
                assert_close(left.point(u, v).y, right.point(u, v).y, "same height");
            }
        }
    }

    #[test]
    fn sashes_get_steadily_narrower_as_the_window_opens() {
        let widths: Vec<f32> = (0..=10)
            .map(|step| Sash::new(Side::Left, step as f32 / 10.0).width())
            .collect();
        assert!(
            widths.windows(2).all(|pair| pair[1] < pair[0]),
            "{widths:?}"
        );
    }

    // --- The scene ------------------------------------------------------

    #[test]
    fn clouds_stay_within_their_band_and_drift_right() {
        for cloud in &CLOUDS {
            let (low, high) = (
                OPENING.x - CLOUD_MARGIN,
                OPENING.x + OPENING.w + CLOUD_MARGIN,
            );
            let mut last = cloud_x(cloud, 0.0);
            let mut wrapped = false;
            for second in 1..2000 {
                let x = cloud_x(cloud, second as f32);
                assert!((low..high).contains(&x), "x = {x}");
                wrapped |= x < last;
                last = x;
            }
            assert!(wrapped, "a cloud should come round again");
        }
    }

    #[test]
    fn a_cloud_moves_right_by_its_speed() {
        let cloud = &CLOUDS[0];
        let moved = cloud_x(cloud, 1.0) - cloud_x(cloud, 0.0);
        assert_close(moved, cloud.speed, "pixels per second");
    }

    #[test]
    fn clouds_float_inside_the_sky() {
        for cloud in &CLOUDS {
            let top = OPENING.y + cloud.height - CLOUD_REACH_UP * cloud.scale;
            let bottom = OPENING.y + cloud.height + CLOUD_REACH_DOWN * cloud.scale;
            assert!(top >= OPENING.y, "cloud pokes out of the top: {top}");
            assert!(bottom <= HORIZON, "cloud sinks into the hills: {bottom}");
        }
    }

    #[test]
    fn the_sunbeam_is_stronger_when_the_window_is_open() {
        assert!(sunbeam_alpha(1.0) > sunbeam_alpha(0.5));
        assert!(sunbeam_alpha(0.5) > sunbeam_alpha(0.0));
        assert!(sunbeam_alpha(1.0) < 0.5, "it is still a soft glow");
    }

    #[test]
    fn colours_blend_between_two_ends() {
        let middle = lerp_color(SKY_TOP, SKY_HORIZON, 0.5);
        assert_close(middle.r, f32::midpoint(SKY_TOP.r, SKY_HORIZON.r), "red");
        assert_close(middle.g, f32::midpoint(SKY_TOP.g, SKY_HORIZON.g), "green");
        let start = lerp_color(SKY_TOP, SKY_HORIZON, 0.0);
        assert_close(start.b, SKY_TOP.b, "start");
    }
}
