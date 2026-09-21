# tui-piano

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE.txt)
![Built with Ratatui](https://img.shields.io/badge/Built_with-Ratatui-orange?logo=rust)

A virtual piano for the terminal, written in Rust. The keyboard layout follows VMPK, so muscle memory from a desktop piano app carries over.

## Features

- Two-octave range on a QWERTY keyboard, lower row `Z`–`M` and upper row `Q`–`I`.
- Hold a key to sustain the note on terminals that support the kitty keyboard protocol (press/release/repeat events). Releasing the key fades the note out.
- On other terminals the app falls back to fixed-length notes, and pressed keys flash for about 150 ms.
- Shift the octave between 1 and 6 with the arrow keys.
- Press `Tab` to toggle between the default C-anchored layout and an E-anchored layout where both ends of each row are E (Mi). The transposition is a uniform +4 semitones, so the white keys form the E major scale and the fingering is identical.
- The name of the last played note is shown in the UI.
- If no audio device is available, the app starts in silent mode instead of failing, so the keyboard display still works.

## Requirements

- A Rust toolchain (edition 2021).
- An audio output device, optional.
- For hold-to-sustain, a terminal that supports the kitty keyboard protocol, such as ghostty. Everything else works in any terminal.

## Usage

```sh
cargo run --release
```

## Key bindings

| Action | Keys |
| --- | --- |
| Lower octave, white keys | `Z X C V B N M ,` |
| Lower octave, black keys | `S D G H J L` |
| Upper octave, white keys | `Q W E R T Y U I` |
| Upper octave, black keys | `2 3 5 6 7 9` |
| Octave down / up | `Left` / `Right` |
| Toggle C ⇔ E layout | `Tab` |
| Quit | `Esc` or `Ctrl+C` |

Letter keys are case-insensitive.

## License

GPL-3.0. See [LICENSE.txt](LICENSE.txt).
