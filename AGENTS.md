# Repository Guidelines

## Project Structure & Module Organization

This repository builds `launch`, a Rust 2021 CLI for app icons, device mockups, screenshots, captures, and collages. The binary entry point is `src/main.rs`, with feature modules split by responsibility: `mockup.rs`, `screenshot.rs`, `collage.rs`, `watch.rs`, `device.rs`, `config.rs`, `clipboard.rs`, `resize.rs`, `zip.rs`, and `contents_json.rs`. Embedded runtime assets live in `resources/`: device templates under `resources/templates/`, masks under `resources/masks/`, plus bundled fonts and backgrounds. Build output goes to `target/` and should not be edited.

## Build, Test, and Development Commands

- `cargo build` builds the debug binary.
- `cargo build --release` builds the optimized release binary at `target/release/launch`.
- `cargo install --path .` installs the local CLI.
- `cargo fmt` formats Rust code with rustfmt.
- `cargo clippy` runs lint checks.
- `cargo test` runs tests when present.

Useful manual smoke checks:

```bash
cargo run -- mockup screenshot.png
cargo run -- icon icon.png -o icons.zip
cargo run -- mockup --list-devices
```

## Coding Style & Naming Conventions

Use Rust 2021 idioms and keep code formatted with `cargo fmt`. Prefer small modules with clear ownership over large cross-cutting helpers. Use `snake_case` for functions, variables, modules, and files; `PascalCase` for types and enum variants; and `SCREAMING_SNAKE_CASE` for constants. Preserve existing clap derive patterns for CLI arguments and use `anyhow::Result` for command-level error propagation.

## Testing Guidelines

There is currently no established test suite, so verify changes with focused CLI runs and add tests when behavior is easy to isolate. Prefer unit tests next to pure logic such as sizing, layout, marker detection, or config generation. Use `tests/` integration tests for command-level behavior if adding broader coverage. Name tests by behavior, for example `skips_already_processed_mockup`.

## Commit & Pull Request Guidelines

Recent history uses short, imperative commit subjects such as `Add awatch subcommand for Android device screen monitoring` and `Improve screenshot styling`. Keep commits focused and explain user-visible behavior in the subject.

Pull requests should include a concise summary, affected commands, manual verification steps, and before/after images when changing generated visuals. Note required platform tools for capture flows, such as `xcrun simctl`, `adb`, or `screencapture`.

## Security & Configuration Tips

Do not add generated images, archives, or `target/` artifacts unless they are intentional fixtures. Keep embedded asset filenames stable because Rust modules may reference them with `include_bytes!`.
