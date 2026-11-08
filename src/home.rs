//! The cat's two-storey home, and the ways round it and up it.
//!
//! The home is drawn as a cutaway, so both floors can be seen inside. The
//! ground floor is open at the front: the cat walks in from the room. The
//! upper floor is reached by the ladder on the left and left by the slide on
//! the right.
//!
//! The room's floor is flat, so a cat crossing the room in a straight line
//! would be drawn straight through the house. [`ground_waypoint`] plans a way
//! round it instead, and in at the front for the cushion downstairs.

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::world::{self, Place};

// --- The house ----------------------------------------------------------

const LEFT: f32 = 780.0;
const RIGHT: f32 = 1080.0;
const WALL_THICKNESS: f32 = 12.0;

/// Where the cat's paws are on the ground floor and upstairs.
pub const GROUND_FLOOR: f32 = 575.0;
pub const UPPER_FLOOR: f32 = 445.0;
/// The upstairs floor is a slab this thick (it is also the downstairs ceiling).
const SLAB: f32 = 12.0;
const BASE: f32 = 10.0;
/// The underside of the roof, above the upper floor.
const CEILING: f32 = 335.0;
const ROOF_APEX: Vec2 = vec2(LEFT + (RIGHT - LEFT) / 2.0, 294.0);
/// How tall the doorways on the upper floor are.
const DOORWAY_HEIGHT: f32 = 52.0;

/// The room inside the walls.
pub const INTERIOR_LEFT: f32 = LEFT + WALL_THICKNESS;
pub const INTERIOR_RIGHT: f32 = RIGHT - WALL_THICKNESS;

// The window above must stay clear of the roof, and each floor must be tall
// enough for the cat to stand up in.
const _: () = assert!(ROOF_APEX.y > 290.0);
const _: () = assert!(GROUND_FLOOR - (UPPER_FLOOR + SLAB) > 100.0);
const _: () = assert!(UPPER_FLOOR - CEILING > 100.0);

// --- Getting round it ---------------------------------------------------

/// The part of the room the home takes up, as the ground-level cat sees it:
/// it can't be walked across, only round or into.
pub const BLOCK: Rect = Rect {
    x: LEFT - 20.0,
    y: 288.0,
    w: 340.0,
    h: 320.0,
};
/// The cat keeps this far from the home when it walks past.
const CLEARANCE: f32 = 8.0;
/// The block and the clearance round it: nothing the cat walks to may be
/// in here, unless it is inside the home.
pub const KEEP_CLEAR: Rect = Rect {
    x: BLOCK.x - CLEARANCE,
    y: BLOCK.y - CLEARANCE,
    w: BLOCK.w + 2.0 * CLEARANCE,
    h: BLOCK.h + 2.0 * CLEARANCE,
};
/// Walking along here, just below the home, gets past it.
pub const CORRIDOR_Y: f32 = 622.0;
const WAYPOINT_LEFT_X: f32 = BLOCK.x - 24.0;
const WAYPOINT_LEFT: Vec2 = vec2(WAYPOINT_LEFT_X, CORRIDOR_Y);
const WAYPOINT_RIGHT: Vec2 = vec2(BLOCK.x + BLOCK.w + 24.0, CORRIDOR_Y);

const _: () = assert!(KEEP_CLEAR.y + KEEP_CLEAR.h < CORRIDOR_Y);
const _: () = assert!(WAYPOINT_LEFT_X < KEEP_CLEAR.x);
const _: () = assert!(BLOCK.x < LEFT && BLOCK.x + BLOCK.w > RIGHT);

// --- The ladder and the slide -------------------------------------------

