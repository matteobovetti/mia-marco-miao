//! The cat: needs, behaviour (a tiny state machine) and drawing.

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::brushes::{self, Brush, Pose};
use crate::home;
use crate::sofa;
use crate::supplies::{LitterBox, Supplies};
use crate::toys::Toy;
use crate::view;
use crate::world::{self, Level, Place};

const WALK_SPEED: f32 = 160.0;
/// Below this level a need is worth acting on.
const URGENT: f32 = 0.45;
/// The cat stops eating or drinking once the need is at least this satisfied.
const FULL: f32 = 0.98;
const ARRIVE_DIST: f32 = 4.0;

// The ladder up to the home's upper floor and the slide back down.
/// How long the climb takes, and the slide.
const CLIMB_SECONDS: f32 = 2.2;
const SLIDE_SECONDS: f32 = 1.0;
/// How long a jump between the floor and the top of the sofa takes.
const JUMP_SECONDS: f32 = 0.6;

// Litter box.
/// How long the cat spends in the box.
const LITTER_USE_SECONDS: f32 = 3.5;
/// Potty need relieved per second while in the box. The cat is completely
/// relieved when it is done, whatever it started at.
const LITTER_RELIEF_PER_SEC: f32 = 0.25;

// Brushing.
/// How long one go with the brush lasts.
const BRUSH_SECONDS: f32 = 4.0;
/// How often a heart floats up from a cat that is being brushed.
const BRUSH_HEART_SECONDS: f32 = 0.6;
/// How far the brush sweeps along the cat's back to each side, and how fast.
const BRUSH_SWEEP: f32 = 24.0;
const BRUSH_SWEEP_SPEED: f32 = 7.0;

// Playing with a toy.
/// How fast a cat runs after a toy.
const CHASE_SPEED: f32 = 250.0;
/// A cat stops this far from the toy and bats at it.
const CATCH_DIST: f32 = 70.0;
/// A cat keeps its eyes on the toy this long after the player stops playing.
const PLAY_LINGER: f32 = 1.5;
/// A cat with a need below this has no time for toys.
const TOO_BUSY: f32 = 0.2;

/// How far from the cat's paws its "body centre" is (for click detection).
const BODY_CENTRE: Vec2 = vec2(0.0, -35.0);
const PET_RADIUS: f32 = 55.0;

/// How many cats share the home.
pub const CAT_COUNT: usize = 2;

/// An oval patch of a different colour on a cat's coat, placed relative to the
/// middle of its body or head.
#[derive(Clone, Copy, Debug)]
struct Spot {
    dx: f32,
    dy: f32,
    rx: f32,
    ry: f32,
}

const fn spot(dx: f32, dy: f32, rx: f32, ry: f32) -> Spot {
    Spot { dx, dy, rx, ry }
}

/// What a cat looks like: its colours and markings. Behaviour is the same for
/// every coat.
#[derive(Clone, Copy, Debug)]
pub struct Coat {
    fur: Color,
    /// Ears, legs and tail.
    dark: Color,
    belly: Color,
    /// The colour of the eyes when open.
    iris: Color,
    /// Closed eyes, and the whiskers.
    line: Color,
    /// White (or otherwise) patches on the body and on the head.
    body_spots: &'static [Spot],
    head_spots: &'static [Spot],
}

impl Coat {
    /// The ginger cat.
    pub const ORANGE: Self = Self {
        fur: Color::new(0.93, 0.6, 0.25, 1.0),
        dark: Color::new(0.75, 0.42, 0.15, 1.0),
        belly: Color::new(1.0, 0.85, 0.65, 1.0),
        iris: BLACK,
        line: BLACK,
        body_spots: &[],
        head_spots: &[],
    };

    /// A black cat with some white spots.
    pub const BLACK_AND_WHITE: Self = Self {
        fur: Color::new(0.13, 0.13, 0.16, 1.0),
        dark: Color::new(0.06, 0.06, 0.08, 1.0),
        belly: Color::new(0.26, 0.26, 0.3, 1.0),
        // Eyes can't be black on a black face.
        iris: Color::new(0.95, 0.8, 0.2, 1.0),
        line: Color::new(0.85, 0.85, 0.88, 1.0),
        body_spots: &[
            spot(-18.0, -7.0, 8.0, 5.0),
            spot(4.0, -12.0, 6.0, 4.5),
            spot(20.0, -1.0, 5.5, 4.0),
            spot(-4.0, 6.0, 4.5, 3.0),
        ],
        head_spots: &[spot(-6.0, -10.0, 4.5, 3.5), spot(9.0, 8.0, 4.0, 3.0)],
    };
}

const SPOT_WHITE: Color = Color::new(0.97, 0.97, 0.95, 1.0);

fn draw_spots(spots: &[Spot], centre: Vec2) {
    for spot in spots {
        draw_ellipse(
            centre.x + spot.dx,
            centre.y + spot.dy,
            spot.rx,
            spot.ry,
            0.0,
            SPOT_WHITE,
        );
    }
}

/// Everything the cat needs to be happy. `0.0` = desperate, `1.0` = content.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Need {
    Hunger,
    Thirst,
    /// Needing the litter box.
    Bladder,
    Energy,
    Fun,
    Affection,
}

impl Need {
    pub const ALL: [Self; 6] = [
        Self::Hunger,
        Self::Thirst,
        Self::Bladder,
        Self::Energy,
        Self::Fun,
        Self::Affection,
    ];
    pub const COUNT: usize = Self::ALL.len();

    pub const fn label(self) -> &'static str {
        match self {
            Self::Hunger => "Food",
            Self::Thirst => "Water",
            Self::Bladder => "Potty",
            Self::Energy => "Rest",
            Self::Fun => "Fun",
            Self::Affection => "Love",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::Hunger => Color::new(0.85, 0.5, 0.2, 1.0),
            Self::Thirst => Color::new(0.3, 0.6, 0.9, 1.0),
            Self::Bladder => Color::new(0.75, 0.65, 0.25, 1.0),
            Self::Energy => Color::new(0.4, 0.45, 0.85, 1.0),
            Self::Fun => Color::new(0.3, 0.75, 0.4, 1.0),
            Self::Affection => Color::new(0.9, 0.35, 0.55, 1.0),
        }
    }

    /// How much the need drops per second (about 1.5-2.5 minutes to empty).
    const fn decay_per_sec(self) -> f32 {
        match self {
            Self::Hunger => 1.0 / 120.0,
            Self::Thirst => 1.0 / 110.0,
            Self::Bladder => 1.0 / 140.0,
            Self::Energy => 1.0 / 150.0,
            Self::Fun => 1.0 / 90.0,
            Self::Affection => 1.0 / 100.0,
        }
    }

    /// How much the need recovers per second while the cat is using it.
    ///
    /// Hunger and thirst are restored by what the player puts in the bowls
    /// (see [`Self::intake_per_sec`]), the bladder by using the litter box,
    /// and affection by the player petting the cat, so none of those recover
    /// through this.
    const fn restore_per_sec(self) -> f32 {
        match self {
            Self::Energy => 0.07,
            Self::Fun => 0.18,
            Self::Hunger | Self::Thirst | Self::Bladder | Self::Affection => 0.0,
        }
    }

    /// How fast the cat eats or drinks from its bowl, in need restored per
    /// second. Zero for needs that don't come from a bowl.
    const fn intake_per_sec(self) -> f32 {
        match self {
            Self::Hunger => 0.12,
            Self::Thirst => 0.2,
            Self::Bladder | Self::Energy | Self::Fun | Self::Affection => 0.0,
        }
    }

    /// Longest the cat keeps doing this before getting bored. For hunger,
    /// thirst and the bladder this is how long it waits at an empty bowl or
    /// a dirty litter box.
    const fn max_duration(self) -> f32 {
        match self {
            Self::Affection => 6.0,
            _ => 30.0,
        }
    }

    /// Where the cat can go to satisfy this need, and what it faces there.
    /// There is more than one place to nap. Empty if it needs no place.
    fn places(self) -> &'static [Place] {
        match self {
            Self::Hunger => &HUNGER_PLACES,
            Self::Thirst => &THIRST_PLACES,
            Self::Bladder => &BLADDER_PLACES,
            Self::Energy => &ENERGY_PLACES,
            // Nothing to play with at the moment (tasty food still cheers the
            // cat up), and affection comes from the player.
            Self::Fun | Self::Affection => &[],
        }
    }
}

static HUNGER_PLACES: [Place; 1] = [Place::ground(world::BOWL_SPOT, world::BOWL)];
static THIRST_PLACES: [Place; 1] = [Place::ground(world::WATER_SPOT, world::WATER)];
static BLADDER_PLACES: [Place; 1] = [Place::ground(world::LITTER_SPOT, world::LITTER)];
/// The cushions downstairs and upstairs in its home, or the top of the sofa.
static ENERGY_PLACES: [Place; 4] = [
    home::GROUND_BED,
    home::UPPER_BED,
    sofa::LEFT_NAP,
    sofa::RIGHT_NAP,
];

#[derive(Clone, Copy, Debug)]
enum Activity {
    /// Sitting around, doing cat things.
    Idle { timer: f32 },
    /// Walking somewhere (`to`, on floor `level`), optionally to do something
    /// on arrival. If that's on the other floor, the cat goes by the ladder or
    /// the slide.
    Walking {
        to: Vec2,
        level: Level,
        then: Option<Need>,
        object: Option<Vec2>,
    },
    /// Going up the ladder, `progress` of the way, on the way to `goal`.
    Climbing { progress: f32, goal: Goal },
    /// Going down the slide, `progress` of the way, on the way to `goal`.
    Sliding { progress: f32, goal: Goal },
    /// Jumping between the floor and the top of the sofa (`up` says which
    /// way), `progress` of the way, on the way to `goal`.
    Jumping { progress: f32, up: bool, goal: Goal },
    /// Sleeping, playing, asking for attention, or (for `Hunger` and `Thirst`)
    /// standing at a bowl: eating or drinking if there is something in it,
    /// otherwise waiting until `timer` runs out of patience. For `Bladder` it
    /// is always the waiting: the cat is at a dirty litter box.
    Doing { need: Need, timer: f32 },
    /// In the litter box for `timer` more seconds.
    UsingLitter { timer: f32 },
    /// Being petted.
    Purring { timer: f32 },
    /// Being brushed with `brush` for `timer` more seconds.
    Brushing { timer: f32, brush: Brush },
    /// Chasing `toy`, or watching it, for `timer` more seconds unless the
    /// player keeps playing. `running` says whether the cat is on the move.
    Playing { toy: Toy, timer: f32, running: bool },
}

