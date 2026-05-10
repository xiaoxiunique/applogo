use std::fs;
use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use base64::Engine;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

const DEFAULT_API_KEY_ENV: &str = "VOLCENGINE_API_KEY";
const DEFAULT_BASE_URL: &str = "https://openspeech.bytedance.com/api/v3/tts/unidirectional";
const DEFAULT_VOLCENGINE_URL: &str = "https://openspeech.bytedance.com/api/v3/tts/unidirectional";
const DEFAULT_RESOURCE_ID: &str = "seed-tts-2.0";
const DEFAULT_VOLCENGINE_RESOURCE_ID: &str = "volc.service_type.10029";
const DEFAULT_VOLCENGINE_APP_KEY: &str = "aGjiRDfUWi";
const DEFAULT_MODEL: &str = "seed-tts-2.0-standard";
const DEFAULT_VOICE: &str = "zh_female_shuangkuaisisi_uranus_bigtts";
const DEFAULT_FORMAT: &str = "mp3";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum TtsProvider {
    Gateway,
    Volcengine,
}

#[derive(Debug, Clone)]
pub struct TtsOptions {
    pub text: Option<String>,
    pub file: Option<PathBuf>,
    pub output: PathBuf,
    pub voice: String,
    pub model: String,
    pub format: String,
    pub speed: f32,
    pub provider: TtsProvider,
    pub base_url: String,
    pub volcengine_url: String,
    pub api_key_env: String,
    pub app_id: Option<String>,
    pub app_key: String,
    pub access_key: Option<String>,
    pub resource_id: String,
    pub play: bool,
}

#[derive(Debug, Clone)]
pub struct TtsConfigOptions {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub provider: Option<TtsProvider>,
    pub app_id: Option<String>,
    pub app_key: Option<String>,
    pub access_key: Option<String>,
    pub resource_id: Option<String>,
    pub voice: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct TtsConfig {
    api_key: Option<String>,
    base_url: Option<String>,
    provider: Option<TtsProvider>,
    app_id: Option<String>,
    app_key: Option<String>,
    access_key: Option<String>,
    resource_id: Option<String>,
    voice: Option<String>,
    model: Option<String>,
}

#[derive(Debug, Serialize)]
struct VolcengineSpeechRequest<'a> {
    user: VolcengineUser<'a>,
    req_params: VolcengineRequestParams<'a>,
}

#[derive(Debug, Serialize)]
struct VolcengineUser<'a> {
    uid: &'a str,
}

#[derive(Debug, Serialize)]
struct VolcengineRequestParams<'a> {
    text: &'a str,
    speaker: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<&'a str>,
    additions: String,
    audio_params: VolcengineAudioParams<'a>,
}

#[derive(Debug, Serialize)]
struct VolcengineAudioParams<'a> {
    format: &'a str,
    speed_ratio: f32,
    sample_rate: u32,
}

#[derive(Debug, Deserialize)]
struct VolcengineResponseLine {
    code: Option<i64>,
    message: Option<String>,
    header: Option<VolcengineResponseHeader>,
    data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VolcengineResponseHeader {
    code: Option<i64>,
    message: Option<String>,
}

pub fn default_output() -> PathBuf {
    PathBuf::from(format!("speech.{DEFAULT_FORMAT}"))
}

pub fn default_voice() -> String {
    DEFAULT_VOICE.to_string()
}

pub fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}

pub fn default_format() -> String {
    DEFAULT_FORMAT.to_string()
}

pub fn default_base_url() -> String {
    DEFAULT_BASE_URL.to_string()
}

pub fn default_volcengine_url() -> String {
    DEFAULT_VOLCENGINE_URL.to_string()
}

pub fn default_api_key_env() -> String {
    DEFAULT_API_KEY_ENV.to_string()
}

pub fn default_resource_id() -> String {
    DEFAULT_RESOURCE_ID.to_string()
}

fn default_volcengine_resource_id() -> String {
    DEFAULT_VOLCENGINE_RESOURCE_ID.to_string()
}

pub fn default_app_key() -> String {
    DEFAULT_VOLCENGINE_APP_KEY.to_string()
}

pub fn config_path() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        let dir = PathBuf::from(dir);
        if !dir.as_os_str().is_empty() {
            return Ok(dir.join("launch").join("tts.json"));
        }
    }

    let home = std::env::var("HOME").context("HOME is not set; cannot locate launch config")?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("launch")
        .join("tts.json"))
}