/// Where the cat's paws are at the foot and at the top of the ladder, which
/// leans on the left side of the home.
pub const LADDER_BASE: Vec2 = vec2(LEFT - 44.0, 583.0);
pub const LADDER_TOP: Vec2 = vec2(INTERIOR_LEFT + 10.0, UPPER_FLOOR);
/// Where the cat steps onto the slide upstairs, the edge it slides over, and
/// where it lands at the bottom, on the right of the home.
pub const SLIDE_TOP: Vec2 = vec2(INTERIOR_RIGHT - 4.0, UPPER_FLOOR);
const SLIDE_SILL: Vec2 = vec2(RIGHT + 2.0, UPPER_FLOOR + 2.0);
pub const SLIDE_END: Vec2 = vec2(RIGHT + 132.0, 578.0);
/// The slide's far end, where the chute meets the floor.
const SLIDE_FOOT: Vec2 = vec2(RIGHT + 140.0, 586.0);
const SLIDE_THICKNESS: f32 = 10.0;

const _: () = assert!(LADDER_BASE.x < BLOCK.x);
const _: () = assert!(LADDER_TOP.x > INTERIOR_LEFT);
const _: () = assert!(SLIDE_END.x > BLOCK.x + BLOCK.w);
// The slide is a long way down.
const _: () = assert!(SLIDE_END.y > SLIDE_TOP.y + 100.0);

// --- Places in the home -------------------------------------------------

/// A cushion on the ground floor, and another upstairs, for a nap.
const GROUND_BED_AT: Vec2 = vec2(LEFT + 90.0, GROUND_FLOOR);
const UPPER_BED_AT: Vec2 = vec2(LEFT + 210.0, UPPER_FLOOR);
pub const GROUND_BED: Place = Place::ground(GROUND_BED_AT, GROUND_BED_AT);
pub const UPPER_BED: Place = Place::upper(UPPER_BED_AT, UPPER_BED_AT);

/// Somewhere to wander to: mostly the room's floor, but sometimes inside the
/// home, on either floor.
pub fn random_place() -> Place {
    let roll = gen_range(0.0, 1.0);
    let x = gen_range(INTERIOR_LEFT + 60.0, INTERIOR_RIGHT - 60.0);
    let (spot, upstairs) = if roll < 0.15 {
        (vec2(x, UPPER_FLOOR), true)
    } else if roll < 0.30 {
        (vec2(x, GROUND_FLOOR), false)
    } else {
        (world::random_floor_point(), false)
    };
    if upstairs {
        Place::upper(spot, spot)
    } else {
        Place::ground(spot, spot)
    }
}

// --- Planning a route on the ground floor -------------------------------

/// Whether the straight line from `a` to `b` goes through `rect` (merely
/// touching its edge doesn't count).
fn crosses(a: Vec2, b: Vec2, rect: Rect) -> bool {
    // Liang-Barsky: clip the line to each side of the rectangle in turn.
    let delta = b - a;
    let (mut enters, mut leaves) = (0.0_f32, 1.0_f32);
    for (p, q) in [
        (-delta.x, a.x - rect.x),
        (delta.x, rect.x + rect.w - a.x),
        (-delta.y, a.y - rect.y),
        (delta.y, rect.y + rect.h - a.y),
    ] {
        if p.abs() < 1e-9 {
            // Parallel to this side: it misses if it is outside it, or along it.
            if q <= 0.0 {
                return false;
            }
        } else {
            let at = q / p;
            if p < 0.0 {
                if at > leaves {
                    return false;
                }
                enters = enters.max(at);
            } else {
                if at < enters {
                    return false;
                }
                leaves = leaves.min(at);
            }
        }
    }
    enters < leaves
}

fn clear(a: Vec2, b: Vec2) -> bool {
    !crosses(a, b, KEEP_CLEAR)
}