/// Where the cat is heading, and what it will do when it gets there.
#[derive(Clone, Copy, Debug)]
struct Goal {
    to: Vec2,
    level: Level,
    then: Option<Need>,
    /// What to face on arrival.
    object: Option<Vec2>,
}

impl Goal {
    fn to_place(place: Place, then: Option<Need>) -> Self {
        Self {
            to: place.spot,
            level: place.level,
            then,
            object: Some(place.object),
        }
    }

    /// The cat sets off for this goal, walking.
    const fn walking(self) -> Activity {
        Activity::Walking {
            to: self.to,
            level: self.level,
            then: self.then,
            object: self.object,
        }
    }
}

struct Heart {
    pos: Vec2,
    age: f32,
}

pub struct Cat {
    name: String,
    /// Position of the paws.
    pos: Vec2,
    /// Which floor of the home the cat is on.
    floor: Level,
    /// `1.0` = looking right, `-1.0` = looking left.
    facing: f32,
    needs: [f32; Need::COUNT],
    activity: Activity,
    /// The need the cat is satisfying from a bowl right now, i.e. actually
    /// chewing or lapping (set every update).
    feeding: Option<Need>,
    clock: f32,
    hearts: Vec<Heart>,
    coat: Coat,
}

impl Cat {
    /// A ginger cat where the first cat starts.
    pub fn new(name: impl Into<String>) -> Self {
        Self::with_coat(name, Coat::ORANGE, world::CAT_START)
    }

    /// A cat with the given looks, starting at `pos`.
    pub fn with_coat(name: impl Into<String>, coat: Coat, pos: Vec2) -> Self {
        Self {
            name: name.into(),
            pos,
            floor: Level::Ground,
            facing: 1.0,
            // In `Need::ALL` order: hunger, thirst, bladder, energy, fun,
            // affection.
            needs: [0.8, 0.8, 0.85, 0.9, 0.7, 0.6],
            activity: Activity::Idle { timer: 1.0 },
            feeding: None,
            clock: 0.0,
            hearts: Vec::new(),
            coat,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// How far down the room the cat's paws are: the cat with the larger
    /// value is in front.
    pub const fn depth(&self) -> f32 {
        self.pos.y
    }

    pub fn level(&self, need: Need) -> f32 {
        self.needs[need as usize]
    }

    /// What the cat is waiting for the player to provide: a bowl with
    /// something in it ([`Need::Hunger`] or [`Need::Thirst`], from when the
    /// cat heads for the bowl), or a clean litter box ([`Need::Bladder`], only
    /// once the cat is at the dirty box).
    pub const fn waiting_for(&self) -> Option<Need> {
        match self.activity {
            Activity::Walking {
                then: Some(need @ (Need::Hunger | Need::Thirst)),
                ..
            } => Some(need),
            Activity::Doing {
                need: need @ (Need::Hunger | Need::Thirst),
                ..
            } if self.feeding.is_none() => Some(need),
            Activity::Doing {
                need: Need::Bladder,
                ..
            } => Some(Need::Bladder),
            _ => None,
        }
    }

    /// What the cat is saying in its speech bubble while it asks for
    /// attention, food or water.
    pub const fn bubble(&self) -> Option<&'static str> {
        match self.activity {
            Activity::Doing {
                need: Need::Affection,
                ..
            } => Some("Meow!"),
            _ => match self.waiting_for() {
                Some(Need::Hunger) => Some("Feed me!"),
                Some(Need::Thirst) => Some("Water!"),
                Some(Need::Bladder) => Some("Clean it!"),
                _ => None,
            },
        }
    }

    /// Whether the cat is calling out for something (see [`Self::bubble`]).
    pub const fn is_calling(&self) -> bool {
        self.bubble().is_some()
    }

    /// Whether the cat is purring because it is being petted or brushed.
    pub const fn is_purring(&self) -> bool {
        matches!(
            self.activity,
            Activity::Purring { .. } | Activity::Brushing { .. }
        )
    }

    /// The need the cat is satisfying from a bowl this very moment: it is
    /// eating ([`Need::Hunger`]) or drinking ([`Need::Thirst`]).
    pub const fn feeding(&self) -> Option<Need> {
        self.feeding
    }

    /// One-line description of what the cat is up to.
    pub const fn status(&self) -> &'static str {
        match self.activity {
            Activity::Idle { .. } => "Sitting and judging you",
            Activity::Walking { then: None, .. } => "Wandering around",
            Activity::Walking {
                then: Some(Need::Hunger),
                ..
            } => "Heading to the food bowl",
            Activity::Walking {
                then: Some(Need::Thirst),
                ..
            } => "Heading to the water bowl",
            Activity::Walking {
                then: Some(Need::Bladder),
                ..
            } => "Heading to the litter box",
            Activity::Walking {
                then: Some(Need::Energy),
                ..
            } => "Looking for a nap spot",
            Activity::Walking {
                then: Some(Need::Fun),
                ..
            } => "Off to play",
            Activity::Walking {
                then: Some(Need::Affection),
                ..
            } => "Wants attention",
            Activity::Doing {
                need: Need::Hunger, ..
            } => {
                if self.feeding.is_some() {
                    "Eating"
                } else {
                    "Hungry! Pick a food on the right"
                }
            }
            Activity::Doing {
                need: Need::Thirst, ..
            } => {
                if self.feeding.is_some() {
                    "Drinking"
                } else {
                    "Thirsty! Fill the water bowl"
                }
            }
            Activity::Doing {
                need: Need::Bladder,
                ..
            } => "The litter box is dirty! Clean it",
            Activity::UsingLitter { .. } => "Using the litter box",
            Activity::Doing {
                need: Need::Energy, ..
            } => "Sleeping",
            Activity::Doing {
                need: Need::Fun, ..
            } => "Playing",
            Activity::Doing {
                need: Need::Affection,
                ..
            } => "Meow! (click to pet)",
            Activity::Purring { .. } => "Purring",
            Activity::Brushing { .. } => "Being brushed. Bliss!",
            Activity::Playing { toy, .. } => toy.chasing(),
            Activity::Climbing { .. } => "Climbing the ladder",
            Activity::Sliding { .. } => "Wheee! Down the slide",
            Activity::Jumping { up: true, .. } => "Jumping up on the sofa",
            Activity::Jumping { up: false, .. } => "Jumping down from the sofa",
        }
    }

    /// Whether `click` landed on a cat that can be petted or brushed right
    /// now. Nobody likes being disturbed in the litter box, and a cat on the
    /// ladder, the slide or in mid-jump is out of reach.
    fn can_be_touched_at(&self, click: Vec2) -> bool {
        !matches!(
            self.activity,
            Activity::UsingLitter { .. }
                | Activity::Climbing { .. }
                | Activity::Sliding { .. }
                | Activity::Jumping { .. }
        ) && click.distance(self.pos + BODY_CENTRE) <= PET_RADIUS
    }

    /// Floats a heart up from the cat's head.
    fn add_heart(&mut self) {
        self.hearts.push(Heart {
            pos: self.pos + vec2(gen_range(-20.0, 20.0), -85.0),
            age: 0.0,
        });
    }

    /// Pets the cat if `click` landed on it. Returns whether it did. A cat
    /// that is being brushed keeps being brushed: it takes the click, but
    /// nothing changes.
    pub fn pet_if_hit(&mut self, click: Vec2) -> bool {
        if !self.can_be_touched_at(click) {
            return false;
        }
        if matches!(self.activity, Activity::Brushing { .. }) {
            return true;
        }
        let affection = &mut self.needs[Need::Affection as usize];
        *affection = (*affection + 0.3).min(1.0);
        self.activity = Activity::Purring { timer: 1.5 };
        self.add_heart();
        true
    }

    /// Starts brushing the cat with `brush` if `click` landed on it, right
    /// away. Returns whether it did.
    pub fn brush_if_hit(&mut self, click: Vec2, brush: Brush) -> bool {
        if !self.can_be_touched_at(click) {
            return false;
        }
        self.activity = Activity::Brushing {
            timer: BRUSH_SECONDS,
            brush,
        };
        self.add_heart();
        true
    }

    /// Advances the cat by `dt` seconds.
    pub fn update(&mut self, dt: f32, supplies: &mut Supplies) {
        self.clock += dt;
        self.feeding = None;

        for need in Need::ALL {
            let level = &mut self.needs[need as usize];
            *level = (*level - need.decay_per_sec() * dt).max(0.0);
        }

        self.follow_routine(dt, supplies);

        for heart in &mut self.hearts {
            heart.age += dt;
            heart.pos.y -= 30.0 * dt;
        }
        self.hearts.retain(|h| h.age < 1.2);
    }

    /// One step of the cat's everyday state machine.
    fn follow_routine(&mut self, dt: f32, supplies: &mut Supplies) {
        match self.activity {
            Activity::Idle { timer } if timer > dt => {
                self.activity = Activity::Idle { timer: timer - dt };
            }
            Activity::Idle { .. } => self.decide(),
            Activity::Walking {
                to,
                level,
                then,
                object,
            } => {
                let goal = Goal {
                    to,
                    level,
                    then,
                    object,
                };
                self.walk(goal, dt, &supplies.litter);
            }
            Activity::Climbing { progress, goal } => self.climb(progress, goal, dt),
            Activity::Sliding { progress, goal } => self.slide(progress, goal, dt),
            Activity::Jumping { progress, up, goal } => self.jump(progress, up, goal, dt),
            Activity::Doing {
                need: need @ (Need::Hunger | Need::Thirst),
                timer,
            } => self.at_bowl(need, timer, dt, supplies),
            Activity::Doing {
                need: Need::Bladder,
                timer,
            } => self.at_litter_box(timer, dt, &supplies.litter),
            Activity::Doing { need, timer } => self.do_activity(need, timer, dt),
            Activity::UsingLitter { timer } => self.use_litter_box(timer, dt, &mut supplies.litter),
            Activity::Purring { timer } if timer > dt => {
                self.activity = Activity::Purring { timer: timer - dt };
            }
            Activity::Purring { .. } => {
                self.activity = Activity::Idle { timer: 1.0 };
            }
            Activity::Brushing { timer, brush } => self.being_brushed(timer, brush, dt),
            // The player keeps the cat playing (see `play_with`); once they
            // stop, it loses interest.
            Activity::Playing { toy, timer, .. } if timer > dt => {
                self.activity = Activity::Playing {
                    toy,
                    timer: timer - dt,
                    running: false,
                };
            }
            Activity::Playing { .. } => {
                self.activity = Activity::Idle {
                    timer: gen_range(1.0, 3.0),
                };
            }
        }
    }

