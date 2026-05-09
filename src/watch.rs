use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::collage;
use crate::mockup;

/// Hash file bytes for change detection.
fn file_hash(path: &Path) -> Option<u64> {
    let data = std::fs::read(path).ok()?;
    let mut hasher = std::hash::DefaultHasher::new();
    data.hash(&mut hasher);
    Some(hasher.finish())
}

/// Find adb binary — check PATH first, then common install locations.
fn find_adb() -> Result<PathBuf> {
    let common_paths = [
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join("Library/Android/sdk/platform-tools/adb")),
        Some(PathBuf::from("/opt/homebrew/bin/adb")),
        Some(PathBuf::from("/usr/local/bin/adb")),
    ];
    if crate::which_exists("adb") {
        Ok(PathBuf::from("adb"))
    } else if let Some(path) = common_paths.iter().flatten().find(|p| p.exists()) {
        eprintln!("Found adb at {}", path.display());
        Ok(path.clone())
    } else {
        anyhow::bail!(
            "adb not found. Install Android SDK or add platform-tools to PATH.\n\
             Common location: ~/Library/Android/sdk/platform-tools/"
        );
    }
}

/// Auto-detect ADB device serial.
fn detect_adb_device(adb: &Path, serial: Option<&str>) -> Result<String> {
    use std::process::Command as Cmd;

    if let Some(s) = serial {
        return Ok(s.to_string());
    }

    let output = Cmd::new(adb)
        .args(["devices"])
        .output()
        .context("Failed to run adb")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let devices: Vec<&str> = stdout
        .lines()
        .skip(1)
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
            Ok(devices[0].to_string())
        }
        _ => {
            eprintln!("Multiple devices, using first: {}", devices[0]);
            Ok(devices[0].to_string())
        }
    }
}

/// Capture a single frame from the source. Returns true if successful.
enum CaptureSource {
    IosSimulator,
    Adb { adb: PathBuf, serial: String },
}

impl CaptureSource {
    fn capture(&self, dst: &Path) -> bool {
        use std::process::Command as Cmd;
        match self {
            CaptureSource::IosSimulator => {
                Cmd::new("xcrun")
                    .args(["simctl", "io", "booted", "screenshot", "--type=png"])
                    .arg(dst)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            }
            CaptureSource::Adb { adb, serial } => {
                let output = Cmd::new(adb)
                    .args(["-s", serial, "exec-out", "screencap", "-p"])
                    .output();
                match output {
                    Ok(o) if o.status.success() && !o.stdout.is_empty() => {
                        std::fs::write(dst, &o.stdout).is_ok()
                    }
                    _ => false,
                }
            }
        }
    }

    fn label(&self) -> &str {
        match self {
            CaptureSource::IosSimulator => "iOS Simulator",
            CaptureSource::Adb { .. } => "Android device",
        }
    }
}

/// Shared watch loop + post-processing.
fn watch_loop(
    source: &CaptureSource,
    output_dir: &Path,
    device_id: &str,
    orientation: &str,
    interval: Duration,
    no_collage: bool,
) -> Result<()> {
    // Verify device is reachable
    let test_path = std::env::temp_dir().join("launch-watch-test.png");
    if !source.capture(&test_path) {
        let _ = std::fs::remove_file(&test_path);
        anyhow::bail!(
            "Cannot capture from {}. Make sure it is running.",
            source.label()
        );
    }
    let _ = std::fs::remove_file(&test_path);

    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create directory: {}", output_dir.display()))?;

    let raw_dir = output_dir.join("raw");
    let mockup_dir = output_dir.join("mockups");
    std::fs::create_dir_all(&raw_dir)?;
    std::fs::create_dir_all(&mockup_dir)?;

    // Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("Failed to set Ctrl+C handler")?;

    let tmp_path = std::env::temp_dir().join("launch-watch-latest.png");
    let mut prev_hash: Option<u64> = None;
    let mut count: u32 = 0;
    let mut saved_paths: Vec<PathBuf> = Vec::new();

    eprintln!(
        "Watching {} (every {:.1}s)...",
        source.label(),
        interval.as_secs_f32()
    );
    eprintln!("Press Ctrl+C to stop.\n");

    while running.load(Ordering::SeqCst) {
        if !source.capture(&tmp_path) {
            eprintln!("{} disconnected, stopping...", source.label());
            break;
        }

        if let Some(current_hash) = file_hash(&tmp_path) {
            if prev_hash != Some(current_hash) {
                count += 1;
                let filename = format!("{:03}.png", count);
                let save_path = raw_dir.join(&filename);
                std::fs::copy(&tmp_path, &save_path)
                    .with_context(|| format!("Failed to save {}", save_path.display()))?;
                saved_paths.push(save_path);
                eprintln!("  #{} captured", count);
                prev_hash = Some(current_hash);
            }
        }

        std::thread::sleep(interval);
    }

    let _ = std::fs::remove_file(&tmp_path);

    if saved_paths.is_empty() {
        eprintln!("\nNo screen changes detected.");
        return Ok(());
    }

    eprintln!("\n{} screenshots captured. Processing...", saved_paths.len());

    // Apply mockup frames
    let mut mockup_paths: Vec<PathBuf> = Vec::new();
    for (i, path) in saved_paths.iter().enumerate() {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let mockup_path = mockup_dir.join(format!("{}.png", stem));
        eprintln!(
            "  [{}/{}] Applying mockup frame...",
            i + 1,
            saved_paths.len()
        );
        mockup::run(path, &mockup_path, device_id, orientation)?;
        mockup_paths.push(mockup_path);
    }

    // Generate collage
    if !no_collage && mockup_paths.len() > 1 {
        let collage_path = output_dir.join("collage.png");
        eprintln!("  Generating collage...");
        collage::run(
            &mockup_paths,
            &collage_path,
            device_id,
            orientation,
            None,
            true,
        )?;
    }

    eprintln!(
        "\nDone! {} screenshots saved to {}",
        saved_paths.len(),
        output_dir.display()
    );

    Ok(())
}

/// Watch iOS Simulator for screen changes.
pub fn run(
    output_dir: &Path,
    device_id: &str,
    orientation: &str,
    interval: Duration,
    no_collage: bool,
) -> Result<()> {
    watch_loop(
        &CaptureSource::IosSimulator,
        output_dir,
        device_id,
        orientation,
        interval,
        no_collage,
    )
}

/// Watch Android device for screen changes.
pub fn run_android(
    output_dir: &Path,
    device_id: &str,
    orientation: &str,
    interval: Duration,
    no_collage: bool,
    serial: Option<&str>,
) -> Result<()> {
    let adb = find_adb()?;
    let serial = detect_adb_device(&adb, serial)?;
    watch_loop(
        &CaptureSource::Adb { adb, serial },
        output_dir,
        device_id,
        orientation,
        interval,
        no_collage,
    )
}
