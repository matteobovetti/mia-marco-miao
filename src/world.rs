//! The room: layout constants and static scenery drawing.

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::home;
use crate::sofa;
use crate::supplies::{Bowl, Food, LitterBox, Supplies, WaterBowl};
use crate::window::Window;

/// Size of the canvas everything is drawn on (16:9). The `view` module scales
/// it to fit the window, so these are not pixel sizes of the window.
pub const WIDTH: f32 = 1280.0;
pub const HEIGHT: f32 = 720.0;
/// Y coordinate where the wall ends and the floor begins.
pub const FLOOR_TOP: f32 = 310.0;

// Objects (where they are drawn).
pub const BOWL: Vec2 = vec2(205.0, 480.0);
pub const WATER: Vec2 = vec2(205.0, 636.0);
/// Centre of the litter box (a tray 170 wide and 74 tall).
pub const LITTER: Vec2 = vec2(420.0, 625.0);

// Spots where the cat stands to use each object (cat position = its paws).
pub const BOWL_SPOT: Vec2 = vec2(270.0, 490.0);
pub const WATER_SPOT: Vec2 = vec2(273.0, 646.0);
/// Inside the litter box.
pub const LITTER_SPOT: Vec2 = vec2(420.0, 642.0);
/// Middle of the room, where the cat sits to ask for attention.
pub const PLEA_SPOT: Vec2 = vec2(640.0, 480.0);
/// Where the cat starts.
pub const CAT_START: Vec2 = vec2(WIDTH / 2.0, 480.0);
/// Where the second cat starts, left of the first.
pub const SECOND_CAT_START: Vec2 = vec2(460.0, 520.0);

/// Which floor the cat is on. The ground floor is the floor of the room itself
/// (and of the downstairs of the cat's home); the upper floor is the one
/// upstairs in the home, which the cat reaches by the ladder. The top of the
/// sofa is its own little level, which the cat jumps up to from the floor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Ground,
    Upper,
    Sofa,
}

/// Somewhere the cat can go to do something.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Place {
    /// Where the cat stands (its paws).
    pub spot: Vec2,
    pub level: Level,
    /// What it faces when it gets there.
    pub object: Vec2,
}

impl Place {
    pub const fn ground(spot: Vec2, object: Vec2) -> Self {
        Self {
            spot,
            level: Level::Ground,
            object,
        }
    }

    pub const fn upper(spot: Vec2, object: Vec2) -> Self {
        Self {
            spot,
            level: Level::Upper,
            object,
        }
    }

    pub const fn sofa(spot: Vec2, object: Vec2) -> Self {
        Self {
            spot,
            level: Level::Sofa,
            object,
        }
    }
}

/// The colour of the back wall. The window paints it over the clouds that
/// drift past its edges.
pub const WALL: Color = Color::new(0.96, 0.89, 0.78, 1.0);
const SHADOW: Color = Color::new(0.0, 0.0, 0.0, 0.2);

/// A random place on the room's floor for the cat to wander to: anywhere but
/// where the cat's home stands.
pub fn random_floor_point() -> Vec2 {
    for _ in 0..20 {
        let point = vec2(
            gen_range(60.0, WIDTH - 60.0),
            gen_range(FLOOR_TOP + 70.0, HEIGHT - 20.0),
        );
        if !home::KEEP_CLEAR.contains(point) {
            return point;
        }
    }
    vec2(60.0, HEIGHT - 20.0)
}

/// The spot on the room's floor where a cat can stand that is nearest to
/// `point`, which may be anywhere on the canvas (on the wall, say): `point`
/// itself if the cat can stand there, otherwise the closest spot on the floor
/// that is clear of the cat's home.
pub fn floor_point_near(point: Vec2) -> Vec2 {
    let point = vec2(
        point.x.clamp(60.0, WIDTH - 60.0),
        point.y.clamp(FLOOR_TOP + 70.0, HEIGHT - 20.0),
    );
    let home = home::KEEP_CLEAR;
    if !home.contains(point) {
        return point;
    }
    // Out through the nearest side that isn't the roof (there is no floor
    // above the home).
    [
        vec2(home.x - 1.0, point.y),
        vec2(home.x + home.w + 1.0, point.y),
        vec2(point.x, home.y + home.h + 1.0),
    ]
    .into_iter()
    .min_by(|a, b| a.distance(point).total_cmp(&b.distance(point)))
    .unwrap_or(point)
}

/// Draws the room.
pub fn draw(supplies: &Supplies, window: Window) {
    draw_walls_and_floor();
    window.draw();
    sofa::draw();
    home::draw();
    draw_bowl(&supplies.food);
    draw_water_bowl(&supplies.water);
    draw_litter_box(&supplies.litter);
}

