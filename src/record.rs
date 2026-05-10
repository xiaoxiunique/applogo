use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use image::GenericImageView;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct RecordOptions {
    pub output_dir: PathBuf,
    pub name: Option<String>,
    pub frame: bool,
    pub frame_device: String,
    pub frame_orientation: String,
    pub frame_padding: u32,
    pub auto_trim: bool,
    pub freeze_min: f64,
    pub freeze_noise: f64,
    pub keep_still: f64,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct FrameVideoOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub device: String,
    pub orientation: String,
    pub padding: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
struct TimeRange {
    start: f64,
    end: f64,
}

#[derive(Debug, Serialize)]
struct MarkerRecord {
    index: usize,
    seconds: f64,
}

#[derive(Debug, Serialize)]
struct MarkerFile {
    raw_video: String,
    duration_seconds: Option<f64>,
    markers: Vec<MarkerRecord>,
}

#[derive(Debug, Serialize)]
struct CutSettings {
    freeze_min_seconds: f64,
    freeze_noise: f64,
    keep_still_seconds: f64,
    dry_run: bool,
}

#[derive(Debug, Serialize)]
struct ClipReport {
    index: usize,
    output: String,
    source: TimeRange,
    removed: Vec<TimeRange>,
    kept: Vec<TimeRange>,
}

#[derive(Debug, Serialize)]
struct CutReport {
    raw_video: String,
    settings: CutSettings,
    detected_freezes: Vec<TimeRange>,
    clips: Vec<ClipReport>,
}

enum InputEvent {
    Marker,
    Quit,
}

struct TerminalModeGuard {
    original_state: Option<String>,
}

impl TerminalModeGuard {
    fn enter_raw_mode() -> Self {
        if !std::io::stdin().is_terminal() {
            return Self {
                original_state: None,
            };
        }

        let state = Command::new("stty")
            .arg("-g")
            .stdin(Stdio::inherit())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        if state.is_none() {
            return Self {
                original_state: None,
            };
        }

        let raw_enabled = Command::new("stty")
            .args(["raw", "-echo"])
            .stdin(Stdio::inherit())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if raw_enabled {
            Self {
                original_state: state,
            }
        } else {
            Self {
                original_state: None,
            }
        }
    }

    fn is_raw(&self) -> bool {
        self.original_state.is_some()
    }
}

impl Drop for TerminalModeGuard {
    fn drop(&mut self) {
        if let Some(state) = &self.original_state {
            let _ = Command::new("stty")
                .arg(state)
                .stdin(Stdio::inherit())
                .status();
        }
    }
}

pub fn run(options: RecordOptions) -> Result<()> {
    if !crate::which_exists("xcrun") {
        anyhow::bail!("xcrun not found. Install Xcode or Xcode Command Line Tools.");
    }

    let session_dir = prepare_session_dir(&options)?;
    std::fs::create_dir_all(&session_dir)
        .with_context(|| format!("Failed to create {}", session_dir.display()))?;

    let raw_video = session_dir.join("raw.mp4");
    let markers_path = session_dir.join("markers.json");
    let cuts_path = session_dir.join("cuts.json");

    eprintln!("Recording iOS Simulator to {}", raw_video.display());
    eprintln!("Keys: m = add marker, q = stop, Ctrl+C = stop\n");

    let mut child = Command::new("xcrun")
        .args(["simctl", "io", "booted", "recordVideo"])
        .arg(&raw_video)
        .spawn()
        .context("Failed to start simctl recordVideo. Is a simulator booted?")?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("Failed to set Ctrl+C handler")?;

    let terminal = TerminalModeGuard::enter_raw_mode();
    let events = spawn_input_reader(terminal.is_raw());

    let started_at = Instant::now();
    let mut markers = Vec::new();
    let mut stopped_by_user = false;

    loop {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                anyhow::bail!(
                    "simctl recordVideo exited before recording completed. Make sure a simulator is running."
                );
            }
            break;
        }

        if !running.load(Ordering::SeqCst) {
            stopped_by_user = true;
            break;
        }

        match events.recv_timeout(Duration::from_millis(100)) {
            Ok(InputEvent::Marker) => {
                let seconds = started_at.elapsed().as_secs_f64();
                if markers
                    .last()
                    .map(|previous| seconds - previous >= 0.5)
                    .unwrap_or(true)
                {
                    markers.push(seconds);
                    eprintln!("\rMarker #{} at {:.2}s", markers.len(), seconds);
                }
            }
            Ok(InputEvent::Quit) => {
                stopped_by_user = true;
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
        }
    }

    if stopped_by_user {
        stop_recording(&mut child)?;
    }

    drop(terminal);

    if !raw_video.exists() || raw_video.metadata()?.len() == 0 {
        anyhow::bail!("Recording did not produce a video file.");
    }

    let duration = probe_duration_optional(&raw_video);
    write_markers(&markers_path, &raw_video, duration, &markers)?;

    eprintln!("\nRaw video saved to {}", raw_video.display());
    eprintln!("Markers saved to {}", markers_path.display());

    let mut clip_paths = if options.auto_trim {
        process_auto_trim(&raw_video, &session_dir, &cuts_path, &markers, &options)?
    } else if !markers.is_empty() {
        split_by_markers(&raw_video, &session_dir.join("clips"), &markers)?
    } else {
        eprintln!("No markers recorded, so no clips were generated.");
        Vec::new()
    };

    clip_paths.sort();

    if options.frame && options.dry_run {
        eprintln!("Dry run enabled; no framed videos were exported.");
    } else if options.frame && clip_paths.is_empty() {
        frame_video(
            &raw_video,
            &session_dir.join("framed.mp4"),
            &session_dir,
            &options,
        )?;
    } else if options.frame {
        eprintln!("Applying device frame to {} clip(s)...", clip_paths.len());
        for (i, clip) in clip_paths.iter().enumerate() {
            let framed = framed_output_path(clip);
            eprintln!("  [{}/{}] {}", i + 1, clip_paths.len(), framed.display());
            frame_video(clip, &framed, &session_dir, &options)?;
        }
    }

    eprintln!("\nDone: {}", session_dir.display());
    Ok(())
}

