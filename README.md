# ICONBV — Windows Icon Studio
<img width="1639" height="972" alt="image" src="https://github.com/user-attachments/assets/30714a9c-5393-4e39-98e8-05a6d07b4e45" />

Desktop app that converts SVG/PNG artwork into
multi-resolution Windows `.ico` files, with per-size embedded-frame selectors
and live previews.

## Features

- **Sources:** SVG (rendered via resvg) and PNG (downscaled via image)
- **Export:** Windows ICO with selectable embedded sizes
- **Embedded sizes:** 16, 20, 24, 32, 40, 48, 64, 96, 128, 256 px
  - Frames `< 64 px` are stored as BMP; `>= 64 px` as PNG
  - All frames keep 32-bit straight alpha
- **Live previews** for every size, with a hero preview and checkerboard backdrop
- **Safe padding** slider to keep artwork clear of edge cropping
- **Export** defaults to `<stem>.ico` beside the source file (second click to overwrite)
- **Dark grayscale glass UI** with gradient background, glow, and glass cards
- **Borderless window** with custom titlebar, drag-to-move, minimize/close
- **Fixed-size, non-resizable** window that fits the monitor work area

## Build

Requires Rust 1.98+ and the Windows SDK (for `windows-sys`).

```sh
# Debug build (windowless by default)
cargo build

# Run with an SVG or PNG source
cargo run -- <your-file.svg>

# Release build
cargo build --release

# Build with a console window (useful for debugging panics/logs)
cargo build --features console
cargo run --features console -- <your-file.svg>
```

### Build artifacts

- `target/debug/iconbv.exe` — windowless debug build
- `target/release/iconbv.exe` — windowless release build
- Use `--features console` to attach `stdout`/`stderr` for debugging

The executable embeds `APP.ico` as its application icon (shown in File Explorer).

## Usage

1. Launch `iconbv.exe` (optionally drag an SVG/PNG onto it, or pass a path as an argument).
2. Click **Replace** in the SOURCE card to pick an SVG or PNG file.
3. Adjust **Safe padding** in the CANVAS card if needed.
4. Toggle the embedded sizes you want in the WINDOWS FRAMES card (use ALL / NONE).
5. Click **Export ICO** — the `.ico` is written next to your source file.

Keyboard shortcuts:

- `Ctrl+O` — browse for a source file
- `Ctrl+R` — replace source
- `Ctrl+X` — remove source
- `Ctrl+E` — export ICO

## Project structure

```
iconbv/
├── Cargo.toml        # Manifest, features, build-dependencies
├── build.rs          # Embeds APP.ico into the executable
├── app.rc            # Windows resource script (app icon)
├── README.md         # This file
├── LICENSE           # MIT License
├── src/
│   ├── main.rs       # Entry point, window sizing, app icon
│   ├── app.rs        # UI, state, file dialogs, drag/drop, previews
│   ├── theme.rs      # Grayscale palette, fonts, glass painting helpers
│   └── convert.rs    # SVG/PNG rendering, ICO encoding
```
## Attribution

**App icon:** `APP.ico` created by Ava, licensed under [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/).

**Rust libraries used:**

| Crate | Version | Purpose | License |
|-------|---------|---------|---------|
| [eframe](https://crates.io/crates/eframe) | 0.32.3 | Immediate-mode GUI framework (egui backend) | MIT / Apache-2.0 |
| [egui](https://crates.io/crates/egui) | 0.32.3 | Immediate-mode GUI library | MIT / Apache-2.0 |
| [ico](https://crates.io/crates/ico) | 0.6.0 | Windows ICO encoding/decoding | MIT |
| [image](https://crates.io/crates/image) | 0.25.10 | Image decoding and manipulation | MIT / Apache-2.0 |
| [resvg](https://crates.io/crates/resvg) | 0.49.0 | SVG rendering | MPL-2.0 / Apache-2.0 |
| [rfd](https://crates.io/crates/rfd) | 0.17.2 | Cross-platform file dialogs | Zlib |
| [windows-sys](https://crates.io/crates/windows-sys) | 0.61.2 | Raw Windows API bindings | MIT |
| [embed-resource](https://crates.io/crates/embed-resource) | 2.3 | Embed resources into Windows executables | MIT |
