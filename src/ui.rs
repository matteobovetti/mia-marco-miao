//! Heads-up display: the cat's name and need bars, the shelf of foods, water
//! and litter cleaning, and the cat's current activity.

use macroquad::prelude::*;

use crate::brushes::Brush;
use crate::cat::{Cat, Need};
use crate::dispenser::Dispenser;
use crate::supplies::{Food, Supplies};
use crate::toys::Toy;
use crate::view;
use crate::window::Window;
use crate::world;

const INK: Color = Color::new(0.25, 0.18, 0.12, 1.0);
const CREAM: Color = Color::new(1.0, 0.96, 0.88, 1.0);

const NAME_BASELINE: f32 = 23.0;
const NAME_SIZE: f32 = 24.0;
const NEEDS_TOP: f32 = 33.0;
/// Distance from one bar to the next.
const BAR_PITCH: f32 = 19.0;
/// Each cat has a column of bars; the bars start this far into it.
const BAR_X_OFFSET: f32 = 46.0;
const NEEDS_COLUMN_W: f32 = 160.0;
const BAR_W: f32 = 98.0;
const BAR_H: f32 = 10.0;
const LABEL_SIZE: f32 = 16.0;
/// Where the lowest need bar ends.
pub const NEEDS_BOTTOM: f32 = NEEDS_TOP + (Need::COUNT - 1) as f32 * BAR_PITCH + BAR_H;

// Shelf of foods, water and litter cleaning, top-right corner.
const SHELF_W: f32 = 136.0;
pub const SHELF_X: f32 = world::WIDTH - SHELF_W - 16.0;
const SHELF_TOP: f32 = 27.0;
const BUTTON_H: f32 = 29.0;
const BUTTON_GAP: f32 = 3.0;
const SHELF_HEADER_SIZE: f32 = 16.0;
const BUTTON_LABEL_SIZE: f32 = 16.0;
const BUTTON_STATS_SIZE: f32 = 12.0;
const ICON_RADIUS: f32 = 9.0;

/// Something the player can click on the shelf.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShelfItem {
    /// Choose this food for the dispenser to pour.
    Food(Food),
    /// Clean the litter box.
    Clean,
}

impl ShelfItem {
    /// Every food first, then cleaning: the order of the buttons.
    fn all() -> impl Iterator<Item = Self> {
        Food::ALL.into_iter().map(Self::Food).chain([Self::Clean])
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Food(food) => food.label(),
            Self::Clean => "Litter",
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Food(food) => food.color(),
            Self::Clean => Color::new(0.93, 0.86, 0.65, 1.0),
        }
    }

    fn stats(self) -> String {
        match self {
            Self::Food(food) => food_stats(food),
            Self::Clean => "Scoop it clean".to_owned(),
        }
    }

    /// Whether clicking this would do anything right now. Any food can be
    /// chosen at any time; the litter box only needs cleaning when dirty.
    fn usable(self, supplies: &Supplies) -> bool {
        match self {
            Self::Food(_) => true,
            Self::Clean => supplies.litter.is_dirty(),
        }
    }

    /// The need that makes the cat wait for the player to click this, if any.
    /// (A hungry or thirsty cat waits for a dispenser button instead.)
    const fn asked_for_by(self) -> Option<Need> {
        match self {
            Self::Food(_) => None,
            Self::Clean => Some(Need::Bladder),
        }
    }
}

fn button_rect(index: usize) -> Rect {
    let y = SHELF_TOP + index as f32 * (BUTTON_H + BUTTON_GAP);
    Rect::new(SHELF_X, y, SHELF_W, BUTTON_H)
}

/// Which shelf button, if any, is under `point`.
pub fn item_at(point: Vec2) -> Option<ShelfItem> {
    ShelfItem::all()
        .enumerate()
        .find(|&(i, _)| button_rect(i).contains(point))
        .map(|(_, item)| item)
}

/// Draws the whole HUD. `sound_muted` is `None` if the game has no sound.
pub fn draw(
    cats: &[Cat],
    supplies: &Supplies,
    window: Window,
    dispenser: &Dispenser,
    held_brush: Option<Brush>,
    held_toy: Option<Toy>,
    sound_muted: Option<bool>,
) {
    for (i, cat) in cats.iter().enumerate() {
        draw_needs(cat, 16.0 + i as f32 * NEEDS_COLUMN_W);
    }
    draw_shelf(cats, supplies, dispenser.selected());
    draw_status(cats, window, held_brush, held_toy, sound_muted);
}

/// One cat's name and need bars, in a column starting at `left`.
fn draw_needs(cat: &Cat, left: f32) {
    view::draw_text(cat.name(), left, NAME_BASELINE, NAME_SIZE, INK);

    for (i, need) in Need::ALL.into_iter().enumerate() {
        let y = NEEDS_TOP + i as f32 * BAR_PITCH;
        let level = cat.level(need);

        // Flash red when a need is critical.
        let critical = level < 0.2 && (get_time() * 4.0).sin() > 0.0;
        let color = if critical { RED } else { need.color() };

        let bar_x = left + BAR_X_OFFSET;
        view::draw_text(need.label(), left, y + BAR_H - 1.0, LABEL_SIZE, INK);
        draw_rectangle(bar_x, y, BAR_W, BAR_H, Color::new(0.0, 0.0, 0.0, 0.2));
        draw_rectangle(bar_x, y, BAR_W * level, BAR_H, color);
        draw_rectangle_lines(bar_x, y, BAR_W, BAR_H, 1.5, INK);
    }
}

