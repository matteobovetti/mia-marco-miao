//! The feeding dispenser against the left wall: a hopper of food above the
//! food bowl and a jug of water above the water bowl.
//!
//! The player picks a food on the shelf, then presses the button on the
//! dispenser and the food pours out of its nozzle into the bowl. The water
//! dispenser has a button of its own. Pouring takes a moment, and the bowl
//! only gets its food or water when the pouring is done.

use macroquad::prelude::*;

use crate::cat::Need;
use crate::supplies::{Food, Supplies};
use crate::view;
use crate::world;

// --- Layout -------------------------------------------------------------
//
// One unit stands against the left wall: the food dispenser on top, the water
// dispenser below it. A pipe from each runs out over its bowl and ends in a
// nozzle.

const UNIT_X: f32 = 30.0;
const UNIT_W: f32 = 92.0;
const UNIT_RIGHT: f32 = UNIT_X + UNIT_W;
const PIPE_THICKNESS: f32 = 12.0;
const BUTTON_RADIUS: f32 = 16.0;

// Food: a hopper on a body with the button, and a pipe to the bowl.
const HOPPER_TOP: f32 = 330.0;
const HOPPER_BOTTOM: f32 = 400.0;
const FOOD_BODY: Rect = Rect {
    x: UNIT_X,
    y: HOPPER_BOTTOM,
    w: UNIT_W,
    h: 78.0,
};
const FOOD_BUTTON: Vec2 = vec2(76.0, 444.0);
const FOOD_PIPE_Y: f32 = 410.0;
/// Where the food leaves the nozzle: right above the middle of the bowl.
const FOOD_SPOUT: Vec2 = vec2(world::BOWL.x, 436.0);

// Water: a jug on a base with the button, and a pipe to the bowl.
const JUG: Rect = Rect {
    x: 44.0,
    y: 498.0,
    w: 64.0,
    h: 60.0,
};
const WATER_BASE: Rect = Rect {
    x: UNIT_X,
    y: JUG.y + JUG.h,
    w: UNIT_W,
    h: 78.0,
};
const WATER_BUTTON: Vec2 = vec2(76.0, 606.0);
const WATER_PIPE_Y: f32 = 566.0;
const WATER_SPOUT: Vec2 = vec2(world::WATER.x, 592.0);

// The nozzles must be above the rims of their bowls...
const _: () = assert!(FOOD_SPOUT.y < world::BOWL.y - 14.0);
const _: () = assert!(WATER_SPOUT.y < world::WATER.y - 14.0);
// ...the unit must stand on the floor and keep clear of the bowls and of the
// status text along the bottom of the screen, and its parts must not overlap.
const _: () = assert!(HOPPER_TOP > world::FLOOR_TOP);
const _: () = assert!(UNIT_RIGHT < world::BOWL.x - 34.0);
const _: () = assert!(FOOD_BODY.y + FOOD_BODY.h < JUG.y);
const _: () = assert!(WATER_BASE.y + WATER_BASE.h < 660.0);

/// How long the food and the water take to pour, in seconds.
const FOOD_POUR_SECONDS: f32 = 0.9;
const WATER_POUR_SECONDS: f32 = 1.2;

const SHADOW: Color = Color::new(0.0, 0.0, 0.0, 0.2);
const INK: Color = Color::new(0.2, 0.22, 0.28, 1.0);
const GLASS: Color = Color::new(0.85, 0.93, 1.0, 0.55);

/// A button on the dispenser.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    /// Pours the chosen food into the food bowl.
    Food,
    /// Pours water into the water bowl.
    Water,
}

/// Food on its way down into the bowl.
#[derive(Clone, Copy, PartialEq, Debug)]
struct FoodPour {
    food: Food,
    seconds_left: f32,
}

/// The dispenser's settings and what it is pouring right now.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Dispenser {
    /// The food the food button will pour.
    selected: Food,
    food_pour: Option<FoodPour>,
    /// Seconds of water still to pour.
    water_pour: Option<f32>,
}

impl Default for Dispenser {
    fn default() -> Self {
        Self {
            selected: Food::Kibble,
            food_pour: None,
            water_pour: None,
        }
    }
}

