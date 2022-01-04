#![no_std]
#![no_main]

use bl602_ws2811::*;

use animations as a;
use colors as c;
use default_animations as da;
use hardware::{DynamicPin, HardwareController};
use leds::ws28xx as strip;
use lighting_controller as lc;

use bl602_hal as hal;
use core::fmt::Write;
use embedded_time::rate::*;
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    pac,
    prelude::*,
    serial::*,
    timer::*,
};
use panic_write as _;
use panic_write::PanicHandler;

// How many LEDs on each wall animation:
pub const NUM_LEDS_SOUTH_WALL: usize = 34;
pub const NUM_LEDS_EAST_WALL: usize = 49;
pub const NUM_LEDS_NORTH_WALL: usize = 35;
pub const NUM_LEDS_WEST_WALL: usize = 49;

// individual strips:
pub const CLOSET_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: 34,
    reversed: false,
    color_order: strip::ColorOrder::BRG,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};
pub const WINDOW_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: 74,
    reversed: false,
    color_order: strip::ColorOrder::BRG,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};
pub const DOOR_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    led_count: 59,
    reversed: true,
    color_order: strip::ColorOrder::BRG,
    strip_timings: strip::StripTimings::WS2812_ADAFRUIT,
};

pub const NUM_STRIPS: usize = 3;
// combined strip group, make sure your pins in main() are in the same order as the strip order here:
pub const ALL_STRIPS: [strip::PhysicalStrip; NUM_STRIPS] = [CLOSET_STRIP, WINDOW_STRIP, DOOR_STRIP];

// calculate the total number of LEDs from the above values:
pub const NUM_LEDS: usize = crate::get_total_num_leds(&ALL_STRIPS);

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
    let mut pins: [DynamicPin; NUM_STRIPS] = [
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

    let mut serial = PanicHandler::new(serial);
    writeln!(serial, "Debug Serial Initialized...\r").ok();

    let mut memory_buffer = [0; NUM_LEDS * 3];
    let mut color_buffer: [c::Color; NUM_LEDS] = [c::Color::default(); NUM_LEDS];
    let office_strip = strip::LogicalStrip::new(&mut memory_buffer, &mut color_buffer, &ALL_STRIPS);

    let mut hc = HardwareController::new(&mut pins, timer_ch0);

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
    let mut s_a = a::Animation::new(da::ANI_TEST, s_ta, 60.Hz());
    let mut e_a = a::Animation::new(da::ANI_TEST, e_ta, 60.Hz());
    let mut n_a = a::Animation::new(da::ANI_TEST, n_ta, 60.Hz());
    let mut w_a = a::Animation::new(da::ANI_TEST, w_ta, 60.Hz());
    let animation_array: [&mut dyn a::Animatable; 4] = [&mut s_a, &mut e_a, &mut n_a, &mut w_a];

    let mut lc =
        lc::LightingController::new(office_strip, animation_array, 60_u32.Hz(), &mut timer_ch1);

    // get a millisecond delay for use with test patterns:
    // let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

    let test_trigger = trigger::Parameters {
        mode: trigger::Mode::ColorShotRainbow,
        direction: a::Direction::Positive,
        fade_in_time_ns: 1_250_000_000,
        fade_out_time_ns: 1_750_000_000,
        starting_offset: 0,
        pixels_per_pixel_group: 1,
    };

    // let mut i = 0_u16;
    let mut last_time = riscv::register::mcycle::read64();
    loop {
        lc.update(&mut hc);
        // i = (i + 1) % a::MAX_OFFSET;
        // lc.set_offset(0, a::AnimationType::Foreground, i);
        if riscv::register::mcycle::read64() - last_time > 160_000_000 * 3 {
            lc.trigger(0, &test_trigger);
            last_time = riscv::register::mcycle::read64();
        }
        // d.delay_ms(1).ok();
    }
}
