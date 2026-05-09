mod clipboard;
mod collage;
mod config;
mod contents_json;
mod device;
mod mockup;
mod resize;
mod screenshot;
mod zip;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use config::Platform;
use zip::ZipEntry;

/// Check if a command exists in PATH.
fn which_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[derive(Parser)]
#[command(name = "launch", about = "App launch toolkit — icons, mockups, and more")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    // --- Backward-compat: allow flat args for icon generation ---
    /// Source image path (for icon generation without subcommand)
    #[arg(global = false)]
    input: Option<PathBuf>,

    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(short, long, value_delimiter = ',')]
    platforms: Option<Vec<Platform>>,

    #[arg(long)]
    android_filename: Option<String>,

    #[arg(long)]
    no_stores: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Generate app icons for all platforms
    Icon(IconArgs),
    /// Wrap screenshot in a device frame mockup
    Mockup(MockupArgs),
    /// Capture screenshot from iOS Simulator and apply mockup
    Capture(CaptureArgs),
    /// Capture screenshot from Android device via ADB
    Acapture(AcaptureArgs),
    /// Capture a macOS app window screenshot
    Wcapture(WcaptureArgs),
    /// Generate App Store screenshot with title and device mockup
    Screenshot(ScreenshotArgs),
    /// Combine multiple screenshots into a single collage image
    Collage(CollageArgs),
}

#[derive(Parser)]
struct IconArgs {
    /// Source image path (1024x1024 recommended)
    input: PathBuf,

    /// Output ZIP file path
    #[arg(short, long, default_value = "AppIcons.zip")]
    output: PathBuf,

    /// Platforms to generate icons for (comma-separated)
    #[arg(short, long, value_delimiter = ',', default_value = "all")]
    platforms: Vec<Platform>,

    /// Custom filename for Android icons
    #[arg(long, default_value = "ic_launcher.png")]
    android_filename: String,

    /// Skip App Store and Play Store icon generation
    #[arg(long)]
    no_stores: bool,
}

#[derive(Parser)]
struct MockupArgs {
    /// Screenshot image to wrap in device frame
    input: Option<PathBuf>,

    /// Output PNG file path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Device ID
    #[arg(short, long, default_value = device::DEFAULT_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Read screenshot from clipboard
    #[arg(short, long)]
    clipboard: bool,

    /// List available devices and exit
    #[arg(long)]
    list_devices: bool,
}

#[derive(Parser)]
struct CaptureArgs {
    /// Output filename
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Device frame ID
    #[arg(short, long, default_value = device::DEFAULT_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Save raw screenshot too (without mockup frame)
    #[arg(long)]
    raw: bool,

    /// Title text — when set, generates a full App Store screenshot
    #[arg(short, long)]
    title: Option<String>,

    /// Font size for title (used with --title)
    #[arg(long, default_value = "200")]
    font_size: f32,

    /// Custom font file for title (used with --title)
    #[arg(long)]
    font: Option<PathBuf>,
}

#[derive(Parser)]
struct AcaptureArgs {
    /// Output filename
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// ADB device serial (when multiple devices connected)
    #[arg(short, long)]
    serial: Option<String>,

    /// Device frame ID
    #[arg(short, long, default_value = device::DEFAULT_ANDROID_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Save raw screenshot too (without processing)
    #[arg(long)]
    raw: bool,

    /// Title text — when set, generates a full Play Store screenshot
    #[arg(short, long)]
    title: Option<String>,

    /// Font size for title (used with --title)
    #[arg(long, default_value = "200")]
    font_size: f32,

    /// Custom font file for title (used with --title)
    #[arg(long)]
    font: Option<PathBuf>,
}

#[derive(Parser)]
struct WcaptureArgs {
    /// App name to capture (e.g. "Simulator", "Safari", "微信")
    app: String,

    /// Output filename
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Save raw screenshot too (without processing)
    #[arg(long)]
    raw: bool,

    /// Device frame ID (auto: iPhone for Simulator, MacBook for other apps)
    #[arg(short, long)]
    device: Option<String>,

    /// Orientation
    #[arg(long)]
    orientation: Option<String>,

    /// Title text — when set, generates a full screenshot with title
    #[arg(short, long)]
    title: Option<String>,

    /// Font size for title (used with --title)
    #[arg(long, default_value = "200")]
    font_size: f32,

    /// Custom font file for title (used with --title)
    #[arg(long)]
    font: Option<PathBuf>,

    /// List windows for the app and exit
    #[arg(long)]
    list: bool,

    /// Skip device mockup frame (just raw window capture)
    #[arg(long)]
    no_mockup: bool,
}