impl Dispenser {
    /// The food the dispenser will pour next.
    pub const fn selected(&self) -> Food {
        self.selected
    }

    /// Chooses which food the food button pours. Food already pouring is not
    /// affected.
    pub const fn select(&mut self, food: Food) {
        self.selected = food;
    }

    /// Which button, if any, is under `point` (in canvas coordinates).
    pub fn button_at(point: Vec2) -> Option<Button> {
        // A little more than the drawn button, to be easy to hit.
        let reach = BUTTON_RADIUS + 4.0;
        if point.distance(FOOD_BUTTON) <= reach {
            Some(Button::Food)
        } else if point.distance(WATER_BUTTON) <= reach {
            Some(Button::Water)
        } else {
            None
        }
    }

    /// Whether pressing the food button would pour: the bowl is empty and
    /// nothing is pouring already.
    pub fn can_pour_food(&self, supplies: &Supplies) -> bool {
        supplies.food.is_empty() && self.food_pour.is_none()
    }

    /// Whether pressing the water button would pour: the bowl isn't full and
    /// nothing is pouring already.
    pub fn can_pour_water(&self, supplies: &Supplies) -> bool {
        !supplies.water.is_full() && self.water_pour.is_none()
    }

    /// Presses a button. Returns `true` if that started something pouring,
    /// and `false` (doing nothing) if the bowl can't take any more.
    pub fn press(&mut self, button: Button, supplies: &Supplies) -> bool {
        match button {
            Button::Food if self.can_pour_food(supplies) => {
                self.food_pour = Some(FoodPour {
                    food: self.selected,
                    seconds_left: FOOD_POUR_SECONDS,
                });
                true
            }
            Button::Water if self.can_pour_water(supplies) => {
                self.water_pour = Some(WATER_POUR_SECONDS);
                true
            }
            Button::Food | Button::Water => false,
        }
    }

    /// Lets the pouring go on for `dt` seconds. The bowl gets its food or
    /// water when the pouring ends.
    pub fn update(&mut self, dt: f32, supplies: &mut Supplies) {
        self.food_pour = match self.food_pour {
            Some(pour) if pour.seconds_left > dt => Some(FoodPour {
                seconds_left: pour.seconds_left - dt,
                ..pour
            }),
            Some(pour) => {
                supplies.food.fill(pour.food);
                None
            }
            None => None,
        };
        self.water_pour = match self.water_pour {
            Some(left) if left > dt => Some(left - dt),
            Some(_) => {
                supplies.water.fill();
                None
            }
            None => None,
        };
    }

    /// Draws both dispensers. `waiting_for` is what each cat is waiting for,
    /// so the button it needs can flash.
    pub fn draw(&self, supplies: &Supplies, waiting_for: &[Option<Need>]) {
        let time = get_time() as f32;
        let flash_on = (time * 5.0).sin() > 0.0;
        let food_flash = flash_on && waiting_for.contains(&Some(Need::Hunger));
        let water_flash = flash_on && waiting_for.contains(&Some(Need::Thirst));

        self.draw_food_unit(supplies, food_flash);
        self.draw_water_unit(supplies, water_flash, time);
        self.draw_food_pour(time);
        self.draw_water_pour(time);
    }

    fn draw_food_unit(&self, supplies: &Supplies, flashing: bool) {
        draw_hopper(self.selected);

        ellipse(
            vec2(76.0, FOOD_BODY.y + FOOD_BODY.h + 4.0),
            62.0,
            8.0,
            SHADOW,
        );
        draw_body(FOOD_BODY, Color::new(0.8, 0.84, 0.9, 1.0));
        draw_pipe(FOOD_PIPE_Y, FOOD_SPOUT);

        draw_centered("FOOD", 76.0, FOOD_BODY.y + 24.0, 14.0, INK);
        draw_round_button(
            FOOD_BUTTON,
            Color::new(0.88, 0.22, 0.2, 1.0),
            ButtonLook {
                enabled: self.can_pour_food(supplies),
                pressed: self.food_pour.is_some(),
                flashing,
            },
        );
        draw_centered(self.selected.label(), 76.0, FOOD_BODY.y + 72.0, 13.0, INK);
    }