fn ellipse(center: Vec2, rx: f32, ry: f32, color: Color) {
    draw_ellipse(center.x, center.y, rx, ry, 0.0, color);
}

fn draw_walls_and_floor() {
    // Not `clear_background`: that would paint over the black bars around the
    // canvas too.
    draw_rectangle(0.0, 0.0, WIDTH, HEIGHT, WALL);
    let floor_height = HEIGHT - FLOOR_TOP;
    draw_rectangle(
        0.0,
        FLOOR_TOP,
        WIDTH,
        floor_height,
        Color::new(0.72, 0.52, 0.34, 1.0),
    );
    // Baseboard.
    draw_rectangle(
        0.0,
        FLOOR_TOP - 8.0,
        WIDTH,
        8.0,
        Color::new(0.55, 0.38, 0.24, 1.0),
    );
    // Floorboards.
    let mut y = FLOOR_TOP + 52.0;
    while y < HEIGHT {
        draw_line(0.0, y, WIDTH, y, 2.0, Color::new(0.6, 0.42, 0.27, 0.6));
        y += 52.0;
    }
}

/// Little bits of food scattered over the mound, relative to its centre.
const CRUMBS: [Vec2; 5] = [
    vec2(-10.0, -1.0),
    vec2(-3.0, -4.0),
    vec2(5.0, -2.0),
    vec2(11.0, 0.0),
    vec2(0.0, 1.0),
];

fn shade(color: Color, factor: f32) -> Color {
    Color::new(
        color.r * factor,
        color.g * factor,
        color.b * factor,
        color.a,
    )
}

/// The bowl is visibly empty until the player puts food in it; the food
/// shrinks as the cat eats.
fn draw_bowl(bowl: &Bowl) {
    ellipse(BOWL + vec2(0.0, 8.0), 36.0, 12.0, SHADOW);
    ellipse(BOWL, 34.0, 14.0, Color::new(0.85, 0.3, 0.35, 1.0));

    let inside = BOWL - vec2(0.0, 3.0);
    ellipse(inside, 26.0, 9.0, Color::new(0.5, 0.2, 0.25, 1.0));

    let Some((food, fullness)) = bowl.contents() else {
        return;
    };
    // Food covers the dish; solid food also piles up in a mound.
    ellipse(inside, 26.0 * (0.6 + 0.4 * fullness), 9.0, food.color());
    if food != Food::Milk {
        let mound = inside - vec2(0.0, 3.0 * fullness);
        ellipse(
            mound,
            6.0 + 16.0 * fullness,
            3.0 + 6.0 * fullness,
            food.color(),
        );
        for crumb in CRUMBS {
            let at = mound + crumb * fullness;
            draw_circle(at.x, at.y, 2.0, shade(food.color(), 0.7));
        }
    }
}

/// Like the food bowl, visibly empty until filled; the water level drops as
/// the cat drinks.
fn draw_water_bowl(water: &WaterBowl) {
    ellipse(WATER + vec2(0.0, 8.0), 36.0, 12.0, SHADOW);
    ellipse(WATER, 34.0, 14.0, Color::new(0.3, 0.5, 0.8, 1.0));

    let inside = WATER - vec2(0.0, 3.0);
    ellipse(inside, 26.0, 9.0, Color::new(0.2, 0.3, 0.5, 1.0));

    if water.is_empty() {
        return;
    }
    let scale = 0.55 + 0.45 * water.level();
    ellipse(
        inside,
        26.0 * scale,
        9.0 * scale,
        Color::new(0.55, 0.8, 0.95, 1.0),
    );
    // A little shimmer drifting across the surface.
    let drift = (get_time() as f32 * 1.5).sin() * 8.0 * scale;
    ellipse(
        inside + vec2(drift, -2.0),
        6.0 * scale,
        2.0,
        Color::new(1.0, 1.0, 1.0, 0.6),
    );
}

/// Dark flecks in the sand, relative to the centre of the box.
const SAND_SPECKS: [Vec2; 8] = [
    vec2(-60.0, -15.0),
    vec2(-35.0, 10.0),
    vec2(-12.0, -18.0),
    vec2(8.0, 14.0),
    vec2(30.0, -8.0),
    vec2(52.0, 16.0),
    vec2(58.0, -17.0),
    vec2(-52.0, 18.0),
];

/// The clumps the cat leaves behind, relative to the centre of the box.
const CLUMPS: [Vec2; 3] = [vec2(-40.0, -4.0), vec2(-2.0, 9.0), vec2(34.0, -7.0)];

