use std::path::Path;

use ab_glyph::{FontVec, PxScale};
use anyhow::{Context, Result};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

use crate::device;
use crate::mockup;

/// Default canvas size (iPhone 6.5" App Store screenshot)
const CANVAS_W: u32 = 1242;
const CANVAS_H: u32 = 2688;

/// Background color — light gray
const BG_COLOR: Rgba<u8> = Rgba([237, 237, 237, 255]);

/// Title text color — black
const TEXT_COLOR: Rgba<u8> = Rgba([0, 0, 0, 255]);

/// Default font size
const FONT_SIZE: f32 = 120.0;

/// Title vertical center position (fraction of canvas height from top)
const TITLE_Y_RATIO: f32 = 0.12;

/// Mockup top position (fraction of canvas height)
const MOCKUP_Y_RATIO: f32 = 0.22;

/// Mockup width (fraction of canvas width)
const MOCKUP_W_RATIO: f32 = 0.68;

/// Load system CJK font (macOS).
fn load_system_font() -> Result<FontVec> {
    let candidates = [
        ("/System/Library/Fonts/STHeiti Medium.ttc", 1u32), // Heiti SC Medium
        ("/System/Library/Fonts/STHeiti Medium.ttc", 0),     // Heiti TC Medium
        ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0),   // Hiragino Sans GB
    ];

    for (path, index) in candidates {
        if Path::new(path).exists() {
            let data = std::fs::read(path)
                .with_context(|| format!("Failed to read font: {}", path))?;
            if let Ok(font) = FontVec::try_from_vec_and_index(data, index) {
                return Ok(font);
            }
        }
    }

    anyhow::bail!(
        "No CJK font found. Use --font to specify a .ttf/.otf/.ttc font file."
    )
}

/// Load font from a file path.
fn load_font_file(path: &Path) -> Result<FontVec> {
    let data = std::fs::read(path)
        .with_context(|| format!("Failed to read font: {}", path.display()))?;
    FontVec::try_from_vec(data).map_err(|_| anyhow::anyhow!("Invalid font file: {}", path.display()))
}

/// Measure text width using glyph advances.
fn measure_text_width(font: &FontVec, scale: PxScale, text: &str) -> f32 {
    use ab_glyph::{Font, ScaleFont};
    let scaled = font.as_scaled(scale);
    let mut width = 0.0f32;
    let mut prev = None;
    for ch in text.chars() {
        let glyph_id = font.glyph_id(ch);
        if let Some(prev_id) = prev {
            width += scaled.kern(prev_id, glyph_id);
        }
        width += scaled.h_advance(glyph_id);
        prev = Some(glyph_id);
    }
    width
}

/// Generate an App Store screenshot with title + device mockup.
pub fn run(
    input: &Path,
    output: &Path,
    title: &str,
    device_id: &str,
    orientation: &str,
    font_path: Option<&Path>,
    font_size: f32,
) -> Result<()> {
    // 1. Prepare the mockup image
    let mockup_img = if mockup::is_already_processed(input) {
        eprintln!("Image already has mockup frame, using as-is");
        image::open(input)
            .context("Failed to open image")?
            .to_rgba8()
    } else {
        eprintln!("Applying mockup frame first...");
        let tmp = std::env::temp_dir().join("launch-screenshot-tmp.png");
        mockup::run(input, &tmp, device_id, orientation)?;
        let img = image::open(&tmp)
            .context("Failed to open mockup result")?
            .to_rgba8();
        let _ = std::fs::remove_file(&tmp);
        img
    };

    // 2. Load font
    let font = if let Some(fp) = font_path {
        load_font_file(fp)?
    } else {
        load_system_font()?
    };

    // 3. Create canvas
    let mut canvas = RgbaImage::from_pixel(CANVAS_W, CANVAS_H, BG_COLOR);

    // 4. Scale and place mockup
    let mockup_target_w = (CANVAS_W as f32 * MOCKUP_W_RATIO) as u32;
    let scale = mockup_target_w as f32 / mockup_img.width() as f32;
    let mockup_target_h = (mockup_img.height() as f32 * scale) as u32;

    let resized_mockup = image::imageops::resize(
        &mockup_img,
        mockup_target_w,
        mockup_target_h,
        image::imageops::FilterType::Lanczos3,
    );

    let mockup_x = ((CANVAS_W - mockup_target_w) / 2) as i64;
    let mockup_y = (CANVAS_H as f32 * MOCKUP_Y_RATIO) as i64;
    image::imageops::overlay(&mut canvas, &resized_mockup, mockup_x, mockup_y);

    // 5. Draw title text
    let scale = PxScale::from(font_size);
    let text_w = measure_text_width(&font, scale, title);
    let text_x = ((CANVAS_W as f32 - text_w) / 2.0).max(0.0) as i32;
    let text_y = (CANVAS_H as f32 * TITLE_Y_RATIO - font_size / 2.0) as i32;

    draw_text_mut(&mut canvas, TEXT_COLOR, text_x, text_y, scale, &font, title);

    // 6. Save
    canvas
        .save(output)
        .with_context(|| format!("Failed to save screenshot to {}", output.display()))?;

    eprintln!("Saved App Store screenshot to {}", output.display());
    Ok(())
}