    fn draw_water_unit(&self, supplies: &Supplies, flashing: bool, time: f32) {
        draw_jug(self.water_pour.is_some(), time);

        ellipse(
            vec2(76.0, WATER_BASE.y + WATER_BASE.h + 4.0),
            62.0,
            8.0,
            SHADOW,
        );
        draw_body(WATER_BASE, Color::new(0.55, 0.72, 0.88, 1.0));
        draw_pipe(WATER_PIPE_Y, WATER_SPOUT);

        draw_centered("WATER", 76.0, WATER_BASE.y + 26.0, 14.0, INK);
        draw_round_button(
            WATER_BUTTON,
            Color::new(0.25, 0.5, 0.92, 1.0),
            ButtonLook {
                enabled: self.can_pour_water(supplies),
                pressed: self.water_pour.is_some(),
                flashing,
            },
        );
    }

    /// Pieces of food falling from the nozzle into the bowl.
    fn draw_food_pour(&self, time: f32) {
        let Some(pour) = self.food_pour else { return };
        let (top, bottom) = (FOOD_SPOUT.y + 4.0, world::BOWL.y - 3.0);
        let colour = pour.food.color();
        let outline = Color::new(colour.r * 0.6, colour.g * 0.6, colour.b * 0.6, 1.0);

        for piece in 0..8 {
            let fall = (time * 3.5 + piece as f32 / 8.0).fract();
            let x = FOOD_SPOUT.x + (piece as f32 * 2.7).sin() * 4.0;
            let y = top + fall * (bottom - top);
            draw_circle(x, y, 3.4, colour);
            draw_circle_lines(x, y, 3.4, 1.0, outline);
        }
    }

    /// A stream of water from the nozzle, with ripples where it lands.
    fn draw_water_pour(&self, time: f32) {
        if self.water_pour.is_none() {
            return;
        }
        let (x, top, bottom) = (WATER_SPOUT.x, WATER_SPOUT.y, world::WATER.y - 3.0);
        draw_line(x, top, x, bottom, 5.0, Color::new(0.45, 0.72, 0.98, 0.85));
        draw_line(
            x - 1.0,
            top,
            x - 1.0,
            bottom,
            2.0,
            Color::new(0.88, 0.96, 1.0, 0.9),
        );

        // Light flecks running down the stream.
        for fleck in 0..4 {
            let fall = (time * 2.5 + fleck as f32 / 4.0).fract();
            let y = top + fall * (bottom - top - 6.0);
            draw_line(
                x + 1.0,
                y,
                x + 1.0,
                y + 6.0,
                2.0,
                Color::new(1.0, 1.0, 1.0, 0.8),
            );
        }

        let spread = (time * 2.0).fract();
        draw_ellipse_lines(
            x,
            bottom,
            10.0 + 14.0 * spread,
            4.0 + 5.0 * spread,
            0.0,
            2.0,
            Color::new(1.0, 1.0, 1.0, 0.7 * (1.0 - spread)),
        );
    }
}

// --- Drawing helpers ------------------------------------------------------

fn ellipse(center: Vec2, rx: f32, ry: f32, color: Color) {
    draw_ellipse(center.x, center.y, rx, ry, 0.0, color);
}

fn fill_quad(corners: [Vec2; 4], color: Color) {
    draw_triangle(corners[0], corners[1], corners[2], color);
    draw_triangle(corners[0], corners[2], corners[3], color);
}

fn draw_centered(text: &str, center_x: f32, baseline_y: f32, size: f32, color: Color) {
    let width = view::text_width(text, size);
    view::draw_text(text, center_x - width / 2.0, baseline_y, size, color);
}

/// The left and right edges of the hopper at height `y`: it is wide at the
/// top and narrows to the opening at the bottom.
fn hopper_edges(y: f32) -> (f32, f32) {
    let narrowing = (y - HOPPER_TOP) / (HOPPER_BOTTOM - HOPPER_TOP) * 14.0;
    (38.0 + narrowing, 114.0 - narrowing)
}

