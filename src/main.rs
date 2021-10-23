#![no_std]
#![no_main]

pub mod animations;
pub mod colors;
pub mod leds;
pub mod pins;

use crate::animations as a;
use crate::colors as c;
use crate::leds::ws28xx as strip;
use crate::pins as p;
use core::convert::Infallible;

use crate::colors::{C_BLUE, C_GREEN, C_RED};
use bl602_hal as hal;
use core::fmt::Write;
use embedded_hal::delay::blocking::DelayMs;
use embedded_hal::digital::blocking::OutputPin;
use embedded_time::rate::*;
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    gpio::{Output, PullDown},
    pac,
    prelude::*,
    serial::*,
    timer::*,
};
use panic_halt as _;

// Hardware specific config for tim's office.
pub const CLOSET_STRIP_PIN: u8 = 0;
pub const WINDOW_STRIP_PIN: u8 = 3;
pub const DOOR_STRIP_PIN: u8 = 1;

// Typedefs to make the code below easier to read:
// Make sure the pin numbers above match the hardware pin numbers here
type LedPinCloset = hal::gpio::Pin0<Output<PullDown>>;
type LedPinWindow = hal::gpio::Pin3<Output<PullDown>>;
type LedPinDoor = hal::gpio::Pin1<Output<PullDown>>;
type PeriodicTimer = ConfiguredTimerChannel0;

// Real Values:
// The number of LEDs on each strip:
// const NUM_LEDS_WINDOW_STRIP: usize = 74;
// const NUM_LEDS_DOOR_STRIP: usize = 61;
// const NUM_LEDS_CLOSET_STRIP: usize = 34;

// Test Strip Values:
// The number of LEDs on each strip:
const NUM_LEDS_WINDOW_STRIP: usize = 4;
const NUM_LEDS_DOOR_STRIP: usize = 4;
const NUM_LEDS_CLOSET_STRIP: usize = 4;

// individual strips:
const CLOSET_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    pin: CLOSET_STRIP_PIN,
    led_count: NUM_LEDS_CLOSET_STRIP,
    reversed: false,
    color_order: strip::ColorOrder::BRG,
};
const WINDOW_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    pin: WINDOW_STRIP_PIN,
    led_count: NUM_LEDS_WINDOW_STRIP,
    reversed: false,
    color_order: strip::ColorOrder::BRG,
};
const DOOR_STRIP: strip::PhysicalStrip = strip::PhysicalStrip {
    pin: DOOR_STRIP_PIN,
    led_count: NUM_LEDS_DOOR_STRIP,
    reversed: true,
    color_order: strip::ColorOrder::BRG,
};

const NUM_STRIPS: usize = 3;
// combined strip group:
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

// let output_pins: [&mut dyn OutputPin<Error = Infallible>; 3] =
//     [&mut closet_led_pin, &mut window_led_pin, &mut door_led_pin];
// output_pins[1].set_high().ok();
//
// fn get_output_pin(pins: Parts, pin: u8) -> Option<dyn OutputPin<Error = Infallible>> {
//     return match pin {
//         0 => Some(pins.pin0.into_pull_down_output()),
//         1 => Some(pins.pin1.into_pull_down_output()),
//         2 => Some(pins.pin2.into_pull_down_output()),
//         3 => Some(pins.pin3.into_pull_down_output()),
//         4 => Some(pins.pin4.into_pull_down_output()),
//         5 => Some(pins.pin5.into_pull_down_output()),
//         6 => Some(pins.pin6.into_pull_down_output()),
//         7 => Some(pins.pin7.into_pull_down_output()),
//         8 => Some(pins.pin8.into_pull_down_output()),
//         9 => Some(pins.pin9.into_pull_down_output()),
//         10 => Some(pins.pin10.into_pull_down_output()),
//         11 => Some(pins.pin11.into_pull_down_output()),
//         12 => Some(pins.pin12.into_pull_down_output()),
//         13 => Some(pins.pin13.into_pull_down_output()),
//         14 => Some(pins.pin14.into_pull_down_output()),
//         15 => Some(pins.pin15.into_pull_down_output()),
//         16 => Some(pins.pin16.into_pull_down_output()),
//         17 => Some(pins.pin17.into_pull_down_output()),
//         18 => Some(pins.pin18.into_pull_down_output()),
//         19 => Some(pins.pin19.into_pull_down_output()),
//         20 => Some(pins.pin20.into_pull_down_output()),
//         21 => Some(pins.pin21.into_pull_down_output()),
//         22 => Some(pins.pin22.into_pull_down_output()),
//         _ => None,
//     };
// }