pub fn frame_existing_video(options: FrameVideoOptions) -> Result<()> {
    let temp_dir = std::env::temp_dir().join(format!(
        "launch-frame-video-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir)
        .with_context(|| format!("Failed to create {}", temp_dir.display()))?;

    let result = frame_video_with_device(
        &options.input,
        &options.output,
        &temp_dir,
        &options.device,
        &options.orientation,
        options.padding,
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
    result
}

fn prepare_session_dir(options: &RecordOptions) -> Result<PathBuf> {
    let name = match &options.name {
        Some(name) => sanitize_name(name),
        None => {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            format!("recording-{}", ts)
        }
    };

    Ok(options.output_dir.join(name))
}

fn sanitize_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();

    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "recording".to_string()
    } else {
        trimmed.to_string()
    }
}

fn spawn_input_reader(raw_mode: bool) -> Receiver<InputEvent> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        if raw_mode {
            let mut stdin = stdin.lock();
            let mut buf = [0u8; 1];
            while stdin.read_exact(&mut buf).is_ok() {
                match buf[0] as char {
                    'm' | 'M' => {
                        let _ = tx.send(InputEvent::Marker);
                    }
                    'q' | 'Q' | '\u{3}' => {
                        let _ = tx.send(InputEvent::Quit);
                        break;
                    }
                    _ => {}
                }
            }
        } else {
            loop {
                let mut line = String::new();
                if std::io::stdin().read_line(&mut line).is_err() {
                    break;
                }
                for c in line.chars() {
                    match c {
                        'm' | 'M' => {
                            let _ = tx.send(InputEvent::Marker);
                        }
                        'q' | 'Q' => {
                            let _ = tx.send(InputEvent::Quit);
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
    });
    rx
}

fn stop_recording(child: &mut Child) -> Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }

    let pid = child.id().to_string();
    let _ = Command::new("kill").args(["-INT", &pid]).status();

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    child.kill().context("Failed to stop simctl recordVideo")?;
    let _ = child.wait();
    Ok(())
}

fn write_markers(
    path: &Path,
    raw_video: &Path,
    duration: Option<f64>,
    markers: &[f64],
) -> Result<()> {
    let marker_file = MarkerFile {
        raw_video: raw_video.display().to_string(),
        duration_seconds: duration,
        markers: markers
            .iter()
            .enumerate()
            .map(|(i, seconds)| MarkerRecord {
                index: i + 1,
                seconds: round3(*seconds),
            })
            .collect(),
    };
    let data = serde_json::to_vec_pretty(&marker_file)?;
    std::fs::write(path, data).with_context(|| format!("Failed to write {}", path.display()))
}

fn split_by_markers(raw_video: &Path, clips_dir: &Path, markers: &[f64]) -> Result<Vec<PathBuf>> {
    if markers.is_empty() {
        return Ok(Vec::new());
    }
    ensure_ffmpeg()?;
    std::fs::create_dir_all(clips_dir)?;

    let times = normalized_markers(markers, None)
        .into_iter()
        .map(format_seconds)
        .collect::<Vec<_>>()
        .join(",");

    let output_pattern = clips_dir.join("clip-%03d.mp4");
    eprintln!("Splitting video by {} marker(s)...", markers.len());

    let output = Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-y")
        .arg("-i")
        .arg(raw_video)
        .args([
            "-c",
            "copy",
            "-map",
            "0",
            "-f",
            "segment",
            "-segment_times",
            &times,
            "-reset_timestamps",
            "1",
            "-segment_start_number",
            "1",
        ])
        .arg(&output_pattern)
        .output()
        .context("Failed to run ffmpeg")?;

    if !output.status.success() {
        anyhow::bail!("ffmpeg split failed:\n{}", last_lines(&output.stderr, 20));
    }

    eprintln!("Clips saved to {}", clips_dir.display());
    collect_mp4s(clips_dir)
}

fn process_auto_trim(
    raw_video: &Path,
    session_dir: &Path,
    cuts_path: &Path,
    markers: &[f64],
    options: &RecordOptions,
) -> Result<Vec<PathBuf>> {
    ensure_ffmpeg()?;
    ensure_ffprobe()?;

    let duration = probe_duration(raw_video)?;
    let freezes = detect_freezes(raw_video, options, duration)?;
    let source_ranges = clip_ranges(duration, markers);
    let clips_dir = session_dir.join("clips");
    std::fs::create_dir_all(&clips_dir)?;

    eprintln!(
        "Detected {} still interval(s). {} clip(s) planned.",
        freezes.len(),
        source_ranges.len()
    );

    let mut reports = Vec::new();
    for (i, source) in source_ranges.iter().enumerate() {
        let removed = removable_freezes(*source, &freezes, options.keep_still);
        let kept = subtract_ranges(*source, &removed);
        let output = clips_dir.join(format!("clip-{:03}.mp4", i + 1));

        if !options.dry_run {
            eprintln!(
                "  [{}/{}] Exporting {}",
                i + 1,
                source_ranges.len(),
                output.display()
            );
            export_trimmed_clip(raw_video, &output, &kept)?;
        }

        reports.push(ClipReport {
            index: i + 1,
            output: output.display().to_string(),
            source: round_range(*source),
            removed: removed.into_iter().map(round_range).collect(),
            kept: kept.into_iter().map(round_range).collect(),
        });
    }

    let report = CutReport {
        raw_video: raw_video.display().to_string(),
        settings: CutSettings {
            freeze_min_seconds: options.freeze_min,
            freeze_noise: options.freeze_noise,
            keep_still_seconds: options.keep_still,
            dry_run: options.dry_run,
        },
        detected_freezes: freezes.into_iter().map(round_range).collect(),
        clips: reports,
    };

    std::fs::write(cuts_path, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("Failed to write {}", cuts_path.display()))?;
    eprintln!("Cut report saved to {}", cuts_path.display());

    if options.dry_run {
        eprintln!("Dry run enabled; no clips were exported.");
        Ok(Vec::new())
    } else {
        eprintln!("Clips saved to {}", clips_dir.display());
        collect_mp4s(&clips_dir)
    }
}

fn collect_mp4s(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("mp4"))
            .unwrap_or(false)
            && !path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(|stem| stem.ends_with("-framed"))
                .unwrap_or(false)
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn frame_video(
    input: &Path,
    output: &Path,
    session_dir: &Path,
    options: &RecordOptions,
) -> Result<()> {
    frame_video_with_device(
        input,
        output,
        session_dir,
        &options.frame_device,
        &options.frame_orientation,
        options.frame_padding,
    )
}

fn frame_video_with_device(
    input: &Path,
    output: &Path,
    session_dir: &Path,
    device_id: &str,
    orientation_name: &str,
    padding: u32,
) -> Result<()> {
    ensure_ffmpeg()?;
    ensure_ffprobe()?;

    let device = crate::device::find_device(device_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown device: {}", device_id))?;
    let orientation = device.find_orientation(orientation_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Device {} does not support orientation: {}",
            device_id,
            orientation_name
        )
    })?;

    let template_path = session_dir.join(".launch-frame-template.png");
    let mask_path = session_dir.join(".launch-frame-mask.png");
    std::fs::write(&template_path, orientation.template)
        .with_context(|| format!("Failed to write {}", template_path.display()))?;
    std::fs::write(&mask_path, orientation.mask)
        .with_context(|| format!("Failed to write {}", mask_path.display()))?;

    let result = frame_video_with_assets(
        input,
        output,
        &template_path,
        &mask_path,
        device,
        orientation,
        padding,
    );

    let _ = std::fs::remove_file(&template_path);
    let _ = std::fs::remove_file(&mask_path);

    result
}

