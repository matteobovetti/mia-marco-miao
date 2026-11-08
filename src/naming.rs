//! The "name your cat" screen shown before the game starts.

use macroquad::prelude::*;

use crate::brushes;
use crate::cat::{CAT_COUNT, Cat, Coat};
use crate::dispenser::Dispenser;
use crate::supplies::Supplies;
use crate::toys;
use crate::view;
use crate::window::Window;
use crate::world;

pub const MAX_LEN: usize = 16;
/// Used when the player presses Enter without typing anything, for each cat.
pub const DEFAULT_NAMES: [&str; CAT_COUNT] = ["Mittens", "Domino"];
const ORDINALS: [&str; CAT_COUNT] = ["first", "second"];

const INK: Color = Color::new(0.25, 0.18, 0.12, 1.0);
const BACKSPACE_DELAY: f32 = 0.4;
const BACKSPACE_REPEAT: f32 = 0.05;

pub struct Naming {
    text: String,
    /// How long backspace has been held down.
    backspace_held: f32,
    /// The names chosen so far, one per cat already named.
    chosen: Vec<String>,
    /// The cats wandering around the room behind the dialog; the one being
    /// named is the one shown.
    previews: [Cat; CAT_COUNT],
    supplies: Supplies,
    /// The window behind the dialog, shut.
    window: Window,
}

impl Naming {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            backspace_held: 0.0,
            chosen: Vec::new(),
            previews: [
                Cat::new(""),
                Cat::with_coat("", Coat::BLACK_AND_WHITE, world::SECOND_CAT_START),
            ],
            supplies: Supplies::default(),
            window: Window::default(),
        }
    }

    /// Adds a typed character, if it's one the default font can show and
    /// there is still room.
    ///
    /// The built-in font only has ASCII glyphs, so anything else is ignored.
    pub fn type_char(&mut self, c: char) {
        let printable = c.is_ascii_graphic() || (c == ' ' && !self.text.is_empty());
        if printable && self.text.chars().count() < MAX_LEN {
            self.text.push(c);
        }
    }

    pub fn backspace(&mut self) {
        self.text.pop();
    }

    /// Which cat is being named: 0 for the first, 1 for the second.
    fn stage(&self) -> usize {
        self.chosen.len().min(CAT_COUNT - 1)
    }

    /// The name that would be used now: what was typed without surrounding
    /// spaces, or the cat's default name if that's nothing.
    pub fn name(&self) -> String {
        match self.text.trim() {
            "" => DEFAULT_NAMES[self.stage()].to_owned(),
            name => name.to_owned(),
        }
    }

    /// Accepts the name typed for the current cat. Returns every name, in cat
    /// order, once the last cat has been named; before that, moves on to the
    /// next cat with an empty text box.
    pub fn confirm(&mut self) -> Option<[String; CAT_COUNT]> {
        self.chosen.push(self.name());
        self.text.clear();
        self.backspace_held = 0.0;
        if self.chosen.len() < CAT_COUNT {
            return None;
        }
        std::mem::take(&mut self.chosen).try_into().ok()
    }

    /// Reads the keyboard. Enter accepts a name; returns all the names once
    /// every cat has one.
    pub fn update(&mut self, dt: f32) -> Option<[String; CAT_COUNT]> {
        while let Some(c) = get_char_pressed() {
            self.type_char(c);
        }

        // Backspace deletes once, then repeats while held.
        if is_key_pressed(KeyCode::Backspace) {
            self.backspace();
            self.backspace_held = 0.0;
        } else if is_key_down(KeyCode::Backspace) {
            self.backspace_held += dt;
            if self.backspace_held > BACKSPACE_DELAY {
                self.backspace_held -= BACKSPACE_REPEAT;
                self.backspace();
            }
        } else {
            self.backspace_held = 0.0;
        }

        let stage = self.stage();
        self.previews[stage].update(dt, &mut self.supplies);

        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            self.confirm()
        } else {
            None
        }
    }

    pub fn draw(&self) {
        let preview = &self.previews[self.stage()];
        world::draw(&self.supplies, self.window);
        brushes::draw_panel(None, Vec2::NEG_ONE);
        toys::draw_panel(None, Vec2::NEG_ONE);
        Dispenser::default().draw(&self.supplies, &[]);
        preview.draw();
        draw_rectangle(
            0.0,
            0.0,
            world::WIDTH,
            world::HEIGHT,
            Color::new(0.0, 0.0, 0.0, 0.35),
        );

        let panel = Rect::new(world::WIDTH / 2.0 - 230.0, 140.0, 460.0, 250.0);
        let cx = world::WIDTH / 2.0;
        draw_rectangle(
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            Color::new(1.0, 0.96, 0.88, 1.0),
        );
        draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 4.0, INK);

        draw_centered("Cat Life", cx, panel.y + 55.0, 44.0, INK);
        let prompt = format!("Name your {} cat:", ORDINALS[self.stage()]);
        draw_centered(&prompt, cx, panel.y + 105.0, 28.0, INK);

        // Text box with a blinking cursor.
        let input = Rect::new(panel.x + 50.0, panel.y + 125.0, panel.w - 100.0, 48.0);
        draw_rectangle(input.x, input.y, input.w, input.h, WHITE);
        draw_rectangle_lines(input.x, input.y, input.w, input.h, 3.0, INK);
        let text_y = input.y + 33.0;
        if self.text.is_empty() {
            view::draw_text(
                DEFAULT_NAMES[self.stage()],
                input.x + 12.0,
                text_y,
                34.0,
                Color::new(0.7, 0.65, 0.6, 1.0),
            );
        } else {
            view::draw_text(&self.text, input.x + 12.0, text_y, 34.0, INK);
        }
        if get_time() % 1.0 < 0.5 {
            let typed_width = view::text_width(&self.text, 34.0);
            draw_rectangle(input.x + 14.0 + typed_width, input.y + 9.0, 3.0, 30.0, INK);
        }

        let count = format!("{}/{MAX_LEN}", self.text.chars().count());
        view::draw_text(
            &count,
            input.x + input.w - 62.0,
            input.y + input.h + 22.0,
            20.0,
            INK,
        );
        let hint = if self.stage() + 1 < CAT_COUNT {
            "Press Enter for the next cat"
        } else {
            "Press Enter to start"
        };
        draw_centered(hint, cx, panel.y + 225.0, 26.0, INK);
    }
}

