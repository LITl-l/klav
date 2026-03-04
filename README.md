# Klav

> ラテン語 *clavis*（鍵）に由来。思考の速度で多言語入力を実現する、軽量・高速なオープンソース ステノタイプエンジン。

Klav is a lightweight, high-performance stenotype engine written in Rust. It captures keyboard input at the kernel level (evdev), detects chords (simultaneous key presses), translates them through a pluggable theory engine, and outputs text via a virtual keyboard (uinput).

## Features

- **Low latency** — Direct evdev/uinput integration, no application-layer hooks
- **Multilingual** — Japanese and English steno with instant stroke-based switching
- **Pluggable theories** — Steno theories are defined in TOML + JSON, not hardcoded
- **Single binary** — No runtime dependencies, no Python, no interpreter
- **Configurable** — TOML-based keymap and theory configuration

## Architecture

```
evdev → Stroke Detector → Theory Engine → Output (uinput)
```

The theory engine has a layered design:

1. **Layer 1**: Rule-based syllable conversion (consonant + vowel → kana)
2. **Layer 2**: Word dictionary (single stroke → word/phrase)
3. **Layer 3**: Direct kanji input (future)

## Building

```sh
cargo build --release
```

## Usage

```sh
sudo ./target/release/klav-daemon --config klav.toml
```

Root (or input group membership) is required for evdev access.

## Project Structure

| Crate | Description |
|-------|-------------|
| `klav-core` | Platform-independent core: stroke detection, theory engine, dictionary |
| `klav-daemon` | System daemon: evdev input, uinput output, main loop |
| `klav-gui` | Settings GUI (egui) |
| `tools/dict-gen` | Dictionary generation tool |

## License

MIT OR Apache-2.0
