use std::path::PathBuf;

use anyhow::{Context, Result};
use arboard::Clipboard;
use image::{ImageFormat, RgbaImage};

/// Read image from system clipboard and save to a temp file.
/// Returns the path to the saved PNG.
pub fn save_clipboard_image() -> Result<PathBuf> {
    let mut clip = Clipboard::new().context("Failed to access clipboard")?;
    let img_data = clip
        .get_image()
        .context("No image found in clipboard. Copy an image first.")?;

    let rgba = RgbaImage::from_raw(
        img_data.width as u32,
        img_data.height as u32,
        img_data.bytes.into_owned(),
    )
    .context("Failed to parse clipboard image data")?;

    let path = std::env::temp_dir().join("applogo-clipboard.png");
    rgba.save_with_format(&path, ImageFormat::Png)
        .context("Failed to save clipboard image")?;

    Ok(path)
}
