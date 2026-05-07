mod clipboard;
mod config;
mod contents_json;
mod device;
mod mockup;
mod resize;
mod zip;

use std::path::PathBuf;

use anyhow::Result;
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Icon(args)) => run_icon(args),
        Some(Command::Mockup(args)) => run_mockup(args),
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
                "Provide a screenshot path or use -c to read from clipboard.\n\
                 Usage: applogo mockup <screenshot.png>\n\
                        applogo mockup -c"
            )
        })?
    };

    let output = args.output.unwrap_or_else(|| {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        PathBuf::from(format!("{}-mockup.png", stem))
    });

    mockup::run(&input, &output, &args.device, &args.orientation)
}
