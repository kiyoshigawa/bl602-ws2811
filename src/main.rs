#![no_std]
#![no_main]

pub mod animations;
pub mod colors;
pub mod pins;
use crate::animations as a;
use crate::colors as c;
use crate::pins as p;
use bl602_hal as hal;
use core::fmt::Write;
use core::mem::MaybeUninit;
use embedded_hal::delay::blocking::DelayMs;
use embedded_hal::digital::blocking::{OutputPin, ToggleableOutputPin};
use embedded_time::duration::Milliseconds;
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    delay::McycleDelay,
    gpio::{Output, PullDown},
    interrupts::*,
    pac,
    prelude::*,
    serial::*,
    timer::*,
};
use panic_halt as _;

/// macro to add Push trait to gpio pins:
/// this wraps the pins' set_high() and set_low() functions in our_set_* wrappers.
macro_rules! push {
    ($p:ty) => {
        impl p::Push for $p {
            fn our_set_low(&mut self) {
                self.set_low().unwrap();
            }
            fn our_set_high(&mut self) {
                self.set_high().unwrap();
            }
        }
    };
}

// make sure to add the pins you're using here and in pins.rs:
push!(bl602_hal::gpio::Pin0<Output<PullDown>>);
push!(bl602_hal::gpio::Pin1<Output<PullDown>>);
push!(bl602_hal::gpio::Pin3<Output<PullDown>>);

// readability consts:
const ONE: bool = true;
const ZERO: bool = false;

// Based on the clock speed of 160MHz as noted in the main() function below,
// one clock is
const CORE_PERIOD_NS: f32 = 6.25;

// Timing values for our 800kHz WS2811 Strips in nanoseconds:
const WS2811_0H_TIME_NS: u32 = 200;
const WS2811_1H_TIME_NS: u32 = 600;
const WS2811_FULL_CYCLE_TIME_NS: u32 = 1250;

// Timing Values converted to equivalent clock cycle values:
const WS2811_0H_TIME_CLOCKS: u64 = (WS2811_0H_TIME_NS as f32 / CORE_PERIOD_NS) as u64;
const WS2811_1H_TIME_CLOCKS: u64 = (WS2811_1H_TIME_NS as f32 / CORE_PERIOD_NS) as u64;
const WS2811_FULL_CYCLE_CLOCKS: u64 = (WS2811_FULL_CYCLE_TIME_NS as f32 / CORE_PERIOD_NS) as u64;

// This is how much to offset from the clock cycle measurement before actually sending data to the strips
// the value was determined experimentally, tweak as needed for consistency
const SEND_START_OFFSET_DELAY_CLOCKS: u64 = 50000;

// The number of LEDs on each strip:
const NUM_LEDS_WINDOW_STRIP: usize = 74;
const NUM_LEDS_DOOR_STRIP: usize = 61;
const NUM_LEDS_CLOSET_STRIP: usize = 34;
const MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH: usize = get_single_strip_buffer_max_length(&ALL_STRIPS);
const MAX_SINGLE_STRIP_BIT_BUFFER_LENGTH: usize = MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH * 8;

// individual strips:
const CLOSET_STRIP: WS2811PhysicalStrip = WS2811PhysicalStrip {
    pin: p::CLOSET_STRIP_PIN,
    led_count: NUM_LEDS_CLOSET_STRIP,
    reversed: false,
    _color_order: ColorOrder::BRG,
};
const WINDOW_STRIP: WS2811PhysicalStrip = WS2811PhysicalStrip {
    pin: p::WINDOW_STRIP_PIN,
    led_count: NUM_LEDS_WINDOW_STRIP,
    reversed: false,
    _color_order: ColorOrder::BRG,
};
const DOOR_STRIP: WS2811PhysicalStrip = WS2811PhysicalStrip {
    pin: p::DOOR_STRIP_PIN,
    led_count: NUM_LEDS_DOOR_STRIP,
    reversed: true,
    _color_order: ColorOrder::BRG,
};

// combined strip group:
const ALL_STRIPS: [WS2811PhysicalStrip; 3] = [CLOSET_STRIP, WINDOW_STRIP, DOOR_STRIP];

// calculate the total number of LEDs from the above values:
const NUM_LEDS: usize = get_total_num_leds(&ALL_STRIPS);

#[allow(dead_code)]
#[allow(clippy::upper_case_acronyms)]
enum ColorOrder {
    RGB,
    RBG,
    GRB,
    GBR,
    BRG,
    BGR,
}

