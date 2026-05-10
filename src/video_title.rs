use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ab_glyph::{FontVec, PxScale};
use anyhow::{Context, Result};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

const EMBEDDED_FONT: &[u8] = include_bytes!("../resources/YouSheBiaoTiHei.ttf");

#[derive(Debug, Clone)]
pub struct TitleOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub text: String,
    pub duration: f64,
    pub y_ratio: f64,
    pub wrap_width: f64,
    pub font_size: Option<u32>,
    pub font: Option<PathBuf>,
}

pub fn run(options: TitleOptions) -> Result<()> {
    ensure_ffmpeg()?;
    ensure_ffprobe()?;

    if !options.input.exists() {
        anyhow::bail!("Input video not found: {}", options.input.display());
    }
    if options.duration <= 0.0 {
        anyhow::bail!("--duration must be greater than 0");
    }
    if !(0.0..=1.0).contains(&options.y_ratio) {
        anyhow::bail!("--y-ratio must be between 0 and 1");
    }
    if !(0.2..=1.0).contains(&options.wrap_width) {
        anyhow::bail!("--wrap-width must be between 0.2 and 1.0");
    }

    let (video_w, video_h) = probe_dimensions(&options.input)?;
    let video_duration = probe_duration(&options.input)?;
    let paragraphs = title_paragraphs(&options.text);
    if paragraphs.is_empty() {
        anyhow::bail!("--text cannot be empty");
    }
    let font = load_font(options.font.as_deref())?;
    let (lines, font_size) = layout_title_lines(
        &paragraphs,
        &font,
        video_w,
        video_h,
        options.font_size,
        options.wrap_width,
    );

    if let Some(parent) = options.output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
    }

    let temp_dir = std::env::temp_dir().join(format!(
        "launch-title-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir)
        .with_context(|| format!("Failed to create {}", temp_dir.display()))?;

    let overlay_path = temp_dir.join("title-overlay.png");
    render_title_overlay(
        &overlay_path,
        video_w,
        video_h,
        &lines,
        &font,
        font_size,
        options.y_ratio,
    )?;

    let title_duration = options.duration.min(video_duration);
    let head_path = temp_dir.join("title-head.mp4");
    let concat_path = temp_dir.join("concat.txt");

    let result = run_title_head(
        &options.input,
        &head_path,
        &overlay_path,
        title_duration,
        true,
    )
    .and_then(|_| {
        if video_duration > title_duration + 0.05 {
            concat_with_inpoint(
                &head_path,
                &options.input,
                title_duration,
                &concat_path,
                &options.output,
            )
        } else {
            std::fs::rename(&head_path, &options.output)
                .or_else(|_| std::fs::copy(&head_path, &options.output).map(|_| ()))
                .with_context(|| format!("Failed to write {}", options.output.display()))
        }
    });

    let _ = std::fs::remove_dir_all(&temp_dir);
    result
}

fn load_font(font: Option<&Path>) -> Result<FontVec> {
    let data = if let Some(font) = font {
        std::fs::read(font).with_context(|| format!("Failed to read font: {}", font.display()))?
    } else {
        EMBEDDED_FONT.to_vec()
    };
    FontVec::try_from_vec(data).map_err(|_| anyhow::anyhow!("Failed to load title font"))
}

fn title_paragraphs(text: &str) -> Vec<String> {
    text.replace("\\n", "\n")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn layout_title_lines(
    paragraphs: &[String],
    font: &FontVec,
    video_w: u32,
    video_h: u32,
    font_size: Option<u32>,
    wrap_width_ratio: f64,
) -> (Vec<String>, u32) {
    let max_width = title_wrap_width(video_w, wrap_width_ratio);
    let mut size = font_size.unwrap_or_else(|| default_font_size(video_w, video_h, paragraphs));
    let mut lines = wrap_title_lines(paragraphs, font, size, max_width);

    if font_size.is_none() {
        for _ in 0..4 {
            let next_size = default_font_size(video_w, video_h, &lines);
            if next_size == size {
                break;
            }
            size = next_size;
            lines = wrap_title_lines(paragraphs, font, size, max_width);
        }
    }

    (lines, size)
}

fn default_font_size(video_w: u32, video_h: u32, lines: &[String]) -> u32 {
    let max_chars = lines
        .iter()
        .map(|line| line.chars().count().max(1))
        .max()
        .unwrap_or(4) as f64;
    let by_width = video_w as f64 / (max_chars * 0.92);
    let by_height = video_h as f64 / ((lines.len() as f64 + 0.5) * 5.2);
    by_width.min(by_height).clamp(72.0, 360.0).round() as u32
}

fn title_wrap_width(video_w: u32, wrap_width_ratio: f64) -> f32 {
    (video_w as f64 * wrap_width_ratio).round() as f32
}

fn wrap_title_lines(
    paragraphs: &[String],
    font: &FontVec,
    font_size: u32,
    max_width: f32,
) -> Vec<String> {
    let scale = PxScale::from(font_size as f32);
    paragraphs
        .iter()
        .flat_map(|paragraph| wrap_title_paragraph(paragraph, font, scale, max_width))
        .collect()
}

fn wrap_title_paragraph(
    paragraph: &str,
    font: &FontVec,
    scale: PxScale,
    max_width: f32,
) -> Vec<String> {
    let words: Vec<&str> = paragraph.split_whitespace().collect();
    if words.len() <= 1 {
        return wrap_unspaced_text(paragraph, font, scale, max_width);
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in words {
        push_wrapped_word(&mut lines, &mut current, word, font, scale, max_width);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn push_wrapped_word(
    lines: &mut Vec<String>,
    current: &mut String,
    word: &str,
    font: &FontVec,
    scale: PxScale,
    max_width: f32,
) {
    if current.is_empty() {
        let mut segments = wrap_unspaced_text(word, font, scale, max_width);
        if let Some(last) = segments.pop() {
            lines.extend(segments);
            *current = last;
        }
        return;
    }

    let candidate = format!("{} {}", current, word);
    if measure_text_width(font, scale, &candidate) <= max_width {
        *current = candidate;
        return;
    }

    lines.push(std::mem::take(current));
    push_wrapped_word(lines, current, word, font, scale, max_width);
}

fn wrap_unspaced_text(text: &str, font: &FontVec, scale: PxScale, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        let candidate = format!("{}{}", current, ch);
        if !current.is_empty() && measure_text_width(font, scale, &candidate) > max_width {
            lines.push(std::mem::take(&mut current));
            current.push(ch);
        } else {
            current = candidate;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn render_title_overlay(
    output: &Path,
    width: u32,
    height: u32,
    lines: &[String],
    font: &FontVec,
    font_size: u32,
    y_ratio: f64,
) -> Result<()> {
    let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    let scale = PxScale::from(font_size as f32);
    let line_gap = (font_size as f32 * 0.92).round() as i32;
    let block_h = line_gap * lines.len() as i32;
    let start_y = ((height as f64 * y_ratio) as i32 - block_h / 2).max(0);

    for (i, line) in lines.iter().enumerate() {
        let y = start_y + i as i32 * line_gap;
        let text_w = measure_text_width(font, scale, line);
        let x = ((width as f32 - text_w) / 2.0).max(0.0) as i32;

        draw_layer(
            &mut canvas,
            font,
            scale,
            line,
            x + 10,
            y + 10,
            Rgba([255, 79, 179, 255]),
            18,
            Rgba([37, 16, 90, 255]),
        );
        draw_layer(
            &mut canvas,
            font,
            scale,
            line,
            x,
            y,
            Rgba([255, 255, 255, 255]),
            30,
            Rgba([255, 255, 255, 255]),
        );
        draw_layer(
            &mut canvas,
            font,
            scale,
            line,
            x,
            y,
            Rgba([255, 245, 107, 255]),
            12,
            Rgba([37, 16, 90, 255]),
        );
    }

    canvas
        .save(output)
        .with_context(|| format!("Failed to save title overlay: {}", output.display()))
}

#[allow(clippy::too_many_arguments)]
fn draw_layer(
    canvas: &mut RgbaImage,
    font: &FontVec,
    scale: PxScale,
    text: &str,
    x: i32,
    y: i32,
    fill: Rgba<u8>,
    stroke: i32,
    stroke_color: Rgba<u8>,
) {
    if stroke > 0 {
        let step = (stroke / 5).max(3);
        for ring in (step..=stroke).step_by(step as usize) {
            let offsets = [
                (-ring, 0),
                (ring, 0),
                (0, -ring),
                (0, ring),
                (-ring, -ring),
                (-ring, ring),
                (ring, -ring),
                (ring, ring),
                (-ring / 2, -ring),
                (ring / 2, -ring),
                (-ring / 2, ring),
                (ring / 2, ring),
                (-ring, -ring / 2),
                (-ring, ring / 2),
                (ring, -ring / 2),
                (ring, ring / 2),
            ];
            for (dx, dy) in offsets {
                draw_text_mut(canvas, stroke_color, x + dx, y + dy, scale, font, text);
            }
        }
    }
    draw_text_mut(canvas, fill, x, y, scale, font, text);
}

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

fn run_title_head(
    input: &Path,
    output: &Path,
    overlay: &Path,
    duration: f64,
    prefer_x264: bool,
) -> Result<()> {
    let filter = "[0:v][1:v]overlay=0:0,format=yuv420p[outv]";

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-hide_banner")
        .arg("-y")
        .args(["-t", &format_seconds(duration)])
        .arg("-i")
        .arg(input)
        .arg("-i")
        .arg(overlay)
        .args(["-filter_complex", filter, "-map", "[outv]", "-map", "0:a?"]);

    if prefer_x264 {
        cmd.args([
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
        ]);
    }

    let command_output = cmd
        .args(["-c:a", "copy", "-movflags", "+faststart"])
        .arg(output)
        .output()
        .context("Failed to run ffmpeg title head")?;

    if command_output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&command_output.stderr);
    if stderr.contains("Unknown encoder 'libx264'") && prefer_x264 {
        return run_title_head(input, output, overlay, duration, false);
    }

    anyhow::bail!(
        "ffmpeg title head failed:\n{}",
        last_lines(&command_output.stderr, 20)
    );
}

fn concat_with_inpoint(
    head: &Path,
    original: &Path,
    inpoint: f64,
    list: &Path,
    output: &Path,
) -> Result<()> {
    let content = format!(
        "file '{}'\nfile '{}'\ninpoint {}\n",
        concat_path(head),
        concat_path(original),
        format_seconds(inpoint)
    );
    std::fs::write(list, content).with_context(|| format!("Failed to write {}", list.display()))?;

    let command_output = Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-y")
        .args(["-f", "concat", "-safe", "0"])
        .arg("-i")
        .arg(list)
        .args(["-c", "copy", "-movflags", "+faststart"])
        .arg(output)
        .output()
        .context("Failed to concatenate title segments")?;

    if command_output.status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "ffmpeg concat failed:\n{}",
            last_lines(&command_output.stderr, 20)
        );
    }
}

fn concat_path(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "'\\''")
}

fn probe_dimensions(video: &Path) -> Result<(u32, u32)> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0:s=x",
        ])
        .arg(video)
        .output()
        .context("Failed to run ffprobe")?;

    if !output.status.success() {
        anyhow::bail!("ffprobe failed:\n{}", last_lines(&output.stderr, 20));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let (w, h) = stdout
        .trim()
        .split_once('x')
        .ok_or_else(|| anyhow::anyhow!("Failed to parse video dimensions: {}", stdout.trim()))?;
    Ok((w.parse()?, h.parse()?))
}

fn probe_duration(video: &Path) -> Result<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(video)
        .output()
        .context("Failed to run ffprobe")?;

    if !output.status.success() {
        anyhow::bail!("ffprobe failed:\n{}", last_lines(&output.stderr, 20));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().parse::<f64>().with_context(|| {
        format!(
            "Failed to parse video duration from ffprobe: {}",
            stdout.trim()
        )
    })
}

fn ensure_ffmpeg() -> Result<()> {
    if crate::which_exists("ffmpeg") {
        Ok(())
    } else {
        anyhow::bail!("ffmpeg not found. Install it with: brew install ffmpeg");
    }
}

fn ensure_ffprobe() -> Result<()> {
    if crate::which_exists("ffprobe") {
        Ok(())
    } else {
        anyhow::bail!("ffprobe not found. Install it with: brew install ffmpeg");
    }
}

fn last_lines(bytes: &[u8], count: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .skip(lines.len().saturating_sub(count))
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_seconds(value: f64) -> String {
    format!("{:.3}", value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_title_lines_from_escaped_newline() {
        assert_eq!(
            title_paragraphs("如何下载\\n搬小书"),
            vec!["如何下载".to_string(), "搬小书".to_string()]
        );
    }

    #[test]
    fn default_font_size_stays_in_reasonable_range() {
        let lines = title_paragraphs("如何下载\\n搬小书");
        let size = default_font_size(1598, 3014, &lines);
        assert!((72..=360).contains(&size));
    }

    #[test]
    fn wraps_title_lines_at_spaces() {
        let font = load_font(None).unwrap();
        let scale = PxScale::from(120.0);
        let paragraphs = title_paragraphs("如何 下载 搬小书");
        let max_width = measure_text_width(&font, scale, "如何 下载") + 1.0;

        assert_eq!(
            wrap_title_lines(&paragraphs, &font, 120, max_width),
            vec!["如何 下载".to_string(), "搬小书".to_string()]
        );
    }

    #[test]
    fn wraps_unspaced_title_by_character_width() {
        let font = load_font(None).unwrap();
        let scale = PxScale::from(120.0);
        let paragraphs = title_paragraphs("如何下载搬小书");
        let max_width = measure_text_width(&font, scale, "如何下载") + 1.0;

        assert_eq!(
            wrap_title_lines(&paragraphs, &font, 120, max_width),
            vec!["如何下载".to_string(), "搬小书".to_string()]
        );
    }
}
