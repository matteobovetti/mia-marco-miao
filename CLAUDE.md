# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

"Cat Life": a 2D desktop game (Rust, edition 2024, MSRV 1.85) built on [Macroquad](https://macroquad.rs). The player looks after two cats (feeding, water, litter, petting, brushing, toys). The README is empty; everything below comes from the code.

## Commands

```sh
cargo run                      # play (add `-- --fullscreen` to start full screen)
cargo build --release
cargo test                     # unit tests live in `#[cfg(test)] mod tests` inside each module
cargo test <name>              # single test, e.g. `cargo test synth::tests::`
cargo clippy --all-targets     # lints are strict: clippy::pedantic is on (see Cargo.toml)
```

`Cargo.toml` sets `clippy::all` + `clippy::pedantic` to warn and `unsafe_code` to warn; only the `cast_*` lints are allowed (intentional int/f32 mixing). Keep the build warning-free. `hound` is a dev-dependency used only to validate generated WAV bytes in `synth` tests.

## Architecture

There are no assets: all graphics are drawn with Macroquad primitives and all sounds are synthesised in code. No `tests/` directory; tests are pure-logic unit tests that don't open a window.

**Frame loop (`main.rs`)** — a `Screen` enum state machine: `Naming` (name-your-cat screen) → `Playing(Game)`. Each frame: cap `dt` at 0.1s, call `view::begin()`, `update(dt)` then `draw()` the current screen, `view::end()`. `Audio::load()` runs once on the transition to `Playing`; if it fails, `Game.audio` is `None` and the game is silent.

**Canvas coordinates (`view.rs`, `world.rs`)** — all logic and drawing use a fixed 1280×720 canvas (`world::WIDTH/HEIGHT`). `view::begin()` points a camera at the largest centred fit in the window. Therefore **use `view::mouse()`, `view::draw_text` and `view::text_width` instead of the Macroquad equivalents**, or pointer position / text sharpness will be wrong when the window is resized or full screen.

**Game state (`game.rs`)** — `Game` owns `[Cat; CAT_COUNT]` (currently 2, defined in `cat.rs`), `Supplies`, `Dispenser`, `Window`, an optional held `Brush`/`Toy` (only one thing in hand), and optional `Audio`. Left-click dispatch order is significant: shelf (`ui::item_at`) → wall-panel brush/toy → dispenser button → cat → window. Cats and draw order are sorted by `Cat::depth()` (lower on screen = in front).

**Cats (`cat.rs`, the largest file)** — a `Cat` has `needs: [f32; Need::COUNT]` and an `Activity` enum state machine (Idle, Walking, Climbing, Sliding, Jumping, Doing, UsingLitter, Purring, Brushing, Playing). Multi-level movement is modelled with `world::Level` (Ground / Upper / Sofa) and `world::Place` (spot + level + object to face); a `Goal` carries the cat through intermediate Climbing/Sliding/Jumping states. Cats mutate shared `Supplies` directly in `update`. Interaction hooks are on the cat (`pet_if_hit`, `brush_if_hit`, `play_with`) — the panels (`brushes.rs`, `toys.rs`) only know their items and how to draw them.

**Scenery / layout modules** — `world.rs` (room constants, spot coordinates, static drawing), `home.rs` (two-storey cat house, ladder, slide, `ground_waypoint` path planning around the house), `sofa.rs`, `window.rs`, `dispenser.rs` (pour timers, bowls only fill when pouring finishes), `supplies.rs` (`Food`, `Bowl`, `WaterBowl`, `LitterBox`, `Supplies`), `ui.rs` (HUD and shelf). Layout is expressed as hard-coded canvas-coordinate constants shared across modules (e.g. `world::BOWL_SPOT`, `home::LADDER_BASE`); changing one usually means checking the others that reference it.

**Audio split (`audio.rs`, `synth.rs`)** — `synth` generates mono f32 samples at 44.1 kHz and wraps them in in-memory WAV bytes for Macroquad. `audio::Scheduler` decides *when* sounds start/stop from cat state and is deliberately free of any speaker/Macroquad playback so it can be unit-tested; `audio::Audio` owns loaded sounds and executes the scheduler's `Cue`s. Keep new timing logic in the scheduler.

**Randomness** — uses `macroquad::rand` (seeded in `main`), not the `rand` crate.

## Controls (from code)

Left-click: pick shelf food / clean litter, take or put back a brush or toy, press dispenser buttons, pet or brush a cat, toggle the window. Right-click: put held item back. Hold Space with a toy: cats chase it. `M`: mute. `F`: fullscreen toggle (disabled while naming, where it's just a letter).