/// The first point to head for on the way from `from` to `goal`, both out in
/// the room: `goal` itself if the way is clear, otherwise the corner of the
/// shortest way round below the home.
fn first_hop(from: Vec2, goal: Vec2) -> Vec2 {
    if clear(from, goal) {
        return goal;
    }
    // A corner we are standing on is no use as a first stop.
    let corners: Vec<Vec2> = [WAYPOINT_LEFT, WAYPOINT_RIGHT]
        .into_iter()
        .filter(|corner| corner.distance(from) > 1.0)
        .collect();

    let mut best: Option<(f32, Vec2)> = None;
    let mut consider = |length: f32, first: Vec2| {
        if best.is_none_or(|(shortest, _)| length < shortest) {
            best = Some((length, first));
        }
    };
    for &corner in &corners {
        if clear(from, corner) && clear(corner, goal) {
            consider(from.distance(corner) + corner.distance(goal), corner);
        }
    }
    for &first in &corners {
        let second = if first == WAYPOINT_LEFT {
            WAYPOINT_RIGHT
        } else {
            WAYPOINT_LEFT
        };
        if clear(from, first) && clear(first, second) && clear(second, goal) {
            let length = from.distance(first) + first.distance(second) + second.distance(goal);
            consider(length, first);
        }
    }
    best.map_or(goal, |(_, first)| first)
}

/// The next point a cat on the ground floor should walk to on its way from
/// `from` to `to`. It is `to` itself unless the home is in the way.
///
/// The cat never crosses the home: it goes round below it, steps in at the
/// front to reach the cushion downstairs, and steps out the same way.
pub fn ground_waypoint(from: Vec2, to: Vec2) -> Vec2 {
    if KEEP_CLEAR.contains(from) {
        // Inside the home, or right against it. Straight on to somewhere
        // inside; otherwise down and out of the front, to where the way round
        // can be planned.
        return if BLOCK.contains(to) {
            to
        } else {
            vec2(from.x, CORRIDOR_Y)
        };
    }
    // Somewhere inside the home is reached from the doorstep below it: from
    // there, straight up.
    if BLOCK.contains(to) && (from.x - to.x).abs() <= 1.0 && from.y > BLOCK.y + BLOCK.h {
        return to;
    }
    let goal = if BLOCK.contains(to) {
        vec2(to.x, CORRIDOR_Y)
    } else {
        to
    };
    first_hop(from, goal)
}

// --- Up the ladder and down the slide ------------------------------------

/// Where the cat is `progress` (`0.0..=1.0`) of the way up the ladder.
pub fn ladder_position(progress: f32) -> Vec2 {
    LADDER_BASE.lerp(LADDER_TOP, progress.clamp(0.0, 1.0))
}

/// Where the cat is `progress` (`0.0..=1.0`) of the way through its slide:
/// it pushes off slowly and speeds up on the way down.
pub fn slide_position(progress: f32) -> Vec2 {
    let eased = progress.clamp(0.0, 1.0).powi(2);
    let (push, ramp) = (
        SLIDE_TOP.distance(SLIDE_SILL),
        SLIDE_SILL.distance(SLIDE_END),
    );
    let travelled = eased * (push + ramp);
    if travelled <= push {
        SLIDE_TOP.lerp(SLIDE_SILL, travelled / push)
    } else {
        SLIDE_SILL.lerp(SLIDE_END, (travelled - push) / ramp)
    }
}

// --- Drawing --------------------------------------------------------------

const SHADOW: Color = Color::new(0.0, 0.0, 0.0, 0.2);
const OUTSIDE_WALL: Color = Color::new(0.88, 0.64, 0.48, 1.0);
const TRIM: Color = Color::new(0.62, 0.42, 0.28, 1.0);
const WOOD: Color = Color::new(0.75, 0.55, 0.35, 1.0);
const INTERIOR_DOWN: Color = Color::new(0.99, 0.94, 0.84, 1.0);
const INTERIOR_UP: Color = Color::new(0.93, 0.9, 0.99, 1.0);

/// Draws the home, the ladder and the slide.
pub fn draw() {
    draw_interior();
    draw_floors();
    draw_cushions();
    draw_side_walls();
    draw_roof();
    draw_ladder();
    draw_slide();
}

fn fill_quad(corners: [Vec2; 4], color: Color) {
    draw_triangle(corners[0], corners[1], corners[2], color);
    draw_triangle(corners[0], corners[2], corners[3], color);
}

