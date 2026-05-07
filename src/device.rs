pub struct DeviceConfig {
    pub id: &'static str,
    pub name: &'static str,
    pub color: &'static str,
    pub display_resolution: (u32, u32),
    pub orientations: &'static [OrientationConfig],
}

pub struct OrientationConfig {
    pub name: &'static str,
    /// Screen coordinates: [top-left, top-right, bottom-right, bottom-left]
    pub screen_coord: [(u32, u32); 4],
    /// Embedded device frame PNG bytes
    pub template: &'static [u8],
    /// Embedded screen mask PNG bytes
    pub mask: &'static [u8],
}

impl DeviceConfig {
    pub fn find_orientation(&self, name: &str) -> Option<&OrientationConfig> {
        self.orientations.iter().find(|o| o.name == name)
    }
}

pub fn all_devices() -> &'static [DeviceConfig] {
    &DEVICES
}

pub fn find_device(id: &str) -> Option<&'static DeviceConfig> {
    DEVICES.iter().find(|d| d.id == id)
}

pub const DEFAULT_DEVICE: &str = "apple-iphone-15-pro-black-titanium";

static DEVICES: [DeviceConfig; 5] = [
    DeviceConfig {
        id: "apple-iphone-15-black",
        name: "iPhone 15",
        color: "Black",
        display_resolution: (1179, 2556),
        orientations: &[
            OrientationConfig {
                name: "portrait",
                screen_coord: [(120, 120), (1299, 120), (1299, 2676), (120, 2676)],
                template: include_bytes!("../resources/templates/apple-iphone-15-black-portrait.png"),
                mask: include_bytes!("../resources/masks/apple-iphone-15-black-portrait.png"),
            },
            OrientationConfig {
                name: "landscape",
                screen_coord: [(2676, 120), (2676, 1299), (120, 1299), (120, 120)],
                template: include_bytes!("../resources/templates/apple-iphone-15-black-landscape.png"),
                mask: include_bytes!("../resources/masks/apple-iphone-15-black-landscape.png"),
            },
        ],
    },
    DeviceConfig {
        id: "apple-iphone-15-pro-black-titanium",
        name: "iPhone 15 Pro",
        color: "Black Titanium",
        display_resolution: (1179, 2556),
        orientations: &[
            OrientationConfig {
                name: "portrait",
                screen_coord: [(120, 120), (1299, 120), (1299, 2676), (120, 2676)],
                template: include_bytes!("../resources/templates/apple-iphone-15-pro-black-titanium-portrait.png"),
                mask: include_bytes!("../resources/masks/apple-iphone-15-pro-black-titanium-portrait.png"),
            },
            OrientationConfig {
                name: "landscape",
                screen_coord: [(2676, 120), (2676, 1299), (120, 1299), (120, 120)],
                template: include_bytes!("../resources/templates/apple-iphone-15-pro-black-titanium-landscape.png"),
                mask: include_bytes!("../resources/masks/apple-iphone-15-pro-black-titanium-landscape.png"),
            },
        ],
    },
    DeviceConfig {
        id: "apple-iphone-15-pro-max-black-titanium",
        name: "iPhone 15 Pro Max",
        color: "Black Titanium",
        display_resolution: (1290, 2796),
        orientations: &[
            OrientationConfig {
                name: "portrait",
                screen_coord: [(120, 120), (1410, 120), (1410, 2916), (120, 2916)],
                template: include_bytes!("../resources/templates/apple-iphone-15-pro-max-black-titanium-portrait.png"),
                mask: include_bytes!("../resources/masks/apple-iphone-15-pro-max-black-titanium-portrait.png"),
            },
            OrientationConfig {
                name: "landscape",
                screen_coord: [(2916, 120), (2916, 1410), (120, 1410), (120, 120)],
                template: include_bytes!("../resources/templates/apple-iphone-15-pro-max-black-titanium-landscape.png"),
                mask: include_bytes!("../resources/masks/apple-iphone-15-pro-max-black-titanium-landscape.png"),
            },
        ],
    },
    DeviceConfig {
        id: "apple-iphone14pro-spaceblack",
        name: "iPhone 14 Pro",
        color: "Space Black",
        display_resolution: (1179, 2556),
        orientations: &[
            OrientationConfig {
                name: "portrait",
                screen_coord: [(78, 78), (1261, 78), (1261, 2638), (78, 2638)],
                template: include_bytes!("../resources/templates/apple-iphone14pro-spaceblack-portrait.png"),
                mask: include_bytes!("../resources/masks/apple-iphone14pro-spaceblack-portrait.png"),
            },
            OrientationConfig {
                name: "landscape",
                screen_coord: [(2638, 79), (2638, 1262), (79, 1262), (79, 79)],
                template: include_bytes!("../resources/templates/apple-iphone14pro-spaceblack-landscape.png"),
                mask: include_bytes!("../resources/masks/apple-iphone14pro-spaceblack-landscape.png"),
            },
        ],
    },
    DeviceConfig {
        id: "apple-iphone14-midnight",
        name: "iPhone 14",
        color: "Midnight",
        display_resolution: (1170, 2532),
        orientations: &[
            OrientationConfig {
                name: "portrait",
                screen_coord: [(98, 98), (1272, 98), (1272, 2634), (98, 2634)],
                template: include_bytes!("../resources/templates/apple-iphone14-midnight-portrait.png"),
                mask: include_bytes!("../resources/masks/apple-iphone14-midnight-portrait.png"),
            },
            OrientationConfig {
                name: "landscape",
                screen_coord: [(2634, 98), (2634, 1272), (98, 1272), (98, 98)],
                template: include_bytes!("../resources/templates/apple-iphone14-midnight-landscape.png"),
                mask: include_bytes!("../resources/masks/apple-iphone14-midnight-landscape.png"),
            },
        ],
    },
];
