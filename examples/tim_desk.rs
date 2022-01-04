#![no_std]
#![no_main]

use bl602_ws2811::*;

use animations as a;
use colors as c;
use default_animations as da;
use hardware::{DynamicPin, HardwareController};
use leds::ws28xx as strip;
use lighting_controller as lc;
use utility as u;

use core::fmt::Write;
use embedded_time::rate::*;

use bl602_hal as hal;
use hal::{gpio::*, pac};

// use panic_halt as _;
use panic_write::PanicHandler;

// individual strips:
pub const STRIP_ONE: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: 4,
    reversed: false,
    color_order: strip::ColorOrder::GRB,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};
pub const STRIP_TWO: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: 4,
    reversed: true,
    color_order: strip::ColorOrder::GRB,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};
pub const STRIP_THREE: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: 4,
    reversed: false,
    color_order: strip::ColorOrder::GRB,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};
pub const STRIP_FOUR: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: 4,
    reversed: true,
    color_order: strip::ColorOrder::GRB,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};

pub const NUM_STRIPS: usize = 4;
// combined strip group, make sure your pins in main() are in the same order as the strip order here:
pub const ALL_STRIPS: [strip::PhysicalStrip; NUM_STRIPS] =
    [STRIP_ONE, STRIP_TWO, STRIP_THREE, STRIP_FOUR];

// calculate the total number of LEDs from the above values:
pub const NUM_LEDS: usize = crate::get_total_num_leds(&ALL_STRIPS);

#[riscv_rt::entry]
fn main() -> ! {
    // get the peripherals
    let dp = pac::Peripherals::take().unwrap();

    // split out the parts
    let mut gpio = dp.GLB.split();

    // Set up all the clocks we need
    let clocks = u::init_clocks(&mut gpio.clk_cfg);

    // configure our two timer channels
    let (timer_ch0, mut timer_ch1) = u::init_timers(dp.TIMER, &clocks);

    // Set up uart output for debug printing. Since this microcontroller has a pin matrix,
    // we need to set up both the pins and the muxes
    let serial = u::init_usb_serial(
        dp.UART,
        clocks,
        2_000_000.Bd(),
        gpio.pin16,
        gpio.pin7,
        gpio.uart_mux0,
        gpio.uart_mux7,
    );

    // writes panic messages to serial to see where things went wrong
    let mut serial = PanicHandler::new(serial);

    writeln!(serial, "Debug Serial Initialized...\r").ok();

    // The order of pins here needs to match the array of strips passed into LogicalStrip::new()
    let mut pins: [DynamicPin; NUM_STRIPS] = [
        &mut gpio.pin0.into_pull_down_output(),
        &mut gpio.pin1.into_pull_down_output(),
        &mut gpio.pin3.into_pull_down_output(),
        &mut gpio.pin4.into_pull_down_output(),
    ];

    let mut memory_buffer = [0; NUM_LEDS * 3];
    let mut color_buffer: [c::Color; NUM_LEDS] = [c::Color::default(); NUM_LEDS];
    let strip = strip::LogicalStrip::new(&mut memory_buffer, &mut color_buffer, &ALL_STRIPS);

    let mut hc = HardwareController::new(&mut pins, timer_ch0);

    let t_a = utility::default_translation_array::<NUM_LEDS>(0);

    // Make a single animation operating on the whole strip:
    let mut a = a::Animation::new(da::ANI_TEST, t_a, 60.Hz());

    let animation_array: [&mut dyn a::Animatable; 1] = [&mut a];

    let mut lc = lc::LightingController::new(strip, animation_array, 60_u32.Hz(), &mut timer_ch1);

    let test_trigger = trigger::Parameters {
        mode: trigger::Mode::ColorShotRainbow,
        direction: a::Direction::Positive,
        fade_in_time_ns: 1_250_000_000,
        fade_out_time_ns: 1_750_000_000,
        starting_offset: 0,
        pixels_per_pixel_group: 1,
    };

    let mut last_time = riscv::register::mcycle::read64();
    loop {
        lc.update(&mut hc);
        if riscv::register::mcycle::read64() - last_time > 160_000_000 {
            lc.trigger(0, &test_trigger);
            last_time = riscv::register::mcycle::read64();
        }
    }
}
