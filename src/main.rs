#![no_std]
#![no_main]

pub mod animations;
pub mod colors;
pub mod hardware;
pub mod leds;
pub mod lighting_controller;

use crate::animations as a;
use crate::colors as c;
use crate::leds::ws28xx as strip;
use crate::lighting_controller as lc;

use crate::colors::{C_BLUE, C_GREEN, C_RED};
use crate::hardware::{DynamicPin, HardwareController};
use bl602_hal as hal;
use core::fmt::Write;
use embedded_hal::delay::blocking::DelayMs;
use embedded_time::rate::*;
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    pac,
    prelude::*,
    serial::*,
    timer::*,
};
use panic_halt as _;

// Real Values:
// The number of LEDs on each strip:
// const NUM_LEDS_WINDOW_STRIP: usize = 74;
// const NUM_LEDS_DOOR_STRIP: usize = 59;
// const NUM_LEDS_CLOSET_STRIP: usize = 34;

// Test Strip Values:
// The number of LEDs on each strip:
const NUM_LEDS_WINDOW_STRIP: usize = 4;
const NUM_LEDS_DOOR_STRIP: usize = 4;
const NUM_LEDS_CLOSET_STRIP: usize = 4;

// individual strips:
const CLOSET_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: NUM_LEDS_CLOSET_STRIP,
    reversed: false,
    color_order: strip::ColorOrder::BRG,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};
const WINDOW_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: NUM_LEDS_WINDOW_STRIP,
    reversed: false,
    color_order: strip::ColorOrder::BRG,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};
const DOOR_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: NUM_LEDS_DOOR_STRIP,
    reversed: true,
    color_order: strip::ColorOrder::BRG,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};

const NUM_STRIPS: usize = 3;
// combined strip group, make sure your pins in main() are in the same order as the strip order here:
const ALL_STRIPS: [strip::PhysicalStrip; NUM_STRIPS] = [CLOSET_STRIP, WINDOW_STRIP, DOOR_STRIP];

// The number of LEDs on each strip:
const MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH: usize = get_single_strip_buffer_max_length(&ALL_STRIPS);

// calculate the total number of LEDs from the above values:
const NUM_LEDS: usize = get_total_num_leds(&ALL_STRIPS);

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

#[riscv_rt::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let mut gpio_pins = dp.GLB.split();

    // Set up all the clocks we need
    let clocks = Strict::new()
        .use_pll(40_000_000u32.Hz())
        .sys_clk(SysclkFreq::Pll160Mhz)
        .uart_clk(UART_PLL_FREQ.Hz())
        .freeze(&mut gpio_pins.clk_cfg);

    let timers = dp.TIMER.split();
    let timer_ch0 = timers
        .channel0
        .set_clock_source(ClockSource::Fclk(&clocks), 160_000_000_u32.Hz());

    // The order of pins here needs to match the array of strips passed into LogicalStrip::new()
    let pins: [DynamicPin; NUM_STRIPS] = [
        &mut gpio_pins.pin0.into_pull_down_output(),
        &mut gpio_pins.pin3.into_pull_down_output(),
        &mut gpio_pins.pin1.into_pull_down_output(),
    ];

    let mut office_strip = strip::LogicalStrip::<NUM_LEDS>::new(&ALL_STRIPS);

    let mut hc = HardwareController::new(pins, timer_ch0);

    // For now, the translation array is just all the leds on the office_strip
    let mut translation_array: [usize; NUM_LEDS] = [0; NUM_LEDS];
    for (index, value) in translation_array.iter_mut().enumerate() {
        *value = index;
    }

    // Make a single animation operating on the whole strip:
    let mut a = a::Animation::new(a::ANI_TEST, translation_array);
    let animation_array = [&mut a];

    let lc = lc::LightingController::new(
        office_strip.color_buffer.as_mut(),
        animation_array,
        60_u32.Hz(),
    );

    // Set up uart output for debug printing. Since this microcontroller has a pin matrix,
    // we need to set up both the pins and the muxs
    let pin16 = gpio_pins.pin16.into_uart_sig0();
    let pin7 = gpio_pins.pin7.into_uart_sig7();
    let mux0 = gpio_pins.uart_mux0.into_uart0_tx();
    let mux7 = gpio_pins.uart_mux7.into_uart0_rx();

    // Configure our UART to 2MBaud, and use the pins we configured above
    let mut serial = Serial::uart0(
        dp.UART,
        Config::default().baudrate(2_000_000.Bd()),
        ((pin16, mux0), (pin7, mux7)),
        clocks,
    );

    writeln!(serial, "Debug Serial Initialized...\r").ok();

    // get a millisecond delay for use with test patterns:
    let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

    // test color pattern before entering main program loop:
    let mut color = c::C_RED;
    office_strip.set_strip_to_solid_color(color);
    office_strip.send_all_sequential(&mut hc);
    d.delay_ms(1000).ok();
    color = c::C_GREEN;
    office_strip.set_strip_to_solid_color(color);
    office_strip.send_all_sequential(&mut hc);
    d.delay_ms(1000).ok();
    color = c::C_BLUE;
    office_strip.set_strip_to_solid_color(color);
    office_strip.send_all_sequential(&mut hc);
    d.delay_ms(1000).ok();
    office_strip.set_strip_to_solid_color(c::C_OFF);
    office_strip.send_all_sequential(&mut hc);
    d.delay_ms(100).ok();

    for i in 0..NUM_LEDS {
        color = c::C_GREEN;
        office_strip.set_strip_to_solid_color(c::C_OFF); // because lazy
        office_strip.set_color_at_index(i, color);
        office_strip.send_all_sequential(&mut hc);
        d.delay_ms(100).ok();
    }

    for i in (0..NUM_LEDS).rev() {
        color = c::C_BLUE;
        office_strip.set_strip_to_solid_color(c::C_OFF); // because lazy
        office_strip.set_color_at_index(i, color);
        office_strip.send_all_sequential(&mut hc);
        d.delay_ms(100).ok();
    }

    loop {
        for i in 0..1000 {
            color = c::Color::color_lerp(i, 0, 1000, C_RED, C_GREEN);
            office_strip.set_strip_to_solid_color(color);
            office_strip.send_all_sequential(&mut hc);
            d.delay_ms(25).ok();
        }
        for i in 0..1000 {
            color = c::Color::color_lerp(i, 0, 1000, C_GREEN, C_BLUE);
            office_strip.set_strip_to_solid_color(color);
            office_strip.send_all_sequential(&mut hc);
            d.delay_ms(25).ok();
        }
        for i in 0..1000 {
            color = c::Color::color_lerp(i, 0, 1000, C_BLUE, C_RED);
            office_strip.set_strip_to_solid_color(color);
            office_strip.send_all_sequential(&mut hc);
            d.delay_ms(25).ok();
        }
    }
}
