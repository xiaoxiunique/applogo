use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};

use crate::device::{self, DeviceConfig, OrientationConfig};

const TEMPLATE_BASE_URL: &str = "https://mockuphone.com/images/mockup_templates/";
const MASK_BASE_URL: &str = "https://mockuphone.com/images/mockup_mask_templates/";

/// Get the cache directory for device resources.
fn cache_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".applogo").join("devices"))
}

/// Download a file if not cached. Returns the local path.
fn ensure_cached(url: &str, subdir: &str, filename: &str) -> Result<PathBuf> {
    let dir = cache_dir()?.join(subdir);
    let path = dir.join(filename);
    if path.exists() {
        return Ok(path);
    }
    fs::create_dir_all(&dir)?;
    eprintln!("Downloading {}...", url);
    let bytes = reqwest::blocking::get(url)
        .with_context(|| format!("Failed to download {}", url))?
        .bytes()
        .with_context(|| format!("Failed to read response from {}", url))?;
    fs::write(&path, &bytes)?;
    Ok(path)
}

/// Ensure device template and mask PNGs are cached locally.
fn ensure_device_resources(
    device: &DeviceConfig,
    orientation: &str,
) -> Result<(PathBuf, PathBuf)> {
    let filename = device.template_filename(orientation);
    let template_path = ensure_cached(
        &format!("{}{}", TEMPLATE_BASE_URL, filename),
        "templates",
        &filename,
    )?;
    let mask_path = ensure_cached(
        &format!("{}{}", MASK_BASE_URL, filename),
        "masks",
        &filename,
    )?;
    Ok((template_path, mask_path))
}

/// Resize screenshot to fit device display resolution with letterboxing.
fn fit_to_resolution(img: &DynamicImage, width: u32, height: u32) -> RgbaImage {
    let (iw, ih) = img.dimensions();
    let img_ratio = iw as f64 / ih as f64;
    let dev_ratio = width as f64 / height as f64;

    // Auto-rotate if screenshot orientation doesn't match device
    let img = if (img_ratio > 1.0) != (dev_ratio > 1.0) {
        img.rotate90()
    } else {
        img.clone()
    };
    let (iw, ih) = img.dimensions();

    // Scale to fit
    let scale = (width as f64 / iw as f64).min(height as f64 / ih as f64);
    let new_w = (iw as f64 * scale).round() as u32;
    let new_h = (ih as f64 * scale).round() as u32;
    let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);

    // Letterbox on black background
    let mut canvas = RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 255]));
    let offset_x = (width - new_w) / 2;
    let offset_y = (height - new_h) / 2;
    image::imageops::overlay(&mut canvas, &resized.to_rgba8(), offset_x as i64, offset_y as i64);
    canvas
}

/// Compose the final mockup image.
fn compose(
    screenshot: &DynamicImage,
    device: &DeviceConfig,
    orientation: &OrientationConfig,
    template_path: &Path,
    mask_path: &Path,
) -> Result<RgbaImage> {
    let (dw, dh) = device.display_resolution;

    // For landscape, swap the display resolution
    let (dw, dh) = if orientation.name == "landscape" {
        (dh, dw)
    } else {
        (dw, dh)
    };

    // 1. Resize screenshot to display resolution
    let fitted = fit_to_resolution(screenshot, dw, dh);

    // 2. Load template and mask
    let template = image::open(template_path)
        .context("Failed to open device template")?
        .to_rgba8();
    let mask = image::open(mask_path)
        .context("Failed to open device mask")?
        .to_rgba8();

    let (tw, th) = (template.width(), template.height());

    // 3. Calculate screen area from coords (TL corner)
    let coords = &orientation.screen_coord;
    let min_x = coords.iter().map(|c| c.0).min().unwrap();
    let min_y = coords.iter().map(|c| c.1).min().unwrap();

    // 4. Compose: screenshot behind mask, frame on top
    // Start with transparent canvas
    let mut result = RgbaImage::from_pixel(tw, th, image::Rgba([0, 0, 0, 0]));

    // Paste screenshot at screen position
    image::imageops::overlay(&mut result, &fitted, min_x as i64, min_y as i64);

    // Apply mask: keep screenshot only where mask is opaque
    for y in 0..th {
        for x in 0..tw {
            let mask_px = mask.get_pixel(x, y);
            let result_px = result.get_pixel(x, y);
            // Use mask alpha to control visibility
            let mask_alpha = mask_px[3] as f32 / 255.0;
            result.put_pixel(
                x,
                y,
                image::Rgba([
                    (result_px[0] as f32 * mask_alpha) as u8,
                    (result_px[1] as f32 * mask_alpha) as u8,
                    (result_px[2] as f32 * mask_alpha) as u8,
                    (mask_alpha * 255.0) as u8,
                ]),
            );
        }
    }

    // Overlay device frame on top (alpha compositing)
    image::imageops::overlay(&mut result, &template, 0, 0);

    Ok(result)
}

/// Run the mockup generation.
pub fn run(
    input: &Path,
    output: &Path,
    device_id: &str,
    orientation_name: &str,
) -> Result<()> {
    let device = device::find_device(device_id)
        .with_context(|| format!("Unknown device: {}", device_id))?;

    let orientation = device
        .find_orientation(orientation_name)
        .with_context(|| {
            let available: Vec<_> = device.orientations.iter().map(|o| o.name).collect();
            format!(
                "Unknown orientation '{}' for {}. Available: {}",
                orientation_name,
                device.name,
                available.join(", ")
            )
        })?;

    // Ensure resources are downloaded
    let (template_path, mask_path) = ensure_device_resources(device, orientation_name)?;

    // Load screenshot
    let screenshot = image::open(input)
        .with_context(|| format!("Failed to open screenshot: {}", input.display()))?;

    eprintln!(
        "Generating {} {} mockup for {}...",
        device.name, orientation_name, input.display()
    );

    // Compose
    let result = compose(&screenshot, device, orientation, &template_path, &mask_path)?;

    // Save
    result
        .save_with_format(output, ImageFormat::Png)
        .with_context(|| format!("Failed to save mockup to {}", output.display()))?;

    eprintln!("Saved mockup to {}", output.display());
    Ok(())
}

/// Print available devices.
pub fn list_devices() {
    eprintln!("Available devices:");
    for device in device::all_devices() {
        let orientations: Vec<_> = device.orientations.iter().map(|o| o.name).collect();
        eprintln!(
            "  {} ({} {}) — {}",
            device.id,
            device.name,
            device.color,
            orientations.join(", ")
        );
    }
}
