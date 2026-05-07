# Launch

App launch toolkit — icons, mockups, and more.

A CLI tool for the app development lifecycle:
- **`launch icon`** — Generate app icons for all platforms (iPhone, iPad, watchOS, macOS, Android)
- **`launch mockup`** — Wrap screenshots in realistic device frames (iPhone 14–17 Pro)
- **`launch capture`** — Capture from iOS Simulator and apply mockup in one step

## Install

### Pre-built binaries

Download from [GitHub Releases](https://github.com/xiaoxiunique/launch/releases/latest):

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `launch-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `launch-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `launch-x86_64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `launch-x86_64-pc-windows-msvc.zip` |

```bash
# Example: macOS Apple Silicon
curl -L https://github.com/xiaoxiunique/launch/releases/latest/download/launch-aarch64-apple-darwin.tar.gz | tar xz
sudo mv launch /usr/local/bin/
```

### Build from source

```bash
cargo install --path .
```

Or:

```bash
git clone https://github.com/xiaoxiunique/launch.git
cd launch
cargo build --release
# binary at target/release/launch
```

## Usage

### Icon Generation

```bash
# Generate icons for all platforms (1024x1024 image recommended)
launch icon icon.png

# Also works without subcommand (backward compat)
launch icon.png

# Specify output path
launch icon icon.png -o MyIcons.zip

# Only generate for specific platforms
launch icon icon.png -p iphone,android

# Skip App Store / Play Store icons
launch icon icon.png --no-stores
```

### Device Mockup

```bash
# Wrap screenshot in iPhone 16 Pro frame (default)
launch mockup screenshot.png

# Read from clipboard
launch mockup -c

# Batch process a folder
launch mockup ./screenshots/

# Choose a different device
launch mockup screenshot.png -d apple-iphone-17-pro-deep-blue

# Landscape orientation
launch mockup screenshot.png --orientation landscape

# List available devices
launch mockup --list-devices
```

### Simulator Capture

```bash
# Capture from running iOS Simulator + apply mockup
launch capture

# Also save the raw screenshot
launch capture --raw

# Choose device frame
launch capture -d apple-iphone-15-pro-black-titanium
```

## Available Devices

| Device | ID |
|--------|----|
| iPhone 17 Pro | `apple-iphone-17-pro-deep-blue` |
| iPhone 16 Pro | `apple-iphone-16-pro-black-titanium` (default) |
| iPhone 15 | `apple-iphone-15-black` |
| iPhone 15 Pro | `apple-iphone-15-pro-black-titanium` |
| iPhone 15 Pro Max | `apple-iphone-15-pro-max-black-titanium` |
| iPhone 14 Pro | `apple-iphone14pro-spaceblack` |
| iPhone 14 | `apple-iphone14-midnight` |

## Features

- Invisible PNG marker prevents reprocessing — batch mode skips already-processed images
- Device templates embedded in binary — fully offline, zero network requests
- High-quality Lanczos3 downsampling for icons
- Xcode Asset Catalog (`Contents.json`) included in icon ZIP output
- Clipboard support for quick mockups (`-c` flag)

## License

MIT
