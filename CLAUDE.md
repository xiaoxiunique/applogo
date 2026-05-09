# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**Launch** — a Rust CLI toolkit for generating app icons, device frame mockups, and App Store screenshots. Single binary with all device templates/fonts/backgrounds embedded.

## Build & Development

```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo install --path .         # Install locally
cargo clippy                   # Lint
cargo fmt                      # Format
```

No test suite exists — verify changes manually via CLI commands.

## Architecture

Rust (edition 2021), CLI built with **clap v4** derive macros. Entry point: `src/main.rs`.

### Subcommands

| Command | Module | Purpose |
|---------|--------|---------|
| `icon` | `config.rs` | Generate platform-specific app icons (iOS/iPad/watchOS/macOS/Android) as ZIP |
| `mockup` | `mockup.rs`, `device.rs` | Wrap screenshots in device frames (iPhone/Samsung/MacBook) |
| `capture` | `main.rs` | iOS Simulator screenshot → mockup pipeline (`xcrun simctl`) |
| `acapture` | `main.rs` | Android ADB screenshot → mockup pipeline |
| `wcapture` | `main.rs` | macOS window capture → mockup pipeline (`screencapture`/CoreGraphics) |
| `screenshot` | `screenshot.rs` | App Store screenshot with title + gradient background |
| `collage` | `collage.rs` | Combine multiple screenshots into auto-grid collage with adaptive padding |

Legacy: bare `launch icon.png` still works via backward-compat arg parsing in `main.rs`.

### Key Modules

- **`device.rs`** — Device template definitions: screen coordinates, template/mask PNG bytes (embedded via `include_bytes!`), orientation support
- **`config.rs`** — Icon platform/size tables, Xcode asset catalog JSON generation (`contents_json.rs`)
- **`mockup.rs`** — Image compositing: resize screenshot into device frame, apply mask, handle rotation. Uses invisible PNG marker to skip already-processed images
- **`screenshot.rs`** — Title text rendering (`ab_glyph` + `imageproc`), gradient backgrounds, font loading (embedded + custom TTF/OTF/TTC). Exports `create_gradient_canvas()` for reuse
- **`collage.rs`** — Auto-grid layout for multiple images, adaptive padding (scales with grid density), reuses mockup pipeline and gradient canvas
- **`clipboard.rs`** — Cross-platform clipboard read/write via `arboard`
- **`resize.rs`** — Lanczos3 downsampling with fill/fit modes
- **`zip.rs`** — ZIP packing with deflate compression, deduplication

### Design Patterns

- **Embedded assets**: All device templates, masks, fonts, and backgrounds are compiled into the binary (`resources/` dir, loaded via `include_bytes!`)
- **Batch processing**: Directory input auto-processes all images, skipping already-mockuped files (detected by invisible PNG marker pixel)
- **Platform tool integration**: Shells out to `xcrun simctl` (iOS), `adb` (Android), `screencapture`/`swift` (macOS) — auto-locates tools on PATH or known SDK locations
- **Smart defaults**: Auto-detects orientation from image dimensions, defaults to latest iPhone device

## Release

GitHub Actions (`.github/workflows/release.yml`) triggers on `v*` tags. Builds for macOS (x86_64 + aarch64), Linux (x86_64), Windows (x86_64-msvc). Outputs tarballs/zips to GitHub Releases.
