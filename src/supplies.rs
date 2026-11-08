//! Everything the player looks after for the cat: foods, the bowls and the litter box.

use macroquad::prelude::*;

/// Something the player can put in the bowl.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Food {
    Kibble,
    Tuna,
    Chicken,
    Milk,
}

impl Food {
    pub const ALL: [Self; 4] = [Self::Kibble, Self::Tuna, Self::Chicken, Self::Milk];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Kibble => "Kibble",
            Self::Tuna => "Tuna",
            Self::Chicken => "Chicken",
            Self::Milk => "Milk",
        }
    }

    pub const fn color(self) -> Color {
        match self {
            Self::Kibble => Color::new(0.6, 0.4, 0.25, 1.0),
            Self::Tuna => Color::new(0.9, 0.55, 0.5, 1.0),
            Self::Chicken => Color::new(0.95, 0.8, 0.55, 1.0),
            Self::Milk => Color::new(0.97, 0.97, 1.0, 1.0),
        }
    }

    /// How much hunger a whole portion satisfies (`1.0` = a completely empty
    /// tummy to a completely full one).
    pub const fn nutrition(self) -> f32 {
        match self {
            Self::Kibble => 0.55,
            Self::Tuna => 0.45,
            Self::Chicken => 0.65,
            Self::Milk => 0.30,
        }
    }

    /// How much fun a whole portion adds because it tastes so good.
    pub const fn joy(self) -> f32 {
        match self {
            Self::Kibble => 0.0,
            Self::Tuna => 0.20,
            Self::Chicken => 0.10,
            Self::Milk => 0.15,
        }
    }
}

/// What the cat got out of one mouthful.
#[derive(Clone, Copy, Debug)]
pub struct Bite {
    pub nutrition: f32,
    pub joy: f32,
}

#[derive(Clone, Copy, Debug)]
struct Portion {
    food: Food,
    /// Nutrition still left in the bowl.
    remaining: f32,
}

/// The cat's bowl: empty until the player puts one portion of food in it.
#[derive(Debug, Default)]
pub struct Bowl {
    portion: Option<Portion>,
}

impl Bowl {
    pub const fn is_empty(&self) -> bool {
        self.portion.is_none()
    }

    /// The food in the bowl and how full the bowl is (`0.0..=1.0`).
    pub fn contents(&self) -> Option<(Food, f32)> {
        self.portion
            .map(|p| (p.food, (p.remaining / p.food.nutrition()).clamp(0.0, 1.0)))
    }

    /// Puts a fresh portion of `food` in the bowl.
    /// Returns `false` (and does nothing) if the bowl is not empty.
    pub fn fill(&mut self, food: Food) -> bool {
        if !self.is_empty() {
            return false;
        }
        self.portion = Some(Portion {
            food,
            remaining: food.nutrition(),
        });
        true
    }

    /// Eats up to `nutrition` worth of food. `None` if the bowl is empty.
    pub fn take(&mut self, nutrition: f32) -> Option<Bite> {
        let portion = self.portion.as_mut()?;
        let eaten = nutrition.min(portion.remaining);
        portion.remaining -= eaten;
        let bite = Bite {
            nutrition: eaten,
            joy: eaten * portion.food.joy() / portion.food.nutrition(),
        };
        if portion.remaining <= f32::EPSILON {
            self.portion = None;
        }
        Some(bite)
    }
}

/// A bowl of water. Unlike the food bowl it can be topped up at any time,
/// and it lasts for several drinks.
#[derive(Debug, Default)]
pub struct WaterBowl {
    /// `0.0` = empty, `1.0` = full. A full bowl would quench a completely
    /// parched cat.
    level: f32,
}

impl WaterBowl {
    pub fn level(&self) -> f32 {
        self.level
    }

    pub fn is_empty(&self) -> bool {
        self.level <= f32::EPSILON
    }

    pub fn is_full(&self) -> bool {
        self.level >= 1.0
    }

    /// Tops the bowl up to the brim. Returns `false` if it was already full.
    pub fn fill(&mut self) -> bool {
        if self.is_full() {
            return false;
        }
        self.level = 1.0;
        true
    }

    /// Drinks up to `amount` of thirst's worth. `None` if the bowl is empty.
    pub fn take(&mut self, amount: f32) -> Option<Bite> {
        if self.is_empty() {
            return None;
        }
        let drunk = amount.min(self.level);
        self.level -= drunk;
        if self.is_empty() {
            self.level = 0.0;
        }
        Some(Bite {
            nutrition: drunk,
            joy: 0.0,
        })
    }
}

/// How long the sparkle after cleaning lasts, in seconds.
const SPARKLE_SECONDS: f32 = 0.8;

/// The cat's litter box: clean until the cat uses it, then the player has to
/// clean it before the cat will use it again.
#[derive(Debug, Default)]
pub struct LitterBox {
    dirty: bool,
    /// Seconds of sparkle left after the box was cleaned.
    sparkle_left: f32,
}

impl LitterBox {
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// The cat has used the box.
    pub const fn soil(&mut self) {
        self.dirty = true;
        self.sparkle_left = 0.0;
    }

