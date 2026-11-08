//! Sound effects: meows, purring while petted, and crunching (or lapping)
//! while eating and drinking.
//!
//! [`Scheduler`] decides *when* each sound starts and stops from what the cat
//! is doing, and knows nothing about speakers, so it can be tested. [`Audio`]
//! owns the loaded sounds and plays whatever the scheduler asks for.

use macroquad::audio::{
    PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound,
};

use crate::cat::{Cat, Need};
use crate::supplies::{Food, Supplies};
use crate::synth;

/// How many different crunches there are (see [`synth::crunch`]); the meow
/// variants are listed in [`synth::MEOWS`].
const CRUNCH_VARIANTS: usize = 2;

/// Seconds between meows while the cat keeps asking for something.
const MEOW_INTERVAL: f32 = 6.0;
/// Seconds between bites or laps while the cat eats or drinks.
const CHEW_INTERVAL: f32 = 0.5;
const PURR_FADE_IN: f32 = 0.15;
const PURR_FADE_OUT: f32 = 0.35;

// Relative loudness of each sound.
const MEOW_VOLUME: f32 = 0.8;
const CRUNCH_VOLUME: f32 = 0.7;
const LAP_VOLUME: f32 = 0.6;
const PURR_VOLUME: f32 = 0.5;

/// What kind of eating noise to make.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Meal {
    /// Kibble, tuna, chicken: crunch.
    Solid,
    /// Milk and water: lap.
    Liquid,
}

/// Everything the scheduler needs to know about the cat right now.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Inputs {
    /// Asking for something: attention, food or water.
    pub calling: bool,
    pub purring: bool,
    pub eating: Option<Meal>,
}

/// One thing to do with the speakers.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cue {
    /// Play meow variant `n`.
    Meow(usize),
    /// Play crunch variant `n`.
    Crunch(usize),
    Lap,
    /// Start the looping purr, silently; [`Cue::PurrVolume`] follows.
    PurrStart,
    /// Set the purr's volume (`0.0..=1.0`, before the overall purr volume).
    PurrVolume(f32),
    PurrStop,
}

/// Turns the cat's behaviour over time into [`Cue`]s.
#[derive(Debug, Default)]
pub struct Scheduler {
    was_calling: bool,
    meow_timer: f32,
    meows: usize,
    chew_timer: f32,
    chews: usize,
    purr_volume: f32,
    purr_playing: bool,
}

impl Scheduler {
    /// Advances by `dt` seconds, adding to `cues` whatever should happen now.
    pub fn update(&mut self, dt: f32, inputs: Inputs, cues: &mut Vec<Cue>) {
        // Meow as soon as the cat starts asking, then again now and then for
        // as long as it keeps asking.
        if inputs.calling {
            if !self.was_calling || self.meow_timer <= 0.0 {
                cues.push(Cue::Meow(self.meows % synth::MEOWS.len()));
                self.meows += 1;
                self.meow_timer = MEOW_INTERVAL;
            }
            self.meow_timer -= dt;
        }
        self.was_calling = inputs.calling;

        // Chew right away when a meal starts, then once per interval.
        if let Some(meal) = inputs.eating {
            if self.chew_timer <= 0.0 {
                cues.push(match meal {
                    Meal::Solid => Cue::Crunch(self.chews % CRUNCH_VARIANTS),
                    Meal::Liquid => Cue::Lap,
                });
                self.chews += 1;
                self.chew_timer = CHEW_INTERVAL;
            }
            self.chew_timer -= dt;
        } else {
            self.chew_timer = 0.0;
        }

        // Purr in and out so it never starts or stops with a click.
        if inputs.purring {
            self.purr_volume = (self.purr_volume + dt / PURR_FADE_IN).min(1.0);
        } else {
            self.purr_volume = (self.purr_volume - dt / PURR_FADE_OUT).max(0.0);
        }
        if self.purr_volume > 0.0 {
            if !self.purr_playing {
                cues.push(Cue::PurrStart);
                self.purr_playing = true;
            }
            cues.push(Cue::PurrVolume(self.purr_volume));
        } else if self.purr_playing {
            cues.push(Cue::PurrStop);
            self.purr_playing = false;
        }
    }
}

/// The loaded sounds, and the player's choice to mute them.
pub struct Audio {
    meows: Vec<Sound>,
    crunches: Vec<Sound>,
    lap: Sound,
    purr: Sound,
    scheduler: Scheduler,
    /// Reused every frame to avoid allocating.
    cues: Vec<Cue>,
    /// The last food seen in the bowl, so the final bite of milk still laps.
    last_food: Option<Food>,
    muted: bool,
}

impl Audio {
    /// Loads every sound. Returns `None` (after saying why on stderr) if that
    /// fails, and the game carries on in silence.
    pub async fn load() -> Option<Self> {
        match Self::try_load().await {
            Ok(audio) => Some(audio),
            Err(reason) => {
                eprintln!("Sound is off: {reason}");
                None
            }
        }
    }

