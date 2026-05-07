use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, RgbaImage};

use crate::device::{self, DeviceConfig, OrientationConfig};

const MARKER_KEY: &str = "launch";
const MARKER_VALUE: &str = "mockup";

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

    // 4. Compose: for each pixel, decide what to show
    //    - template opaque → show template (device frame)
    //    - template transparent + mask opaque → show screenshot (screen area)
    //    - both transparent → nothing (outside device)
    let mut result = RgbaImage::from_pixel(tw, th, image::Rgba([0, 0, 0, 0]));

    for y in 0..th {
        for x in 0..tw {
            let tpl_px = template.get_pixel(x, y);
            let mask_px = mask.get_pixel(x, y);
            let tpl_a = tpl_px[3] as f32 / 255.0;

            if tpl_a > 0.99 {
                // Fully opaque frame pixel — show template directly
                result.put_pixel(x, y, *tpl_px);
            } else if mask_px[3] > 0 {
                // Screen area — show screenshot, blend template on top if semi-transparent
                let sx = x as i64 - min_x as i64;
                let sy = y as i64 - min_y as i64;
                let scr_px = if sx >= 0
                    && sy >= 0
                    && (sx as u32) < fitted.width()
                    && (sy as u32) < fitted.height()
                {
                    *fitted.get_pixel(sx as u32, sy as u32)
                } else {
                    image::Rgba([0, 0, 0, 255]) // black letterbox
                };

                if tpl_a < 0.01 {
                    // Fully transparent template — pure screenshot
                    result.put_pixel(x, y, scr_px);
                } else {
                    // Semi-transparent template edge — blend frame over screenshot
                    let r = (tpl_px[0] as f32 * tpl_a + scr_px[0] as f32 * (1.0 - tpl_a)) as u8;
                    let g = (tpl_px[1] as f32 * tpl_a + scr_px[1] as f32 * (1.0 - tpl_a)) as u8;
                    let b = (tpl_px[2] as f32 * tpl_a + scr_px[2] as f32 * (1.0 - tpl_a)) as u8;
                    result.put_pixel(x, y, image::Rgba([r, g, b, 255]));
                }
            }
            // else: outside device — stays transparent
        }
    }

    Ok(result)
}

/// Check if a PNG file was already processed by launch.
pub fn is_already_processed(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let decoder = png::Decoder::new(BufReader::new(file));
    let Ok(reader) = decoder.read_info() else {
        return false;
    };
    reader
        .info()
        .uncompressed_latin1_text
        .iter()
        .any(|t| t.keyword == MARKER_KEY && t.text == MARKER_VALUE)
}

/// Save an RGBA image as PNG with the launch marker.
fn save_with_marker(img: &RgbaImage, output: &Path) -> Result<()> {
    let file = File::create(output)
        .with_context(|| format!("Failed to create {}", output.display()))?;
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, img.width(), img.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.add_text_chunk(MARKER_KEY.to_string(), MARKER_VALUE.to_string())?;
    let mut writer = encoder.write_header()?;
    writer.write_image_data(img.as_raw())?;
    writer.finish()?;
    Ok(())
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
    save_with_marker(&result, output)?;

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