// a timer-based GPIO pin to control for testing:
static mut GPIO5: MaybeUninit<hal::gpio::Pin5<Output<PullDown>>> = MaybeUninit::uninit();
static mut TIMER_CH0: MaybeUninit<hal::timer::ConfiguredTimerChannel0> = MaybeUninit::uninit();

fn get_gpio5() -> &'static mut hal::gpio::Pin5<Output<PullDown>> {
    unsafe { &mut *GPIO5.as_mut_ptr() }
}

fn get_timer_ch0() -> &'static mut hal::timer::ConfiguredTimerChannel0 {
    unsafe { &mut *TIMER_CH0.as_mut_ptr() }
}

const fn get_total_num_leds(strips: &[WS2811PhysicalStrip]) -> usize {
    let mut index = 0;
    let mut total = 0;
    while index < strips.len() {
        total += strips[index].led_count;
        index += 1;
    }
    total
}

const fn get_single_strip_buffer_max_length(strips: &[WS2811PhysicalStrip]) -> usize {
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

struct WS2811PhysicalStrip {
    pin: u8,
    led_count: usize,
    reversed: bool,
    _color_order: ColorOrder,
}

impl WS2811PhysicalStrip {
    fn send_bits<P1, P2, P3>(&self, pins: &mut p::PinControl<P1, P2, P3>, timings: &[(u64, u64)])
    where
        P1: OutputPin + p::Push,
        P2: OutputPin + p::Push,
        P3: OutputPin + p::Push,
    {
        for timing in timings {
            delay_until(timing.0);
            p::PinControl::push_high(self.pin, pins);
            delay_until(timing.1);
            p::PinControl::pull_low(self.pin, pins);
        }
    }
}

struct LogicalStrip<'a, const NUM_LEDS: usize> {
    color_buffer: [c::Color; NUM_LEDS],
    strips: &'a [WS2811PhysicalStrip],
    animation: a::Animation,
}

impl<'a, const NUM_LEDS: usize> LogicalStrip<'a, NUM_LEDS> {
    fn new(strips: &'a [WS2811PhysicalStrip], animation: a::Animation) -> Self {
        LogicalStrip::<NUM_LEDS> {
            color_buffer: [c::Color::default(); NUM_LEDS],
            strips,
            animation,
        }
    }

    //this sets the color value in the color array at index:
    fn set_color_at_index(&mut self, index: usize, color: c::Color) {
        self.color_buffer[index].r = color.r;
        self.color_buffer[index].g = color.g;
        self.color_buffer[index].b = color.b;
    }

    // this fills the entire strip with a single color:
    fn set_strip_to_solid_color(&mut self, color: c::Color) {
        for i in 0..self.color_buffer.len() {
            self.set_color_at_index(i, color);
        }
    }

    // this will iterate over all the strips and send the led data in series:
    fn send_all_sequential<P1, P2, P3>(&self, pins: &mut p::PinControl<P1, P2, P3>)
    where
        P1: OutputPin + p::Push,
        P2: OutputPin + p::Push,
        P3: OutputPin + p::Push,
    {
        let mut start_index = 0;

        for strip in self.strips {
            let end_index = start_index + strip.led_count;

            // generate byte array from color array (taking care of color order)
            let current_strip_colors = &self.color_buffer[start_index..end_index];
            let byte_count = strip.led_count * 3;
            let bit_count = byte_count * 8;
            let mut byte_buffer = [0_u8; MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH];
            if strip.reversed {
                for (i, color) in current_strip_colors.iter().rev().enumerate() {
                    let base = i * 3;
                    byte_buffer[base] = color.g;
                    byte_buffer[base + 1] = color.r;
                    byte_buffer[base + 2] = color.b;
                }
            } else {
                for (i, color) in current_strip_colors.iter().enumerate() {
                    let base = i * 3;
                    byte_buffer[base] = color.g;
                    byte_buffer[base + 1] = color.r;
                    byte_buffer[base + 2] = color.b;
                }
            }

            // from byte array to bit array
            let mut bit_buffer = [ZERO; MAX_SINGLE_STRIP_BIT_BUFFER_LENGTH];
            for (i, byte) in byte_buffer.iter().take(byte_count).enumerate() {
                //the base is the 0th bit of that byte's index in terms of total bits:
                let base = i * 8;
                for bit in 0..8_u8 {
                    //have to use (7 - bit) so it sends MSB first:
                    bit_buffer[base + (7 - bit) as usize] = match (byte >> bit) & 0x01 {
                        0x01 => ONE,
                        0x00 => ZERO,
                        _ => unreachable!(),
                    };
                }
            }

            // from bit array to timing array
            let mut timings = [(0_u64, 0_u64); MAX_SINGLE_STRIP_BIT_BUFFER_LENGTH];
            for (i, &bit) in bit_buffer.iter().take(bit_count).enumerate() {
                let bit_timing = match bit {
                    ONE => WS2811_1H_TIME_CLOCKS,
                    ZERO => WS2811_0H_TIME_CLOCKS,
                };
                let base_time = WS2811_FULL_CYCLE_CLOCKS * i as u64;
                timings[i] = (base_time, base_time + bit_timing);
            }

            // add clock + offset to timing array
            let offset_clocks = SEND_START_OFFSET_DELAY_CLOCKS;
            let clock_and_offset = McycleDelay::get_cycle_count() + offset_clocks;
            for timing in timings.iter_mut() {
                timing.0 += clock_and_offset;
                timing.1 += clock_and_offset;
            }

            // call send bits and send the timing array
            strip.send_bits(pins, &timings);

            start_index = end_index;
        }
    }
}

