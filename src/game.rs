//! One play session: the cats, their supplies and the player's input.

use macroquad::prelude::*;

use crate::audio::Audio;
use crate::brushes::{self, Brush};
use crate::cat::{CAT_COUNT, Cat, Coat};
use crate::dispenser::Dispenser;
use crate::supplies::Supplies;
use crate::toys::{self, Toy};
use crate::ui::{self, ShelfItem};
use crate::view;
use crate::window::Window;
use crate::world;

/// What the player has taken off a wall panel. Only one thing fits in a hand.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Held {
    Brush(Brush),
    Toy(Toy),
}

impl Held {
    const fn brush(held: Option<Self>) -> Option<Brush> {
        match held {
            Some(Self::Brush(brush)) => Some(brush),
            _ => None,
        }
    }

    const fn toy(held: Option<Self>) -> Option<Toy> {
        match held {
            Some(Self::Toy(toy)) => Some(toy),
            _ => None,
        }
    }
}

pub struct Game {
    /// The first cat is ginger, the second black with white spots. They share
    /// the bowls, the litter box and the home.
    cats: [Cat; CAT_COUNT],
    supplies: Supplies,
    /// Pours food and water into the bowls.
    dispenser: Dispenser,
    /// The big window on the wall, which the player can open.
    window: Window,
    /// The brush or toy the player has taken off a panel, if any. Clicking a
    /// cat brushes it while a brush is held; a toy is played with by moving
    /// the mouse around and holding Space.
    held: Option<Held>,
    /// `None` if the sounds couldn't be loaded; the game is then silent.
    audio: Option<Audio>,
}

impl Game {
    pub fn new(cat_names: [String; CAT_COUNT], audio: Option<Audio>) -> Self {
        let [first, second] = cat_names;
        Self {
            cats: [
                Cat::new(first),
                Cat::with_coat(second, Coat::BLACK_AND_WHITE, world::SECOND_CAT_START),
            ],
            supplies: Supplies::default(),
            dispenser: Dispenser::default(),
            window: Window::default(),
            held: None,
            audio,
        }
    }

    pub fn update(&mut self, dt: f32) {
        let pointer = view::mouse();

        if is_mouse_button_pressed(MouseButton::Left) {
            match ui::item_at(pointer) {
                Some(item) => self.use_item(item),
                None => self.click_the_room(pointer),
            }
        }

        // Right-click puts the brush or toy back.
        if is_mouse_button_pressed(MouseButton::Right) {
            self.held = None;
        }

        for cat in &mut self.cats {
            cat.update(dt, &mut self.supplies);
        }
        if let Some(toy) = self.toy_in_play() {
            for cat in &mut self.cats {
                cat.play_with(toy, pointer, dt);
            }
        }
        self.dispenser.update(dt, &mut self.supplies);
        self.supplies.litter.update(dt);
        self.window.update(dt);

        if let Some(audio) = &mut self.audio {
            if is_key_pressed(KeyCode::M) {
                audio.toggle_mute();
            }
            audio.update(dt, &self.cats, &self.supplies);
        }
    }

    /// The toy being played with: the one in hand, while Space is held down.
    fn toy_in_play(&self) -> Option<Toy> {
        Held::toy(self.held).filter(|_| is_key_down(KeyCode::Space))
    }

    /// Takes `thing` off its panel, or puts it back if it is already in hand.
    /// Taking one puts down whatever was in hand before.
    fn take_or_put_back(&mut self, thing: Held) {
        self.held = (self.held != Some(thing)).then_some(thing);
    }

    /// A click that wasn't on the shelf: a brush or a toy on a panel first,
    /// then a dispenser button, then a cat (they can be standing in front of
    /// the window), then the window.
    fn click_the_room(&mut self, pointer: Vec2) {
        if let Some(brush) = brushes::brush_at(pointer) {
            self.take_or_put_back(Held::Brush(brush));
        } else if let Some(toy) = toys::toy_at(pointer) {
            self.take_or_put_back(Held::Toy(toy));
        } else if let Some(button) = Dispenser::button_at(pointer) {
            // Does nothing if the bowl can't take any more.
            self.dispenser.press(button, &self.supplies);
        } else if !self.touch_cat_at(pointer) && Window::contains(pointer) {
            self.window.toggle();
        }
    }

    /// Brushes the cat under `pointer` if the player is holding a brush, pets
    /// it otherwise (even with a toy in hand). It is the one in front if two overlap. Returns whether
    /// there was one.
    fn touch_cat_at(&mut self, pointer: Vec2) -> bool {
        let mut order: Vec<usize> = (0..CAT_COUNT).collect();
        order.sort_by(|&a, &b| self.cats[b].depth().total_cmp(&self.cats[a].depth()));
        order.into_iter().any(|i| match Held::brush(self.held) {
            Some(brush) => self.cats[i].brush_if_hit(pointer, brush),
            None => self.cats[i].pet_if_hit(pointer),
        })
    }

    /// Chooses the food the dispenser will pour, or cleans the litter box
    /// (which does nothing if it is already clean).
    fn use_item(&mut self, item: ShelfItem) {
        match item {
            ShelfItem::Food(food) => self.dispenser.select(food),
            ShelfItem::Clean => {
                self.supplies.litter.clean();
            }
        }
    }

    pub fn draw(&self) {
        world::draw(&self.supplies, self.window);
        brushes::draw_panel(Held::brush(self.held), view::mouse());
        toys::draw_panel(Held::toy(self.held), view::mouse());
        self.dispenser
            .draw(&self.supplies, &self.cats.each_ref().map(Cat::waiting_for));

        // The cat further down the room is in front.
        let mut order: Vec<&Cat> = self.cats.iter().collect();
        order.sort_by(|a, b| a.depth().total_cmp(&b.depth()));
        for cat in order {
            cat.draw();
        }
        ui::draw(
            &self.cats,
            &self.supplies,
            self.window,
            &self.dispenser,
            Held::brush(self.held),
            Held::toy(self.held),
            self.audio.as_ref().map(Audio::muted),
        );
        // In the player's hand, in front of everything.
        match self.held {
            Some(Held::Brush(brush)) => brushes::draw_held(brush, view::mouse()),
            Some(Held::Toy(toy)) => toys::draw_held(
                toy,
                view::mouse(),
                self.toy_in_play().is_some(),
                get_time() as f32,
            ),
            None => {}
        }
    }
}
