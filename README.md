# Cat Life (mia-marco-miao)

A cosy 2D desktop game about looking after two cats. Name them, keep their bowls full and their litter box clean, pet and brush them, and play with them. Written in Rust with [Macroquad](https://macroquad.rs).

There are no image or sound files: all the graphics are drawn with code and all the sounds (meows, purring, crunching, lapping) are synthesised at startup.

## Running

You need Rust 1.85 or newer (edition 2024).

```sh
cargo run --release
```

Start full screen with:

```sh
cargo run --release -- --fullscreen
```

## How to play

Each cat has six needs that slowly drain: **Food**, **Water**, **Potty**, **Rest**, **Fun** and **Love**. They show as bars at the top of the screen. The cats go about their lives on their own (eating, drinking, sleeping on the sofa, climbing to the upper floor of their house and sliding back down), but they rely on you for the rest.

| Need | What to do |
|------|------------|
| Food | Pick a food from the shelf (kibble, tuna, chicken or milk), then press the dispenser button to pour it into the bowl. Tastier foods also add some fun. |
| Water | Press the water dispenser's button to fill the water bowl. |
| Potty | Clean the litter box using the shelf. |
| Rest | The cats nap on their own. |
| Fun | Take a toy (laser pointer, red ball, ball of yarn or toy mouse) off the wall panel, move the mouse around and hold **Space**. |
| Love | Click a cat to pet it, or take a brush (soft, paddle or slicker) off the other wall panel and click a cat to brush it. |

### Controls

| Input | Action |
|-------|--------|
| Left click | Use a shelf item, take or put back a brush or toy, press a dispenser button, pet or brush a cat, open or close the window |
| Right click | Put the brush or toy back |
| Space (hold) | Play with the toy in hand |
| `M` | Mute or unmute |
| `F` / `F11` | Toggle full screen (`Esc` leaves it) |
| Typing, `Enter` | On the naming screen: type each cat's name (up to 16 characters) and press Enter to accept it. Enter on an empty box picks the default name (Mittens, Domino) |

## Development

```sh
cargo test                  # unit tests
cargo clippy --all-targets  # lints (clippy::pedantic is enabled)
```

See [CLAUDE.md](CLAUDE.md) for an overview of the code architecture.

## License

[MIT](LICENSE)