fn frame_video_with_assets(
    input: &Path,
    output: &Path,
    template_path: &Path,
    mask_path: &Path,
    device: &crate::device::DeviceConfig,
    orientation: &crate::device::OrientationConfig,
    padding: u32,
) -> Result<()> {
    let duration = probe_duration(input)?;
    let template = image::load_from_memory(orientation.template)
        .context("Failed to decode device template")?;
    let (template_w, template_h) = template.dimensions();
    let (screen_w, screen_h) = display_size(device, orientation);
    let (screen_x, screen_y) = screen_origin(orientation);
    let filter = frame_filter(
        template_w, template_h, screen_w, screen_h, screen_x, screen_y, padding,
    );

    let first = run_frame_ffmpeg(
        input,
        output,
        template_path,
        mask_path,
        &filter,
        duration,
        true,
    )?;
    if first.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&first.stderr);
    if stderr.contains("Unknown encoder 'libx264'") {
        let fallback = run_frame_ffmpeg(
            input,
            output,
            template_path,
            mask_path,
            &filter,
            duration,
            false,
        )?;
        if fallback.status.success() {
            return Ok(());
        }
        anyhow::bail!("ffmpeg frame failed:\n{}", last_lines(&fallback.stderr, 20));
    }

    anyhow::bail!("ffmpeg frame failed:\n{}", last_lines(&first.stderr, 20));
}