pub fn run(options: TtsOptions) -> Result<()> {
    validate_options(&options)?;
    let input = read_input(options.text.as_deref(), options.file.as_deref())?;
    if input.trim().is_empty() {
        anyhow::bail!("TTS input is empty");
    }

    let config = load_config()?;
    let base_url = if options.base_url == DEFAULT_BASE_URL {
        config
            .base_url
            .clone()
            .unwrap_or_else(|| options.base_url.clone())
    } else {
        options.base_url.clone()
    };
    let model = if options.model == DEFAULT_MODEL {
        config
            .model
            .clone()
            .unwrap_or_else(|| options.model.clone())
    } else {
        options.model.clone()
    };
    let voice = if options.voice == DEFAULT_VOICE {
        config
            .voice
            .clone()
            .unwrap_or_else(|| options.voice.clone())
    } else {
        options.voice.clone()
    };
    let provider = if options.provider == TtsProvider::Gateway {
        config.provider.unwrap_or(options.provider)
    } else {
        options.provider
    };

    if let Some(parent) = options.output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
    }

    if !crate::which_exists("curl") {
        anyhow::bail!("curl not found. Install curl or add it to PATH.");
    }

    match provider {
        TtsProvider::Gateway => run_gateway(
            &input,
            &options,
            GatewayResolved {
                api_key: resolve_gateway_api_key(&options.api_key_env, &config)?,
                base_url,
                model,
                voice,
                resource_id: resolve_resource_id(
                    &options.resource_id,
                    config.resource_id.as_deref(),
                    DEFAULT_RESOURCE_ID,
                ),
            },
        ),
        TtsProvider::Volcengine => run_volcengine(
            &input,
            &options,
            VolcengineResolved {
                app_id: resolve_field(
                    "Volcengine App ID",
                    "launch tts config --provider volcengine --app-id <id>",
                    options.app_id.as_deref(),
                    config.app_id.as_deref(),
                )?,
                access_key: resolve_field(
                    "Volcengine Access Key",
                    "launch tts config --provider volcengine --access-key <key>",
                    options.access_key.as_deref(),
                    config.access_key.as_deref(),
                )?,
                resource_id: if options.resource_id == DEFAULT_VOLCENGINE_RESOURCE_ID {
                    config
                        .resource_id
                        .clone()
                        .unwrap_or_else(|| options.resource_id.clone())
                } else if options.resource_id == DEFAULT_RESOURCE_ID {
                    config
                        .resource_id
                        .clone()
                        .unwrap_or_else(default_volcengine_resource_id)
                } else {
                    options.resource_id.clone()
                },
                app_key: if options.app_key == DEFAULT_VOLCENGINE_APP_KEY {
                    config
                        .app_key
                        .clone()
                        .unwrap_or_else(|| options.app_key.clone())
                } else {
                    options.app_key.clone()
                },
                endpoint: options.volcengine_url.clone(),
                voice,
            },
        ),
    }
}

struct GatewayResolved {
    api_key: String,
    base_url: String,
    model: String,
    voice: String,
    resource_id: String,
}

struct VolcengineResolved {
    app_id: String,
    app_key: String,
    access_key: String,
    resource_id: String,
    endpoint: String,
    voice: String,
}