#[derive(Parser)]
struct ScreenshotArgs {
    /// Screenshot image (or directory for batch processing)
    input: PathBuf,

    /// Title text to display above the device mockup
    #[arg(short, long)]
    title: String,

    /// Output file or directory path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Device frame ID
    #[arg(short, long, default_value = device::DEFAULT_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Custom font file (.ttf/.otf/.ttc)
    #[arg(long)]
    font: Option<PathBuf>,

    /// Font size in pixels
    #[arg(long, default_value = "200")]
    font_size: f32,
}

#[derive(Parser)]
struct CollageArgs {
    /// Directory containing screenshots (defaults to current directory)
    #[arg(default_value = ".")]
    input: PathBuf,

    /// Output file path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Device frame ID
    #[arg(short, long, default_value = device::DEFAULT_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Padding between images in pixels (auto-scaled if not set)
    #[arg(long)]
    padding: Option<u32>,

    /// Skip device frame mockup (use raw screenshots)
    #[arg(long)]
    no_frame: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Icon(args)) => run_icon(args),
        Some(Command::Mockup(args)) => run_mockup(args),
        Some(Command::Capture(args)) => run_capture(args),
        Some(Command::Acapture(args)) => run_acapture(args),
        Some(Command::Wcapture(args)) => run_wcapture(args),
        Some(Command::Screenshot(args)) => run_screenshot(args),
        Some(Command::Collage(args)) => run_collage(args),
        None => {
            // Backward compat: treat as icon generation if input is provided
            if let Some(input) = cli.input {
                let args = IconArgs {
                    input,
                    output: cli.output.unwrap_or_else(|| PathBuf::from("AppIcons.zip")),
                    platforms: cli.platforms.unwrap_or_else(|| vec![Platform::All]),
                    android_filename: cli
                        .android_filename
                        .unwrap_or_else(|| "ic_launcher.png".into()),
                    no_stores: cli.no_stores,
                };
                run_icon(args)
            } else {
                // No input, no subcommand — show help
                use clap::CommandFactory;
                Cli::command().print_help()?;
                Ok(())
            }
        }
    }
}

fn run_icon(args: IconArgs) -> Result<()> {
    let platforms = config::expand_platforms(&args.platforms);

    let img = resize::load_image(&args.input)?;

    // Collect unique (size, fill_white_bg) pairs
    let mut size_requests: Vec<(u32, bool)> = Vec::new();

    for &platform in &platforms {
        let fill_white = !config::preserves_alpha(platform);
        let entries = config::get_entries(platform, &args.android_filename);
        for entry in &entries {
            let key = (entry.expected_size, fill_white);
            if !size_requests.contains(&key) {
                size_requests.push(key);
            }
        }
    }

    if !args.no_stores {
        for entry in &config::store_entries() {
            let fill_white = !config::store_preserves_alpha(&entry.filename);
            let key = (entry.expected_size, fill_white);
            if !size_requests.contains(&key) {
                size_requests.push(key);
            }
        }
    }

    eprintln!(
        "Resizing {} unique icon sizes from {}...",
        size_requests.len(),
        args.input.display()
    );
    let resized = resize::resize_all(&img, &size_requests)?;

    // Build ZIP entries
    let mut zip_entries: Vec<ZipEntry> = Vec::new();
    let mut icon_count: usize = 0;

    for &platform in &platforms {
        let fill_white = !config::preserves_alpha(platform);
        let entries = config::get_entries(platform, &args.android_filename);
        for entry in &entries {
            let data = resized[&(entry.expected_size, fill_white)].clone();
            let path = format!("{}{}", entry.folder, entry.filename);
            zip_entries.push(ZipEntry { path, data });
            icon_count += 1;
        }
    }

    if !args.no_stores {
        for entry in &config::store_entries() {
            let fill_white = !config::store_preserves_alpha(&entry.filename);
            let data = resized[&(entry.expected_size, fill_white)].clone();
            let path = entry.filename.clone();
            zip_entries.push(ZipEntry { path, data });
            icon_count += 1;
        }
    }

    let has_apple = platforms.iter().any(|p| config::is_apple_platform(*p));
    if has_apple {
        let contents = contents_json::generate(&platforms, &args.android_filename);
        zip_entries.push(ZipEntry {
            path: "Assets.xcassets/AppIcon.appiconset/Contents.json".into(),
            data: contents.into_bytes(),
        });
    }

    zip::build_zip(&args.output, zip_entries)?;

    eprintln!(
        "Generated {} icons for {} platform(s) -> {}",
        icon_count,
        platforms.len(),
        args.output.display()
    );

    Ok(())
}