/// The glass hopper, with a lid, and the selected food inside it.
fn draw_hopper(food: Food) {
    let corners = |top: f32| {
        let ((top_left, top_right), (bottom_left, bottom_right)) =
            (hopper_edges(top), hopper_edges(HOPPER_BOTTOM));
        [
            vec2(top_left, top),
            vec2(top_right, top),
            vec2(bottom_right, HOPPER_BOTTOM),
            vec2(bottom_left, HOPPER_BOTTOM),
        ]
    };
    fill_quad(corners(HOPPER_TOP), GLASS);

    // The food fills most of the hopper.
    let level = HOPPER_TOP + 18.0;
    fill_quad(corners(level), food.color());
    let dark = Color::new(
        food.color().r * 0.7,
        food.color().g * 0.7,
        food.color().b * 0.7,
        1.0,
    );
    for (i, (across, down)) in [
        (0.2, 0.2),
        (0.5, 0.1),
        (0.8, 0.25),
        (0.35, 0.5),
        (0.65, 0.6),
        (0.5, 0.85),
        (0.25, 0.8),
        (0.75, 0.9),
    ]
    .into_iter()
    .enumerate()
    {
        let y = level + down * (HOPPER_BOTTOM - level - 4.0);
        let (left, right) = hopper_edges(y);
        let x = left + across * (right - left);
        draw_circle(x, y, 2.6 + (i % 2) as f32, dark);
    }

    // Glass edges and a glint, then the lid.
    let outline = Color::new(0.45, 0.55, 0.65, 1.0);
    let [a, b, c, d] = corners(HOPPER_TOP);
    for (from, to) in [(a, d), (b, c)] {
        draw_line(from.x, from.y, to.x, to.y, 2.0, outline);
    }
    draw_line(
        a.x + 8.0,
        a.y + 6.0,
        d.x + 6.0,
        d.y - 10.0,
        3.0,
        Color::new(1.0, 1.0, 1.0, 0.5),
    );
    draw_rectangle(
        34.0,
        HOPPER_TOP - 10.0,
        84.0,
        10.0,
        Color::new(0.5, 0.55, 0.62, 1.0),
    );
}

/// A box with a shaded front.
fn draw_body(body: Rect, color: Color) {
    draw_rectangle(body.x, body.y, body.w, body.h, color);
    draw_rectangle(body.x, body.y, body.w, 6.0, Color::new(1.0, 1.0, 1.0, 0.25));
    draw_rectangle(
        body.x + body.w - 8.0,
        body.y,
        8.0,
        body.h,
        Color::new(0.0, 0.0, 0.0, 0.1),
    );
    draw_rectangle_lines(body.x, body.y, body.w, body.h, 3.0, INK);
}

/// The pipe from the dispenser out over the bowl, ending in a nozzle at
/// `spout` (its bottom, where the food or water comes out).
fn draw_pipe(pipe_y: f32, spout: Vec2) {
    let metal = Color::new(0.62, 0.66, 0.72, 1.0);
    let start = UNIT_RIGHT - 4.0;
    draw_rectangle(start, pipe_y, spout.x + 9.0 - start, PIPE_THICKNESS, metal);
    draw_rectangle_lines(
        start,
        pipe_y,
        spout.x + 9.0 - start,
        PIPE_THICKNESS,
        2.0,
        INK,
    );

    let top = pipe_y + PIPE_THICKNESS;
    fill_quad(
        [
            vec2(spout.x - 12.0, top),
            vec2(spout.x + 12.0, top),
            vec2(spout.x + 6.0, spout.y),
            vec2(spout.x - 6.0, spout.y),
        ],
        metal,
    );
}

/// How a round button looks.
#[derive(Clone, Copy)]
struct ButtonLook {
    /// Greyed out when it can't be used.
    enabled: bool,
    /// Pushed in, while its pouring goes on.
    pressed: bool,
    /// Flashing to ask for attention.
    flashing: bool,
}

fn draw_round_button(center: Vec2, color: Color, look: ButtonLook) {
    let face_color = if look.enabled || look.pressed {
        color
    } else {
        Color::new(0.62, 0.64, 0.68, 1.0)
    };
    let dip = if look.pressed { 3.0 } else { 0.0 };
    let ring = if look.flashing {
        Color::new(1.0, 0.85, 0.1, 1.0)
    } else {
        INK
    };

    draw_circle(center.x, center.y, BUTTON_RADIUS + 4.0, ring);
    draw_circle(
        center.x,
        center.y + 1.0,
        BUTTON_RADIUS + 1.0,
        Color::new(0.0, 0.0, 0.0, 0.3),
    );
    draw_circle(
        center.x,
        center.y + dip,
        BUTTON_RADIUS - dip * 0.3,
        face_color,
    );
    draw_circle(
        center.x - 5.0,
        center.y - 5.0 + dip,
        4.0,
        Color::new(1.0, 1.0, 1.0, 0.45),
    );
}