/// The back walls of both floors, with a round window upstairs and a paw
/// print on the wall downstairs.
fn draw_interior() {
    let width = INTERIOR_RIGHT - INTERIOR_LEFT;
    let slab_bottom = UPPER_FLOOR + SLAB;
    draw_rectangle(
        INTERIOR_LEFT,
        CEILING,
        width,
        UPPER_FLOOR - CEILING,
        INTERIOR_UP,
    );
    draw_rectangle(
        INTERIOR_LEFT,
        slab_bottom,
        width,
        GROUND_FLOOR - slab_bottom,
        INTERIOR_DOWN,
    );

    // A round window upstairs.
    let window = vec2(LEFT + (RIGHT - LEFT) / 2.0, 392.0);
    draw_circle(window.x, window.y, 28.0, TRIM);
    draw_circle(window.x, window.y, 23.0, Color::new(0.6, 0.82, 0.98, 1.0));
    draw_line(
        window.x - 23.0,
        window.y,
        window.x + 23.0,
        window.y,
        3.0,
        TRIM,
    );
    draw_line(
        window.x,
        window.y - 23.0,
        window.x,
        window.y + 23.0,
        3.0,
        TRIM,
    );

    // A paw print on the wall downstairs.
    let paw = Color::new(0.85, 0.7, 0.6, 1.0);
    let pad = vec2(window.x, 528.0);
    draw_circle(pad.x, pad.y, 14.0, paw);
    for toe in [(-24.0, -20.0), (-8.0, -30.0), (8.0, -30.0), (24.0, -20.0)] {
        draw_circle(pad.x + toe.0, pad.y + toe.1, 7.0, paw);
    }
}

/// The upstairs floor, which is also the downstairs ceiling, and the base.
fn draw_floors() {
    draw_rectangle(LEFT, UPPER_FLOOR, RIGHT - LEFT, SLAB, WOOD);
    draw_rectangle(
        LEFT,
        UPPER_FLOOR,
        RIGHT - LEFT,
        3.0,
        Color::new(1.0, 1.0, 1.0, 0.3),
    );
    draw_rectangle(
        LEFT,
        UPPER_FLOOR + SLAB,
        RIGHT - LEFT,
        3.0,
        Color::new(0.0, 0.0, 0.0, 0.12),
    );

    draw_rectangle(LEFT - 6.0, GROUND_FLOOR, RIGHT - LEFT + 12.0, BASE, TRIM);
    draw_ellipse(
        LEFT + (RIGHT - LEFT) / 2.0,
        GROUND_FLOOR + BASE + 6.0,
        (RIGHT - LEFT) / 2.0 + 10.0,
        8.0,
        0.0,
        SHADOW,
    );
}

fn draw_cushions() {
    for (place, outer, inner) in [
        (
            GROUND_BED,
            Color::new(0.88, 0.45, 0.5, 1.0),
            Color::new(0.97, 0.7, 0.72, 1.0),
        ),
        (
            UPPER_BED,
            Color::new(0.4, 0.55, 0.88, 1.0),
            Color::new(0.68, 0.78, 0.97, 1.0),
        ),
    ] {
        let at = place.spot - vec2(0.0, 4.0);
        draw_ellipse(at.x, at.y, 56.0, 12.0, 0.0, outer);
        draw_ellipse(at.x, at.y - 2.0, 42.0, 8.0, 0.0, inner);
    }
}

/// The walls at each side. Upstairs each has a doorway at floor level, for
/// the ladder on the left and the slide on the right.
fn draw_side_walls() {
    let doorway_top = UPPER_FLOOR - DOORWAY_HEIGHT;
    for x in [LEFT, RIGHT - WALL_THICKNESS] {
        draw_rectangle(
            x,
            CEILING,
            WALL_THICKNESS,
            doorway_top - CEILING,
            OUTSIDE_WALL,
        );
        draw_rectangle(
            x,
            UPPER_FLOOR,
            WALL_THICKNESS,
            GROUND_FLOOR - UPPER_FLOOR,
            OUTSIDE_WALL,
        );
        // The frame round the doorway.
        draw_rectangle(x - 2.0, doorway_top - 4.0, WALL_THICKNESS + 4.0, 4.0, TRIM);
    }
}