fn display_size(
    device: &crate::device::DeviceConfig,
    orientation: &crate::device::OrientationConfig,
) -> (u32, u32) {
    let (w, h) = device.display_resolution;
    if orientation.name == "landscape" {
        (h, w)
    } else {
        (w, h)
    }
}

fn screen_origin(orientation: &crate::device::OrientationConfig) -> (u32, u32) {
    let min_x = orientation.screen_coord.iter().map(|c| c.0).min().unwrap();
    let min_y = orientation.screen_coord.iter().map(|c| c.1).min().unwrap();
    (min_x, min_y)
}

fn frame_filter(
    template_w: u32,
    template_h: u32,
    screen_w: u32,
    screen_h: u32,
    screen_x: u32,
    screen_y: u32,
    padding: u32,
) -> String {
    let canvas_w = template_w + padding * 2;
    let canvas_h = template_h + padding * 2;
    let screen_x = screen_x + padding;
    let screen_y = screen_y + padding;
    format!(
        "[0:v]scale=w={screen_w}:h={screen_h}:force_original_aspect_ratio=decrease,\
pad={screen_w}:{screen_h}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,format=rgba,\
pad={canvas_w}:{canvas_h}:{screen_x}:{screen_y}:color=black@0[screen_full];\
[2:v]format=rgba,alphaextract[mask];\
[mask]pad={canvas_w}:{canvas_h}:{padding}:{padding}:color=black[mask_full];\
[screen_full][mask_full]alphamerge[screen_masked];\
color=c=0xf2f2f7:s={canvas_w}x{canvas_h}:r=30,format=rgba[bg];\
[bg][screen_masked]overlay=0:0:shortest=1[with_screen];\
[with_screen][1:v]overlay={padding}:{padding}:shortest=1,format=yuv420p[outv]"
    )
}

fn run_frame_ffmpeg(
    input: &Path,
    output: &Path,
    template_path: &Path,
    mask_path: &Path,
    filter: &str,
    duration: f64,
    prefer_x264: bool,
) -> Result<std::process::Output> {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-hide_banner")
        .arg("-y")
        .arg("-i")
        .arg(input)
        .args(["-loop", "1"])
        .arg("-i")
        .arg(template_path)
        .args(["-loop", "1"])
        .arg("-i")
        .arg(mask_path)
        .args(["-filter_complex", filter, "-map", "[outv]", "-an"]);

    if prefer_x264 {
        cmd.args([
            "-c:v", "libx264", "-preset", "veryfast", "-crf", "18", "-pix_fmt", "yuv420p",
        ]);
    }

    cmd.args(["-movflags", "+faststart", "-t", &format_seconds(duration)])
        .arg(output)
        .output()
        .context("Failed to run ffmpeg frame")
}