/// The upside-down water jug. It bubbles while the water pours.
fn draw_jug(pouring: bool, time: f32) {
    let outline = Color::new(0.35, 0.5, 0.7, 1.0);
    let water = Color::new(0.4, 0.68, 0.95, 0.85);
    let Rect { x, y, w, h } = JUG;
    let r = 12.0;

    // Rounded shape: a body, a top bar and two round top corners.
    let shape = |inset: f32, color: Color| {
        draw_rectangle(x + inset, y + r, w - 2.0 * inset, h - r, color);
        draw_rectangle(x + r, y + inset, w - 2.0 * r, r, color);
        draw_circle(x + r, y + r, r - inset, color);
        draw_circle(x + w - r, y + r, r - inset, color);
    };
    shape(0.0, outline);
    shape(3.0, water);

    // The cap where the water comes out at the bottom.
    draw_rectangle(
        66.0,
        y + h - 2.0,
        20.0,
        6.0,
        Color::new(0.3, 0.35, 0.45, 1.0),
    );
    draw_line(
        x + 10.0,
        y + 12.0,
        x + 10.0,
        y + h - 12.0,
        3.0,
        Color::new(1.0, 1.0, 1.0, 0.45),
    );

    if pouring {
        for bubble in 0..3 {
            let rise = (time * 1.6 + bubble as f32 / 3.0).fract();
            let bx = x + 20.0 + bubble as f32 * 15.0 + (time * 3.0 + bubble as f32).sin() * 2.0;
            let by = y + h - 10.0 - rise * (h - 24.0);
            draw_circle(bx, by, 3.0 - rise, Color::new(1.0, 1.0, 1.0, 0.8));
        }
    }

    // The surface of the water sways gently.
    let sway = (time * 1.5).sin() * 1.5;
    let surface = Color::new(1.0, 1.0, 1.0, 0.35);
    draw_line(
        x + 6.0,
        y + 20.0 + sway,
        x + w - 6.0,
        y + 20.0 - sway,
        2.0,
        surface,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cat::Cat;

    const DT: f32 = 1.0 / 60.0;

    fn run(dispenser: &mut Dispenser, supplies: &mut Supplies, seconds: f32) {
        for _ in 0..(seconds / DT).round() as usize {
            dispenser.update(DT, supplies);
        }
    }

    // --- Choosing a food ------------------------------------------------

    #[test]
    fn it_starts_with_kibble_selected() {
        assert_eq!(Dispenser::default().selected(), Food::Kibble);
    }

    #[test]
    fn selecting_a_food_changes_what_it_will_pour() {
        let mut dispenser = Dispenser::default();
        for food in Food::ALL {
            dispenser.select(food);
            assert_eq!(dispenser.selected(), food);
        }
    }

    // --- The food button --------------------------------------------------

    #[test]
    fn pressing_the_food_button_pours_the_selected_food_into_the_empty_bowl() {
        let (mut dispenser, mut supplies) = (Dispenser::default(), Supplies::default());
        dispenser.select(Food::Tuna);

        assert!(dispenser.press(Button::Food, &supplies));
        assert!(supplies.food.is_empty(), "nothing in the bowl yet");

        run(&mut dispenser, &mut supplies, FOOD_POUR_SECONDS + 0.1);
        assert_eq!(
            supplies.food.contents().map(|(food, _)| food),
            Some(Food::Tuna)
        );
    }

    #[test]
    fn the_food_arrives_when_the_pouring_ends_not_before() {
        let (mut dispenser, mut supplies) = (Dispenser::default(), Supplies::default());
        dispenser.press(Button::Food, &supplies);

        run(&mut dispenser, &mut supplies, FOOD_POUR_SECONDS - 0.1);
        assert!(supplies.food.is_empty(), "still falling");

        run(&mut dispenser, &mut supplies, 0.2);
        assert!(!supplies.food.is_empty(), "landed");
    }

    #[test]
    fn the_food_button_does_nothing_while_the_bowl_has_food() {
        let (mut dispenser, mut supplies) = (Dispenser::default(), Supplies::default());
        supplies.food.fill(Food::Chicken);

        assert!(!dispenser.can_pour_food(&supplies));
        assert!(!dispenser.press(Button::Food, &supplies));
        run(&mut dispenser, &mut supplies, 2.0);
        assert_eq!(
            supplies.food.contents().map(|(food, _)| food),
            Some(Food::Chicken)
        );
    }

    #[test]
    fn pressing_twice_pours_only_once() {
        let (mut dispenser, supplies) = (Dispenser::default(), Supplies::default());
        assert!(dispenser.press(Button::Food, &supplies));
        assert!(!dispenser.press(Button::Food, &supplies), "already pouring");
        assert!(!dispenser.can_pour_food(&supplies));
    }

    #[test]
    fn changing_the_selection_while_pouring_does_not_change_the_food_on_its_way() {
        let (mut dispenser, mut supplies) = (Dispenser::default(), Supplies::default());
        dispenser.select(Food::Milk);
        dispenser.press(Button::Food, &supplies);

        dispenser.select(Food::Chicken);
        run(&mut dispenser, &mut supplies, FOOD_POUR_SECONDS + 0.1);

        assert_eq!(
            supplies.food.contents().map(|(food, _)| food),
            Some(Food::Milk)
        );
        assert_eq!(
            dispenser.selected(),
            Food::Chicken,
            "the next one is chicken"
        );
    }

    #[test]
    fn the_food_button_works_again_once_the_cat_has_emptied_the_bowl() {
        let (mut dispenser, mut supplies) = (Dispenser::default(), Supplies::default());
        dispenser.press(Button::Food, &supplies);
        run(&mut dispenser, &mut supplies, 1.5);
        assert!(!dispenser.can_pour_food(&supplies), "the bowl is full");

        while supplies.food.take(0.2).is_some() {}
        assert!(dispenser.can_pour_food(&supplies), "empty again");
    }

    // --- The water button -------------------------------------------------

    #[test]
    fn pressing_the_water_button_fills_the_water_bowl_after_a_moment() {
        let (mut dispenser, mut supplies) = (Dispenser::default(), Supplies::default());

        assert!(dispenser.press(Button::Water, &supplies));
        assert!(supplies.water.is_empty(), "not yet");

        run(&mut dispenser, &mut supplies, WATER_POUR_SECONDS - 0.1);
        assert!(supplies.water.is_empty(), "still pouring");

        run(&mut dispenser, &mut supplies, 0.2);
        assert!(supplies.water.is_full());
    }

    #[test]
    fn the_water_button_does_nothing_when_the_bowl_is_full() {
        let (mut dispenser, mut supplies) = (Dispenser::default(), Supplies::default());
        supplies.water.fill();

        assert!(!dispenser.can_pour_water(&supplies));
        assert!(!dispenser.press(Button::Water, &supplies));
    }

    #[test]
    fn the_water_bowl_can_be_topped_up_when_partly_drunk() {
        let (mut dispenser, mut supplies) = (Dispenser::default(), Supplies::default());
        supplies.water.fill();
        supplies.water.take(0.4);

        assert!(dispenser.press(Button::Water, &supplies));
        run(&mut dispenser, &mut supplies, WATER_POUR_SECONDS + 0.1);
        assert!(supplies.water.is_full());
    }

    #[test]
    fn food_and_water_can_pour_at_the_same_time() {
        let (mut dispenser, mut supplies) = (Dispenser::default(), Supplies::default());
        assert!(dispenser.press(Button::Food, &supplies));
        assert!(dispenser.press(Button::Water, &supplies));

        run(&mut dispenser, &mut supplies, 2.0);
        assert!(!supplies.food.is_empty() && supplies.water.is_full());
    }

    // --- The buttons on the machine ------------------------------------------

    #[test]
    fn each_button_is_found_where_it_is_drawn() {
        assert_eq!(Dispenser::button_at(FOOD_BUTTON), Some(Button::Food));
        assert_eq!(Dispenser::button_at(WATER_BUTTON), Some(Button::Water));
        let near_edge = FOOD_BUTTON + vec2(BUTTON_RADIUS, 0.0);
        assert_eq!(Dispenser::button_at(near_edge), Some(Button::Food));
    }

    #[test]
    fn clicks_beside_the_buttons_miss_them() {
        assert_eq!(Dispenser::button_at(Vec2::ZERO), None);
        assert_eq!(Dispenser::button_at(vec2(300.0, 300.0)), None);
        assert_eq!(Dispenser::button_at(FOOD_BUTTON + vec2(0.0, 40.0)), None);
        assert_eq!(Dispenser::button_at(WATER_BUTTON + vec2(60.0, 0.0)), None);
    }

    #[test]
    fn the_buttons_sit_on_their_dispensers_and_do_not_overlap() {
        let inside = |rect: Rect, centre: Vec2| {
            Rect::new(
                rect.x + BUTTON_RADIUS,
                rect.y + BUTTON_RADIUS,
                rect.w - 2.0 * BUTTON_RADIUS,
                rect.h - 2.0 * BUTTON_RADIUS,
            )
            .contains(centre)
        };
        assert!(inside(FOOD_BODY, FOOD_BUTTON));
        assert!(inside(WATER_BASE, WATER_BUTTON));
        assert!(FOOD_BUTTON.distance(WATER_BUTTON) > 4.0 * BUTTON_RADIUS);
    }

    #[test]
    fn the_hopper_is_wide_at_the_top_and_narrow_at_the_bottom() {
        let (top_left, top_right) = hopper_edges(HOPPER_TOP);
        let (bottom_left, bottom_right) = hopper_edges(HOPPER_BOTTOM);
        assert!(top_right - top_left > bottom_right - bottom_left);
        assert!((top_left - 38.0).abs() < 1e-4 && (top_right - 114.0).abs() < 1e-4);
        // Symmetric about the middle of the unit.
        assert!((f32::midpoint(bottom_left, bottom_right) - 76.0).abs() < 1e-4);
    }

    // --- With the cat -------------------------------------------------------

    /// Plays the dispenser the way a player would: press the button the cat
    /// is waiting at, and clean the litter box.
    #[test]
    fn a_player_using_the_dispenser_keeps_the_cat_fed_and_watered() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();
        let mut dispenser = Dispenser::default();

        let (mut ate, mut drank, mut presses) = (false, false, 0);
        for _ in 0..(200.0 / DT) as usize {
            match cat.waiting_for() {
                Some(Need::Hunger) => {
                    presses += i32::from(dispenser.press(Button::Food, &supplies));
                }
                Some(Need::Thirst) => {
                    presses += i32::from(dispenser.press(Button::Water, &supplies));
                }
                Some(Need::Bladder) => {
                    supplies.litter.clean();
                }
                _ => {}
            }
            cat.update(DT, &mut supplies);
            dispenser.update(DT, &mut supplies);

            ate |= cat.feeding() == Some(Need::Hunger);
            drank |= cat.feeding() == Some(Need::Thirst);
        }

        assert!(ate, "the cat never got to eat");
        assert!(drank, "the cat never got to drink");
        assert!(presses >= 2, "only {presses} presses");
    }

    #[test]
    fn a_hungry_cat_waits_while_the_food_is_still_falling() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();
        let mut dispenser = Dispenser::default();

        // Wait for the cat to ask for food (it takes a while to get hungry).
        let mut waited = false;
        for _ in 0..(150.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            if cat.waiting_for() == Some(Need::Hunger) {
                waited = true;
                break;
            }
        }
        assert!(waited, "the cat never asked for food");

        dispenser.press(Button::Food, &supplies);
        for _ in 0..((FOOD_POUR_SECONDS - 0.2) / DT) as usize {
            cat.update(DT, &mut supplies);
            dispenser.update(DT, &mut supplies);
            assert_eq!(cat.feeding(), None, "can't eat food that hasn't landed");
        }
        assert_eq!(cat.waiting_for(), Some(Need::Hunger));
    }
}
