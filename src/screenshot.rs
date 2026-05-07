use std::path::Path;

use ab_glyph::{FontVec, PxScale};
use anyhow::{Context, Result};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

use crate::mockup;

/// Default canvas size (iPhone 6.5" App Store screenshot)
const CANVAS_W: u32 = 1242;
const CANVAS_H: u32 = 2688;

/// Gradient top color — #EEEEEE
const BG_TOP: [u8; 3] = [0xEE, 0xEE, 0xEE];
/// Gradient bottom color — #E4E4E4
const BG_BOTTOM: [u8; 3] = [0xE4, 0xE4, 0xE4];
/// Gradient end position (fraction of canvas height)
const BG_GRADIENT_END: f32 = 0.77;

/// Title text color — #525252
const TEXT_COLOR: Rgba<u8> = Rgba([82, 82, 82, 255]);

/// Title vertical center position (fraction of canvas height from top)
const TITLE_Y_RATIO: f32 = 0.12;

/// Mockup top position (fraction of canvas height)
const MOCKUP_Y_RATIO: f32 = 0.20;

/// Mockup width (fraction of canvas width)
const MOCKUP_W_RATIO: f32 = 0.85;

/// Embedded default font (优设标题黑)
const EMBEDDED_FONT: &[u8] = include_bytes!("../resources/YouSheBiaoTiHei.ttf");

/// Load embedded default font.
fn load_embedded_font() -> Result<FontVec> {
    FontVec::try_from_vec(EMBEDDED_FONT.to_vec())
        .map_err(|_| anyhow::anyhow!("Failed to load embedded font"))
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
        load_embedded_font()?
    };

    // 3. Create canvas with linear gradient background
    let mut canvas = RgbaImage::new(CANVAS_W, CANVAS_H);
    let gradient_end_y = (CANVAS_H as f32 * BG_GRADIENT_END) as u32;
    for y in 0..CANVAS_H {
        let t = if y < gradient_end_y {
            y as f32 / gradient_end_y as f32
        } else {
            1.0
        };
        let r = BG_TOP[0] as f32 + (BG_BOTTOM[0] as f32 - BG_TOP[0] as f32) * t;
        let g = BG_TOP[1] as f32 + (BG_BOTTOM[1] as f32 - BG_TOP[1] as f32) * t;
        let b = BG_TOP[2] as f32 + (BG_BOTTOM[2] as f32 - BG_TOP[2] as f32) * t;
        for x in 0..CANVAS_W {
            canvas.put_pixel(x, y, Rgba([r as u8, g as u8, b as u8, 255]));
        }
    }

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
