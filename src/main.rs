#![no_std]
#![no_main]

pub mod animations;
pub mod colors;
pub mod leds;

use crate::animations as a;
use crate::colors as c;
use crate::leds::ws28xx as strip;

use bitvec::prelude::*;
use bitvec::ptr::Mut;
use bitvec::slice::BitSlice;
use bl602_hal as hal;
use core::cell::{Ref, RefCell};
use core::fmt::Write;
use core::ops::DerefMut;
use embedded_hal::digital::blocking::{OutputPin, ToggleableOutputPin};
use embedded_hal::timer::nb::CountDown;
use embedded_time::{duration::*, rate::*};
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    gpio::{Output, PullDown},
    interrupts::*,
    pac,
    prelude::*,
    serial::*,
    timer::*,
};
use nb::block;
use panic_halt as _;
use riscv::interrupt::Mutex;
use riscv::register::medeleg::clear_load_page_fault;

// Hardware specific config for tim's office.
pub const CLOSET_STRIP_PIN: u8 = 0;
pub const WINDOW_STRIP_PIN: u8 = 1;
pub const DOOR_STRIP_PIN: u8 = 3;

// Typedefs to make the code below easier to read:
// Make sure the pin numbers above match the hardware pin numbers here
type LedPinCloset = hal::gpio::Pin0<Output<PullDown>>;
type LedPinWindow = hal::gpio::Pin1<Output<PullDown>>;
type LedPinDoor = hal::gpio::Pin3<Output<PullDown>>;
type LedTimer = hal::timer::ConfiguredTimerChannel0;

// Set up global container for variables that will need to be accessed inside of the timer interrupt
static G_LED_PIN_CLOSET: Mutex<RefCell<Option<LedPinCloset>>> = Mutex::new(RefCell::new(None));
static G_LED_PIN_WINDOW: Mutex<RefCell<Option<LedPinWindow>>> = Mutex::new(RefCell::new(None));
static G_LED_PIN_DOOR: Mutex<RefCell<Option<LedPinDoor>>> = Mutex::new(RefCell::new(None));
static G_LED_TIMER: Mutex<RefCell<Option<LedTimer>>> = Mutex::new(RefCell::new(None));

// The number of LEDs on each strip:
const NUM_LEDS_WINDOW_STRIP: usize = 74;
const NUM_LEDS_DOOR_STRIP: usize = 61;
const NUM_LEDS_CLOSET_STRIP: usize = 34;

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

// combined strip group:
const ALL_STRIPS: [strip::PhysicalStrip; 3] = [CLOSET_STRIP, WINDOW_STRIP, DOOR_STRIP];