    /// The player cleans the box. Returns `false` (and does nothing) if it
    /// was already clean.
    pub fn clean(&mut self) -> bool {
        if !self.dirty {
            return false;
        }
        self.dirty = false;
        self.sparkle_left = SPARKLE_SECONDS;
        true
    }

    /// Lets time pass, so the sparkle after cleaning fades.
    pub fn update(&mut self, dt: f32) {
        self.sparkle_left = (self.sparkle_left - dt).max(0.0);
    }

    /// How far through the "just cleaned" sparkle the box is
    /// (`0.0..1.0`), or `None` when there is no sparkle.
    pub fn sparkle(&self) -> Option<f32> {
        (self.sparkle_left > 0.0).then(|| 1.0 - self.sparkle_left / SPARKLE_SECONDS)
    }
}

/// Everything the player looks after for the cat: its two bowls and its
/// litter box.
#[derive(Debug, Default)]
pub struct Supplies {
    pub food: Bowl,
    pub water: WaterBowl,
    pub litter: LitterBox,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_litter_box_starts_clean() {
        assert!(!LitterBox::default().is_dirty());
    }

    #[test]
    fn using_the_box_makes_it_dirty_and_cleaning_makes_it_clean() {
        let mut litter = LitterBox::default();
        litter.soil();
        assert!(litter.is_dirty());

        assert!(litter.clean());
        assert!(!litter.is_dirty());
    }

    #[test]
    fn cleaning_a_clean_box_does_nothing() {
        let mut litter = LitterBox::default();
        assert!(!litter.clean());
        assert!(litter.sparkle().is_none(), "nothing to sparkle about");
    }

    #[test]
    fn a_freshly_cleaned_box_sparkles_for_a_moment() {
        let mut litter = LitterBox::default();
        litter.soil();
        litter.clean();

        let start = litter.sparkle().expect("sparkles right after cleaning");
        assert!(start < 0.05, "starts near 0, was {start}");

        litter.update(SPARKLE_SECONDS / 2.0);
        let middle = litter.sparkle().expect("still sparkling");
        assert!((middle - 0.5).abs() < 0.01, "halfway, was {middle}");

        litter.update(SPARKLE_SECONDS);
        assert!(litter.sparkle().is_none(), "the sparkle fades out");
    }

    #[test]
    fn soiling_the_box_ends_any_sparkle() {
        let mut litter = LitterBox::default();
        litter.soil();
        litter.clean();
        litter.soil();
        assert!(litter.sparkle().is_none());
    }

    #[test]
    fn water_bowl_starts_empty_and_can_be_topped_up_until_full() {
        let mut water = WaterBowl::default();
        assert!(water.is_empty());

        assert!(water.fill());
        assert!(water.is_full());
        assert!(!water.fill(), "a full bowl can't be filled again");

        water.take(0.3);
        assert!(!water.is_full() && !water.is_empty());
        assert!(water.fill(), "a part-full bowl can be topped up");
        assert!(water.is_full());
    }

    #[test]
    fn drinking_drains_the_water_bowl_and_stops_when_dry() {
        let mut water = WaterBowl::default();
        water.fill();

        let mut drunk = 0.0;
        while let Some(bite) = water.take(0.01) {
            drunk += bite.nutrition;
            assert!(bite.joy.abs() < f32::EPSILON, "water has no flavour");
        }

        assert!((drunk - 1.0).abs() < 1e-3);
        assert!(water.is_empty());
        assert!(water.take(0.1).is_none());
    }

    #[test]
    fn bowl_starts_empty() {
        let bowl = Bowl::default();
        assert!(bowl.is_empty());
        assert!(bowl.contents().is_none());
    }

    #[test]
    fn fill_only_works_on_an_empty_bowl() {
        let mut bowl = Bowl::default();
        assert!(bowl.fill(Food::Tuna));
        assert!(!bowl.fill(Food::Kibble), "second fill must be refused");
        assert_eq!(bowl.contents().map(|(f, _)| f), Some(Food::Tuna));
    }

    #[test]
    fn eating_a_whole_portion_gives_its_full_value_and_empties_the_bowl() {
        for food in Food::ALL {
            let mut bowl = Bowl::default();
            bowl.fill(food);

            let (mut nutrition, mut joy) = (0.0, 0.0);
            while let Some(bite) = bowl.take(0.01) {
                nutrition += bite.nutrition;
                joy += bite.joy;
            }

            assert!((nutrition - food.nutrition()).abs() < 1e-3, "{food:?}");
            assert!((joy - food.joy()).abs() < 1e-3, "{food:?}");
            assert!(bowl.is_empty(), "{food:?}");
        }
    }

    #[test]
    fn partial_bites_leave_food_in_the_bowl() {
        let mut bowl = Bowl::default();
        bowl.fill(Food::Chicken);
        bowl.take(0.2);

        let (_, fullness) = bowl.contents().expect("food left");
        assert!(fullness > 0.0 && fullness < 1.0);
    }

    #[test]
    fn taking_from_an_empty_bowl_gives_nothing() {
        assert!(Bowl::default().take(0.1).is_none());
    }
}
