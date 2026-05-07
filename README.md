# applogo

Generate app icons and device mockups from a single image.

A fast CLI tool with two features:
- **Icon generation**: takes a 1024×1024 source image and produces correctly sized icons for iPhone, iPad, watchOS, macOS, and Android
- **Device mockup**: wraps a screenshot in a realistic iPhone device frame (iPhone 14/15 series)

Inspired by [appicon.co](https://www.appicon.co/) and [mockuphone.com](https://mockuphone.com/).

## Install

### Pre-built binaries

Download from [GitHub Releases](https://github.com/xiaoxiunique/applogo/releases/latest):

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `applogo-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `applogo-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `applogo-x86_64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `applogo-x86_64-pc-windows-msvc.zip` |

```bash
# Example: macOS Apple Silicon
curl -L https://github.com/xiaoxiunique/applogo/releases/latest/download/applogo-aarch64-apple-darwin.tar.gz | tar xz
sudo mv applogo /usr/local/bin/
```

### Build from source

```bash
cargo install --path .
```

Or:

```bash
git clone https://github.com/xiaoxiunique/applogo.git
cd applogo
cargo build --release
# binary at target/release/applogo
```

## Usage

### Icon Generation

```bash
# Generate icons for all platforms
applogo icon.png

# Specify output path
applogo icon.png -o MyIcons.zip

# Only generate for specific platforms
applogo icon.png -p iphone,android

# Custom Android icon filename
applogo icon.png --android-filename app_icon.png

# Skip App Store / Play Store icons
applogo icon.png --no-stores
```

### Device Mockup

```bash
# Wrap screenshot in iPhone 15 Pro frame (default)
applogo mockup screenshot.png

# Choose a different device
applogo mockup screenshot.png -d apple-iphone-15-black

# Landscape orientation
applogo mockup screenshot.png --orientation landscape

# Custom output path
applogo mockup screenshot.png -o my-mockup.png

# List available devices
applogo mockup --list-devices
```

Available devices: iPhone 15, iPhone 15 Pro, iPhone 15 Pro Max, iPhone 14 Pro, iPhone 14. Device frame templates are auto-downloaded and cached at `~/.applogo/devices/` on first use.

## Output

```
AppIcons.zip
├── Assets.xcassets/AppIcon.appiconset/
│   ├── Contents.json
│   └── *.png (all Apple platform sizes)
├── android/
│   ├── mipmap-mdpi/ic_launcher.png      (48×48)
│   ├── mipmap-hdpi/ic_launcher.png      (72×72)
│   ├── mipmap-xhdpi/ic_launcher.png     (96×96)
│   ├── mipmap-xxhdpi/ic_launcher.png    (144×144)
│   └── mipmap-xxxhdpi/ic_launcher.png   (192×192)
├── appstore.png                          (1024×1024)
└── playstore.png                         (512×512)
```

### Icon counts per platform

| Platform | Sizes |
|----------|-------|
| iPhone   | 12 (20px – 1024px) |
| iPad     | 13 (20px – 167px) |
| watchOS  | 17 (48px – 1024px) |
| macOS    | 10 (16px – 1024px) |
| Android  | 5 (mdpi – xxxhdpi) |
| Stores   | 2 (App Store + Play Store) |

## How it works

- Loads source image with the [image](https://crates.io/crates/image) crate
- Resizes using **Lanczos3** filter for high-quality downsampling
- Apple platform icons get a white background fill (matching App Store requirements)
- Android and Play Store icons preserve transparency
- Deduplicates shared sizes across platforms (resize once, reference multiple times)
- Packages everything into a ZIP with [zip](https://crates.io/crates/zip) crate

## License

MIT
