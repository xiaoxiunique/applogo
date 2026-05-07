use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Platform {
    Iphone,
    Ipad,
    Watch,
    Mac,
    Android,
    All,
}

#[derive(Debug, Clone)]
pub struct IconEntry {
    pub size: &'static str,
    pub expected_size: u32,
    pub filename: String,
    pub folder: &'static str,
    pub idiom: &'static str,
    pub scale: &'static str,
    pub role: Option<&'static str>,
    pub subtype: Option<&'static str>,
}

pub fn iphone_entries() -> Vec<IconEntry> {
    vec![
        ie("60x60", 180, "180.png", APPICONSET, "iphone", "3x"),
        ie("40x40", 80, "80.png", APPICONSET, "iphone", "2x"),
        ie("40x40", 120, "120.png", APPICONSET, "iphone", "3x"),
        ie("60x60", 120, "120.png", APPICONSET, "iphone", "2x"),
        ie("57x57", 57, "57.png", APPICONSET, "iphone", "1x"),
        ie("29x29", 58, "58.png", APPICONSET, "iphone", "2x"),
        ie("29x29", 29, "29.png", APPICONSET, "iphone", "1x"),
        ie("29x29", 87, "87.png", APPICONSET, "iphone", "3x"),
        ie("57x57", 114, "114.png", APPICONSET, "iphone", "2x"),
        ie("20x20", 40, "40.png", APPICONSET, "iphone", "2x"),
        ie("20x20", 60, "60.png", APPICONSET, "iphone", "3x"),
        ie("1024x1024", 1024, "1024.png", APPICONSET, "ios-marketing", "1x"),
    ]
}

pub fn ipad_entries() -> Vec<IconEntry> {
    vec![
        ie("40x40", 80, "80.png", APPICONSET, "ipad", "2x"),
        ie("72x72", 72, "72.png", APPICONSET, "ipad", "1x"),
        ie("76x76", 152, "152.png", APPICONSET, "ipad", "2x"),
        ie("50x50", 100, "100.png", APPICONSET, "ipad", "2x"),
        ie("29x29", 58, "58.png", APPICONSET, "ipad", "2x"),
        ie("76x76", 76, "76.png", APPICONSET, "ipad", "1x"),
        ie("29x29", 29, "29.png", APPICONSET, "ipad", "1x"),
        ie("50x50", 50, "50.png", APPICONSET, "ipad", "1x"),
        ie("72x72", 144, "144.png", APPICONSET, "ipad", "2x"),
        ie("40x40", 40, "40.png", APPICONSET, "ipad", "1x"),
        ie("83.5x83.5", 167, "167.png", APPICONSET, "ipad", "2x"),
        ie("20x20", 20, "20.png", APPICONSET, "ipad", "1x"),
        ie("20x20", 40, "40.png", APPICONSET, "ipad", "2x"),
    ]
}

pub fn watch_entries() -> Vec<IconEntry> {
    vec![
        iew("86x86", 172, "172.png", "watch", "2x", "quickLook", "38mm"),
        iew("40x40", 80, "80.png", "watch", "2x", "appLauncher", "38mm"),
        iew("44x44", 88, "88.png", "watch", "2x", "appLauncher", "40mm"),
        iew("51x51", 102, "102.png", "watch", "2x", "appLauncher", "45mm"),
        iew("54x54", 108, "108.png", "watch", "2x", "appLauncher", "49mm"),
        iew("46x46", 92, "92.png", "watch", "2x", "appLauncher", "41mm"),
        iew("50x50", 100, "100.png", "watch", "2x", "appLauncher", "44mm"),
        iew("98x98", 196, "196.png", "watch", "2x", "quickLook", "42mm"),
        iew("108x108", 216, "216.png", "watch", "2x", "quickLook", "44mm"),
        iew("117x117", 234, "234.png", "watch", "2x", "quickLook", "45mm"),
        iew("129x129", 258, "258.png", "watch", "2x", "quickLook", "49mm"),
        iew("24x24", 48, "48.png", "watch", "2x", "notificationCenter", "38mm"),
        iew("27.5x27.5", 55, "55.png", "watch", "2x", "notificationCenter", "42mm"),
        iew("33x33", 66, "66.png", "watch", "2x", "notificationCenter", "45mm"),
        ier("29x29", 87, "87.png", "watch", "3x", "companionSettings"),
        ier("29x29", 58, "58.png", "watch", "2x", "companionSettings"),
        ie("1024x1024", 1024, "1024.png", APPICONSET, "watch-marketing", "1x"),
    ]
}

pub fn mac_entries() -> Vec<IconEntry> {
    vec![
        ie("128x128", 128, "128.png", APPICONSET, "mac", "1x"),
        ie("256x256", 256, "256.png", APPICONSET, "mac", "1x"),
        ie("128x128", 256, "256.png", APPICONSET, "mac", "2x"),
        ie("256x256", 512, "512.png", APPICONSET, "mac", "2x"),
        ie("32x32", 32, "32.png", APPICONSET, "mac", "1x"),
        ie("512x512", 512, "512.png", APPICONSET, "mac", "1x"),
        ie("16x16", 16, "16.png", APPICONSET, "mac", "1x"),
        ie("16x16", 32, "32.png", APPICONSET, "mac", "2x"),
        ie("32x32", 64, "64.png", APPICONSET, "mac", "2x"),
        ie("512x512", 1024, "1024.png", APPICONSET, "mac", "2x"),
    ]
}