fn run_gateway(input: &str, options: &TtsOptions, resolved: GatewayResolved) -> Result<()> {
    let request = VolcengineSpeechRequest {
        user: VolcengineUser { uid: "launch" },
        req_params: VolcengineRequestParams {
            text: input.trim(),
            speaker: &resolved.voice,
            model: Some(&resolved.model),
            additions: volcengine_additions()?,
            audio_params: VolcengineAudioParams {
                format: &options.format,
                speed_ratio: options.speed,
                sample_rate: 24_000,
            },
        },
    };
    let temp_dir = create_temp_dir()?;
    let request_path = temp_dir.join("request.json");
    let curl_config_path = temp_dir.join("curl.conf");
    let response_path = temp_dir.join("response.ndjson");
    let body = serde_json::to_string(&request).context("Failed to serialize TTS request")?;
    fs::write(&request_path, body)
        .with_context(|| format!("Failed to write {}", request_path.display()))?;
    write_private_file(
        &curl_config_path,
        &format!(
            "header = \"Content-Type: application/json\"\n\
             header = \"X-Api-Key: {}\"\n\
             header = \"X-Api-Resource-Id: {}\"\n\
             header = \"X-Api-Request-Id: {}\"\n",
            escape_curl_config_value(&resolved.api_key),
            escape_curl_config_value(&resolved.resource_id),
            request_id()
        ),
    )?;

    eprintln!(
        "Generating speech with {} / {}...",
        resolved.resource_id, resolved.voice
    );

    let output = base_curl_command(&curl_config_path, &request_path, &response_path)
        .arg(&resolved.base_url)
        .output()
        .with_context(|| format!("Failed to call TTS endpoint: {}", resolved.base_url))?;

    let (status_code, _content_type, bytes) = read_curl_response(&output, &response_path);
    let response_text = String::from_utf8_lossy(&bytes);

    if !output.status.success() || !(200..300).contains(&status_code) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if response_text.trim().is_empty() {
            stderr.trim()
        } else {
            response_text.trim()
        };
        let _ = fs::remove_dir_all(&temp_dir);
        anyhow::bail!(
            "TTS request failed with HTTP {}.\n{}",
            status_code,
            truncate_error_body(detail)
        );
    }

    let audio = parse_volcengine_audio(&response_text)?;
    write_audio_output(options, &audio)?;
    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

fn run_volcengine(input: &str, options: &TtsOptions, resolved: VolcengineResolved) -> Result<()> {
    let request = VolcengineSpeechRequest {
        user: VolcengineUser { uid: "launch" },
        req_params: VolcengineRequestParams {
            text: input.trim(),
            speaker: &resolved.voice,
            model: None,
            additions: volcengine_additions()?,
            audio_params: VolcengineAudioParams {
                format: &options.format,
                speed_ratio: options.speed,
                sample_rate: 24_000,
            },
        },
    };
    let temp_dir = create_temp_dir()?;
    let request_path = temp_dir.join("request.json");
    let curl_config_path = temp_dir.join("curl.conf");
    let response_path = temp_dir.join("response.ndjson");
    let body =
        serde_json::to_string(&request).context("Failed to serialize Volcengine TTS request")?;
    fs::write(&request_path, body)
        .with_context(|| format!("Failed to write {}", request_path.display()))?;
    write_private_file(
        &curl_config_path,
        &format!(
            "header = \"Content-Type: application/json\"\n\
             header = \"Accept: text/event-stream\"\n\
             header = \"Connection: keep-alive\"\n\
             header = \"X-Api-App-Id: {}\"\n\
             header = \"X-Api-App-Key: {}\"\n\
             header = \"X-Api-Access-Key: {}\"\n\
             header = \"X-Api-Resource-Id: {}\"\n\
             header = \"X-Api-Request-Id: {}\"\n",
            escape_curl_config_value(&resolved.app_id),
            escape_curl_config_value(&resolved.app_key),
            escape_curl_config_value(&resolved.access_key),
            escape_curl_config_value(&resolved.resource_id),
            request_id()
        ),
    )?;

    eprintln!(
        "Generating speech with Volcengine {} / {}...",
        resolved.resource_id, resolved.voice
    );

    let output = base_curl_command(&curl_config_path, &request_path, &response_path)
        .arg(&resolved.endpoint)
        .output()
        .with_context(|| format!("Failed to call TTS endpoint: {}", resolved.endpoint))?;

    let (status_code, _content_type, bytes) = read_curl_response(&output, &response_path);
    let response_text = String::from_utf8_lossy(&bytes);

    if !output.status.success() || !(200..300).contains(&status_code) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if response_text.trim().is_empty() {
            stderr.trim()
        } else {
            response_text.trim()
        };
        let _ = fs::remove_dir_all(&temp_dir);
        anyhow::bail!(
            "Volcengine TTS request failed with HTTP {}.\n{}",
            status_code,
            truncate_error_body(detail)
        );
    }

    let audio = parse_volcengine_audio(&response_text)?;
    write_audio_output(options, &audio)?;
    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

fn base_curl_command(config: &Path, request: &Path, response: &Path) -> std::process::Command {
    let mut command = Command::new("curl");
    command
        .arg("--config")
        .arg(config)
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg("120")
        .arg("-X")
        .arg("POST")
        .arg("--data")
        .arg(format!("@{}", request.display()))
        .arg("--output")
        .arg(response)
        .arg("--write-out")
        .arg("%{http_code} %{content_type}");
    command
}

