pub const IS_GAMMA_CORRECTION_ENABLED: bool = true;

#[allow(dead_code)]
#[derive(Default, Copy, Clone, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    // new color object takes rgb color values:
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        let r = GAMMA8[r as usize];
        let g = GAMMA8[g as usize];
        let b = GAMMA8[b as usize];
        Color { r, g, b }
    }

    // change RGB color values for mutable color
    pub fn set_rgb(&mut self, r: u8, g: u8, b: u8) {
        if IS_GAMMA_CORRECTION_ENABLED {
            self.r = GAMMA8[r as usize];
            self.g = GAMMA8[g as usize];
            self.b = GAMMA8[b as usize];
        } else {
            self.r = r;
            self.g = g;
            self.b = b;
        }
    }

    // change RGB color values for mutable color
    pub fn set_color(&mut self, color: Color) {
        self.set_rgb(color.r, color.g, color.b);
    }

    // t=This maps a color to a fractional mid-color based on the position of the factor
    // between the in_min and in_max values. It will automatically truncate any values
    // below 0 or larger than 255 when it is cast back to a u8 at the end of the calculation.
    pub fn color_lerp(
        factor: i32,
        in_min: i32,
        in_max: i32,
        start_color: Color,
        end_color: Color,
    ) -> Color {
        let lerp = |start: u8, end: u8| {
            let start = start as i32;
            let end = end as i32;
            ((factor - in_min) * (end - start) / (in_max - in_min) + start) as u8
        };
        let mut mid_color = C_OFF;

        mid_color.r = lerp(start_color.r, end_color.r);
        mid_color.g = lerp(start_color.g, end_color.g);
        mid_color.b = lerp(start_color.b, end_color.b);

        mid_color
    }
}

// Generic colors:
pub const C_RED: Color = Color { r: 255, g: 0, b: 0 };
pub const C_ORANGE: Color = Color { r: 255, g: 127, b: 0 };
pub const C_YELLOW: Color = Color { r: 255, g: 255, b: 0 };
pub const C_YELLOW_GREEN: Color = Color { r: 127, g: 255, b: 0 };
pub const C_GREEN: Color = Color { r: 0, g: 255, b: 0 };
pub const C_GREEN_BLUE: Color = Color { r: 0, g: 255, b: 127 };
pub const C_SKY_BLUE: Color = Color { r: 0, g: 255, b: 255 };
pub const C_DEEP_BLUE: Color = Color { r: 0, g: 127, b: 255 };
pub const C_BLUE: Color = Color { r: 0, g: 0, b: 255 };
pub const C_BLUE_PURPLE: Color = Color { r: 127, g: 0, b: 255 };
pub const C_PURPLE: Color = Color { r: 255, g: 0, b: 255 };
pub const C_DARK_PURPLE: Color = Color { r: 255, g: 0, b: 127 };
pub const C_WHITE: Color = Color { r: 255, g: 255, b: 255 };
pub const C_OFF: Color = Color { r: 0, g: 0, b: 0 };
pub const C_T_3000K: Color = Color { r: 255, g: 180, b: 107 };
pub const C_T_3500K: Color = Color { r: 255, g: 196, b: 137 };
pub const C_T_4000K: Color = Color { r: 255, g: 209, b: 163 };
pub const C_T_5000K: Color = Color { r: 255, g: 228, b: 206 };

// Use const generic rainbows to make iterable rainbows of various sizes. Rainbows contain a
// list of colors in order, which will be used by animations as a color rainbow.
pub type Rainbow<'a> = &'a [Color];

pub const R_OFF: Rainbow = &[C_OFF];
pub const R_ON: Rainbow = &[C_WHITE];
pub const R_RED: Rainbow = &[C_RED];
pub const R_ORANGE: Rainbow = &[C_ORANGE];
pub const R_YELLOW: Rainbow = &[C_YELLOW];
pub const R_YELLOW_GREEN: Rainbow = &[C_YELLOW_GREEN];
pub const R_GREEN: Rainbow = &[C_GREEN];
pub const R_GREEN_BLUE: Rainbow = &[C_GREEN_BLUE];
pub const R_SKY_BLUE: Rainbow = &[C_SKY_BLUE];
pub const R_DEEP_BLUE: Rainbow = &[C_DEEP_BLUE];
pub const R_BLUE: Rainbow = &[C_BLUE];
pub const R_BLUE_PURPLE: Rainbow = &[C_BLUE_PURPLE];
pub const R_PURPLE: Rainbow = &[C_PURPLE];
pub const R_DARK_PURPLE: Rainbow = &[C_DARK_PURPLE];
pub const R_ROYGBIV: Rainbow = &[C_RED, C_ORANGE, C_YELLOW, C_GREEN, C_BLUE, C_DARK_PURPLE];
pub const R_RYB: Rainbow = &[C_RED, C_OFF, C_YELLOW, C_OFF, C_BLUE, C_OFF];
pub const R_OGP: Rainbow = &[C_ORANGE, C_OFF, C_GREEN, C_OFF, C_PURPLE, C_OFF];
pub const R_RGB: Rainbow = &[C_RED, C_OFF, C_GREEN, C_OFF, C_BLUE, C_OFF];
pub const R_BY: Rainbow = &[C_BLUE, C_OFF, C_YELLOW, C_OFF];
pub const R_RB: Rainbow = &[C_RED, C_OFF, C_SKY_BLUE, C_OFF];
pub const R_OB: Rainbow = &[C_ORANGE, C_OFF, C_DEEP_BLUE, C_OFF];
pub const R_BW: Rainbow = &[C_BLUE, C_OFF, C_WHITE, C_OFF];
pub const R_RW: Rainbow = &[C_RED, C_OFF, C_WHITE, C_OFF];
pub const R_GW: Rainbow = &[C_GREEN, C_OFF, C_WHITE, C_OFF];
pub const R_T_3000K: Rainbow = &[C_T_4000K];
pub const R_T_3500K: Rainbow = &[C_T_4000K];
pub const R_T_4000K: Rainbow = &[C_T_4000K];
pub const R_T_5000K: Rainbow = &[C_T_4000K];