fn run_mockup(args: MockupArgs) -> Result<()> {
    if args.list_devices {
        mockup::list_devices();
        return Ok(());
    }

    let input = if args.clipboard {
        let path = clipboard::save_clipboard_image()?;
        eprintln!("Read image from clipboard");
        path
    } else {
        args.input.ok_or_else(|| {
            anyhow::anyhow!(
                "Provide a screenshot path, directory, or use -c to read from clipboard.\n\
                 Usage: launch mockup <screenshot.png>\n\
                        launch mockup <screenshots_dir/>\n\
                        launch mockup -c"
            )
        })?
    };

    // If input is a directory, batch process all images
    if input.is_dir() {
        let out_dir = args.output.unwrap_or_else(|| input.join("mockups"));
        std::fs::create_dir_all(&out_dir)?;

        let mut count = 0;
        for entry in std::fs::read_dir(&input)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                continue;
            }
            if mockup::is_already_processed(&path) {
                eprintln!("Skipping {} (already processed)", path.display());
                continue;
            }
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out_path = out_dir.join(format!("{}-mockup.png", stem));
            mockup::run(&path, &out_path, &args.device, &args.orientation)?;
            count += 1;
        }
        eprintln!("Processed {} images -> {}", count, out_dir.display());
        return Ok(());
    }

    if mockup::is_already_processed(&input) {
        anyhow::bail!("Image already has mockup frame: {}", input.display());
    }

    let output = args.output.unwrap_or_else(|| {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        PathBuf::from(format!("{}-mockup.png", stem))
    });

    mockup::run(&input, &output, &args.device, &args.orientation)
}

fn run_capture(args: CaptureArgs) -> Result<()> {
    use std::process::Command as Cmd;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Generate timestamp for filename
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let raw_path = std::env::temp_dir().join(format!("launch-capture-{}.png", ts));

    // Capture from booted simulator
    eprintln!("Capturing screenshot from iOS Simulator...");
    let status = Cmd::new("xcrun")
        .args(["simctl", "io", "booted", "screenshot", "--type=png"])
        .arg(&raw_path)
        .status()
        .context("Failed to run xcrun simctl. Is Xcode installed?")?;

    if !status.success() {
        anyhow::bail!(
            "Simulator screenshot failed. Make sure a simulator is running.\n\
             Start one with: open -a Simulator"
        );
    }

    eprintln!("Captured simulator screenshot");

    // Determine output path
    let has_title = args.title.is_some();
    let default_suffix = if has_title { "screenshot" } else { "mockup" };
    let output = args.output.unwrap_or_else(|| {
        PathBuf::from(format!("screenshot-{}-{}.png", ts, default_suffix))
    });

    // Optionally keep raw screenshot
    if args.raw {
        let raw_out = output
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("screenshot-{}-raw.png", ts));
        std::fs::copy(&raw_path, &raw_out)?;
        eprintln!("Raw screenshot saved to {}", raw_out.display());
    }

    if let Some(title) = &args.title {
        // Full App Store screenshot: capture → mockup → title
        screenshot::run(
            &raw_path,
            &output,
            title,
            &args.device,
            &args.orientation,
            args.font.as_deref(),
            args.font_size,
        )?;
    } else {
        // Just mockup
        mockup::run(&raw_path, &output, &args.device, &args.orientation)?;
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&raw_path);

    Ok(())
}