fn framed_output_path(input: &Path) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let parent = input.parent().unwrap_or(Path::new("."));
    parent.join(format!("{}-framed.mp4", stem))
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

fn probe_duration_optional(video: &Path) -> Option<f64> {
    if !crate::which_exists("ffprobe") {
        return None;
    }
    probe_duration(video).ok().map(round3)
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

fn detect_freezes(video: &Path, options: &RecordOptions, duration: f64) -> Result<Vec<TimeRange>> {
    let filter = format!(
        "freezedetect=n={}:d={}",
        options.freeze_noise, options.freeze_min
    );
    let output = Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-i")
        .arg(video)
        .args(["-vf", &filter, "-map", "0:v:0", "-f", "null", "-"])
        .output()
        .context("Failed to run ffmpeg freezedetect")?;

    if !output.status.success() {
        anyhow::bail!(
            "ffmpeg freezedetect failed:\n{}",
            last_lines(&output.stderr, 20)
        );
    }

    Ok(parse_freezes(
        &String::from_utf8_lossy(&output.stderr),
        Some(duration),
    ))
}

fn parse_freezes(stderr: &str, duration: Option<f64>) -> Vec<TimeRange> {
    let mut freezes = Vec::new();
    let mut current_start = None;

    for line in stderr.lines() {
        if let Some(start) = parse_metric(line, "freeze_start:") {
            current_start = Some(start);
        }
        if let Some(end) = parse_metric(line, "freeze_end:") {
            if let Some(start) = current_start.take() {
                if end > start {
                    freezes.push(TimeRange { start, end });
                }
            }
        }
    }

    if let (Some(start), Some(end)) = (current_start, duration) {
        if end > start {
            freezes.push(TimeRange { start, end });
        }
    }

    freezes
}

fn parse_metric(line: &str, key: &str) -> Option<f64> {
    let start = line.find(key)? + key.len();
    line[start..].split_whitespace().next()?.parse().ok()
}

fn clip_ranges(duration: f64, markers: &[f64]) -> Vec<TimeRange> {
    let mut boundaries = Vec::with_capacity(markers.len() + 2);
    boundaries.push(0.0);
    boundaries.extend(normalized_markers(markers, Some(duration)));
    boundaries.push(duration);

    boundaries
        .windows(2)
        .filter_map(|pair| {
            let start = pair[0];
            let end = pair[1];
            if end - start > 0.1 {
                Some(TimeRange { start, end })
            } else {
                None
            }
        })
        .collect()
}

fn normalized_markers(markers: &[f64], duration: Option<f64>) -> Vec<f64> {
    let mut normalized: Vec<f64> = markers
        .iter()
        .copied()
        .filter(|marker| *marker > 0.1)
        .filter(|marker| duration.map(|d| *marker < d - 0.1).unwrap_or(true))
        .collect();
    normalized.sort_by(|a, b| a.total_cmp(b));
    normalized.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    normalized
}

fn removable_freezes(source: TimeRange, freezes: &[TimeRange], keep_still: f64) -> Vec<TimeRange> {
    freezes
        .iter()
        .filter_map(|freeze| {
            let intersection = intersect(*freeze, source)?;
            let start = intersection.start + keep_still;
            let end = intersection.end - keep_still;
            if end - start >= 0.1 {
                Some(TimeRange { start, end })
            } else {
                None
            }
        })
        .collect()
}

fn intersect(a: TimeRange, b: TimeRange) -> Option<TimeRange> {
    let start = a.start.max(b.start);
    let end = a.end.min(b.end);
    (end > start).then_some(TimeRange { start, end })
}

fn subtract_ranges(source: TimeRange, removed: &[TimeRange]) -> Vec<TimeRange> {
    let mut kept = Vec::new();
    let mut cursor = source.start;

    let mut removed = removed.to_vec();
    removed.sort_by(|a, b| a.start.total_cmp(&b.start));

    for range in removed {
        let range = match intersect(range, source) {
            Some(range) => range,
            None => continue,
        };
        if range.start > cursor + 0.05 {
            kept.push(TimeRange {
                start: cursor,
                end: range.start,
            });
        }
        cursor = cursor.max(range.end);
    }

    if source.end > cursor + 0.05 {
        kept.push(TimeRange {
            start: cursor,
            end: source.end,
        });
    }

    kept
}

fn export_trimmed_clip(raw_video: &Path, output: &Path, kept: &[TimeRange]) -> Result<()> {
    if kept.is_empty() {
        eprintln!("    Skipping empty clip {}", output.display());
        return Ok(());
    }

    let filter = trim_filter(kept);
    let first = run_trim_ffmpeg(raw_video, output, &filter, true)?;
    if first.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&first.stderr);
    if stderr.contains("Unknown encoder 'libx264'") {
        let fallback = run_trim_ffmpeg(raw_video, output, &filter, false)?;
        if fallback.status.success() {
            return Ok(());
        }
        anyhow::bail!("ffmpeg trim failed:\n{}", last_lines(&fallback.stderr, 20));
    }

    anyhow::bail!("ffmpeg trim failed:\n{}", last_lines(&first.stderr, 20));
}