fn draw_roof() {
    let eave = 18.0;
    let roof = Color::new(0.72, 0.3, 0.28, 1.0);
    draw_triangle(
        vec2(LEFT - eave, CEILING),
        vec2(RIGHT + eave, CEILING),
        ROOF_APEX,
        roof,
    );
    draw_triangle(
        vec2(LEFT - eave, CEILING),
        vec2(ROOF_APEX.x, CEILING),
        ROOF_APEX,
        Color::new(0.0, 0.0, 0.0, 0.1),
    );
    draw_rectangle(
        LEFT - eave,
        CEILING - 2.0,
        RIGHT - LEFT + 2.0 * eave,
        8.0,
        Color::new(0.5, 0.2, 0.2, 1.0),
    );
}

/// The ladder: two rails with rungs, leaning on the left of the home.
fn draw_ladder() {
    let rail = Color::new(0.65, 0.45, 0.25, 1.0);
    let dark = Color::new(0.45, 0.3, 0.17, 1.0);
    let spread = vec2(22.0, 0.0);
    // The rails reach a little beyond the cat's feet at both ends.
    let (bottom, top) = (LADDER_BASE + vec2(0.0, 10.0), LADDER_TOP + vec2(8.0, -4.0));

    for offset in [-spread / 2.0, spread / 2.0] {
        let (a, b) = (bottom + offset, top + offset);
        draw_line(a.x, a.y, b.x, b.y, 6.0, rail);
    }
    for rung in 1..=6 {
        let along = rung as f32 / 7.0;
        let centre = bottom.lerp(top, along);
        let (a, b) = (centre - spread / 2.0, centre + spread / 2.0);
        draw_line(a.x, a.y, b.x, b.y, 4.0, dark);
    }
}