fn run_acapture(args: AcaptureArgs) -> Result<()> {
    use std::process::Command as Cmd;
    use std::time::{SystemTime, UNIX_EPOCH};

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let raw_path = std::env::temp_dir().join(format!("launch-acapture-{}.png", ts));

    // Find adb binary — check PATH first, then common install locations
    let adb = {
        let common_paths = [
            std::env::var("HOME").ok().map(|h| PathBuf::from(h).join("Library/Android/sdk/platform-tools/adb")),
            Some(PathBuf::from("/opt/homebrew/bin/adb")),
            Some(PathBuf::from("/usr/local/bin/adb")),
        ];
        if which_exists("adb") {
            PathBuf::from("adb")
        } else if let Some(path) = common_paths.iter().flatten().find(|p| p.exists()) {
            eprintln!("Found adb at {}", path.display());
            path.clone()
        } else {
            anyhow::bail!(
                "adb not found. Install Android SDK or add platform-tools to PATH.\n\
                 Common location: ~/Library/Android/sdk/platform-tools/"
            );
        }
    };

    // Auto-detect ADB device if not specified
    let serial = if let Some(s) = args.serial {
        s
    } else {
        let output = Cmd::new(&adb)
            .args(["devices"])
            .output()
            .context("Failed to run adb. Is Android SDK installed?")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let devices: Vec<&str> = stdout
            .lines()
            .skip(1) // skip "List of devices attached"
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == "device" {
                    Some(parts[0])
                } else {
                    None
                }
            })
            .collect();

        match devices.len() {
            0 => anyhow::bail!(
                "No Android device found.\n\
                 Connect a device via USB or start an emulator, then check: adb devices"
            ),
            1 => {
                eprintln!("Detected device: {}", devices[0]);
                devices[0].to_string()
            }
            _ => {
                eprintln!("Multiple devices detected:");
                for d in &devices {
                    eprintln!("  {}", d);
                }
                eprintln!("Using first device: {}", devices[0]);
                eprintln!("Tip: use -s <serial> to specify a device");
                devices[0].to_string()
            }
        }
    };

    // Capture from Android device via ADB
    eprintln!("Capturing screenshot from {}...", serial);
    let output_data = Cmd::new(&adb)
        .args(["-s", &serial, "exec-out", "screencap", "-p"])
        .output()
        .context("Failed to run adb screencap")?;

    if !output_data.status.success() {
        anyhow::bail!(
            "ADB screenshot failed for device {}.\n\
             Check with: adb devices",
            serial
        );
    }

    std::fs::write(&raw_path, &output_data.stdout)
        .context("Failed to write screenshot")?;

    eprintln!("Captured Android screenshot");

    // Determine output path
    let has_title = args.title.is_some();
    let default_suffix = if has_title { "screenshot" } else { "mockup" };
    let output = args.output.unwrap_or_else(|| {
        PathBuf::from(format!("android-{}-{}.png", ts, default_suffix))
    });

    // Optionally keep raw screenshot
    if args.raw {
        let raw_out = output
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("android-{}-raw.png", ts));
        std::fs::copy(&raw_path, &raw_out)?;
        eprintln!("Raw screenshot saved to {}", raw_out.display());
    }

    if let Some(title) = &args.title {
        // Full Play Store screenshot: capture → mockup → title
        screenshot::run_android(
            &raw_path,
            &output,
            title,
            &args.device,
            &args.orientation,
            args.font.as_deref(),
            args.font_size,
        )?;
    } else {
        // Just mockup
        mockup::run(&raw_path, &output, &args.device, &args.orientation)?;
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&raw_path);

    Ok(())
}