    async fn try_load() -> Result<Self, String> {
        let mut meows = Vec::new();
        for (seed, &(top_hz, seconds)) in (1..).zip(&synth::MEOWS) {
            meows.push(load_clip(&synth::meow(top_hz, seconds, seed)).await?);
        }
        let mut crunches = Vec::new();
        for seed in 1..=CRUNCH_VARIANTS as u32 {
            crunches.push(load_clip(&synth::crunch(seed)).await?);
        }

        Ok(Self {
            meows,
            crunches,
            lap: load_clip(&synth::lap(1)).await?,
            purr: load_clip(&synth::purr()).await?,
            scheduler: Scheduler::default(),
            cues: Vec::new(),
            last_food: None,
            muted: false,
        })
    }

    pub const fn muted(&self) -> bool {
        self.muted
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if self.muted {
            // Cut everything off now rather than letting the purr fade out.
            stop_sound(&self.purr);
            self.scheduler = Scheduler::default();
        }
    }

    /// Plays whatever the cats' behaviour calls for this frame.
    pub fn update(&mut self, dt: f32, cats: &[Cat], supplies: &Supplies) {
        if let Some((food, _)) = supplies.food.contents() {
            self.last_food = Some(food);
        }

        let inputs = if self.muted {
            Inputs::default()
        } else {
            Inputs {
                calling: cats.iter().any(Cat::is_calling),
                purring: cats.iter().any(Cat::is_purring),
                eating: cats
                    .iter()
                    .find_map(Cat::feeding)
                    .map(|need| self.meal_for(need)),
            }
        };

        let mut cues = std::mem::take(&mut self.cues);
        cues.clear();
        self.scheduler.update(dt, inputs, &mut cues);
        for cue in cues.drain(..) {
            self.play(cue);
        }
        self.cues = cues;
    }

    fn meal_for(&self, need: Need) -> Meal {
        match (need, self.last_food) {
            (Need::Hunger, Some(food)) if food != Food::Milk => Meal::Solid,
            (Need::Hunger, None) => Meal::Solid,
            _ => Meal::Liquid,
        }
    }

    fn play(&self, cue: Cue) {
        let once = |sound: &Sound, volume: f32| {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume,
                },
            );
        };
        match cue {
            Cue::Meow(n) => once(&self.meows[n % self.meows.len()], MEOW_VOLUME),
            Cue::Crunch(n) => once(&self.crunches[n % self.crunches.len()], CRUNCH_VOLUME),
            Cue::Lap => once(&self.lap, LAP_VOLUME),
            Cue::PurrStart => play_sound(
                &self.purr,
                PlaySoundParams {
                    looped: true,
                    volume: 0.0,
                },
            ),
            Cue::PurrVolume(volume) => set_sound_volume(&self.purr, volume * PURR_VOLUME),
            Cue::PurrStop => stop_sound(&self.purr),
        }
    }
}

