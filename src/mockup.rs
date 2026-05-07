use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};

use crate::device::{self, DeviceConfig, OrientationConfig};

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
    image::imageops::overlay(
        &mut canvas,
        &resized.to_rgba8(),
        offset_x as i64,
        offset_y as i64,
    );
    canvas
}

/// Load an image from embedded bytes.
fn load_from_bytes(bytes: &[u8]) -> Result<RgbaImage> {
    let img = image::load_from_memory(bytes).context("Failed to decode embedded image")?;
    Ok(img.to_rgba8())
}

/// Compose the final mockup image.
fn compose(
    screenshot: &DynamicImage,
    device: &DeviceConfig,
    orientation: &OrientationConfig,
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

    // 2. Load template and mask from embedded bytes
    let template = load_from_bytes(orientation.template)?;
    let mask = load_from_bytes(orientation.mask)?;

    let (tw, th) = (template.width(), template.height());

    // 3. Calculate screen area from coords (TL corner)
    let coords = &orientation.screen_coord;
    let min_x = coords.iter().map(|c| c.0).min().unwrap();
    let min_y = coords.iter().map(|c| c.1).min().unwrap();

    // 4. Compose: screenshot behind mask, frame on top
    let mut result = RgbaImage::from_pixel(tw, th, image::Rgba([0, 0, 0, 0]));

    // Paste screenshot at screen position
    image::imageops::overlay(&mut result, &fitted, min_x as i64, min_y as i64);

    // Apply mask: keep screenshot only where mask is opaque
    for y in 0..th {
        for x in 0..tw {
            let mask_px = mask.get_pixel(x, y);
            let result_px = result.get_pixel(x, y);
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

    // Overlay device frame on top
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
    let device =
        device::find_device(device_id).with_context(|| format!("Unknown device: {}", device_id))?;

    let orientation = device.find_orientation(orientation_name).with_context(|| {
        let available: Vec<_> = device.orientations.iter().map(|o| o.name).collect();
        format!(
            "Unknown orientation '{}' for {}. Available: {}",
            orientation_name,
            device.name,
            available.join(", ")
        )
    })?;

    let screenshot = image::open(input)
        .with_context(|| format!("Failed to open screenshot: {}", input.display()))?;

    eprintln!(
        "Generating {} {} mockup...",
        device.name, orientation_name
    );

    let result = compose(&screenshot, device, orientation)?;

    let mut buf = Cursor::new(Vec::new());
    result
        .write_to(&mut buf, ImageFormat::Png)
        .with_context(|| format!("Failed to encode mockup PNG"))?;
    std::fs::write(output, buf.into_inner())
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
            device.id, device.name, device.color,
            orientations.join(", ")
        );
    }
}