pub const fn dark_pattern(base: Color) -> [Color; 6] {
    let mut colors = [C_OFF; 6];
    let mut i = 0;
    while i < 3 {
        colors[i * 2] = Color { r: base.r / 2, g: base.g / 2, b: base.b / 2 };
        colors[i * 2 + 1] = Color { r: base.r / 4, g: base.g / 4, b: base.b / 4 };
        i += 1;
    }
    colors
}

pub const R_DARK_RED_PATTERN: Rainbow = &dark_pattern(C_RED);
pub const R_DARK_YELLOW_PATTERN: Rainbow = &dark_pattern(C_YELLOW);
pub const R_DARK_GREEN_PATTERN: Rainbow = &dark_pattern(C_GREEN);
pub const R_DARK_SKY_BLUE_PATTERN: Rainbow = &dark_pattern(C_SKY_BLUE);
pub const R_DARK_BLUE_PATTERN: Rainbow = &dark_pattern(C_BLUE);
pub const R_DARK_PURPLE_PATTERN: Rainbow = &dark_pattern(C_PURPLE);
pub const R_WHITE_PATTERN: Rainbow = &dark_pattern(C_WHITE);
pub const R_VU_METER: Rainbow = &[
    C_GREEN, C_GREEN, C_GREEN, C_GREEN, C_GREEN, C_GREEN, C_GREEN, C_YELLOW, C_YELLOW, C_RED,
];

pub const NUM_RAINBOWS: usize = 31;

/// This is an array of the rainbow consts above that can be used to cycle through rainbows in animations.
pub const RAINBOW_ARRAY: [&[Color]; NUM_RAINBOWS] = [
    R_OFF,
    R_ON,
    R_RED,
    R_ORANGE,
    R_YELLOW,
    R_YELLOW_GREEN,
    R_GREEN,
    R_GREEN_BLUE,
    R_SKY_BLUE,
    R_DEEP_BLUE,
    R_BLUE,
    R_BLUE_PURPLE,
    R_PURPLE,
    R_DARK_PURPLE,
    R_ROYGBIV,
    R_RYB,
    R_OGP,
    R_RGB,
    R_BY,
    R_RB,
    R_OB,
    R_BW,
    R_RW,
    R_GW,
    R_DARK_RED_PATTERN,
    R_DARK_YELLOW_PATTERN,
    R_DARK_GREEN_PATTERN,
    R_DARK_SKY_BLUE_PATTERN,
    R_DARK_BLUE_PATTERN,
    R_DARK_PURPLE_PATTERN,
    R_WHITE_PATTERN,
];

/// A color correction table for LEDs to make them look like the color you expect:
/// Shamelessly stolen from Adafruit's neopixel library somewhere a long time ago.
pub static GAMMA8: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5,
    5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 12, 12, 13, 13, 13, 14,
    14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 24, 24, 25, 25, 26, 27,
    27, 28, 29, 29, 30, 31, 32, 32, 33, 34, 35, 35, 36, 37, 38, 39, 39, 40, 41, 42, 43, 44, 45, 46,
    47, 48, 49, 50, 50, 51, 52, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 66, 67, 68, 69, 70, 72,
    73, 74, 75, 77, 78, 79, 81, 82, 83, 85, 86, 87, 89, 90, 92, 93, 95, 96, 98, 99, 101, 102, 104,
    105, 107, 109, 110, 112, 114, 115, 117, 119, 120, 122, 124, 126, 127, 129, 131, 133, 135, 137,
    138, 140, 142, 144, 146, 148, 150, 152, 154, 156, 158, 160, 162, 164, 167, 169, 171, 173, 175,
    177, 180, 182, 184, 186, 189, 191, 193, 196, 198, 200, 203, 205, 208, 210, 213, 215, 218, 220,
    223, 225, 228, 231, 233, 236, 239, 241, 244, 247, 249, 252, 255,
];

/// This is the first color in the GAMMA8 array that is not completely turned off.
pub const FIRST_NON_OFF_COLOR: usize = 28;
