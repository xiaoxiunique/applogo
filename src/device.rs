pub struct DeviceConfig {
    pub id: &'static str,
    pub name: &'static str,
    pub color: &'static str,
    pub display_resolution: (u32, u32),
    pub is_mockup_image_at_front: bool,
    pub orientations: &'static [OrientationConfig],
}

pub struct OrientationConfig {
    pub name: &'static str,
    /// Screen coordinates: [top-left, top-right, bottom-right, bottom-left]
    pub screen_coord: [(u32, u32); 4],
}

impl DeviceConfig {
    pub fn template_filename(&self, orientation: &str) -> String {
        format!("{}-{}.png", self.id, orientation)
    }

    pub fn find_orientation(&self, name: &str) -> Option<&OrientationConfig> {
        self.orientations.iter().find(|o| o.name == name)
    }
}

/// Returns all curated device configs.
pub fn all_devices() -> &'static [DeviceConfig] {
    &DEVICES
}

/// Find a device by ID.
pub fn find_device(id: &str) -> Option<&'static DeviceConfig> {
    DEVICES.iter().find(|d| d.id == id)
}

/// Default device ID.
pub const DEFAULT_DEVICE: &str = "apple-iphone-15-pro-black-titanium";

static DEVICES: [DeviceConfig; 5] = [
    // iPhone 15
    DeviceConfig {
        id: "apple-iphone-15-black",
        name: "iPhone 15",
        color: "Black",
        display_resolution: (1179, 2556),
        is_mockup_image_at_front: true,
        orientations: &[
            OrientationConfig {
                name: "portrait",
                screen_coord: [(120, 120), (1299, 120), (1299, 2676), (120, 2676)],
            },
            OrientationConfig {
                name: "landscape",
                screen_coord: [(2676, 120), (2676, 1299), (120, 1299), (120, 120)],
            },
        ],
    },
    // iPhone 15 Pro
    DeviceConfig {
        id: "apple-iphone-15-pro-black-titanium",
        name: "iPhone 15 Pro",
        color: "Black Titanium",
        display_resolution: (1179, 2556),
        is_mockup_image_at_front: true,
        orientations: &[
            OrientationConfig {
                name: "portrait",
                screen_coord: [(120, 120), (1299, 120), (1299, 2676), (120, 2676)],
            },
            OrientationConfig {
                name: "landscape",
                screen_coord: [(2676, 120), (2676, 1299), (120, 1299), (120, 120)],
            },
        ],
    },
    // iPhone 15 Pro Max
    DeviceConfig {
        id: "apple-iphone-15-pro-max-black-titanium",
        name: "iPhone 15 Pro Max",
        color: "Black Titanium",
        display_resolution: (1290, 2796),
        is_mockup_image_at_front: true,
        orientations: &[
            OrientationConfig {
                name: "portrait",
                screen_coord: [(120, 120), (1410, 120), (1410, 2916), (120, 2916)],
            },
            OrientationConfig {
                name: "landscape",
                screen_coord: [(2916, 120), (2916, 1410), (120, 1410), (120, 120)],
            },
        ],
    },
    // iPhone 14 Pro
    DeviceConfig {
        id: "apple-iphone14pro-spaceblack",
        name: "iPhone 14 Pro",
        color: "Space Black",
        display_resolution: (1179, 2556),
        is_mockup_image_at_front: true,
        orientations: &[
            OrientationConfig {
                name: "portrait",
                screen_coord: [(78, 78), (1261, 78), (1261, 2638), (78, 2638)],
            },
            OrientationConfig {
                name: "landscape",
                screen_coord: [(2638, 79), (2638, 1262), (79, 1262), (79, 79)],
            },
        ],
    },
    // iPhone 14
    DeviceConfig {
        id: "apple-iphone14-midnight",
        name: "iPhone 14",
        color: "Midnight",
        display_resolution: (1170, 2532),
        is_mockup_image_at_front: true,
        orientations: &[
            OrientationConfig {
                name: "portrait",
                screen_coord: [(98, 98), (1272, 98), (1272, 2634), (98, 2634)],
            },
            OrientationConfig {
                name: "landscape",
                screen_coord: [(2634, 98), (2634, 1272), (98, 1272), (98, 98)],
            },
        ],
    },
];