async fn load_clip(samples: &[f32]) -> Result<Sound, String> {
    load_sound_from_bytes(&synth::wav_bytes(samples))
        .await
        .map_err(|error| format!("could not load a sound: {error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// Runs the scheduler for `seconds` with fixed inputs, collecting cues.
    fn run(scheduler: &mut Scheduler, seconds: f32, inputs: Inputs) -> Vec<Cue> {
        let mut all = Vec::new();
        for _ in 0..(seconds / DT).round() as usize {
            scheduler.update(DT, inputs, &mut all);
        }
        all
    }

    fn meows(cues: &[Cue]) -> Vec<Cue> {
        cues.iter()
            .copied()
            .filter(|c| matches!(c, Cue::Meow(_)))
            .collect()
    }

    fn calling() -> Inputs {
        Inputs {
            calling: true,
            ..Inputs::default()
        }
    }

    fn eating(meal: Meal) -> Inputs {
        Inputs {
            eating: Some(meal),
            ..Inputs::default()
        }
    }

    fn purring() -> Inputs {
        Inputs {
            purring: true,
            ..Inputs::default()
        }
    }

    #[test]
    fn a_quiet_cat_makes_no_sound() {
        let mut scheduler = Scheduler::default();
        assert!(run(&mut scheduler, 30.0, Inputs::default()).is_empty());
    }

    #[test]
    fn the_cat_meows_when_it_starts_asking() {
        let mut scheduler = Scheduler::default();
        let cues = run(&mut scheduler, 0.1, calling());
        assert_eq!(cues, [Cue::Meow(0)]);
    }

    #[test]
    fn it_keeps_meowing_now_and_then_while_it_waits() {
        let mut scheduler = Scheduler::default();
        let cues = run(&mut scheduler, MEOW_INTERVAL * 2.5, calling());
        // At the start, then after one interval, then after two.
        assert_eq!(meows(&cues).len(), 3, "{cues:?}");
    }

    #[test]
    fn it_does_not_meow_again_before_the_interval() {
        let mut scheduler = Scheduler::default();
        let cues = run(&mut scheduler, MEOW_INTERVAL - 0.5, calling());
        assert_eq!(meows(&cues).len(), 1);
    }

    #[test]
    fn meows_cycle_through_the_variants() {
        let mut scheduler = Scheduler::default();
        let cues = run(&mut scheduler, MEOW_INTERVAL * 3.5, calling());
        let variants: Vec<_> = meows(&cues)
            .into_iter()
            .map(|c| match c {
                Cue::Meow(n) => n,
                other => unreachable!("{other:?}"),
            })
            .collect();
        assert_eq!(variants, [0, 1, 2, 0]);
    }

    #[test]
    fn asking_again_after_stopping_meows_straight_away() {
        let mut scheduler = Scheduler::default();
        run(&mut scheduler, 1.0, calling());
        run(&mut scheduler, 1.0, Inputs::default());
        let cues = run(&mut scheduler, 0.1, calling());
        assert_eq!(meows(&cues).len(), 1, "a new request gets a new meow");
    }

    #[test]
    fn solid_food_is_crunched_straight_away_then_on_a_beat() {
        let mut scheduler = Scheduler::default();
        let cues = run(&mut scheduler, 0.1, eating(Meal::Solid));
        assert_eq!(cues, [Cue::Crunch(0)], "first bite is immediate");

        let mut scheduler = Scheduler::default();
        let cues = run(&mut scheduler, 2.0, eating(Meal::Solid));
        assert_eq!(
            cues,
            [
                Cue::Crunch(0),
                Cue::Crunch(1),
                Cue::Crunch(0),
                Cue::Crunch(1)
            ],
            "one bite every {CHEW_INTERVAL} s, alternating variants"
        );
    }

    #[test]
    fn liquids_are_lapped_not_crunched() {
        let mut scheduler = Scheduler::default();
        let cues = run(&mut scheduler, 1.2, eating(Meal::Liquid));
        assert_eq!(cues, [Cue::Lap, Cue::Lap, Cue::Lap]);
    }

    #[test]
    fn a_new_meal_starts_with_a_bite_right_away() {
        let mut scheduler = Scheduler::default();
        run(&mut scheduler, 0.1, eating(Meal::Solid));
        run(&mut scheduler, 0.1, Inputs::default());
        let cues = run(&mut scheduler, 0.1, eating(Meal::Solid));
        assert_eq!(cues.len(), 1);
    }

    #[test]
    fn purring_fades_in_loops_and_fades_out_without_cutting_off() {
        let mut scheduler = Scheduler::default();

        let start = run(&mut scheduler, 0.5, purring());
        assert_eq!(start[0], Cue::PurrStart);
        assert_eq!(start.iter().filter(|c| **c == Cue::PurrStart).count(), 1);
        let Some(Cue::PurrVolume(first)) = start.get(1).copied() else {
            panic!("volume should follow the start: {start:?}");
        };
        assert!(first < 0.5, "fades in from silence, was {first}");
        assert_eq!(start.last(), Some(&Cue::PurrVolume(1.0)));

        let end = run(&mut scheduler, 1.0, Inputs::default());
        assert_eq!(end.last(), Some(&Cue::PurrStop));
        let volumes: Vec<f32> = end
            .iter()
            .filter_map(|c| match c {
                Cue::PurrVolume(v) => Some(*v),
                _ => None,
            })
            .collect();
        assert!(
            volumes.windows(2).all(|w| w[1] < w[0]),
            "fades out smoothly"
        );
        assert_eq!(end.iter().filter(|c| **c == Cue::PurrStop).count(), 1);
    }

    #[test]
    fn a_purr_that_is_renewed_keeps_playing() {
        let mut scheduler = Scheduler::default();
        run(&mut scheduler, 1.0, purring());
        // Petted again just before it fades away completely.
        run(&mut scheduler, 0.1, Inputs::default());
        let cues = run(&mut scheduler, 1.0, purring());
        assert!(!cues.contains(&Cue::PurrStart), "no restart: {cues:?}");
        assert!(!cues.contains(&Cue::PurrStop));
    }

    #[test]
    fn sounds_are_independent_of_each_other() {
        let mut scheduler = Scheduler::default();
        let both = Inputs {
            calling: true,
            purring: true,
            eating: Some(Meal::Solid),
        };
        let cues = run(&mut scheduler, 0.1, both);
        assert!(cues.contains(&Cue::Meow(0)));
        assert!(cues.contains(&Cue::Crunch(0)));
        assert!(cues.contains(&Cue::PurrStart));
    }
}