/// Where the sparkles fly after cleaning, relative to the centre of the box.
const SPARKLES: [Vec2; 4] = [
    vec2(-52.0, -6.0),
    vec2(-17.0, -22.0),
    vec2(22.0, -12.0),
    vec2(54.0, -2.0),
];

/// A tray of sand. Clean it looks raked and fresh; after the cat has used it,
/// it has clumps and a smell, until the player cleans it.
fn draw_litter_box(litter: &LitterBox) {
    let Vec2 { x, y } = LITTER;
    let sand = Color::new(0.93, 0.86, 0.65, 1.0);
    let sand_dark = Color::new(0.75, 0.66, 0.45, 1.0);

    ellipse(LITTER + vec2(0.0, 42.0), 96.0, 14.0, SHADOW);
    draw_rectangle(
        x - 85.0,
        y - 37.0,
        170.0,
        74.0,
        Color::new(0.35, 0.6, 0.68, 1.0),
    );
    draw_rectangle_lines(
        x - 85.0,
        y - 37.0,
        170.0,
        74.0,
        4.0,
        Color::new(0.24, 0.43, 0.5, 1.0),
    );
    draw_rectangle(x - 74.0, y - 26.0, 148.0, 52.0, sand);
    for speck in SAND_SPECKS {
        draw_circle(x + speck.x, y + speck.y, 1.8, sand_dark);
    }

    if litter.is_dirty() {
        for clump in CLUMPS {
            ellipse(LITTER + clump, 10.0, 6.5, Color::new(0.45, 0.3, 0.18, 1.0));
        }
        // Wavy stink lines rising from the box.
        let time = get_time() as f32;
        let stink = Color::new(0.45, 0.62, 0.2, 0.75);
        for i in 0..3 {
            let sx = x - 32.0 + i as f32 * 32.0;
            let sway = (time * 3.0 + i as f32 * 1.7).sin() * 6.0;
            draw_line(sx, y - 42.0, sx + sway, y - 64.0, 3.0, stink);
            draw_line(sx + sway, y - 64.0, sx, y - 86.0, 3.0, stink);
        }
    } else {
        // Freshly raked lines.
        for row in 0..4 {
            let ry = y - 18.0 + row as f32 * 12.0;
            draw_line(
                x - 66.0,
                ry,
                x + 66.0,
                ry,
                1.5,
                Color {
                    a: 0.5,
                    ..sand_dark
                },
            );
        }
    }

    if let Some(progress) = litter.sparkle() {
        draw_sparkles(progress);
    }
}

/// Little stars that rise and fade over the litter box, `progress` being how
/// far through the sparkle we are (`0.0..1.0`).
fn draw_sparkles(progress: f32) {
    let alpha = 1.0 - progress;
    let colour = Color::new(1.0, 0.95, 0.5, alpha);
    for (i, offset) in SPARKLES.into_iter().enumerate() {
        // Each star starts a little later than the one before.
        let rise = (progress * 40.0) + i as f32 * 4.0;
        let at = LITTER + offset - vec2(0.0, rise);
        let size = 6.0 + (i % 2) as f32 * 3.0;
        draw_line(at.x - size, at.y, at.x + size, at.y, 2.5, colour);
        draw_line(at.x, at.y - size, at.x, at.y + size, 2.5, colour);
        draw_circle(at.x, at.y, 2.5, Color { a: alpha, ..WHITE });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spot_on_the_open_floor_is_where_the_cat_goes() {
        let spot = vec2(300.0, 500.0);
        assert_eq!(floor_point_near(spot), spot);
    }

    #[test]
    fn a_spot_on_the_wall_is_the_floor_below_it() {
        let point = floor_point_near(vec2(500.0, 40.0));
        assert!((point.x - 500.0).abs() < 0.01);
        assert!(point.y >= FLOOR_TOP);
    }

    #[test]
    fn a_spot_in_the_home_or_off_the_canvas_is_somewhere_a_cat_can_stand() {
        for y in (-50..800).step_by(25) {
            for x in (-50..1350).step_by(25) {
                let point = floor_point_near(vec2(x as f32, y as f32));
                assert!(!home::KEEP_CLEAR.contains(point), "{point} is in the home");
                assert!(
                    (0.0..=WIDTH).contains(&point.x),
                    "{point} is off the canvas"
                );
                assert!(
                    (FLOOR_TOP..=HEIGHT).contains(&point.y),
                    "{point} is not on the floor"
                );
            }
        }
    }
}