fn draw_centered(text: &str, center_x: f32, baseline_y: f32, size: f32, color: Color) {
    let width = view::text_width(text, size);
    view::draw_text(text, center_x - width / 2.0, baseline_y, size, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(text: &str) -> Naming {
        let mut naming = Naming::new();
        text.chars().for_each(|c| naming.type_char(c));
        naming
    }

    #[test]
    fn typed_characters_become_the_name() {
        assert_eq!(typed("Luna").name(), "Luna");
        assert_eq!(typed("Mr. Whiskers 2").name(), "Mr. Whiskers 2");
    }

    #[test]
    fn empty_or_blank_input_gives_the_default_name() {
        assert_eq!(typed("").name(), DEFAULT_NAMES[0]);
        assert_eq!(
            typed("   ").name(),
            DEFAULT_NAMES[0],
            "leading spaces are ignored"
        );
    }

    #[test]
    fn surrounding_spaces_are_trimmed() {
        assert_eq!(typed("Luna   ").name(), "Luna");
    }

    #[test]
    fn control_and_non_ascii_characters_are_ignored() {
        // Enter, backspace, tab and a glyph the default font can't draw.
        assert_eq!(typed("Lu\r\u{8}\t\u{7f}é🐱na").name(), "Luna");
    }

    #[test]
    fn the_name_is_capped_at_the_maximum_length() {
        let naming = typed(&"x".repeat(MAX_LEN + 10));
        assert_eq!(naming.name().chars().count(), MAX_LEN);
    }

    #[test]
    fn backspace_removes_the_last_character_and_is_safe_when_empty() {
        let mut naming = typed("Lunaa");
        naming.backspace();
        assert_eq!(naming.name(), "Luna");

        let mut empty = Naming::new();
        empty.backspace();
        assert_eq!(empty.name(), DEFAULT_NAMES[0]);
    }

    #[test]
    fn both_cats_are_named_one_after_the_other() {
        let mut naming = typed("Luna");
        assert_eq!(naming.confirm(), None, "the second cat still needs a name");
        assert_eq!(naming.name(), DEFAULT_NAMES[1], "text box starts empty");

        "Shadow".chars().for_each(|c| naming.type_char(c));
        assert_eq!(
            naming.confirm(),
            Some(["Luna".to_owned(), "Shadow".to_owned()])
        );
    }

    #[test]
    fn each_cat_gets_its_own_default_name() {
        let mut naming = Naming::new();
        assert_eq!(naming.confirm(), None);
        assert_eq!(
            naming.confirm(),
            Some(DEFAULT_NAMES.map(str::to_owned)),
            "Enter alone names both cats"
        );
    }
}
