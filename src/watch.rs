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

/// Watch iOS Simulator for screen changes and auto-capture.
pub fn run(
    output_dir: &Path,
    device_id: &str,
    orientation: &str,
    interval: Duration,
    no_collage: bool,
) -> Result<()> {
    use std::process::Command as Cmd;

    // Verify simulator is running
    let test_path = std::env::temp_dir().join("launch-watch-test.png");
    let status = Cmd::new("xcrun")
        .args(["simctl", "io", "booted", "screenshot", "--type=png"])
        .arg(&test_path)
        .stderr(std::process::Stdio::null())
        .status()
        .context("Failed to run xcrun simctl. Is Xcode installed?")?;
    let _ = std::fs::remove_file(&test_path);

    if !status.success() {
        anyhow::bail!(
            "No booted iOS Simulator found.\n\
             Start one with: open -a Simulator"
        );
    }

    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;

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

    eprintln!("Watching iOS Simulator (every {:.1}s)...", interval.as_secs_f32());
    eprintln!("Interact with the Simulator. Press Ctrl+C to stop.\n");

    while running.load(Ordering::SeqCst) {
        // Capture screenshot
        let status = Cmd::new("xcrun")
            .args(["simctl", "io", "booted", "screenshot", "--type=png"])
            .arg(&tmp_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if let Ok(s) = status {
            if !s.success() {
                // Simulator may have been closed
                eprintln!("Simulator disconnected, stopping...");
                break;
            }
        } else {
            break;
        }

        // Compare with previous frame
        if let Some(current_hash) = file_hash(&tmp_path) {
            if prev_hash != Some(current_hash) {
                count += 1;
                let filename = format!("{:03}.png", count);
                let save_path = output_dir.join(&filename);
                std::fs::copy(&tmp_path, &save_path)
                    .with_context(|| format!("Failed to save {}", save_path.display()))?;
                saved_paths.push(save_path);
                eprintln!("  #{} captured", count);
                prev_hash = Some(current_hash);
            }
        }

        std::thread::sleep(interval);
    }

    // Cleanup temp file
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
        let mockup_path = output_dir.join(format!("{}-mockup.png", stem));
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
            true, // no_frame: mockups are already framed
        )?;
    }

    eprintln!(
        "\nDone! {} screenshots saved to {}",
        saved_paths.len(),
        output_dir.display()
    );

    Ok(())
}