// The number of LEDs on each strip:
const MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH: usize = get_single_strip_buffer_max_length(&ALL_STRIPS);
const MAX_SINGLE_STRIP_BIT_BUFFER_LENGTH: usize = MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH * 8;

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
    // make the logical strip:
    let _initial_animation = a::Animation::new(NUM_LEDS);
    let mut office_strip = strip::LogicalStrip::<NUM_LEDS>::new(&ALL_STRIPS);

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
    let _ = closet_led_pin.set_low();
    let mut window_led_pin: LedPinWindow = gpio_pins.pin1.into_pull_down_output();
    let _ = window_led_pin.set_low();
    let mut door_led_pin: LedPinDoor = gpio_pins.pin3.into_pull_down_output();
    let _ = door_led_pin.set_low();

    // Get the timer and initialize it to count up every clock cycle:
    let timers = dp.TIMER.split();
    let mut timer_ch0 = timers
        .channel0
        .set_clock_source(ClockSource::Fclk(&clocks), 160_000_000_u32.Hz());

    //move the gpio pins & Timer to the global Mutex containers:
    // riscv::interrupt::free(|cs| G_LED_TIMER.borrow(*cs).replace(Some(timer_ch0)));
    // riscv::interrupt::free(|cs| G_LED_PIN_CLOSET.borrow(*cs).replace(Some(closet_led_pin)));
    // riscv::interrupt::free(|cs| G_LED_PIN_WINDOW.borrow(*cs).replace(Some(window_led_pin)));
    // riscv::interrupt::free(|cs| G_LED_PIN_DOOR.borrow(*cs).replace(Some(door_led_pin)));

    //only enable the timer interrupt after it's been borrowed in the global scope:
    // enable_interrupt(Interrupt::TimerCh0);

    // let mut color = c::C_RED;
    // office_strip.set_strip_to_solid_color(color);
    //office_strip.send_all_sequential(&mut pins);
    // color = c::C_GREEN;
    // office_strip.set_strip_to_solid_color(color);
    //office_strip.send_all_sequential(&mut pins);
    // color = c::C_BLUE;
    // office_strip.set_strip_to_solid_color(color);
    //office_strip.send_all_sequential(&mut pins);

    //
    // let mut index = 0_u64;
    //
    // #[derive(Default, Debug, Clone, Copy)]
    // struct LoopTimes {
    //     index: u64,
    //     target: Nanoseconds<u64>,
    //     time: Nanoseconds<u64>,
    // }

    // let mut loop_array: [LoopTimes; 10] = [LoopTimes::default(); 10];
    loop {
        // timer_ch0
        //     // .start((strip::StripTimings::WS2812_ADAFRUIT.full_cycle / 3).nanoseconds())
        //     .start(200.nanoseconds())
        //     .ok();

        periodic_start(
            &mut timer_ch0,
            (strip::StripTimings::WS2812_ADAFRUIT.full_cycle / 3).nanoseconds(),
        );
        closet_led_pin.set_low().ok();
        for _ in 0..900 {
            faster_wait(&mut timer_ch0);
        }
        // let mut loop_times = 0_u64;
        for i in 0..96 {
            // office_strip.set_strip_to_solid_color(c::Color::new(9, 3, 5));
            //office_strip.send_all_sequential(&mut pins);
            closet_led_pin.set_high().ok();
            faster_wait(&mut timer_ch0);
            faster_wait(&mut timer_ch0);
            // block!(timer_ch0.wait()).ok();
            closet_led_pin.set_low().ok();
            faster_wait(&mut timer_ch0);
            // block!(timer_ch0.wait()).ok();
            // block!(timer_ch0.wait()).ok();
            // index += 1;
            // loop_array[i].index = index;
            // let _ = timer_ch0.current_ticks();
            // loop_array[i].ticks = timer_ch0.current_ticks();
            // loop_array[i].time = timer_ch0.current_time();
            // let _ = timer_ch0.current_time();
        }
        //
        // for LoopTimes { index, target, time } in loop_array {
        //     writeln!(serial, "Loop: {}\t\tTarget: {}\t\tTime: {}\r", index, target, time).ok();
        // }
    }
}
fn periodic_start(timer: &mut ConfiguredTimerChannel0, time: impl Into<Nanoseconds<u64>>) {
    let time: Nanoseconds<u64> = time.into();
    timer.set_match2(time);
    timer.enable_match2_interrupt();
    timer.set_preload_value(0.nanoseconds());
    timer.set_preload(Preload::PreloadMatchComparator2);
    timer.enable();
}

fn faster_wait(timer: &mut ConfiguredTimerChannel0) {
    loop {
        if timer.is_match2() {
            timer.clear_match2_interrupt();
            break;
        }
    }
}

#[allow(non_snake_case)]
#[no_mangle]
fn TimerCh0(_trap_frame: &mut TrapFrame) {
    disable_interrupt(Interrupt::TimerCh0);
    clear_interrupt(Interrupt::TimerCh0);

    //since the free() disables interrupts, we can clear the match0 without enabling/disabling first:
    // riscv::interrupt::free(|cs| {
    //     if let Some(timer) = G_LED_TIMER.borrow(*cs).borrow_mut().deref_mut() {
    //         // if timer.is_match0() {
    //         //     timer.clear_match0_interrupt();
    //         //     if let Some(led_pin) = G_LED_PIN_CLOSET.borrow(*cs).borrow_mut().deref_mut() {
    //         //         led_pin.set_high().ok();
    //         //     }
    //         // }
    //         // if timer.is_match1() {
    //         //     timer.clear_match1_interrupt();
    //         //     if let Some(led_pin) = G_LED_PIN_CLOSET.borrow(*cs).borrow_mut().deref_mut() {
    //         //         led_pin.set_low().ok();
    //         //     }
    //         // }
    //         // if timer.is_match2() {
    //         //     timer.clear_match2_interrupt();
    //         //     if let Some(led_pin) = G_LED_PIN_CLOSET.borrow(*cs).borrow_mut().deref_mut() {
    //         //         led_pin.set_low().ok();
    //         //     }
    //         // }
    //     };
    // });
    enable_interrupt(Interrupt::TimerCh0);
}