// this is a delay function that will prevent progress to a specified number of
// clock cycles as measured by the get_cycle_count() function.
fn delay_until(clocks: u64) {
    loop {
        if McycleDelay::get_cycle_count() > clocks {
            break;
        }
    }
}

#[riscv_rt::entry]
fn main() -> ! {
    // make the logical strip:
    let initial_animation = a::Animation::new(NUM_LEDS);
    let mut office_strip = LogicalStrip::<NUM_LEDS>::new(&ALL_STRIPS, initial_animation);

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

    serial.write_str("Debug Serial Initialized...\r\n").ok();

    // make sure the pin numbers here match the const pin numbers and macros above and in pins.rs:
    let closet_led_control_gpio = gpio_pins.pin0.into_pull_down_output();
    let window_led_control_gpio = gpio_pins.pin1.into_pull_down_output();
    let door_led_control_gpio = gpio_pins.pin3.into_pull_down_output();
    let mut pins = p::PinControl {
        p1: closet_led_control_gpio,
        p2: window_led_control_gpio,
        p3: door_led_control_gpio,
    };

    // timer-controlled blinky LED for testing:
    let mut gpio5 = gpio_pins.pin5.into_pull_down_output();
    gpio5.set_low().unwrap();

    let timers = dp.TIMER.split();
    let timer_ch0 = timers
        .channel0
        .set_clock_source(ClockSource::Clock1Khz, 1_000_u32.Hz());
    timer_ch0.enable_match0_interrupt();
    timer_ch0.set_preload_value(Milliseconds::new(0));
    timer_ch0.set_preload(hal::timer::Preload::PreloadMatchComparator0);
    timer_ch0.set_match0(Milliseconds::new(1000_u32));
    timer_ch0.enable();

    unsafe {
        *(GPIO5.as_mut_ptr()) = gpio5;
        *(TIMER_CH0.as_mut_ptr()) = timer_ch0;
    }

    enable_interrupt(Interrupt::TimerCh0);

    // Create a blocking delay function based on the current cpu frequency
    let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

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

    loop {
        // for (i, color) in c::R_ROYGBIV
        //     .colors
        //     .iter()
        //     .take(c::R_ROYGBIV.num_colors)
        //     .enumerate()
        // {
        //     for j in 0..100 {
        //         let current_color = color.unwrap_or(c::C_OFF);
        //         let next_color: c::Color;
        //         if i != c::R_ROYGBIV.num_colors - 1 {
        //             next_color = c::R_ROYGBIV.colors[i + 1].unwrap_or(c::C_OFF);
        //         } else {
        //             next_color = c::R_ROYGBIV.colors[0].unwrap_or(c::C_OFF);
        //         }
        //         let intermediate_color =
        //             c::Color::color_lerp(j as i32, 0, 100, current_color, next_color);
        //         office_strip.set_strip_to_solid_color(intermediate_color);
        //         office_strip.send_all_sequential(&mut pins);
        //         d.delay_ms(10).ok();
        //     }
        // }
        office_strip.set_strip_to_solid_color(c::Color::new(9, 3, 5));
        office_strip.send_all_sequential(&mut pins);
        d.delay_ms(1000).ok();
    }
}

#[allow(non_snake_case)]
#[no_mangle]
fn TimerCh0(_trap_frame: &mut TrapFrame) {
    disable_interrupt(Interrupt::TimerCh0);
    get_timer_ch0().disable();

    clear_interrupt(Interrupt::TimerCh0);
    get_timer_ch0().clear_match0_interrupt();

    get_gpio5().toggle().unwrap();

    get_timer_ch0().enable();
    enable_interrupt(Interrupt::TimerCh0);
}