/// The slide: a yellow chute from the right doorway down to the floor, held
/// up by two blue legs and edged with blue rails.
fn draw_slide() {
    let surface = Color::new(0.98, 0.8, 0.2, 1.0);
    let blue = Color::new(0.3, 0.5, 0.85, 1.0);
    let (start, foot) = (SLIDE_SILL, SLIDE_FOOT);

    // Legs, under the chute.
    for along in [0.35, 0.7] {
        let top = start.lerp(foot, along);
        draw_line(
            top.x,
            top.y + SLIDE_THICKNESS,
            top.x,
            foot.y + 4.0,
            5.0,
            blue,
        );
    }
    fill_quad(
        [
            start,
            foot,
            foot + vec2(0.0, SLIDE_THICKNESS),
            start + vec2(0.0, SLIDE_THICKNESS),
        ],
        surface,
    );
    draw_line(start.x, start.y - 3.0, foot.x, foot.y - 3.0, 4.0, blue);
    draw_line(
        start.x,
        start.y + SLIDE_THICKNESS,
        foot.x,
        foot.y + SLIDE_THICKNESS,
        3.0,
        Color::new(0.8, 0.6, 0.1, 1.0),
    );
    draw_ellipse(foot.x + 4.0, foot.y + 16.0, 30.0, 6.0, 0.0, SHADOW);
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: f32 = 8.0;

    // --- Does a line go through the home? ---------------------------------

    #[test]
    fn a_line_through_the_middle_crosses_the_rectangle() {
        let rect = Rect::new(10.0, 10.0, 20.0, 20.0);
        assert!(crosses(vec2(0.0, 20.0), vec2(40.0, 20.0), rect));
        assert!(crosses(vec2(20.0, 0.0), vec2(20.0, 40.0), rect));
        assert!(crosses(vec2(0.0, 0.0), vec2(40.0, 40.0), rect));
    }

    #[test]
    fn a_line_beside_the_rectangle_does_not() {
        let rect = Rect::new(10.0, 10.0, 20.0, 20.0);
        assert!(!crosses(vec2(0.0, 5.0), vec2(40.0, 5.0), rect), "above");
        assert!(!crosses(vec2(0.0, 35.0), vec2(40.0, 35.0), rect), "below");
        assert!(!crosses(vec2(5.0, 0.0), vec2(5.0, 40.0), rect), "left");
        assert!(
            !crosses(vec2(0.0, 65.0), vec2(65.0, 0.0), rect),
            "diagonal past the far corner"
        );
    }

    #[test]
    fn a_line_that_stops_short_or_only_touches_does_not_cross() {
        let rect = Rect::new(10.0, 10.0, 20.0, 20.0);
        assert!(
            !crosses(vec2(0.0, 20.0), vec2(8.0, 20.0), rect),
            "stops short"
        );
        assert!(
            !crosses(vec2(0.0, 10.0), vec2(40.0, 10.0), rect),
            "along an edge"
        );
    }

    #[test]
    fn a_line_that_starts_or_ends_inside_crosses() {
        let rect = Rect::new(10.0, 10.0, 20.0, 20.0);
        assert!(crosses(vec2(20.0, 20.0), vec2(100.0, 100.0), rect));
        assert!(crosses(vec2(0.0, 0.0), vec2(20.0, 20.0), rect));
    }

    // --- Walking round the home -------------------------------------------

    /// Follows the waypoints from `from` to `to`, `STEP` units at a time,
    /// like the cat does. Returns every point it stands on.
    fn walk(from: Vec2, to: Vec2) -> Vec<Vec2> {
        let mut here = from;
        let mut path = vec![here];
        for _ in 0..2000 {
            let waypoint = ground_waypoint(here, to);
            let delta = waypoint - here;
            if delta.length() <= STEP {
                here = waypoint;
                path.push(here);
                if waypoint == to {
                    return path;
                }
            } else {
                here += delta.normalize() * STEP;
                path.push(here);
            }
        }
        panic!("never got from {from} to {to}; stuck near {here}");
    }

    /// Whether `point` is somewhere a cat may be inside the home: on its
    /// floor, between its walls.
    fn inside_the_home_properly(point: Vec2) -> bool {
        point.x >= INTERIOR_LEFT - 0.01
            && point.x <= INTERIOR_RIGHT + 0.01
            && point.y >= GROUND_FLOOR - 0.01
    }

    #[test]
    fn an_open_way_is_walked_straight() {
        let (from, to) = (vec2(100.0, 400.0), vec2(300.0, 500.0));
        assert_eq!(ground_waypoint(from, to), to);
        assert_eq!(walk(from, to).last(), Some(&to));
    }

    #[test]
    fn crossing_the_room_goes_round_below_the_home() {
        let (from, to) = (vec2(640.0, 480.0), vec2(1150.0, 480.0));
        assert_eq!(
            ground_waypoint(from, to),
            WAYPOINT_LEFT,
            "goes left-below first"
        );

        let path = walk(from, to);
        assert_eq!(path.last(), Some(&to));
        assert!(
            path.iter().all(|p| !BLOCK.contains(*p)),
            "walked through the home"
        );
    }

    #[test]
    fn it_goes_round_whichever_way_it_is_crossing() {
        let (left, right) = (vec2(600.0, 500.0), vec2(1150.0, 500.0));
        for (from, to) in [(left, right), (right, left)] {
            let path = walk(from, to);
            assert!(path.iter().all(|p| !BLOCK.contains(*p)), "{from} -> {to}");
            assert!(
                path.iter().any(
                    |p| p.y > BLOCK.y + BLOCK.h && (BLOCK.x..=BLOCK.x + BLOCK.w).contains(&p.x)
                ),
                "should pass below the home"
            );
        }
    }

    #[test]
    fn it_never_walks_through_the_home_between_any_two_places_on_the_floor() {
        let mut points = Vec::new();
        for x in (40..=1240).step_by(100) {
            for y in (380..=700).step_by(80) {
                let point = vec2(x as f32, y as f32);
                // Places the cat goes to are never against the home.
                if !KEEP_CLEAR.contains(point) {
                    points.push(point);
                }
            }
        }
        assert!(points.len() > 50, "only {} points", points.len());

        for &from in &points {
            for &to in &points {
                let path = walk(from, to);
                assert!(
                    path.iter().all(|p| !BLOCK.contains(*p)),
                    "{from} -> {to} went through the home"
                );
            }
        }
    }

    #[test]
    fn the_way_round_is_never_much_longer_than_it_needs_to_be() {
        let length = |path: &[Vec2]| path.windows(2).map(|w| w[0].distance(w[1])).sum::<f32>();
        let (from, to) = (vec2(600.0, 500.0), vec2(1150.0, 500.0));
        let detour = length(&walk(from, to));
        // Down to the corridor, along it, and back up.
        let ideal = from.distance(WAYPOINT_LEFT)
            + WAYPOINT_LEFT.distance(WAYPOINT_RIGHT)
            + WAYPOINT_RIGHT.distance(to);
        assert!(detour < ideal * 1.02, "{detour} vs {ideal}");
    }

    #[test]
    fn the_cushion_downstairs_is_reached_from_the_front() {
        let from = vec2(640.0, 480.0);
        let path = walk(from, GROUND_BED.spot);
        assert_eq!(path.last(), Some(&GROUND_BED.spot));

        // The only time it is inside the home's block is on the way in.
        for point in path.iter().filter(|p| BLOCK.contains(**p)) {
            assert!(
                inside_the_home_properly(*point),
                "cut through a wall at {point}"
            );
        }
        let doorstep = vec2(GROUND_BED.spot.x, CORRIDOR_Y);
        assert!(
            path.iter().any(|p| p.distance(doorstep) < 1.0),
            "never used the doorstep"
        );
    }

    #[test]
    fn going_in_does_not_dither_at_the_doorstep() {
        // Regression: the cat used to step up from the doorstep, be told to go
        // back to it, and do so for ever.
        for to in [GROUND_BED.spot, vec2(960.0, GROUND_FLOOR)] {
            let path = walk(vec2(640.0, 480.0), to);
            let doorstep = vec2(to.x, CORRIDOR_Y);
            let at = path
                .iter()
                .position(|p| p.distance(doorstep) < 1.0)
                .expect("reaches the doorstep");
            assert!(
                path[at..]
                    .windows(2)
                    .all(|pair| pair[1].y <= pair[0].y + 1e-3),
                "went back down after the doorstep"
            );
        }
    }

    #[test]
    fn leaving_the_home_is_by_the_front_too() {
        let to = world::BOWL_SPOT;
        let path = walk(GROUND_BED.spot, to);
        assert_eq!(path.last(), Some(&to));
        assert!(
            (path[1].x - GROUND_BED.spot.x).abs() < 1e-3,
            "steps straight out first"
        );
        for point in path.iter().filter(|p| BLOCK.contains(**p)) {
            assert!(
                inside_the_home_properly(*point),
                "cut through a wall at {point}"
            );
        }
    }

    #[test]
    fn inside_the_home_it_walks_straight() {
        let (from, to) = (vec2(840.0, GROUND_FLOOR), vec2(1020.0, GROUND_FLOOR));
        assert_eq!(ground_waypoint(from, to), to);
    }

    // --- Places -----------------------------------------------------------

    #[test]
    fn random_floor_points_are_never_inside_the_home() {
        for _ in 0..3000 {
            let point = world::random_floor_point();
            assert!(!KEEP_CLEAR.contains(point), "{point}");
            assert!((60.0..=world::WIDTH - 60.0).contains(&point.x));
            assert!((world::FLOOR_TOP..=world::HEIGHT).contains(&point.y));
        }
    }

    #[test]
    fn wandering_goes_to_both_floors_and_the_room() {
        let (mut upper, mut downstairs, mut room) = (0, 0, 0);
        for _ in 0..3000 {
            let place = random_place();
            match place.level {
                world::Level::Upper => {
                    upper += 1;
                    assert!((place.spot.y - UPPER_FLOOR).abs() < 1e-3);
                    assert!((LADDER_TOP.x..=SLIDE_TOP.x).contains(&place.spot.x));
                }
                world::Level::Ground if BLOCK.contains(place.spot) => {
                    downstairs += 1;
                    assert!((place.spot.y - GROUND_FLOOR).abs() < 1e-3);
                    assert!((INTERIOR_LEFT..=INTERIOR_RIGHT).contains(&place.spot.x));
                }
                world::Level::Ground => room += 1,
                world::Level::Sofa => unreachable!("wandering never goes up on the sofa"),
            }
        }
        assert!(upper > 200, "upstairs {upper} times");
        assert!(downstairs > 200, "downstairs {downstairs} times");
        assert!(room > 1500, "in the room {room} times");
    }

    #[test]
    fn the_cushions_are_where_the_cat_can_stand() {
        assert!(BLOCK.contains(GROUND_BED.spot));
        assert_eq!(GROUND_BED.level, world::Level::Ground);
        assert!(inside_the_home_properly(GROUND_BED.spot));

        assert_eq!(UPPER_BED.level, world::Level::Upper);
        assert!((LADDER_TOP.x..=SLIDE_TOP.x).contains(&UPPER_BED.spot.x));
        assert!((UPPER_BED.spot.y - UPPER_FLOOR).abs() < f32::EPSILON);
    }

    // --- Ladder and slide ---------------------------------------------------

    #[test]
    fn the_ladder_leads_from_the_floor_to_the_upper_floor() {
        assert_eq!(ladder_position(0.0), LADDER_BASE);
        assert_eq!(ladder_position(1.0), LADDER_TOP);
        assert_eq!(ladder_position(-3.0), LADDER_BASE, "clamped");
        assert_eq!(ladder_position(9.0), LADDER_TOP, "clamped");

        let heights: Vec<f32> = (0..=10)
            .map(|i| ladder_position(i as f32 / 10.0).y)
            .collect();
        assert!(
            heights.windows(2).all(|pair| pair[1] < pair[0]),
            "always climbing"
        );
    }

    #[test]
    fn the_ladder_is_outside_the_home_and_the_top_is_upstairs() {
        assert!(!BLOCK.contains(LADDER_BASE));
        assert!((LADDER_TOP.y - UPPER_FLOOR).abs() < f32::EPSILON);
    }

    #[test]
    fn the_slide_runs_from_the_upper_floor_to_the_room_floor() {
        assert_eq!(slide_position(0.0), SLIDE_TOP);
        assert!(slide_position(1.0).distance(SLIDE_END) < 1e-3);
        assert!(!BLOCK.contains(SLIDE_END), "it lands outside the home");
    }

    #[test]
    fn the_cat_speeds_up_going_down_the_slide() {
        let length = SLIDE_TOP.distance(SLIDE_SILL) + SLIDE_SILL.distance(SLIDE_END);
        let travelled = |t: f32| {
            // Straight-line distance is close enough on the long ramp part.
            slide_position(t).distance(SLIDE_TOP)
        };
        assert!(travelled(0.25) < 0.25 * length, "slow at first");
        let (early, late) = (
            travelled(0.5) - travelled(0.25),
            travelled(1.0) - travelled(0.75),
        );
        assert!(late > early * 1.5, "faster at the end: {early} then {late}");
    }

    #[test]
    fn the_slide_surface_is_where_the_cat_slides() {
        // The cat's path along the ramp lies on the drawn chute surface.
        let slope = (SLIDE_FOOT.y - SLIDE_SILL.y) / (SLIDE_FOOT.x - SLIDE_SILL.x);
        for t in [0.4, 0.6, 0.8, 1.0] {
            let at = slide_position(t);
            if at.x > SLIDE_SILL.x + 1.0 {
                let surface_y = SLIDE_SILL.y + (at.x - SLIDE_SILL.x) * slope;
                assert!(
                    (at.y - surface_y).abs() < 4.0,
                    "t={t}: {} vs {surface_y}",
                    at.y
                );
            }
        }
    }
}