fn run_wcapture(args: WcaptureArgs) -> Result<()> {
    use std::process::Command as Cmd;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Find window IDs using Swift + CoreGraphics
    let swift_code = format!(
        r#"
        import CoreGraphics
        let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as! [[String: Any]]
        for w in windows {{
            let name = w["kCGWindowOwnerName"] as? String ?? ""
            let title = w["kCGWindowName"] as? String ?? ""
            let wid = w["kCGWindowNumber"] as? Int ?? 0
            let bounds = w["kCGWindowBounds"] as? [String: Any] ?? [:]
            let ww = bounds["Width"] as? Int ?? 0
            let wh = bounds["Height"] as? Int ?? 0
            if name.localizedCaseInsensitiveContains("{}") && ww > 100 && wh > 100 {{
                print("\(wid)|\(name)|\(title)|\(ww)x\(wh)")
            }}
        }}
        "#,
        args.app
    );

    let output = Cmd::new("swift")
        .args(["-e", &swift_code])
        .output()
        .context("Failed to run swift")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to list windows: {}", err);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let windows: Vec<&str> = stdout.trim().lines().filter(|l| !l.is_empty()).collect();

    if windows.is_empty() {
        anyhow::bail!(
            "No window found for \"{}\". Make sure the app is running and has a visible window.",
            args.app
        );
    }

    if args.list {
        eprintln!("Windows matching \"{}\":", args.app);
        for w in &windows {
            let parts: Vec<&str> = w.splitn(4, '|').collect();
            if parts.len() >= 4 {
                eprintln!("  ID: {}  App: {}  Title: {}  Size: {}", parts[0], parts[1], parts[2], parts[3]);
            }
        }
        return Ok(());
    }

    // Use the first matching window
    let parts: Vec<&str> = windows[0].splitn(4, '|').collect();
    let window_id = parts[0];
    let app_name = parts.get(1).unwrap_or(&"");
    let window_title = parts.get(2).unwrap_or(&"");

    if windows.len() > 1 {
        eprintln!("Found {} windows for \"{}\", using first:", windows.len(), args.app);
    }
    eprintln!("Capturing: {} - {} (ID: {})", app_name, window_title, window_id);

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let raw_path = std::env::temp_dir().join(format!("launch-wcapture-{}.png", ts));

    // For Simulator, use xcrun simctl for a clean screenshot (no window chrome)
    let is_simulator = app_name.to_lowercase().contains("simulator");
    if is_simulator {
        eprintln!("Detected Simulator — using simctl for clean capture");
        let status = Cmd::new("xcrun")
            .args(["simctl", "io", "booted", "screenshot", "--type=png"])
            .arg(&raw_path)
            .status()
            .context("Failed to run xcrun simctl")?;
        if !status.success() {
            anyhow::bail!("Simulator screenshot failed. Is a simulator booted?");
        }
    } else {
        // Generic window capture (-o: no shadow, -x: no sound)
        let status = Cmd::new("screencapture")
            .args(["-l", window_id, "-o", "-x"])
            .arg(&raw_path)
            .status()
            .context("Failed to run screencapture")?;
        if !status.success() {
            anyhow::bail!("screencapture failed for window ID {}", window_id);
        }
    }

    eprintln!("Captured window screenshot");

    // Auto-select device: Simulator → iPhone, other Mac apps → MacBook
    let device_id = args.device.unwrap_or_else(|| {
        if is_simulator {
            device::DEFAULT_DEVICE.to_string()
        } else {
            device::DEFAULT_MAC_DEVICE.to_string()
        }
    });
    let orientation = args.orientation.unwrap_or_else(|| {
        if is_simulator { "portrait".to_string() } else { "front".to_string() }
    });

    // Determine output path
    let has_title = args.title.is_some();
    let default_suffix = if has_title { "screenshot" } else if args.no_mockup { "raw" } else { "mockup" };
    let safe_name: String = args.app.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    let output = args.output.unwrap_or_else(|| {
        PathBuf::from(format!("{}-{}-{}.png", safe_name, ts, default_suffix))
    });

    // Optionally keep raw screenshot
    if args.raw {
        let raw_out = output
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("{}-{}-raw.png", safe_name, ts));
        std::fs::copy(&raw_path, &raw_out)?;
        eprintln!("Raw screenshot saved to {}", raw_out.display());
    }

    if let Some(title) = &args.title {
        screenshot::run(
            &raw_path,
            &output,
            title,
            &device_id,
            &orientation,
            args.font.as_deref(),
            args.font_size,
        )?;
    } else if args.no_mockup {
        std::fs::copy(&raw_path, &output)?;
        eprintln!("Saved to {}", output.display());
    } else {
        mockup::run(&raw_path, &output, &device_id, &orientation)?;
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&raw_path);

    Ok(())
}

fn run_screenshot(args: ScreenshotArgs) -> Result<()> {
    let font_path = args.font.as_deref();

    if args.input.is_dir() {
        let out_dir = args.output.unwrap_or_else(|| args.input.join("screenshots"));
        std::fs::create_dir_all(&out_dir)?;

        let mut count = 0;
        for entry in std::fs::read_dir(&args.input)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                continue;
            }
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out_path = out_dir.join(format!("{}-screenshot.png", stem));
            screenshot::run(
                &path,
                &out_path,
                &args.title,
                &args.device,
                &args.orientation,
                font_path,
                args.font_size,
            )?;
            count += 1;
        }
        eprintln!("Processed {} images -> {}", count, out_dir.display());
        return Ok(());
    }

    let output = args.output.unwrap_or_else(|| {
        let stem = args.input.file_stem().unwrap_or_default().to_string_lossy();
        PathBuf::from(format!("{}-screenshot.png", stem))
    });

    screenshot::run(
        &args.input,
        &output,
        &args.title,
        &args.device,
        &args.orientation,
        font_path,
        args.font_size,
    )
}

fn run_collage(args: CollageArgs) -> Result<()> {
    if !args.input.is_dir() {
        anyhow::bail!(
            "Expected a directory of screenshots.\n\
             Usage: launch collage <screenshots_dir/>"
        );
    }

    // Collect image files from directory, sorted by name
    let mut images: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&args.input)
        .with_context(|| format!("Failed to read directory: {}", args.input.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            images.push(path);
        }
    }
    images.sort();

    if images.is_empty() {
        anyhow::bail!(
            "No images found in {}. Supported formats: png, jpg, jpeg, webp",
            args.input.display()
        );
    }

    let output = args.output.unwrap_or_else(|| {
        let dir_name = args
            .input
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        PathBuf::from(format!("{}-collage.png", dir_name))
    });

    collage::run(
        &images,
        &output,
        &args.device,
        &args.orientation,
        args.padding,
        args.no_frame,
    )
}