fn trim_filter(kept: &[TimeRange]) -> String {
    let mut filter = String::new();
    for (i, range) in kept.iter().enumerate() {
        filter.push_str(&format!(
            "[0:v]trim=start={}:end={},setpts=PTS-STARTPTS[v{}];",
            format_seconds(range.start),
            format_seconds(range.end),
            i
        ));
    }

    if kept.len() == 1 {
        filter.push_str("[v0]format=yuv420p[outv]");
    } else {
        for i in 0..kept.len() {
            filter.push_str(&format!("[v{}]", i));
        }
        filter.push_str(&format!(
            "concat=n={}:v=1:a=0,format=yuv420p[outv]",
            kept.len()
        ));
    }

    filter
}

fn run_trim_ffmpeg(
    raw_video: &Path,
    output: &Path,
    filter: &str,
    prefer_x264: bool,
) -> Result<std::process::Output> {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-hide_banner")
        .arg("-y")
        .arg("-i")
        .arg(raw_video)
        .args(["-filter_complex", filter, "-map", "[outv]", "-an"]);

    if prefer_x264 {
        cmd.args([
            "-c:v", "libx264", "-preset", "veryfast", "-crf", "18", "-pix_fmt", "yuv420p",
        ]);
    }

    cmd.args(["-movflags", "+faststart"])
        .arg(output)
        .output()
        .context("Failed to run ffmpeg trim")
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

fn round_range(range: TimeRange) -> TimeRange {
    TimeRange {
        start: round3(range.start),
        end: round3(range.end),
    }
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn format_seconds(value: f64) -> String {
    format!("{:.3}", value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_freezedetect_output() {
        let stderr = "\
[freezedetect @ 0x123] freeze_start: 1.25
[freezedetect @ 0x123] freeze_duration: 2.50
[freezedetect @ 0x123] freeze_end: 3.75
[freezedetect @ 0x123] freeze_start: 8.00";

        assert_eq!(
            parse_freezes(stderr, Some(10.0)),
            vec![
                TimeRange {
                    start: 1.25,
                    end: 3.75
                },
                TimeRange {
                    start: 8.0,
                    end: 10.0
                }
            ]
        );
    }

    #[test]
    fn subtracts_removed_ranges_from_source() {
        let kept = subtract_ranges(
            TimeRange {
                start: 0.0,
                end: 10.0,
            },
            &[
                TimeRange {
                    start: 2.0,
                    end: 4.0,
                },
                TimeRange {
                    start: 6.0,
                    end: 8.0,
                },
            ],
        );

        assert_eq!(
            kept,
            vec![
                TimeRange {
                    start: 0.0,
                    end: 2.0
                },
                TimeRange {
                    start: 4.0,
                    end: 6.0
                },
                TimeRange {
                    start: 8.0,
                    end: 10.0
                }
            ]
        );
    }

    #[test]
    fn keeps_context_around_freezes() {
        let removed = removable_freezes(
            TimeRange {
                start: 0.0,
                end: 12.0,
            },
            &[TimeRange {
                start: 3.0,
                end: 9.0,
            }],
            0.5,
        );

        assert_eq!(
            removed,
            vec![TimeRange {
                start: 3.5,
                end: 8.5
            }]
        );
    }
}