    /// Whether a toy could tempt the cat right now: it is on the room's floor,
    /// not busy with something it won't give up (sleeping, eating, using the
    /// litter box, being brushed) and has no pressing need.
    fn can_be_lured(&self) -> bool {
        let free = matches!(
            self.activity,
            Activity::Idle { .. }
                | Activity::Playing { .. }
                | Activity::Walking {
                    then: None | Some(Need::Fun | Need::Affection),
                    ..
                }
                | Activity::Doing {
                    need: Need::Fun | Need::Affection,
                    ..
                }
        );
        let pressing = [Need::Hunger, Need::Thirst, Need::Bladder, Need::Energy]
            .into_iter()
            .any(|need| self.level(need) < TOO_BUSY);
        free && !pressing && self.floor == Level::Ground
    }

    /// Plays with `toy`, which the player is working at `point` on the
    /// canvas, for the `dt` seconds of this frame: the cat runs after it, bats
    /// at it when it catches up, and has fun. Call it every frame the toy is
    /// active, after [`Self::update`]. Cats that aren't in the mood ignore it.
    pub fn play_with(&mut self, toy: Toy, point: Vec2, dt: f32) {
        if !self.can_be_lured() {
            return;
        }
        let target = world::floor_point_near(point);
        let running = self.pos.distance(target) > CATCH_DIST;
        if running {
            self.step_toward_at(target, CHASE_SPEED, dt);
        } else if (point.x - self.pos.x).abs() > ARRIVE_DIST {
            self.facing = (point.x - self.pos.x).signum();
        }

        let fun = &mut self.needs[Need::Fun as usize];
        *fun = (*fun + toy.fun_per_sec() * dt).min(1.0);
        self.activity = Activity::Playing {
            toy,
            timer: PLAY_LINGER,
            running,
        };
    }

    /// Being brushed: it feels lovely, and the cat purrs for a moment once the
    /// brushing is over.
    fn being_brushed(&mut self, timer: f32, brush: Brush, dt: f32) {
        let affection = &mut self.needs[Need::Affection as usize];
        *affection = (*affection + brush.love_per_sec() * dt).min(1.0);

        // A heart every so often.
        let hearts_before = (timer / BRUSH_HEART_SECONDS).floor();
        let hearts_after = ((timer - dt) / BRUSH_HEART_SECONDS).floor();
        if hearts_after < hearts_before {
            self.add_heart();
        }

        self.activity = if timer > dt {
            Activity::Brushing {
                timer: timer - dt,
                brush,
            }
        } else {
            Activity::Purring { timer: 1.0 }
        };
    }

    /// Picks what to do next: the most urgent need, otherwise wander.
    fn decide(&mut self) {
        let urgent = Need::ALL
            .into_iter()
            .filter(|need| !need.places().is_empty())
            .min_by(|a, b| self.level(*a).total_cmp(&self.level(*b)))
            .filter(|&need| self.level(need) < URGENT);

        let goal = if let Some(need) = urgent {
            // Somewhere to satisfy it: for a nap, any of several places.
            let places = need.places();
            Goal::to_place(places[gen_range(0, places.len())], Some(need))
        } else if self.level(Need::Affection) < URGENT && gen_range(0.0, 1.0) < 0.5 {
            let plea = Place::ground(world::PLEA_SPOT, world::PLEA_SPOT);
            Goal::to_place(plea, Some(Need::Affection))
        } else {
            Goal::to_place(home::random_place(), None)
        };
        self.activity = goal.walking();
    }

    /// Takes a step towards `target` along the current floor, and says whether
    /// the cat has got there. On the ground it goes round the home if need be.
    fn step_toward(&mut self, target: Vec2, dt: f32) -> bool {
        self.step_toward_at(target, WALK_SPEED, dt)
    }

    /// Like [`Self::step_toward`], at `speed` units per second.
    fn step_toward_at(&mut self, target: Vec2, speed: f32, dt: f32) -> bool {
        let waypoint = match self.floor {
            Level::Ground => home::ground_waypoint(self.pos, target),
            Level::Upper | Level::Sofa => target,
        };
        let delta = waypoint - self.pos;
        let dist = delta.length();
        let step = speed * dt;

        if dist <= step.max(ARRIVE_DIST) {
            self.pos = waypoint;
            // Reaching a corner on the way round isn't arriving.
            return waypoint == target;
        }
        let dir = delta / dist;
        self.pos += dir * step;
        if dir.x.abs() > 0.2 {
            self.facing = dir.x.signum();
        }
        false
    }

    /// Walks towards the goal. If it is on the other floor, the way there is
    /// by the ladder (up) or the slide (down). The top of the sofa is reached
    /// by a jump from the floor in front of it, and left by a jump back down.
    fn walk(&mut self, goal: Goal, dt: f32, litter: &LitterBox) {
        if self.floor == goal.level {
            if self.step_toward(goal.to, dt) {
                self.arrive(goal, litter);
            }
            return;
        }

        if self.floor == Level::Sofa {
            self.activity = Activity::Jumping {
                progress: 0.0,
                up: false,
                goal,
            };
            return;
        }
        // The sofa is reached from the room's floor: a cat upstairs in the
        // home slides down first.
        if goal.level == Level::Sofa && self.floor == Level::Ground {
            let take_off = vec2(goal.to.x, sofa::TAKE_OFF_Y);
            if self.step_toward(take_off, dt) {
                self.activity = Activity::Jumping {
                    progress: 0.0,
                    up: true,
                    goal,
                };
            }
            return;
        }

        let going_up = self.floor == Level::Ground;
        let way = if going_up {
            home::LADDER_BASE
        } else {
            home::SLIDE_TOP
        };
        if self.step_toward(way, dt) {
            // Both the ladder and the slide face right.
            self.facing = 1.0;
            self.activity = if going_up {
                Activity::Climbing {
                    progress: 0.0,
                    goal,
                }
            } else {
                Activity::Sliding {
                    progress: 0.0,
                    goal,
                }
            };
        }
    }

    /// Climbs the ladder to the upper floor, then carries on to the goal.
    fn climb(&mut self, progress: f32, goal: Goal, dt: f32) {
        let progress = progress + dt / CLIMB_SECONDS;
        if progress >= 1.0 {
            self.pos = home::LADDER_TOP;
            self.floor = Level::Upper;
            self.activity = goal.walking();
        } else {
            self.pos = home::ladder_position(progress);
            self.activity = Activity::Climbing { progress, goal };
        }
    }

    /// Slides down to the room's floor, then carries on to the goal.
    fn slide(&mut self, progress: f32, goal: Goal, dt: f32) {
        let progress = progress + dt / SLIDE_SECONDS;
        if progress >= 1.0 {
            self.pos = home::SLIDE_END;
            self.floor = Level::Ground;
            self.activity = goal.walking();
        } else {
            self.pos = home::slide_position(progress);
            self.activity = Activity::Sliding { progress, goal };
        }
    }

    /// Jumps up onto the sofa or down from it, then carries on to the goal.
    fn jump(&mut self, progress: f32, up: bool, goal: Goal, dt: f32) {
        let progress = progress + dt / JUMP_SECONDS;
        // The jump goes straight up or down, so the cat keeps its `x`.
        self.pos = sofa::jump_position(self.pos.x, progress.min(1.0), up);
        if progress >= 1.0 {
            self.floor = if up { Level::Sofa } else { Level::Ground };
            self.activity = goal.walking();
        } else {
            self.activity = Activity::Jumping { progress, up, goal };
        }
    }

    /// Gets to where it was going: does what it came to do, or just stops.
    fn arrive(&mut self, goal: Goal, litter: &LitterBox) {
        self.activity = if let Some(need) = goal.then {
            if let Some(object) = goal.object {
                self.facing = if object.x < self.pos.x { -1.0 } else { 1.0 };
            }
            Activity::Doing {
                need,
                timer: need.max_duration(),
            }
        } else {
            Activity::Idle {
                timer: gen_range(1.5, 4.0),
            }
        };
        // Look at the litter box on arrival, so that a clean one is used
        // straight away instead of the cat calling out for a frame.
        if goal.then == Some(Need::Bladder) {
            self.at_litter_box(Need::Bladder.max_duration(), 0.0, litter);
        }
    }

    /// Standing at the litter box: get in if it's clean, otherwise wait for
    /// the player to clean it until patience runs out.
    fn at_litter_box(&mut self, patience: f32, dt: f32, litter: &LitterBox) {
        self.activity = if !litter.is_dirty() {
            Activity::UsingLitter {
                timer: LITTER_USE_SECONDS,
            }
        } else if patience <= dt {
            Activity::Idle {
                timer: gen_range(1.0, 3.0),
            }
        } else {
            Activity::Doing {
                need: Need::Bladder,
                timer: patience - dt,
            }
        };
    }

    /// In the litter box. The box only gets dirty when the cat is done, so
    /// the player can't clean it while it is being used.
    fn use_litter_box(&mut self, timer: f32, dt: f32, litter: &mut LitterBox) {
        let bladder = &mut self.needs[Need::Bladder as usize];
        if timer > dt {
            *bladder = (*bladder + LITTER_RELIEF_PER_SEC * dt).min(1.0);
            self.activity = Activity::UsingLitter { timer: timer - dt };
        } else {
            *bladder = 1.0;
            litter.soil();
            self.activity = Activity::Idle {
                timer: gen_range(1.0, 3.0),
            };
        }
    }

