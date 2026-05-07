use serde::Serialize;

use crate::config::{self, IconEntry, Platform};

#[derive(Serialize)]
struct ContentsJson {
    images: Vec<ContentsImage>,
}

#[derive(Serialize)]
struct ContentsImage {
    size: String,
    #[serde(rename = "expected-size")]
    expected_size: String,
    filename: String,
    idiom: String,
    scale: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtype: Option<String>,
}

impl From<&IconEntry> for ContentsImage {
    fn from(e: &IconEntry) -> Self {
        ContentsImage {
            size: e.size.to_string(),
            expected_size: e.expected_size.to_string(),
            filename: e.filename.clone(),
            idiom: e.idiom.to_string(),
            scale: e.scale.to_string(),
            role: e.role.map(|s| s.to_string()),
            subtype: e.subtype.map(|s| s.to_string()),
        }
    }
}

/// Generate Contents.json for Xcode asset catalog.
/// Only includes Apple platforms (iPhone, iPad, watchOS, macOS).
pub fn generate(platforms: &[Platform], android_filename: &str) -> String {
    let mut images: Vec<ContentsImage> = Vec::new();

    for &platform in platforms {
        if !config::is_apple_platform(platform) {
            continue;
        }
        let entries = config::get_entries(platform, android_filename);
        for entry in &entries {
            images.push(ContentsImage::from(entry));
        }
    }

    let contents = ContentsJson { images };
    serde_json::to_string_pretty(&contents).expect("Failed to serialize Contents.json")
}