fn read_curl_response(
    output: &std::process::Output,
    response_path: &Path,
) -> (u16, String, Vec<u8>) {
    let curl_meta = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let status_code = curl_meta
        .split_whitespace()
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);
    let content_type = curl_meta
        .split_once(' ')
        .map(|(_, value)| value)
        .unwrap_or("")
        .to_string();
    let bytes = fs::read(response_path).unwrap_or_default();
    (status_code, content_type, bytes)
}

fn parse_volcengine_audio(response: &str) -> Result<Vec<u8>> {
    let mut audio = Vec::new();
    let mut last_error = None;

    for line in response
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let parsed: VolcengineResponseLine = serde_json::from_str(line)
            .with_context(|| "Failed to parse Volcengine TTS response")?;
        if let Some(code) = parsed.code {
            if code != 0 && code != 20000000 {
                last_error = Some(format!(
                    "{}: {}",
                    code,
                    parsed
                        .message
                        .clone()
                        .unwrap_or_else(|| "unknown error".to_string())
                ));
            }
        }
        if let Some(header) = parsed.header {
            if let Some(code) = header.code {
                if code != 0 && code != 20000000 {
                    last_error = Some(format!(
                        "{}: {}",
                        code,
                        header
                            .message
                            .unwrap_or_else(|| "unknown error".to_string())
                    ));
                }
            }
        }
        if let Some(data) = parsed.data {
            let chunk = base64::engine::general_purpose::STANDARD
                .decode(data.trim())
                .context("Failed to decode Volcengine audio data")?;
            audio.extend(chunk);
        }
    }

    if audio.is_empty() {
        if let Some(error) = last_error {
            anyhow::bail!("Volcengine TTS returned no audio: {error}");
        }
        anyhow::bail!("Volcengine TTS returned no audio data");
    }

    Ok(audio)
}

fn volcengine_additions() -> Result<String> {
    let additions = serde_json::json!({
        "disable_markdown_filter": true,
        "enable_language_detector": true,
        "enable_latex_tn": true,
        "disable_default_bit_rate": true,
        "max_length_to_filter_parenthesis": 0,
        "cache_config": {
            "text_type": 1,
            "use_cache": true
        }
    });
    serde_json::to_string(&additions).context("Failed to serialize Volcengine additions")
}

fn write_audio_output(options: &TtsOptions, bytes: &[u8]) -> Result<()> {
    fs::write(&options.output, bytes)
        .with_context(|| format!("Failed to write {}", options.output.display()))?;
    eprintln!("Speech saved to {}", options.output.display());

    if options.play {
        play_audio(&options.output)?;
    }
    Ok(())
}

pub fn configure(options: TtsConfigOptions) -> Result<()> {
    let mut config = load_config()?;

    if let Some(api_key) = options.api_key {
        config.api_key = Some(api_key);
    }
    if let Some(base_url) = options.base_url {
        config.base_url = Some(base_url);
    }
    if let Some(provider) = options.provider {
        config.provider = Some(provider);
    }
    if let Some(app_id) = options.app_id {
        config.app_id = Some(app_id);
    }
    if let Some(app_key) = options.app_key {
        config.app_key = Some(app_key);
    }
    if let Some(access_key) = options.access_key {
        config.access_key = Some(access_key);
    }
    if let Some(resource_id) = options.resource_id {
        config.resource_id = Some(resource_id);
    }
    if let Some(voice) = options.voice {
        config.voice = Some(voice);
    }
    if let Some(model) = options.model {
        config.model = Some(model);
    }

    if matches!(config.provider, Some(TtsProvider::Volcengine)) {
        if config.app_id.is_none() || config.access_key.is_none() {
            anyhow::bail!(
                "Volcengine provider requires --app-id and --access-key.\n\
                 Example: launch tts config --provider volcengine --app-id <id> --access-key <key>"
            );
        }
    } else if config.api_key.is_none() {
        config.api_key = Some(prompt_api_key()?);
    }

    save_config(&config)?;
    eprintln!("TTS config saved to {}", config_path()?.display());
    Ok(())
}