pub fn android_entries(filename: &str) -> Vec<IconEntry> {
    let f = if filename.is_empty() {
        "ic_launcher.png"
    } else if filename.ends_with(".png") {
        filename
    } else {
        return vec![
            android_entry(48, "mipmap-mdpi", &format!("{filename}.png")),
            android_entry(72, "mipmap-hdpi", &format!("{filename}.png")),
            android_entry(96, "mipmap-xhdpi", &format!("{filename}.png")),
            android_entry(144, "mipmap-xxhdpi", &format!("{filename}.png")),
            android_entry(192, "mipmap-xxxhdpi", &format!("{filename}.png")),
        ];
    };
    vec![
        android_entry(48, "mipmap-mdpi", f),
        android_entry(72, "mipmap-hdpi", f),
        android_entry(96, "mipmap-xhdpi", f),
        android_entry(144, "mipmap-xxhdpi", f),
        android_entry(192, "mipmap-xxxhdpi", f),
    ]
}

pub fn store_entries() -> Vec<IconEntry> {
    vec![
        IconEntry {
            size: "1024x1024",
            expected_size: 1024,
            filename: "appstore.png".into(),
            folder: "",
            idiom: "ios-marketing",
            scale: "1x",
            role: None,
            subtype: None,
        },
        IconEntry {
            size: "512x512",
            expected_size: 512,
            filename: "playstore.png".into(),
            folder: "",
            idiom: "other",
            scale: "1x",
            role: None,
            subtype: None,
        },
    ]
}

/// Returns true if this platform's icons should preserve alpha (transparency).
/// Matches appicon.co behavior: Android + playstore preserve alpha; Apple + appstore fill white.
pub fn preserves_alpha(platform: Platform) -> bool {
    matches!(platform, Platform::Android)
}

pub fn store_preserves_alpha(filename: &str) -> bool {
    filename == "playstore.png"
}

pub fn get_entries(platform: Platform, android_filename: &str) -> Vec<IconEntry> {
    match platform {
        Platform::Iphone => iphone_entries(),
        Platform::Ipad => ipad_entries(),
        Platform::Watch => watch_entries(),
        Platform::Mac => mac_entries(),
        Platform::Android => android_entries(android_filename),
        Platform::All => unreachable!("All should be expanded before calling get_entries"),
    }
}

pub fn is_apple_platform(platform: Platform) -> bool {
    matches!(
        platform,
        Platform::Iphone | Platform::Ipad | Platform::Watch | Platform::Mac
    )
}

pub fn expand_platforms(platforms: &[Platform]) -> Vec<Platform> {
    if platforms.contains(&Platform::All) || platforms.is_empty() {
        vec![
            Platform::Iphone,
            Platform::Ipad,
            Platform::Watch,
            Platform::Mac,
            Platform::Android,
        ]
    } else {
        let mut result = Vec::new();
        for &p in platforms {
            if !result.contains(&p) {
                result.push(p);
            }
        }
        result
    }
}

// --- helpers ---

const APPICONSET: &str = "Assets.xcassets/AppIcon.appiconset/";

fn ie(
    size: &'static str,
    expected_size: u32,
    filename: &'static str,
    folder: &'static str,
    idiom: &'static str,
    scale: &'static str,
) -> IconEntry {
    IconEntry {
        size,
        expected_size,
        filename: filename.into(),
        folder,
        idiom,
        scale,
        role: None,
        subtype: None,
    }
}

fn iew(
    size: &'static str,
    expected_size: u32,
    filename: &'static str,
    idiom: &'static str,
    scale: &'static str,
    role: &'static str,
    subtype: &'static str,
) -> IconEntry {
    IconEntry {
        size,
        expected_size,
        filename: filename.into(),
        folder: APPICONSET,
        idiom,
        scale,
        role: Some(role),
        subtype: Some(subtype),
    }
}

fn ier(
    size: &'static str,
    expected_size: u32,
    filename: &'static str,
    idiom: &'static str,
    scale: &'static str,
    role: &'static str,
) -> IconEntry {
    IconEntry {
        size,
        expected_size,
        filename: filename.into(),
        folder: APPICONSET,
        idiom,
        scale,
        role: Some(role),
        subtype: None,
    }
}

fn android_entry(size: u32, density: &'static str, filename: &str) -> IconEntry {
    IconEntry {
        size: match size {
            48 => "48x48",
            72 => "72x72",
            96 => "96x96",
            144 => "144x144",
            192 => "192x192",
            _ => unreachable!(),
        },
        expected_size: size,
        filename: filename.into(),
        folder: match density {
            "mipmap-mdpi" => "android/mipmap-mdpi/",
            "mipmap-hdpi" => "android/mipmap-hdpi/",
            "mipmap-xhdpi" => "android/mipmap-xhdpi/",
            "mipmap-xxhdpi" => "android/mipmap-xxhdpi/",
            "mipmap-xxxhdpi" => "android/mipmap-xxxhdpi/",
            _ => unreachable!(),
        },
        idiom: "android",
        scale: "1x",
        role: None,
        subtype: None,
    }
}