    /// Standing at the food or water bowl (`need` says which): eat or drink
    /// while there is something in it and the cat wants it, otherwise wait for
    /// the player to fill it until patience runs out.
    fn at_bowl(&mut self, need: Need, patience: f32, dt: f32, supplies: &mut Supplies) {
        let level = self.level(need);
        let idle = Activity::Idle {
            timer: gen_range(1.0, 3.0),
        };

        // Bites are capped by 1.0 rather than `FULL`: capping at `FULL` would
        // let the need's decay keep the cat hovering just below it, nibbling
        // forever instead of walking away from the bowl.
        let wanted = (need.intake_per_sec() * dt).min(1.0 - level);
        let bite = if level >= FULL {
            None
        } else {
            match need {
                Need::Hunger => supplies.food.take(wanted),
                Need::Thirst => supplies.water.take(wanted),
                Need::Bladder | Need::Energy | Need::Fun | Need::Affection => None,
            }
        };

        self.activity = if level >= FULL {
            idle
        } else if let Some(bite) = bite {
            self.feeding = Some(need);
            let fun = &mut self.needs[Need::Fun as usize];
            *fun = (*fun + bite.joy).min(1.0);
            self.needs[need as usize] = (level + bite.nutrition).min(1.0);
            Activity::Doing {
                need,
                timer: patience,
            }
        } else if level >= URGENT || patience <= dt {
            // Satisfied for now, or tired of waiting at an empty bowl.
            idle
        } else {
            Activity::Doing {
                need,
                timer: patience - dt,
            }
        };
    }

    fn do_activity(&mut self, need: Need, timer: f32, dt: f32) {
        let level = &mut self.needs[need as usize];
        *level = (*level + need.restore_per_sec() * dt).min(1.0);

        let full = need.restore_per_sec() > 0.0 && *level >= 0.98;
        self.activity = if full || timer <= dt {
            Activity::Idle {
                timer: gen_range(1.0, 3.0),
            }
        } else {
            Activity::Doing {
                need,
                timer: timer - dt,
            }
        };
    }

    pub fn draw(&self) {
        let sleeping = matches!(
            self.activity,
            Activity::Doing {
                need: Need::Energy,
                ..
            }
        );
        let walking = matches!(
            self.activity,
            Activity::Walking { .. }
                | Activity::Climbing { .. }
                | Activity::Playing { running: true, .. }
        );
        let purring = self.is_purring();

        // Shadow.
        let p = self.pos;
        draw_ellipse(
            p.x,
            p.y + 2.0,
            40.0,
            9.0,
            0.0,
            Color::new(0.0, 0.0, 0.0, 0.2),
        );

        if sleeping {
            self.draw_sleeping();
        } else {
            self.draw_awake(walking, self.feeding.is_some(), purring);
        }

        if let Activity::Brushing { brush, .. } = self.activity {
            self.draw_brush_stroke(brush);
        }

        if let Some(text) = self.bubble() {
            draw_speech_bubble(vec2(p.x - 28.0, p.y - 100.0), text);
        }

        for heart in &self.hearts {
            let alpha = (1.0 - heart.age / 1.2).clamp(0.0, 1.0);
            draw_heart(heart.pos, 8.0, Color::new(0.95, 0.25, 0.4, alpha));
        }
    }

    /// The brush sweeping to and fro along the cat's back.
    fn draw_brush_stroke(&self, brush: Brush) {
        let phase = self.clock * BRUSH_SWEEP_SPEED;
        let back = self.pos + vec2(phase.sin() * BRUSH_SWEEP, -50.0 + phase.cos().abs() * 2.0);
        brushes::draw_brush(
            brush,
            Pose {
                head: back,
                // Handle up and to the right, leaning with the sweep.
                angle: 2.1 + phase.cos() * 0.15,
                scale: 1.0,
                alpha: 1.0,
            },
        );
    }

    fn draw_awake(&self, walking: bool, eating: bool, purring: bool) {
        let Coat {
            fur, dark, belly, ..
        } = self.coat;
        let (p, f) = (self.pos, self.facing);
        // Crouching in the litter box lowers the body.
        let squatting = matches!(
            self.activity,
            Activity::UsingLitter { .. } | Activity::Sliding { .. }
        );
        // Batting at a toy it has caught up with.
        let batting = matches!(self.activity, Activity::Playing { running: false, .. });
        let bob = if walking {
            (self.clock * 14.0).sin() * 2.5
        } else if squatting {
            10.0
        } else if batting {
            (self.clock * 16.0).sin() * 3.0
        } else {
            0.0
        };

        // Tail.
        let wag_speed = if purring {
            6.0
        } else if batting || walking && matches!(self.activity, Activity::Playing { .. }) {
            12.0
        } else {
            2.5
        };
        let wag = (self.clock * wag_speed).sin() * 10.0;
        draw_line(
            p.x - f * 30.0,
            p.y - 30.0,
            p.x - f * 52.0,
            p.y - 58.0 + wag,
            7.0,
            dark,
        );

        // Legs: two pairs that swing when walking.
        let swing = if walking {
            (self.clock * 14.0).sin() * 6.0
        } else {
            0.0
        };
        for (dx, s) in [
            (-22.0, swing),
            (-10.0, -swing),
            (10.0, swing),
            (22.0, -swing),
        ] {
            draw_rectangle(p.x + dx - 4.0 + s, p.y - 16.0, 9.0, 16.0, dark);
        }

        // Body.
        draw_ellipse(p.x, p.y - 30.0 + bob, 34.0, 22.0, 0.0, fur);
        draw_ellipse(p.x, p.y - 24.0 + bob, 20.0, 12.0, 0.0, belly);
        draw_spots(self.coat.body_spots, vec2(p.x, p.y - 30.0 + bob));

        // A front paw swatting at the toy.
        if batting {
            let swat = (self.clock * 16.0).sin().max(0.0);
            draw_circle(
                p.x + f * (40.0 + swat * 16.0),
                p.y - 22.0 - swat * 8.0,
                6.0,
                dark,
            );
        }

        // Head (lowered while eating).
        let head = vec2(
            p.x + f * 32.0,
            p.y - 50.0 + bob + if eating { 18.0 } else { 0.0 },
        );
        draw_triangle(
            vec2(head.x - 15.0, head.y - 6.0),
            vec2(head.x - 12.0, head.y - 28.0),
            vec2(head.x - 2.0, head.y - 14.0),
            dark,
        );
        draw_triangle(
            vec2(head.x + 15.0, head.y - 6.0),
            vec2(head.x + 12.0, head.y - 28.0),
            vec2(head.x + 2.0, head.y - 14.0),
            dark,
        );
        draw_circle(head.x, head.y, 18.0, fur);
        draw_spots(self.coat.head_spots, head);

        // Eyes are closed when purring, eating, concentrating or blinking.
        let blinking = self.clock % 4.0 < 0.15;
        draw_face(head, purring || blinking || eating || squatting, &self.coat);
    }

    fn draw_sleeping(&self) {
        let Coat {
            fur, dark, line, ..
        } = self.coat;
        let (p, f) = (self.pos, self.facing);
        let breathe = (self.clock * 2.0).sin() * 1.5;

        // Curled-up body, tail wrapped around the front.
        draw_ellipse(p.x, p.y - 20.0, 38.0, 20.0 + breathe, 0.0, fur);
        draw_ellipse(p.x + f * 4.0, p.y - 8.0, 32.0, 7.0, 0.0, dark);
        draw_spots(self.coat.body_spots, vec2(p.x, p.y - 22.0));

        // Head resting on paws.
        let head = vec2(p.x + f * 26.0, p.y - 22.0);
        draw_triangle(
            vec2(head.x - 14.0, head.y - 6.0),
            vec2(head.x - 11.0, head.y - 25.0),
            vec2(head.x - 2.0, head.y - 12.0),
            dark,
        );
        draw_triangle(
            vec2(head.x + 14.0, head.y - 6.0),
            vec2(head.x + 11.0, head.y - 25.0),
            vec2(head.x + 2.0, head.y - 12.0),
            dark,
        );
        draw_circle(head.x, head.y, 16.0, fur);
        draw_spots(self.coat.head_spots, head);
        for side in [-1.0, 1.0] {
            let eye = vec2(head.x + side * 6.0, head.y - 2.0);
            draw_line(eye.x - 3.0, eye.y, eye.x + 3.0, eye.y, 2.0, line);
        }
        draw_circle(head.x, head.y + 4.0, 2.5, Color::new(0.95, 0.5, 0.55, 1.0));

        // Floating Zzz.
        for i in 0..3 {
            let phase = (self.clock * 0.8 + i as f32 * 0.33) % 1.0;
            let size = 18.0 + i as f32 * 6.0;
            draw_text(
                "z",
                head.x + 18.0 + phase * 22.0,
                head.y - 22.0 - phase * 34.0,
                size,
                Color::new(0.2, 0.2, 0.5, 1.0 - phase),
            );
        }
    }
}

/// The face of a cat looking sideways-on at the player: eyes, nose and
/// whiskers, around the middle of its head at `head`.
fn draw_face(head: Vec2, eyes_closed: bool, coat: &Coat) {
    for side in [-1.0, 1.0] {
        let eye = vec2(head.x + side * 7.0, head.y - 3.0);
        if eyes_closed {
            draw_line(eye.x - 3.5, eye.y, eye.x + 3.5, eye.y, 2.0, coat.line);
        } else {
            draw_circle(eye.x, eye.y, 3.5, coat.iris);
            draw_circle(eye.x, eye.y, 2.0, BLACK);
            draw_circle(eye.x + 1.0, eye.y - 1.0, 1.2, WHITE);
        }
    }

    // Nose and whiskers.
    draw_triangle(
        vec2(head.x - 3.0, head.y + 3.0),
        vec2(head.x + 3.0, head.y + 3.0),
        vec2(head.x, head.y + 7.0),
        Color::new(0.95, 0.5, 0.55, 1.0),
    );
    for side in [-1.0, 1.0] {
        for (from_y, to_y) in [(5.0, 2.0), (8.0, 10.0)] {
            draw_line(
                head.x + side * 9.0,
                head.y + from_y,
                head.x + side * 26.0,
                head.y + to_y,
                1.5,
                coat.line,
            );
        }
    }
}

fn draw_speech_bubble(top_left: Vec2, text: &str) {
    const FONT_SIZE: f32 = 24.0;
    const PAD: f32 = 6.0;
    let (w, h) = (view::text_width(text, FONT_SIZE) + PAD * 2.0, 28.0);

    draw_rectangle(top_left.x, top_left.y, w, h, WHITE);
    draw_rectangle_lines(top_left.x, top_left.y, w, h, 2.0, BLACK);
    view::draw_text(text, top_left.x + PAD, top_left.y + 21.0, FONT_SIZE, BLACK);
}