fn validate_options(options: &TtsOptions) -> Result<()> {
    if options.text.is_some() && options.file.is_some() {
        anyhow::bail!("Provide either positional text or --file, not both");
    }
    if options.text.is_none() && options.file.is_none() {
        anyhow::bail!(
            "Provide text or --file.\n\
             Example: launch tts \"欢迎使用 Launch\" -o speech.mp3"
        );
    }
    if !(0.25..=4.0).contains(&options.speed) {
        anyhow::bail!("--speed must be between 0.25 and 4.0");
    }
    if options.format.trim().is_empty() {
        anyhow::bail!("--format cannot be empty");
    }
    if options.base_url.trim().is_empty() {
        anyhow::bail!("--base-url cannot be empty");
    }
    if options.volcengine_url.trim().is_empty() {
        anyhow::bail!("--volcengine-url cannot be empty");
    }
    Ok(())
}

fn read_input(text: Option<&str>, file: Option<&Path>) -> Result<String> {
    if let Some(text) = text {
        return Ok(text.to_string());
    }
    let file = file.expect("validated by caller");
    fs::read_to_string(file).with_context(|| format!("Failed to read {}", file.display()))
}

fn resolve_gateway_api_key(api_key_env: &str, config: &TtsConfig) -> Result<String> {
    if let Ok(value) = std::env::var(api_key_env) {
        let value = value.trim().to_string();
        if !value.is_empty() {
            return Ok(value);
        }
    }

    if let Some(value) = config.api_key.as_deref() {
        let value = value.trim();
        if !value.is_empty() {
            return Ok(value.to_string());
        }
    }

    eprintln!("TTS API key is not configured.");
    eprintln!("Set it with: export {api_key_env}=your_key");
    eprintln!("Or save it with: launch tts config --api-key your_key");

    prompt_api_key()
}

fn resolve_resource_id(cli_value: &str, config_value: Option<&str>, default_value: &str) -> String {
    if cli_value != default_value {
        return cli_value.to_string();
    }
    config_value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| default_value.to_string())
}

fn resolve_field(
    label: &str,
    command_hint: &str,
    cli_value: Option<&str>,
    config_value: Option<&str>,
) -> Result<String> {
    if let Some(value) = cli_value {
        let value = value.trim();
        if !value.is_empty() {
            return Ok(value.to_string());
        }
    }
    if let Some(value) = config_value {
        let value = value.trim();
        if !value.is_empty() {
            return Ok(value.to_string());
        }
    }
    anyhow::bail!("{label} is not configured.\nSet it with: {command_hint}")
}

fn prompt_api_key() -> Result<String> {
    if !io::stdin().is_terminal() {
        anyhow::bail!(
            "No TTS API key configured and stdin is not interactive.\n\
             Set VOLCENGINE_API_KEY or run: launch tts config --api-key <key>"
        );
    }

    eprint!("Enter Volcengine API key: ");
    io::stderr().flush().ok();

    let mut api_key = String::new();
    io::stdin()
        .read_line(&mut api_key)
        .context("Failed to read API key")?;
    let api_key = api_key.trim().to_string();
    if api_key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }
    Ok(api_key)
}

fn load_config() -> Result<TtsConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(TtsConfig::default());
    }
    let text =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("Failed to parse {}", path.display()))
}

fn save_config(config: &TtsConfig) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(config).context("Failed to serialize TTS config")?;
    fs::write(&path, format!("{text}\n"))
        .with_context(|| format!("Failed to write {}", path.display()))
}

fn create_temp_dir() -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir().join(format!("launch-tts-{}", request_id()));
    fs::create_dir_all(&temp_dir)
        .with_context(|| format!("Failed to create {}", temp_dir.display()))?;
    restrict_dir_permissions(&temp_dir)?;
    Ok(temp_dir)
}

fn write_private_file(path: &Path, text: &str) -> Result<()> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("Failed to write {}", path.display()))
}

fn restrict_dir_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
    }
    Ok(())
}

fn truncate_error_body(body: &str) -> String {
    const LIMIT: usize = 1200;
    let trimmed = body.trim();
    if trimmed.chars().count() <= LIMIT {
        return trimmed.to_string();
    }
    let mut result: String = trimmed.chars().take(LIMIT).collect();
    result.push_str("...");
    result
}

fn escape_curl_config_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn request_id() -> String {
    format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

fn play_audio(path: &Path) -> Result<()> {
    if !crate::which_exists("afplay") {
        anyhow::bail!("--play requires macOS afplay in PATH");
    }
    let status = Command::new("afplay")
        .arg(path)
        .status()
        .context("Failed to run afplay")?;
    if !status.success() {
        anyhow::bail!("afplay failed for {}", path.display());
    }
    Ok(())
}
