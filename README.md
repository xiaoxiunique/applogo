# Launch

App launch toolkit — icons, mockups, and more.

A CLI tool for the app development lifecycle:
- **`launch icon`** — Generate app icons for all platforms (iPhone, iPad, watchOS, macOS, Android)
- **`launch generate logo`** — Generate a starter logo and icon ZIP
- **`launch mockup`** — Wrap screenshots in realistic device frames (iPhone 14–17 Pro, Samsung Galaxy S21, MacBook Air)
- **`launch capture`** — Capture from iOS Simulator and apply mockup in one step
- **`launch acapture`** — Capture from Android device via ADB and apply mockup
- **`launch wcapture`** — Capture a macOS app window and apply mockup
- **`launch screenshot`** — Generate App Store screenshots with title and device mockup
- **`launch collage`** — Combine multiple screenshots into a single grid image
- **`launch watch`** — Monitor iOS Simulator and auto-capture on screen changes
- **`launch awatch`** — Monitor Android device and auto-capture on screen changes
- **`launch record`** — Record iOS Simulator video, add markers, and export clips
- **`launch frame-video`** — Apply a device frame to an existing video
- **`launch title`** — Add large cover-style title text to a video

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

### Logo Generation

```bash
# Generate logo.png and AppIcons.zip in the current directory
launch generate logo

# Short form
l g logo

# Add a short wordmark and make the result reproducible
launch generate logo -o ./assets --text BX --seed 42

# Only generate selected platform icons
launch generate logo -o ./assets -p iphone,android
```

`generate logo` creates a random 1024x1024 PNG locally, then runs the same icon pipeline as `launch icon`.

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

# Full App Store screenshot with title
launch capture --title "Your App Title"
```

### Android Capture

```bash
# Capture from connected Android device + apply mockup
launch acapture

# Specify device serial (when multiple connected)
launch acapture -s SERIAL

# Full Play Store screenshot with title
launch acapture --title "Your App Title"
```

### Window Capture (macOS)

```bash
# Capture a macOS app window
launch wcapture Safari

# Capture Simulator window (auto-uses simctl for clean capture)
launch wcapture Simulator

# List matching windows
launch wcapture Safari --list
```

### App Store Screenshot

```bash
# Generate screenshot with title + device mockup + gradient background
launch screenshot screenshot.png --title "Amazing Feature"

# Batch process a directory
launch screenshot ./screenshots/ --title "Amazing Feature"

# Custom font
launch screenshot screenshot.png --title "功能亮点" --font custom.ttf
```

### Collage

```bash
# Combine all screenshots in current directory into a grid
launch collage

# Specify a directory
launch collage ./screenshots/

# Specify output path
launch collage ./screenshots/ -o preview.png

# Skip device frame (use raw screenshots)
launch collage --no-frame

# Custom padding between images
launch collage --padding 80
```

### Watch (iOS Simulator)

```bash
# Auto-capture on screen changes, Ctrl+C to stop
launch watch

# Custom output directory
launch watch -o ./my-shots

# Faster polling (0.5s)
launch watch --interval 0.5

# Skip collage generation on exit
launch watch --no-collage
```

Output structure:
```
watch-screenshots/
├── raw/           # Original screenshots
├── mockups/       # With device frames
└── collage.png    # Combined grid
```

### Awatch (Android)

```bash
# Auto-capture from Android device, Ctrl+C to stop
launch awatch

# Specify device serial
launch awatch -s SERIAL

# Custom output and interval
launch awatch -o ./android-shots --interval 0.5
```

### Record (iOS Simulator)

```bash
# Record the booted iOS Simulator. Press m to mark a new clip, q to stop.
launch record

# Save under recordings/demo-flow/
launch record --name demo-flow

# Apply the default iPhone 16 Pro frame to the full video or generated clips
launch record --frame

# Increase or reduce the outer breathing room
launch record --frame --frame-padding 140

# Choose a different frame
launch record --frame -d apple-iphone-15-pro-black-titanium

# Detect long still sections and remove repeated frames from each clip
launch record --auto-trim

# Preview the cut plan without exporting trimmed clips
launch record --auto-trim --dry-run

# Tune still-frame detection and keep short context around removals
launch record --auto-trim --freeze-min 1.5 --keep-still 0.5
```

Output structure:
```
recordings/recording-<timestamp>/
├── raw.mp4          # Full unmodified recording
├── framed.mp4       # Full framed video when --frame is used without clips
├── markers.json    # Manual marker timestamps
├── cuts.json       # Auto-trim plan, when --auto-trim is used
└── clips/          # Exported clip-001.mp4 and clip-001-framed.mp4, ...
```

`record` requires a booted iOS Simulator. Clip export and `--auto-trim` require FFmpeg.

### Frame Video

```bash
# Apply the default iPhone 16 Pro frame to an existing video
launch frame-video raw.mp4 -o framed.mp4

# Increase or reduce the outer breathing room
launch frame-video raw.mp4 -o framed.mp4 --padding 140

# Choose a different device frame
launch frame-video raw.mp4 -o framed.mp4 -d apple-iphone-15-pro-black-titanium
```

`frame-video` uses the same device templates and light gray background as `record --frame`.

### Video Title

```bash
# Add large cover text to the first second
launch title framed.mp4 --text "如何下载\\n搬小书" -o titled.mp4

# Long text wraps automatically; spaces are preferred break points
launch title framed.mp4 --text "如何下载 搬小书 以及更多功能" -o titled.mp4

# Keep the title visible longer and move it down
launch title framed.mp4 --text "如何下载\\n搬小书" -o titled.mp4 --duration 5 --y-ratio 0.46

# Use a narrower title block
launch title framed.mp4 --text "如何下载 搬小书 以及更多功能" -o titled.mp4 --wrap-width 0.65

# Override auto font sizing
launch title framed.mp4 --text "如何下载\\n搬小书" -o titled.mp4 --font-size 220
```

`title` uses the bundled Chinese title font to create yellow text with bold outlines for short-video cover frames. It preserves manual `\n` breaks, wraps spaced text at word boundaries, and wraps Chinese-style unspaced text by character width.

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
| Samsung Galaxy S21 Ultra | `samsung-galaxys21ultra-black` |
| MacBook Air 13" | `apple-macbookair13` |

## Features

- Invisible PNG marker prevents reprocessing — batch mode skips already-processed images
- Device templates embedded in binary — fully offline, zero network requests
- High-quality Lanczos3 downsampling for icons
- Xcode Asset Catalog (`Contents.json`) included in icon ZIP output
- Clipboard support for quick mockups (`-c` flag)

## License

MIT