fn draw_heart(c: Vec2, s: f32, color: Color) {
    draw_circle(c.x - s * 0.5, c.y - s * 0.25, s * 0.55, color);
    draw_circle(c.x + s * 0.5, c.y - s * 0.25, s * 0.55, color);
    draw_triangle(
        vec2(c.x - s * 0.98, c.y),
        vec2(c.x + s * 0.98, c.y),
        vec2(c.x, c.y + s * 1.1),
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supplies::Food;

    const DT: f32 = 1.0 / 60.0;

    fn simulate(cat: &mut Cat, supplies: &mut Supplies, seconds: f32) {
        for _ in 0..(seconds / DT) as usize {
            cat.update(DT, supplies);
        }
    }

    fn cat_with(need: Need, level: f32) -> Cat {
        let mut cat = Cat::new("Test");
        cat.needs[need as usize] = level;
        cat
    }

    /// A cat already standing at the bowl for `need` (hunger or thirst).
    fn cat_at_bowl(need: Need, level: f32) -> Cat {
        let mut cat = cat_with(need, level);
        cat.pos = need.places()[0].spot;
        cat.activity = Activity::Doing {
            need,
            timer: need.max_duration(),
        };
        cat
    }

    fn is_sleeping(cat: &Cat) -> bool {
        matches!(
            cat.activity,
            Activity::Doing {
                need: Need::Energy,
                ..
            }
        )
    }

    fn is_climbing(cat: &Cat) -> bool {
        matches!(cat.activity, Activity::Climbing { .. })
    }

    fn is_sliding(cat: &Cat) -> bool {
        matches!(cat.activity, Activity::Sliding { .. })
    }

    /// Whether the cat, on the ground floor, is where a cat may be in the
    /// space the home takes up: only downstairs inside it, on its floor.
    fn in_the_home_properly(pos: Vec2) -> bool {
        (home::INTERIOR_LEFT..=home::INTERIOR_RIGHT).contains(&pos.x)
            && pos.y >= home::GROUND_FLOOR - 0.01
    }

    /// A cat on the given floor, standing at `spot` and heading for `goal`.
    fn cat_going(from: Place, to: Place, then: Option<Need>) -> Cat {
        let mut cat = Cat::new("Test");
        cat.pos = from.spot;
        cat.floor = from.level;
        cat.activity = Goal::to_place(to, then).walking();
        cat
    }

    /// A cat that is very tired, so a nap at its destination lasts a while.
    fn sleepy(mut cat: Cat) -> Cat {
        cat.needs[Need::Energy as usize] = 0.05;
        cat
    }

    /// A cat that is very hungry, so it waits at its bowl.
    fn starving(mut cat: Cat) -> Cat {
        cat.needs[Need::Hunger as usize] = 0.05;
        cat
    }

    /// Looks after the cat perfectly, so it never has to wait.
    fn look_after(supplies: &mut Supplies) {
        supplies.food.fill(Food::Kibble);
        supplies.water.fill();
        supplies.litter.clean();
    }

    // --- Basics ---------------------------------------------------------

    #[test]
    fn the_cat_keeps_the_name_it_was_given() {
        assert_eq!(Cat::new("Luna").name(), "Luna");
        assert_eq!(
            Cat::new(String::from("Mr. Whiskers")).name(),
            "Mr. Whiskers"
        );
    }

    #[test]
    fn a_black_cat_looks_after_its_needs_just_like_a_ginger_one() {
        let mut cat = Cat::with_coat("Domino", Coat::BLACK_AND_WHITE, world::SECOND_CAT_START);
        cat.needs[Need::Hunger as usize] = 0.05;
        let mut supplies = Supplies::default();
        supplies.food.fill(Food::Kibble);

        let (mut ate, mut peak) = (false, 0.0_f32);
        for _ in 0..(60.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            ate |= cat.feeding == Some(Need::Hunger);
            peak = peak.max(cat.level(Need::Hunger));
        }

        assert!(ate, "the black cat never ate");
        assert!(peak > 0.5, "hunger was not restored (peak {peak})");
    }

    #[test]
    fn a_cat_starts_where_it_is_told() {
        let start = world::SECOND_CAT_START;
        let cat = Cat::with_coat("Domino", Coat::BLACK_AND_WHITE, start);
        assert_eq!(cat.pos, start);
        assert_eq!(cat.name(), "Domino");
    }

    // --- Playing with toys ----------------------------------------------

    fn is_playing(cat: &Cat) -> bool {
        matches!(cat.activity, Activity::Playing { .. })
    }

    /// The player works `toy` at `point` for `seconds`, a frame at a time.
    fn play(cat: &mut Cat, supplies: &mut Supplies, toy: Toy, point: Vec2, seconds: f32) {
        for _ in 0..(seconds * 60.0) as usize {
            cat.update(1.0 / 60.0, supplies);
            cat.play_with(toy, point, 1.0 / 60.0);
        }
    }

    #[test]
    fn a_cat_runs_after_the_toy_and_stops_close_to_it() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();
        let toy_at = vec2(300.0, 450.0);

        play(&mut cat, &mut supplies, Toy::RedBall, toy_at, 10.0);

        assert!(is_playing(&cat));
        let gap = cat.pos.distance(toy_at);
        assert!(gap <= CATCH_DIST + 5.0, "still {gap} away");
        assert!(gap > 20.0, "the cat should not sit on the toy: {gap}");
    }

    #[test]
    fn a_cat_runs_faster_after_a_toy_than_it_walks() {
        let mut cat = Cat::new("Test");
        let start = cat.pos;

        cat.play_with(Toy::Mouse, start + vec2(-500.0, 0.0), 0.1);

        let moved = cat.pos.distance(start);
        assert!(moved > WALK_SPEED * 0.1 + 1.0, "only moved {moved}");
        assert!(matches!(
            cat.activity,
            Activity::Playing { running: true, .. }
        ));
    }

    #[test]
    fn playing_makes_the_cat_have_fun_the_more_the_better_the_toy() {
        let fun_after_playing = |toy| {
            let mut cat = cat_with(Need::Fun, 0.1);
            let mut supplies = Supplies::default();
            let toy_at = cat.pos + vec2(40.0, 0.0);
            play(&mut cat, &mut supplies, toy, toy_at, 2.0);
            cat.level(Need::Fun)
        };

        let fun = Toy::ALL.map(fun_after_playing);

        assert!(fun.iter().all(|&level| level > 0.3), "{fun:?}");
        assert!(
            fun[0] < fun[3],
            "the laser is no match for a mouse: {fun:?}"
        );
    }

    #[test]
    fn the_cat_loses_interest_when_the_player_stops() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();
        play(&mut cat, &mut supplies, Toy::Yarn, vec2(500.0, 450.0), 2.0);
        assert!(is_playing(&cat));

        simulate(&mut cat, &mut supplies, PLAY_LINGER - 0.5);
        assert!(is_playing(&cat), "still watching the yarn");
        assert!(matches!(
            cat.activity,
            Activity::Playing { running: false, .. }
        ));

        simulate(&mut cat, &mut supplies, 1.0);
        assert!(!is_playing(&cat), "back to its routine");
    }

    #[test]
    fn the_cat_chases_a_laser_dot_on_the_wall_along_the_floor() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();

        play(
            &mut cat,
            &mut supplies,
            Toy::Laser,
            vec2(200.0, 100.0),
            10.0,
        );

        assert!(cat.pos.y >= world::FLOOR_TOP);
        assert!(cat.pos.distance(vec2(200.0, cat.pos.y)) <= CATCH_DIST + 5.0);
    }

    #[test]
    fn the_cat_never_chases_a_toy_into_its_home() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();
        let middle_of_the_home = home::BLOCK.center();

        for _ in 0..(20.0 * 60.0) as usize {
            cat.update(1.0 / 60.0, &mut supplies);
            cat.play_with(Toy::RedBall, middle_of_the_home, 1.0 / 60.0);
            assert!(
                !home::BLOCK.contains(cat.pos),
                "walked through the wall to {}",
                cat.pos
            );
        }
    }

    #[test]
    fn the_cat_faces_the_toy() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();
        for side in [-1.0, 1.0] {
            let toy_at = cat.pos + vec2(side * 40.0, 0.0);
            play(&mut cat, &mut supplies, Toy::Mouse, toy_at, 0.2);
            assert!((cat.facing - side).abs() < 0.01);
        }
    }

    #[test]
    fn cats_that_are_busy_ignore_the_toy() {
        let busy = |activity| {
            let mut cat = Cat::new("Test");
            cat.activity = activity;
            let (start, activity_before) = (cat.pos, format!("{:?}", cat.activity));
            cat.play_with(Toy::Mouse, start + vec2(100.0, 0.0), 0.1);
            cat.pos == start && format!("{:?}", cat.activity) == activity_before
        };

        assert!(busy(Activity::UsingLitter { timer: 2.0 }));
        assert!(busy(Activity::Brushing {
            timer: 2.0,
            brush: Brush::Soft
        }));
        assert!(busy(Activity::Doing {
            need: Need::Energy,
            timer: 20.0
        }));
        assert!(busy(Activity::Purring { timer: 1.0 }));
        assert!(busy(Activity::Walking {
            to: vec2(300.0, 500.0),
            level: Level::Ground,
            then: Some(Need::Hunger),
            object: None,
        }));
    }

    #[test]
    fn a_starving_cat_has_no_time_for_toys() {
        let mut cat = starving(Cat::new("Test"));
        let start = cat.pos;

        cat.play_with(Toy::RedBall, start + vec2(100.0, 0.0), 0.1);

        assert_eq!(cat.pos, start);
        assert!(!is_playing(&cat));
    }

    #[test]
    fn a_cat_on_the_sofa_or_upstairs_ignores_the_toy() {
        for floor in [Level::Sofa, Level::Upper] {
            let mut cat = Cat::new("Test");
            cat.floor = floor;
            let start = cat.pos;

            cat.play_with(Toy::Yarn, start + vec2(100.0, 0.0), 0.1);

            assert_eq!(cat.pos, start, "{floor:?}");
            assert!(!is_playing(&cat), "{floor:?}");
        }
    }

    #[test]
    fn a_cat_can_be_petted_while_it_plays() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();
        let toy_at = cat.pos + vec2(40.0, 0.0);
        play(&mut cat, &mut supplies, Toy::Yarn, toy_at, 1.0);

        assert!(cat.pet_if_hit(cat.pos + BODY_CENTRE));

        assert!(cat.is_purring());
        assert!(!is_playing(&cat));
    }

    // --- Brushing -------------------------------------------------------

    fn is_being_brushed(cat: &Cat) -> bool {
        matches!(cat.activity, Activity::Brushing { .. })
    }

    #[test]
    fn clicking_a_cat_with_a_brush_starts_brushing_it_straight_away() {
        let mut cat = Cat::new("Test");

        assert!(cat.brush_if_hit(cat.pos + BODY_CENTRE, Brush::Soft));

        assert!(is_being_brushed(&cat));
        assert!(cat.is_purring());
        assert_eq!(cat.hearts.len(), 1, "a heart straight away");
    }

    #[test]
    fn a_click_beside_the_cat_does_not_brush_it() {
        let mut cat = Cat::new("Test");
        let start = cat.activity;

        assert!(!cat.brush_if_hit(
            cat.pos + BODY_CENTRE + vec2(PET_RADIUS + 5.0, 0.0),
            Brush::Soft
        ));

        assert!(!is_being_brushed(&cat));
        assert_eq!(format!("{:?}", cat.activity), format!("{start:?}"));
    }

    #[test]
    fn brushing_makes_the_cat_love_you_more_the_better_the_brush() {
        let love_after_brushing = |brush| {
            let mut cat = cat_with(Need::Affection, 0.1);
            let mut supplies = Supplies::default();
            assert!(cat.brush_if_hit(cat.pos + BODY_CENTRE, brush));
            simulate(&mut cat, &mut supplies, BRUSH_SECONDS - 0.5);
            cat.level(Need::Affection)
        };

        let loves = Brush::ALL.map(love_after_brushing);

        assert!(loves[0] > 0.3, "brushing should be rewarding: {loves:?}");
        assert!(loves.windows(2).all(|pair| pair[0] < pair[1]), "{loves:?}");
    }

    #[test]
    fn the_cat_keeps_still_while_brushed_then_purrs_and_carries_on() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();
        let start = cat.pos;
        assert!(cat.brush_if_hit(cat.pos + BODY_CENTRE, Brush::Paddle));

        simulate(&mut cat, &mut supplies, BRUSH_SECONDS - 0.5);
        assert!(is_being_brushed(&cat), "still brushing");
        assert_eq!(cat.pos, start, "stays put");

        simulate(&mut cat, &mut supplies, 1.0);
        assert!(!is_being_brushed(&cat), "brushing is over");
        assert!(cat.is_purring() || matches!(cat.activity, Activity::Idle { .. }));

        simulate(&mut cat, &mut supplies, 5.0);
        assert!(!cat.is_purring(), "back to its routine");
    }

    #[test]
    fn brushing_a_sleeping_cat_wakes_it() {
        let mut cat = cat_with(Need::Energy, 0.5);
        cat.activity = Activity::Doing {
            need: Need::Energy,
            timer: 30.0,
        };

        assert!(cat.brush_if_hit(cat.pos + BODY_CENTRE, Brush::Soft));

        assert!(!is_sleeping(&cat));
        assert!(is_being_brushed(&cat));
    }

    #[test]
    fn a_cat_in_the_litter_box_or_on_the_move_between_floors_cannot_be_brushed() {
        for activity in [
            Activity::UsingLitter { timer: 3.0 },
            Activity::Climbing {
                progress: 0.5,
                goal: Goal::to_place(home::UPPER_BED, None),
            },
            Activity::Sliding {
                progress: 0.5,
                goal: Goal::to_place(home::GROUND_BED, None),
            },
            Activity::Jumping {
                progress: 0.5,
                up: true,
                goal: Goal::to_place(sofa::LEFT_NAP, None),
            },
        ] {
            let mut cat = Cat::new("Test");
            cat.activity = activity;
            assert!(
                !cat.brush_if_hit(cat.pos + BODY_CENTRE, Brush::Slicker),
                "{activity:?}"
            );
            assert!(!is_being_brushed(&cat), "{activity:?}");
        }
    }

    #[test]
    fn petting_does_not_interrupt_the_brushing_but_does_not_open_the_window_either() {
        let mut cat = Cat::new("Test");
        cat.brush_if_hit(cat.pos + BODY_CENTRE, Brush::Soft);
        let affection = cat.level(Need::Affection);

        assert!(cat.pet_if_hit(cat.pos + BODY_CENTRE), "the click is taken");

        assert!(is_being_brushed(&cat));
        assert!((cat.level(Need::Affection) - affection).abs() < f32::EPSILON);
    }

    #[test]
    fn brushing_the_cat_again_starts_a_fresh_go() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();
        cat.brush_if_hit(cat.pos + BODY_CENTRE, Brush::Soft);
        simulate(&mut cat, &mut supplies, BRUSH_SECONDS - 0.5);

        assert!(cat.brush_if_hit(cat.pos + BODY_CENTRE, Brush::Slicker));

        simulate(&mut cat, &mut supplies, BRUSH_SECONDS - 0.5);
        assert!(is_being_brushed(&cat), "a whole new go with the new brush");
    }

    #[test]
    fn needs_decay_over_time() {
        for need in Need::ALL {
            let mut cat = Cat::new("Test");
            let before = cat.level(need);
            cat.update(1.0, &mut Supplies::default());
            assert!(cat.level(need) < before, "{need:?} did not decay");
        }
    }

    #[test]
    fn petting_restores_affection_only_when_clicking_the_cat() {
        let mut cat = cat_with(Need::Affection, 0.2);

        assert!(
            !cat.pet_if_hit(cat.pos + vec2(300.0, 0.0)),
            "missed click must not pet"
        );
        assert!((cat.level(Need::Affection) - 0.2).abs() < f32::EPSILON);

        assert!(cat.pet_if_hit(cat.pos + BODY_CENTRE));
        assert!(cat.level(Need::Affection) > 0.45);
        assert!(matches!(cat.activity, Activity::Purring { .. }));
    }

    #[test]
    fn tired_cat_sleeps_in_one_of_its_beds() {
        let mut cat = cat_with(Need::Energy, 0.05);
        let mut supplies = Supplies::default();

        let mut slept = None;
        for _ in 0..(60.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            if is_sleeping(&cat) {
                slept = Some((cat.pos, cat.floor));
            }
        }

        let (spot, floor) = slept.expect("the cat never went to sleep");
        let bed = Need::Energy
            .places()
            .iter()
            .find(|place| place.spot == spot)
            .expect("slept somewhere that isn't a bed");
        assert_eq!(bed.level, floor, "on the floor the bed is on");
    }

    #[test]
    fn needs_never_leave_zero_to_one() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();

        for _ in 0..30 {
            // Look after the cat.
            supplies.food.fill(Food::Kibble);
            supplies.water.fill();
            supplies.litter.clean();
            simulate(&mut cat, &mut supplies, 10.0);
            for need in Need::ALL {
                assert!(
                    (0.0..=1.0).contains(&cat.level(need)),
                    "{need:?} out of range"
                );
            }
        }
    }

    // --- Food bowl ------------------------------------------------------

    #[test]
    fn hungry_cat_eats_what_the_player_puts_in_the_bowl() {
        let mut cat = cat_with(Need::Hunger, 0.05);
        let mut supplies = Supplies::default();
        supplies.food.fill(Food::Kibble);

        let (mut ate, mut peak) = (false, 0.0_f32);
        for _ in 0..(60.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            ate |= cat.feeding == Some(Need::Hunger);
            peak = peak.max(cat.level(Need::Hunger));
        }

        assert!(ate, "cat never started eating");
        assert!(peak > 0.5, "hunger was not restored (peak {peak})");
        assert!(
            supplies.food.is_empty(),
            "the cat should finish the portion"
        );
    }

    #[test]
    fn hungry_cat_waits_at_an_empty_bowl_then_eats_when_fed() {
        let mut cat = cat_with(Need::Hunger, 0.05);
        let mut supplies = Supplies::default();

        simulate(&mut cat, &mut supplies, 10.0);
        assert_eq!(cat.pos, world::BOWL_SPOT, "cat should wait at the bowl");
        assert_eq!(cat.waiting_for(), Some(Need::Hunger));
        assert!(cat.feeding.is_none());
        assert!(cat.level(Need::Hunger) <= 0.05, "nothing to eat yet");

        supplies.food.fill(Food::Tuna);
        simulate(&mut cat, &mut supplies, 2.0);
        assert_eq!(cat.feeding, Some(Need::Hunger));
        assert_eq!(cat.waiting_for(), None);
        assert!(cat.level(Need::Hunger) > 0.2);
    }

    #[test]
    fn cat_stops_when_full_and_leaves_the_rest() {
        let mut cat = cat_at_bowl(Need::Hunger, 0.9);
        let mut supplies = Supplies::default();
        supplies.food.fill(Food::Chicken);

        simulate(&mut cat, &mut supplies, 3.0);

        assert!(
            !supplies.food.is_empty(),
            "leftovers should stay in the bowl"
        );
        assert!(cat.level(Need::Hunger) > 0.9);
        assert!(cat.feeding.is_none());
    }

    #[test]
    fn milk_alone_does_not_satisfy_a_starving_cat() {
        let mut cat = cat_with(Need::Hunger, 0.05);
        let mut supplies = Supplies::default();
        supplies.food.fill(Food::Milk);

        simulate(&mut cat, &mut supplies, 8.0);

        assert!(supplies.food.is_empty());
        assert!(cat.level(Need::Hunger) < URGENT);
        assert_eq!(
            cat.waiting_for(),
            Some(Need::Hunger),
            "cat should ask for more"
        );
    }

    #[test]
    fn tasty_food_cheers_the_cat_up() {
        let mut cat = cat_at_bowl(Need::Hunger, 0.05);
        cat.needs[Need::Fun as usize] = 0.3;
        let mut supplies = Supplies::default();
        supplies.food.fill(Food::Tuna);

        simulate(&mut cat, &mut supplies, 6.0);

        assert!(cat.level(Need::Fun) > 0.4, "tuna should add fun");
    }

    // --- Water bowl -----------------------------------------------------

    #[test]
    fn thirsty_cat_waits_at_an_empty_water_bowl_then_drinks_when_filled() {
        let mut cat = cat_with(Need::Thirst, 0.05);
        let mut supplies = Supplies::default();

        simulate(&mut cat, &mut supplies, 12.0);
        assert_eq!(cat.pos, world::WATER_SPOT, "cat should wait at the water");
        assert_eq!(cat.waiting_for(), Some(Need::Thirst));
        assert!(cat.level(Need::Thirst) <= 0.05, "nothing to drink yet");

        supplies.water.fill();
        simulate(&mut cat, &mut supplies, 2.0);
        assert_eq!(cat.feeding, Some(Need::Thirst));
        assert_eq!(cat.waiting_for(), None);
        assert!(cat.level(Need::Thirst) > 0.2);
        assert!(!supplies.water.is_full(), "drinking should lower the water");
    }

    #[test]
    fn cat_only_drinks_what_it_needs() {
        let mut cat = cat_at_bowl(Need::Thirst, 0.7);
        let mut supplies = Supplies::default();
        supplies.water.fill();

        simulate(&mut cat, &mut supplies, 3.0);

        assert!(cat.level(Need::Thirst) > 0.9);
        assert!(cat.feeding.is_none(), "should have stopped drinking");
        assert!(
            supplies.water.level() > 0.5,
            "most of the water should be left"
        );
    }

    #[test]
    fn drinking_water_adds_no_fun() {
        let mut cat = cat_at_bowl(Need::Thirst, 0.3);
        cat.needs[Need::Fun as usize] = 0.5;
        let mut supplies = Supplies::default();
        supplies.water.fill();

        simulate(&mut cat, &mut supplies, 1.0);

        assert!(cat.level(Need::Fun) <= 0.5);
    }

    #[test]
    fn the_cat_gives_up_waiting_at_either_empty_bowl() {
        for need in [Need::Hunger, Need::Thirst] {
            let mut cat = cat_at_bowl(need, 0.05);
            let mut supplies = Supplies::default();

            let mut gave_up = false;
            for _ in 0..(40.0 / DT) as usize {
                cat.update(DT, &mut supplies);
                gave_up |= matches!(cat.activity, Activity::Idle { .. });
            }

            assert!(gave_up, "{need:?}: patience should run out after ~30 s");
        }
    }

    // --- Litter box -----------------------------------------------------

    fn using_litter(cat: &Cat) -> bool {
        matches!(cat.activity, Activity::UsingLitter { .. })
    }

    /// A cat waiting at a dirty litter box, as it would after walking there.
    fn cat_at_dirty_box() -> (Cat, Supplies) {
        let mut cat = cat_with(Need::Bladder, 0.05);
        cat.pos = world::LITTER_SPOT;
        cat.activity = Activity::Doing {
            need: Need::Bladder,
            timer: Need::Bladder.max_duration(),
        };
        let mut supplies = Supplies::default();
        supplies.litter.soil();
        (cat, supplies)
    }

    #[test]
    fn a_cat_that_needs_the_toilet_uses_the_clean_litter_box_and_dirties_it() {
        let mut cat = cat_with(Need::Bladder, 0.05);
        let mut supplies = Supplies::default();

        let mut frames_in_box = 0;
        let mut was_dirty_while_in_box = false;
        let mut bladder_on_leaving = None;
        for _ in 0..(30.0 / DT) as usize {
            let was_in_box = using_litter(&cat);
            cat.update(DT, &mut supplies);
            if using_litter(&cat) {
                frames_in_box += 1;
                assert_eq!(cat.pos, world::LITTER_SPOT);
                assert_eq!(cat.feeding(), None, "no crunching or lapping in the box");
                was_dirty_while_in_box |= supplies.litter.is_dirty();
            } else if was_in_box {
                bladder_on_leaving = Some(cat.level(Need::Bladder));
            }
        }

        let seconds_in_box = frames_in_box as f32 * DT;
        assert!(
            (seconds_in_box - LITTER_USE_SECONDS).abs() < 0.2,
            "was in the box for {seconds_in_box} s"
        );
        assert!(!was_dirty_while_in_box, "dirty only once the cat is done");
        assert!(
            supplies.litter.is_dirty(),
            "the box needs cleaning afterwards"
        );
        let relieved = bladder_on_leaving.expect("the cat finished");
        assert!(relieved > 0.99, "relieved on leaving, was {relieved}");
    }

    #[test]
    fn a_cat_that_does_not_need_the_toilet_leaves_the_box_alone() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();

        // Well within the time before the bladder becomes urgent.
        simulate(&mut cat, &mut supplies, 30.0);

        assert!(!supplies.litter.is_dirty());
        assert!(cat.level(Need::Bladder) > URGENT);
    }

    #[test]
    fn a_cat_does_not_use_a_dirty_box_but_asks_for_it_to_be_cleaned() {
        let mut cat = cat_with(Need::Bladder, 0.05);
        let mut supplies = Supplies::default();
        supplies.litter.soil();

        simulate(&mut cat, &mut supplies, 12.0);

        assert_eq!(cat.pos, world::LITTER_SPOT, "it goes to the box anyway");
        assert_eq!(cat.waiting_for(), Some(Need::Bladder));
        assert_eq!(cat.bubble(), Some("Clean it!"));
        assert!(cat.is_calling(), "so that it meows");
        assert!(cat.level(Need::Bladder) <= 0.05, "it can't go yet");
        assert!(!using_litter(&cat));
    }

    #[test]
    fn the_cat_uses_the_box_as_soon_as_the_player_has_cleaned_it() {
        let (mut cat, mut supplies) = cat_at_dirty_box();
        simulate(&mut cat, &mut supplies, 5.0);
        assert!(!using_litter(&cat));

        assert!(supplies.litter.clean());
        simulate(&mut cat, &mut supplies, 0.1);
        assert!(using_litter(&cat), "should get in right away");

        simulate(&mut cat, &mut supplies, LITTER_USE_SECONDS + 0.5);
        assert!(cat.level(Need::Bladder) > 0.9);
        assert!(
            supplies.litter.is_dirty(),
            "and the box needs cleaning again"
        );
        assert_eq!(cat.waiting_for(), None);
    }

    #[test]
    fn arriving_at_a_clean_box_the_cat_never_calls_out() {
        let mut cat = cat_with(Need::Bladder, 0.05);
        let mut supplies = Supplies::default();

        for _ in 0..(20.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            assert_ne!(cat.bubble(), Some("Clean it!"), "at {:?}", cat.activity);
        }
    }

    #[test]
    fn the_cat_gives_up_waiting_at_a_box_nobody_cleans() {
        let (mut cat, mut supplies) = cat_at_dirty_box();

        let mut gave_up = false;
        for _ in 0..(40.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            gave_up |= matches!(cat.activity, Activity::Idle { .. });
        }

        assert!(gave_up, "patience should run out after about 30 seconds");
    }

    #[test]
    fn the_cat_cannot_be_petted_in_the_litter_box() {
        let mut cat = cat_with(Need::Bladder, 0.5);
        cat.activity = Activity::UsingLitter { timer: 3.0 };
        let affection = cat.level(Need::Affection);

        assert!(!cat.pet_if_hit(cat.pos + BODY_CENTRE));

        assert!(using_litter(&cat), "still busy");
        assert!((cat.level(Need::Affection) - affection).abs() < f32::EPSILON);
    }

    #[test]
    fn a_player_who_keeps_up_has_a_cat_that_keeps_using_the_box() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();

        let (mut visits, mut was_in_box) = (0, false);
        for _ in 0..(500.0 / DT) as usize {
            // The ideal player: fills the bowls and cleans the box at once.
            supplies.food.fill(Food::Kibble);
            supplies.water.fill();
            supplies.litter.clean();
            cat.update(DT, &mut supplies);

            let in_box = using_litter(&cat);
            visits += i32::from(in_box && !was_in_box);
            was_in_box = in_box;
        }

        assert!(visits >= 4, "only {visits} visits in 500 s");
    }

    // --- The cat's home -------------------------------------------------

    #[test]
    fn the_cat_climbs_the_ladder_to_nap_upstairs() {
        let ground = Place::ground(world::CAT_START, world::CAT_START);
        let mut cat = sleepy(cat_going(ground, home::UPPER_BED, Some(Need::Energy)));
        let mut supplies = Supplies::default();

        let (mut climbed, mut slid) = (false, false);
        for _ in 0..(12.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            climbed |= is_climbing(&cat);
            slid |= is_sliding(&cat);
        }

        assert!(climbed, "should use the ladder");
        assert!(!slid, "but not the slide");
        assert_eq!(cat.floor, Level::Upper);
        assert_eq!(cat.pos, home::UPPER_BED.spot);
        assert!(is_sleeping(&cat), "napping upstairs");
    }

    #[test]
    fn the_climb_takes_a_couple_of_seconds_and_goes_up() {
        let ground = Place::ground(home::LADDER_BASE, home::LADDER_BASE);
        let mut cat = cat_going(ground, home::UPPER_BED, None);
        let mut supplies = Supplies::default();

        let mut frames_climbing = 0;
        let mut last_y = f32::MAX;
        for _ in 0..(10.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            if is_climbing(&cat) {
                frames_climbing += 1;
                assert!(cat.pos.y <= last_y + 1e-3, "moving down the ladder");
                last_y = cat.pos.y;
            }
        }
        let seconds = frames_climbing as f32 * DT;
        assert!(
            (seconds - CLIMB_SECONDS).abs() < 0.2,
            "climbed for {seconds} s"
        );
    }

    #[test]
    fn the_cat_slides_down_to_leave_the_upper_floor() {
        let bowl = Need::Hunger.places()[0];
        let mut cat = starving(cat_going(home::UPPER_BED, bowl, Some(Need::Hunger)));
        let mut supplies = Supplies::default();

        let (mut climbed, mut slid) = (false, false);
        for _ in 0..(20.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            climbed |= is_climbing(&cat);
            slid |= is_sliding(&cat);
        }

        assert!(slid, "should use the slide");
        assert!(!climbed, "but not the ladder");
        assert_eq!(cat.floor, Level::Ground);
        assert_eq!(cat.pos, bowl.spot);
        assert_eq!(cat.waiting_for(), Some(Need::Hunger), "at the empty bowl");
    }

    #[test]
    fn the_slide_takes_about_a_second_and_lands_at_the_bottom() {
        let mut cat = cat_going(
            home::UPPER_BED,
            Place::ground(home::SLIDE_END, home::SLIDE_END),
            None,
        );
        let mut supplies = Supplies::default();

        let mut frames_sliding = 0;
        let mut last_y = f32::MIN;
        let mut landed = None;
        for _ in 0..(10.0 / DT) as usize {
            let was_sliding = is_sliding(&cat);
            cat.update(DT, &mut supplies);
            if is_sliding(&cat) {
                frames_sliding += 1;
                assert!(cat.pos.y >= last_y - 1e-3, "moving back up the slide");
                last_y = cat.pos.y;
            } else if was_sliding {
                landed = Some((cat.pos, cat.floor));
                break;
            }
        }
        let seconds = frames_sliding as f32 * DT;
        assert!(
            (seconds - SLIDE_SECONDS).abs() < 0.2,
            "slid for {seconds} s"
        );
        assert_eq!(landed, Some((home::SLIDE_END, Level::Ground)));
    }

    #[test]
    fn a_cat_walks_between_the_two_floors_by_both_the_front_door_and_the_ladder() {
        let mut cat = sleepy(cat_going(
            home::GROUND_BED,
            home::UPPER_BED,
            Some(Need::Energy),
        ));
        let mut supplies = Supplies::default();

        for _ in 0..(10.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            if cat.floor == Level::Ground && !is_climbing(&cat) {
                assert!(
                    !home::BLOCK.contains(cat.pos) || in_the_home_properly(cat.pos),
                    "through a wall at {}",
                    cat.pos
                );
            }
        }
        assert_eq!(cat.floor, Level::Upper);
        assert!(is_sleeping(&cat));
    }

    #[test]
    fn a_cat_crossing_the_room_goes_round_the_home_not_through_it() {
        // A nap on the other side of the home.
        let start = Place::ground(vec2(600.0, 480.0), vec2(600.0, 480.0));
        let across = Place::ground(vec2(1180.0, 480.0), vec2(1180.0, 480.0));
        let mut cat = sleepy(cat_going(start, across, Some(Need::Energy)));
        let mut supplies = Supplies::default();

        for _ in 0..(12.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            assert!(
                !home::BLOCK.contains(cat.pos),
                "walked through at {}",
                cat.pos
            );
        }
        assert_eq!(cat.pos, across.spot, "got to the other side");
        assert!(is_sleeping(&cat));
    }

    #[test]
    fn a_cat_given_time_naps_in_each_of_its_four_beds() {
        let mut beds = std::collections::HashSet::new();
        for _ in 0..80 {
            let mut cat = cat_with(Need::Energy, 0.05);
            let mut supplies = Supplies::default();
            for _ in 0..(30.0 / DT) as usize {
                cat.update(DT, &mut supplies);
                if is_sleeping(&cat) {
                    beds.insert((cat.pos.x as i32, cat.pos.y as i32));
                    break;
                }
            }
        }
        assert_eq!(beds.len(), 4, "napped in {beds:?}");
    }

    // --- The sofa -----------------------------------------------------------

    fn is_jumping(cat: &Cat) -> bool {
        matches!(cat.activity, Activity::Jumping { .. })
    }

    #[test]
    fn the_cat_jumps_up_on_the_sofa_to_nap_on_top_of_it() {
        let on_the_floor = Place::ground(world::PLEA_SPOT, world::PLEA_SPOT);
        let mut cat = sleepy(cat_going(on_the_floor, sofa::LEFT_NAP, Some(Need::Energy)));
        let mut supplies = Supplies::default();

        let mut jumped = false;
        for _ in 0..(20.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            jumped |= is_jumping(&cat);
            if is_sleeping(&cat) {
                break;
            }
        }

        assert!(jumped, "never jumped");
        assert!(is_sleeping(&cat), "napping on the sofa");
        assert_eq!(cat.floor, Level::Sofa);
        assert_eq!(cat.pos, sofa::LEFT_NAP.spot);
    }

    #[test]
    fn the_jump_takes_off_in_front_of_the_sofa_and_goes_straight_up() {
        let on_the_floor = Place::ground(world::PLEA_SPOT, world::PLEA_SPOT);
        let mut cat = cat_going(on_the_floor, sofa::RIGHT_NAP, None);
        let mut supplies = Supplies::default();

        let mut last = None;
        for _ in 0..(20.0 / DT) as usize {
            let before = cat.pos;
            cat.update(DT, &mut supplies);
            if is_jumping(&cat) {
                assert!(
                    (cat.pos.x - sofa::RIGHT_NAP.spot.x).abs() < 0.01,
                    "took off from {before}"
                );
                last = Some(cat.pos);
            }
        }
        let last = last.expect("never jumped");
        assert!(last.y < sofa::TAKE_OFF_Y, "left the floor");
    }

    #[test]
    fn the_cat_jumps_down_before_walking_off_the_sofa() {
        let bowl = Place::ground(world::BOWL_SPOT, world::BOWL);
        let mut cat = starving(cat_going(sofa::LEFT_NAP, bowl, Some(Need::Hunger)));
        let mut supplies = Supplies::default();
        supplies.food.fill(Food::Kibble);

        let mut jumped = false;
        for _ in 0..(20.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            if cat.floor == Level::Sofa && !is_jumping(&cat) {
                // Still up there: it must not be walking across the room.
                assert_eq!(cat.pos, sofa::LEFT_NAP.spot);
            }
            jumped |= is_jumping(&cat);
            if jumped && cat.floor == Level::Ground {
                break;
            }
        }

        assert!(jumped, "never jumped down");
        assert_eq!(cat.floor, Level::Ground);
        assert!(cat.pos.y >= sofa::TAKE_OFF_Y - 1.0, "back on the floor");
    }

    #[test]
    fn a_cat_upstairs_slides_down_before_heading_for_the_sofa() {
        let mut cat = cat_going(home::UPPER_BED, sofa::LEFT_NAP, Some(Need::Energy));
        let mut supplies = Supplies::default();

        let mut reached_the_sofa = false;
        for _ in 0..(30.0 / DT) as usize {
            cat.update(DT, &mut supplies);
            if cat.floor == Level::Upper && !is_climbing(&cat) && !is_sliding(&cat) {
                assert!(
                    (cat.pos.y - home::UPPER_FLOOR).abs() < 0.01,
                    "left the upper floor at {}",
                    cat.pos
                );
            }
            reached_the_sofa |= cat.floor == Level::Sofa && !is_jumping(&cat);
        }
        assert!(reached_the_sofa, "never got to the sofa");
    }

    #[test]
    fn a_cat_cannot_be_petted_in_mid_air() {
        let mut cat = cat_going(sofa::LEFT_NAP, sofa::LEFT_NAP, None);

        cat.activity = Activity::Jumping {
            progress: 0.5,
            up: false,
            goal: Goal::to_place(Place::ground(world::PLEA_SPOT, world::PLEA_SPOT), None),
        };
        assert!(!cat.pet_if_hit(cat.pos + BODY_CENTRE));
    }

    #[test]
    fn the_cats_share_the_sofa_and_stay_inside_the_room_in_everyday_life() {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();

        let mut on_the_sofa = false;
        for _ in 0..(1800.0 / DT) as usize {
            look_after(&mut supplies);
            cat.update(DT, &mut supplies);
            if cat.floor == Level::Sofa && !is_jumping(&cat) {
                on_the_sofa = true;
                assert!(
                    (cat.pos.y - sofa::LEFT_NAP.spot.y).abs() < 0.01,
                    "{}",
                    cat.pos
                );
            }
        }
        assert!(on_the_sofa, "never got up on the sofa");
    }

    #[test]
    fn the_cat_uses_both_floors_ladder_and_slide_in_everyday_life_without_ever_walking_through_the_home()
     {
        let mut cat = Cat::new("Test");
        let mut supplies = Supplies::default();

        let (mut upstairs, mut downstairs_inside) = (false, false);
        let (mut climbs, mut slides) = (0, 0);
        let (mut was_climbing, mut was_sliding) = (false, false);
        for _ in 0..(900.0 / DT) as usize {
            look_after(&mut supplies);
            cat.update(DT, &mut supplies);

            let (climbing, sliding) = (is_climbing(&cat), is_sliding(&cat));
            climbs += i32::from(climbing && !was_climbing);
            slides += i32::from(sliding && !was_sliding);
            (was_climbing, was_sliding) = (climbing, sliding);

            if climbing || sliding {
                continue;
            }
            match cat.floor {
                Level::Upper => {
                    upstairs = true;
                    assert!((cat.pos.y - home::UPPER_FLOOR).abs() < 0.01, "{}", cat.pos);
                    assert!((home::LADDER_TOP.x..=home::SLIDE_TOP.x).contains(&cat.pos.x));
                }
                Level::Ground if home::BLOCK.contains(cat.pos) => {
                    downstairs_inside = true;
                    assert!(
                        in_the_home_properly(cat.pos),
                        "through a wall at {}",
                        cat.pos
                    );
                }
                Level::Sofa | Level::Ground => {}
            }
        }

        assert!(upstairs, "never went upstairs");
        assert!(downstairs_inside, "never went in downstairs");
        assert!(climbs >= 2, "only climbed {climbs} times");
        assert!(
            (climbs - slides).abs() <= 1,
            "climbed {climbs} times but slid {slides} times"
        );
    }

    #[test]
    fn nobody_can_pet_the_cat_on_the_ladder_or_the_slide() {
        for activity in [
            Activity::Climbing {
                progress: 0.5,
                goal: Goal::to_place(home::UPPER_BED, None),
            },
            Activity::Sliding {
                progress: 0.5,
                goal: Goal::to_place(home::GROUND_BED, None),
            },
        ] {
            let mut cat = Cat::new("Test");
            cat.activity = activity;
            assert!(!cat.pet_if_hit(cat.pos + BODY_CENTRE), "{activity:?}");
        }
    }

    #[test]
    fn no_walk_goes_on_for_ever() {
        // Several cats, each living ten minutes with a perfect player. The
        // longest real walk (right across the room and round the home) takes
        // about ten seconds; one that doesn't end means the cat is lost.
        for life in 0..6 {
            let mut cat = Cat::new("Test");
            let mut supplies = Supplies::default();

            let (mut frames, mut longest) = (0, 0);
            for _ in 0..(600.0 / DT) as usize {
                look_after(&mut supplies);
                cat.update(DT, &mut supplies);

                if matches!(cat.activity, Activity::Walking { .. }) {
                    frames += 1;
                    longest = longest.max(frames);
                } else {
                    frames = 0;
                }
            }

            let seconds = longest as f32 * DT;
            assert!(seconds < 20.0, "life {life}: one walk lasted {seconds} s");
        }
    }
}
