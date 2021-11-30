#![no_std]
#![no_main]

pub mod animations;
pub mod colors;
pub mod default_animations;
pub mod hardware;
pub mod leds;
pub mod lighting_controller;

use crate::animations as a;
use crate::colors as c;
use crate::default_animations as da;
use crate::leds::ws28xx as strip;
use crate::lighting_controller as lc;

use crate::hardware::{DynamicPin, HardwareController};
use bl602_hal as hal;
use core::fmt::Write;
// use embedded_hal::delay::blocking::DelayMs;
use crate::animations::Animatable;
use embedded_time::rate::*;
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    pac,
    prelude::*,
    serial::*,
    timer::*,
};
use panic_halt as _;

// How many LEDs on each wall:
const NUM_LEDS_SOUTH_WALL: usize = 34;
const NUM_LEDS_EAST_WALL: usize = 49;
const NUM_LEDS_NORTH_WALL: usize = 35;
const NUM_LEDS_WEST_WALL: usize = 49;

// Real Values:
// The number of LEDs on each strip:
const NUM_LEDS_WINDOW_STRIP: usize = 74;
const NUM_LEDS_DOOR_STRIP: usize = 59;
const NUM_LEDS_CLOSET_STRIP: usize = 34;

// Test Strip Values:
// The number of LEDs on each strip:
// const NUM_LEDS_WINDOW_STRIP: usize = 4;
// const NUM_LEDS_DOOR_STRIP: usize = 4;
// const NUM_LEDS_CLOSET_STRIP: usize = 4;

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

    let mut timer_ch1 = timers
        .channel1
        .set_clock_source(ClockSource::Fclk(&clocks), 160_000_000_u32.Hz());

    // The order of pins here needs to match the array of strips passed into LogicalStrip::new()
    let pins: [DynamicPin; NUM_STRIPS] = [
        &mut gpio_pins.pin0.into_pull_down_output(),
        &mut gpio_pins.pin3.into_pull_down_output(),
        &mut gpio_pins.pin1.into_pull_down_output(),
    ];

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

    let mut color_buffer: [c::Color; NUM_LEDS] = [c::Color::default(); NUM_LEDS];
    let office_strip = strip::LogicalStrip::new(&mut color_buffer, &ALL_STRIPS);

    let mut hc = HardwareController::new(pins, timer_ch0);

    let mut s_ta: [usize; NUM_LEDS_SOUTH_WALL] = [0; NUM_LEDS_SOUTH_WALL];
    for (index, value) in s_ta.iter_mut().enumerate() {
        *value = index;
    }

    let mut e_ta: [usize; NUM_LEDS_EAST_WALL] = [0; NUM_LEDS_EAST_WALL];
    for (index, value) in e_ta.iter_mut().enumerate() {
        *value = index + NUM_LEDS_SOUTH_WALL;
    }

    let mut n_ta: [usize; NUM_LEDS_NORTH_WALL] = [0; NUM_LEDS_NORTH_WALL];
    for (index, value) in n_ta.iter_mut().enumerate() {
        *value = index + NUM_LEDS_SOUTH_WALL + NUM_LEDS_EAST_WALL;
    }

    let mut w_ta: [usize; NUM_LEDS_WEST_WALL] = [0; NUM_LEDS_WEST_WALL];
    for (index, value) in w_ta.iter_mut().enumerate() {
        *value = index + NUM_LEDS_SOUTH_WALL + NUM_LEDS_EAST_WALL + NUM_LEDS_NORTH_WALL;
    }

    // Make a single animation operating on the whole strip:
    let mut s_a = a::Animation::new(da::ANI_TEST, s_ta, 2173481723);
    let mut e_a = a::Animation::new(da::ANI_TEST, e_ta, 9238479238);
    let mut n_a = a::Animation::new(da::ANI_TEST, n_ta, 2309489849);
    let mut w_a = a::Animation::new(da::ANI_TEST, w_ta, 3928392389);
    let animation_array: [&mut dyn Animatable; 4] = [&mut s_a, &mut e_a, &mut n_a, &mut w_a];

    let mut lc =
        lc::LightingController::new(office_strip, animation_array, 60_u32.Hz(), &mut timer_ch1);

    // get a millisecond delay for use with test patterns:
    // let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

    let test_trigger = a::AnimationTriggerParameters {
        mode: a::TriggerMode::FlashSlowFade,
        direction: a::Direction::Stopped,
        step_time_ns: 0,
        fade_in_time_ns: 250_000_000,
        fade_out_time_ns: 750_000_000,
        starting_offset: 0,
    };

    // let mut i = 0_u16;
    let mut last_time = riscv::register::mcycle::read64();
    loop {
        lc.update(&mut hc);
        // i = (i + 1) % a::MAX_OFFSET;
        // lc.set_offset(0, a::AnimationType::Foreground, i);
        if riscv::register::mcycle::read64() - last_time > 160_000_000 {
            lc.trigger(0, &test_trigger);
            last_time = riscv::register::mcycle::read64();
        }
        // d.delay_ms(1).ok();
    }
}