fn draw_status(
    cats: &[Cat],
    window: Window,
    held_brush: Option<Brush>,
    held_toy: Option<Toy>,
    sound_muted: Option<bool>,
) {
    // What each cat is up to, the last one on the bottom line.
    for (i, cat) in cats.iter().rev().enumerate() {
        let line = format!("{}: {}", cat.name(), cat.status());
        let y = world::HEIGHT - 16.0 - i as f32 * 28.0;
        view::draw_text(&line, 16.0, y, 26.0, CREAM);
    }

    let (take, put_back) = match (held_brush, held_toy) {
        (Some(brush), _) => (
            format!("{}: click a cat to brush it", brush.name()),
            "Right-click: put the brush back",
        ),
        (None, Some(toy)) => (
            format!("{}: move it around and hold Space", toy.name()),
            "Right-click: put the toy back",
        ),
        (None, None) => (
            "Take a brush to groom a cat or a toy to play".to_owned(),
            "Click a cat to pet it",
        ),
    };
    let mut hints = vec![
        take,
        put_back.to_owned(),
        if window.is_open() {
            "Click the window to close it".to_owned()
        } else {
            "Click the window to open it".to_owned()
        },
    ];
    match sound_muted {
        Some(true) => hints.push("M: turn sound on".to_owned()),
        Some(false) => hints.push("M: turn sound off".to_owned()),
        None => {}
    }
    hints.push(
        if view::is_fullscreen() {
            "F or Esc: leave full screen"
        } else {
            "F: full screen"
        }
        .to_owned(),
    );

    // Bottom-right, last hint on the bottom line.
    for (i, hint) in hints.iter().rev().enumerate() {
        let width = view::text_width(hint, 20.0);
        let y = world::HEIGHT - 16.0 - i as f32 * 22.0;
        view::draw_text(hint, world::WIDTH - 16.0 - width, y, 20.0, CREAM);
    }
}

/// One button per food, to choose what the dispenser pours (the chosen one is
/// marked), and one to clean the litter box, which flashes when a cat is
/// waiting at a dirty box.
fn draw_shelf(cats: &[Cat], supplies: &Supplies, chosen: Food) {
    let mouse = view::mouse();
    let calling = |item: ShelfItem| {
        item.usable(supplies)
            && item
                .asked_for_by()
                .is_some_and(|need| cats.iter().any(|cat| cat.waiting_for() == Some(need)))
    };
    let flash_on = (get_time() * 5.0).sin() > 0.0;

    let header = if cats.iter().any(|cat| cat.waiting_for().is_some()) {
        "Cat needs you!"
    } else {
        "Choose a food:"
    };
    view::draw_text(header, SHELF_X, SHELF_TOP - 7.0, SHELF_HEADER_SIZE, INK);

    for (i, item) in ShelfItem::all().enumerate() {
        let rect = button_rect(i);
        let usable = item.usable(supplies);
        let hovered = usable && rect.contains(mouse);
        let selected = item == ShelfItem::Food(chosen);
        // Everything fades while the button can't be used.
        let alpha = if usable { 1.0 } else { 0.45 };

        let background = if selected {
            Color::new(0.9, 1.0, 0.86, alpha)
        } else if hovered {
            Color::new(1.0, 0.98, 0.88, alpha)
        } else {
            Color::new(1.0, 0.93, 0.78, alpha)
        };
        let border = if calling(item) && flash_on {
            Color::new(0.9, 0.25, 0.2, 1.0)
        } else if selected {
            Color::new(0.15, 0.55, 0.2, 1.0)
        } else {
            Color { a: alpha, ..INK }
        };
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, background);
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if selected { 3.0 } else { 2.0 },
            border,
        );

        // A dot in the item's colour as its icon.
        let icon = vec2(rect.x + 16.0, rect.y + rect.h / 2.0);
        draw_circle(
            icon.x,
            icon.y,
            ICON_RADIUS,
            Color {
                a: alpha,
                ..item.color()
            },
        );
        draw_circle_lines(icon.x, icon.y, ICON_RADIUS, 1.5, Color { a: alpha, ..INK });

        let ink = Color { a: alpha, ..INK };
        let text_x = rect.x + 32.0;
        view::draw_text(item.label(), text_x, rect.y + 12.0, BUTTON_LABEL_SIZE, ink);
        view::draw_text(&item.stats(), text_x, rect.y + 24.0, BUTTON_STATS_SIZE, ink);

        if selected {
            draw_tick(vec2(rect.x + rect.w - 13.0, rect.y + rect.h / 2.0));
        }
    }
}

/// A green tick mark, centred on `centre`.
fn draw_tick(centre: Vec2) {
    let green = Color::new(0.15, 0.55, 0.2, 1.0);
    let (left, bottom, right) = (
        centre + vec2(-5.0, 0.0),
        centre + vec2(-1.5, 4.5),
        centre + vec2(6.0, -5.0),
    );
    draw_line(left.x, left.y, bottom.x, bottom.y, 2.5, green);
    draw_line(bottom.x, bottom.y, right.x, right.y, 2.5, green);
}

fn food_stats(food: Food) -> String {
    let food_pct = food.nutrition() * 100.0;
    let fun_pct = food.joy() * 100.0;
    if fun_pct > 0.0 {
        format!("Food +{food_pct:.0}  Fun +{fun_pct:.0}")
    } else {
        format!("Food +{food_pct:.0}")
    }
}
