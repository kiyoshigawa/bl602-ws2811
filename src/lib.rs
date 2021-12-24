#![no_std]

pub mod animations;
pub mod background;
pub mod colors;
pub mod default_animations;
pub mod foreground;
pub mod hardware;
pub mod leds;
pub mod lighting_controller;
pub mod trigger;
pub mod utility;

use leds::ws28xx as strip;

// How many LEDs on each wall:
pub const NUM_LEDS_SOUTH_WALL: usize = 34;
pub const NUM_LEDS_EAST_WALL: usize = 49;
pub const NUM_LEDS_NORTH_WALL: usize = 35;
pub const NUM_LEDS_WEST_WALL: usize = 49;

// Real Values:
// The number of LEDs on each strip:
pub const NUM_LEDS_WINDOW_STRIP: usize = 74;
pub const NUM_LEDS_DOOR_STRIP: usize = 59;
pub const NUM_LEDS_CLOSET_STRIP: usize = 34;

// Test Strip Values:
// The number of LEDs on each strip:
// const NUM_LEDS_WINDOW_STRIP: usize = 4;
// const NUM_LEDS_DOOR_STRIP: usize = 4;
// const NUM_LEDS_CLOSET_STRIP: usize = 4;

// individual strips:
pub const CLOSET_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: NUM_LEDS_CLOSET_STRIP,
    reversed: false,
    color_order: strip::ColorOrder::BRG,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};
pub const WINDOW_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: NUM_LEDS_WINDOW_STRIP,
    reversed: false,
    color_order: strip::ColorOrder::BRG,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};
pub const DOOR_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: NUM_LEDS_DOOR_STRIP,
    reversed: true,
    color_order: strip::ColorOrder::BRG,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};

pub const NUM_STRIPS: usize = 3;
// combined strip group, make sure your pins in main() are in the same order as the strip order here:
pub const ALL_STRIPS: [strip::PhysicalStrip; NUM_STRIPS] = [CLOSET_STRIP, WINDOW_STRIP, DOOR_STRIP];

// The number of LEDs on each strip:
pub const MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH: usize =
    get_single_strip_buffer_max_length(&ALL_STRIPS);

// calculate the total number of LEDs from the above values:
pub const NUM_LEDS: usize = get_total_num_leds(&ALL_STRIPS);

const fn get_single_strip_buffer_max_length(strips: &[strip::PhysicalStrip]) -> usize {
    let mut max_len = 0;
    let mut index = 0;
    while index < strips.len() {
        if strips[index].led_count > max_len {
            max_len = strips[index].led_count;
        }
        index += 1;
    }
    // three bytes per led
    max_len * 3
}

const fn get_total_num_leds(strips: &[strip::PhysicalStrip]) -> usize {
    let mut index = 0;
    let mut total = 0;
    while index < strips.len() {
        total += strips[index].led_count;
        index += 1;
    }
    total
}
