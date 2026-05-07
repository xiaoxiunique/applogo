mod config;
mod contents_json;
mod resize;
mod zip;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use config::Platform;
use zip::ZipEntry;

#[derive(Parser)]
#[command(name = "applogo", about = "Generate app icons for all platforms from a single image")]
struct Cli {
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let platforms = config::expand_platforms(&cli.platforms);

    // Load source image
    let img = resize::load_image(&cli.input)?;

    // Collect all unique (size, fill_white_bg) pairs we need to generate
    let mut size_requests: Vec<(u32, bool)> = Vec::new();

    for &platform in &platforms {
        let fill_white = !config::preserves_alpha(platform);
        let entries = config::get_entries(platform, &cli.android_filename);
        for entry in &entries {
            let key = (entry.expected_size, fill_white);
            if !size_requests.contains(&key) {
                size_requests.push(key);
            }
        }
    }

    if !cli.no_stores {
        for entry in &config::store_entries() {
            let fill_white = !config::store_preserves_alpha(&entry.filename);
            let key = (entry.expected_size, fill_white);
            if !size_requests.contains(&key) {
                size_requests.push(key);
            }
        }
    }

    // Resize all unique sizes
    eprintln!(
        "Resizing {} unique icon sizes from {}...",
        size_requests.len(),
        cli.input.display()
    );
    let resized = resize::resize_all(&img, &size_requests)?;

    // Build ZIP entries
    let mut zip_entries: Vec<ZipEntry> = Vec::new();
    let mut icon_count: usize = 0;

    for &platform in &platforms {
        let fill_white = !config::preserves_alpha(platform);
        let entries = config::get_entries(platform, &cli.android_filename);
        for entry in &entries {
            let data = resized[&(entry.expected_size, fill_white)].clone();
            let path = format!("{}{}", entry.folder, entry.filename);
            zip_entries.push(ZipEntry { path, data });
            icon_count += 1;
        }
    }

    // Store icons
    if !cli.no_stores {
        for entry in &config::store_entries() {
            let fill_white = !config::store_preserves_alpha(&entry.filename);
            let data = resized[&(entry.expected_size, fill_white)].clone();
            let path = entry.filename.clone();
            zip_entries.push(ZipEntry { path, data });
            icon_count += 1;
        }
    }

    // Contents.json (only if at least one Apple platform is selected)
    let has_apple = platforms.iter().any(|p| config::is_apple_platform(*p));
    if has_apple {
        let contents = contents_json::generate(&platforms, &cli.android_filename);
        zip_entries.push(ZipEntry {
            path: "Assets.xcassets/AppIcon.appiconset/Contents.json".into(),
            data: contents.into_bytes(),
        });
    }

    // Write ZIP
    zip::build_zip(&cli.output, zip_entries)?;

    eprintln!(
        "Generated {} icons for {} platform(s) -> {}",
        icon_count,
        platforms.len(),
        cli.output.display()
    );

    Ok(())
}