#[riscv_rt::entry]
fn main() -> ! {
    // make the logical strip:
    let _initial_animation = a::Animation::new(NUM_LEDS);

    let dp = pac::Peripherals::take().unwrap();
    let mut gpio_pins = dp.GLB.split();

    // Set up all the clocks we need
    let clocks = Strict::new()
        .use_pll(40_000_000u32.Hz())
        .sys_clk(SysclkFreq::Pll160Mhz)
        .uart_clk(UART_PLL_FREQ.Hz())
        .freeze(&mut gpio_pins.clk_cfg);

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

    // Make sure the pin numbers here match the const pin numbers on the strips:
    let mut closet_led_pin: LedPinCloset = gpio_pins.pin0.into_pull_down_output();
    closet_led_pin.set_low().ok();
    let mut window_led_pin: LedPinWindow = gpio_pins.pin3.into_pull_down_output();
    window_led_pin.set_low().ok();
    let mut door_led_pin: LedPinDoor = gpio_pins.pin1.into_pull_down_output();
    door_led_pin.set_low().ok();

    // Get the timer and initialize it to count up every clock cycle:
    let timers = dp.TIMER.split();
    let timer_ch0: PeriodicTimer = timers
        .channel0
        .set_clock_source(ClockSource::Fclk(&clocks), 160_000_000_u32.Hz());

    let mut pins = p::PinControl {
        p1: closet_led_pin,
        p2: window_led_pin,
        p3: door_led_pin,
        timer: timer_ch0,
    };

    let mut office_strip = strip::LogicalStrip::<NUM_LEDS>::new(&ALL_STRIPS);

    // get a millisecond delay for use with test patterns:
    let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

    // test color pattern before entering main program loop:
    let mut color = c::C_RED;
    office_strip.set_strip_to_solid_color(color);
    office_strip.send_all_sequential(&mut pins);
    d.delay_ms(1000).ok();
    color = c::C_GREEN;
    office_strip.set_strip_to_solid_color(color);
    office_strip.send_all_sequential(&mut pins);
    d.delay_ms(1000).ok();
    color = c::C_BLUE;
    office_strip.set_strip_to_solid_color(color);
    office_strip.send_all_sequential(&mut pins);
    d.delay_ms(1000).ok();
    office_strip.set_strip_to_solid_color(c::C_OFF);
    office_strip.send_all_sequential(&mut pins);
    d.delay_ms(1000).ok();

    for i in 0..NUM_LEDS {
        color = c::C_GREEN;
        office_strip.set_strip_to_solid_color(c::C_OFF); // because lazy
        office_strip.set_color_at_index(i, color);
        office_strip.send_all_sequential(&mut pins);
        d.delay_ms(1000).ok();
    }

    for i in (0..NUM_LEDS).rev() {
        color = c::C_BLUE;
        office_strip.set_strip_to_solid_color(c::C_OFF); // because lazy
        office_strip.set_color_at_index(i, color);
        office_strip.send_all_sequential(&mut pins);
        d.delay_ms(1000).ok();
    }

    loop {
        for i in 0..100 {
            color = c::Color::color_lerp(i, 0, 100, C_RED, C_GREEN);
            office_strip.set_strip_to_solid_color(color);
            office_strip.send_all_sequential(&mut pins);
            d.delay_ms(250).ok();
        }
        for i in 0..100 {
            color = c::Color::color_lerp(i, 0, 100, C_GREEN, C_BLUE);
            office_strip.set_strip_to_solid_color(color);
            office_strip.send_all_sequential(&mut pins);
            d.delay_ms(250).ok();
        }
        for i in 0..100 {
            color = c::Color::color_lerp(i, 0, 100, C_BLUE, C_RED);
            office_strip.set_strip_to_solid_color(color);
            office_strip.send_all_sequential(&mut pins);
            d.delay_ms(250).ok();
        }
    }
}
