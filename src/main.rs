mod clipboard;
mod config;
mod contents_json;
mod device;
mod mockup;
mod resize;
mod zip;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use config::Platform;
use zip::ZipEntry;

#[derive(Parser)]
#[command(name = "applogo", about = "Generate app icons and device mockups")]
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
    /// Output filename (default: screenshot-{timestamp}-mockup.png)
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Icon(args)) => run_icon(args),
        Some(Command::Mockup(args)) => run_mockup(args),
        Some(Command::Capture(args)) => run_capture(args),
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
                 Usage: applogo mockup <screenshot.png>\n\
                        applogo mockup <screenshots_dir/>\n\
                        applogo mockup -c"
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

    let raw_path = std::env::temp_dir().join(format!("applogo-capture-{}.png", ts));

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
    let output = args.output.unwrap_or_else(|| {
        PathBuf::from(format!("screenshot-{}-mockup.png", ts))
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

    // Apply mockup
    mockup::run(&raw_path, &output, &args.device, &args.orientation)?;

    // Clean up temp file
    let _ = std::fs::remove_file(&raw_path);

    Ok(())
}
