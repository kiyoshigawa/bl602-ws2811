#![no_std]
#![no_main]

use bl602_ws2811::*;

use animations as a;
use animations::{Animatable, Animation, AnimationParameters};
use colors as c;
use default_animations as da;
use hardware::{DynamicPin, HardwareController};
use leds::ws28xx as strip;
use lighting_controller as lc;

use core::fmt::Write;
use embedded_time::rate::*;

use bl602_hal as hal;
use hal::{
    clock::{Clocks, Strict, SysclkFreq, UART_PLL_FREQ},
    gpio::*,
    pac,
    serial::*,
    timer::*,
};

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
    let clocks = init_clocks(&mut gpio.clk_cfg);

    // configure our two timer channels
    let (timer_ch0, mut timer_ch1) = init_timers(dp.TIMER, &clocks);

    // Set up uart output for debug printing. Since this microcontroller has a pin matrix,
    // we need to set up both the pins and the muxes
    let serial = init_usb_serial(
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

    let mut t_a = utility::default_translation_array::<NUM_LEDS>(0);

    // Make a single animation operating on the whole strip:
    let mut a = Animation::new(da::ANI_TEST, t_a, 60.Hz());

    let animation_array: [&mut dyn Animatable; 1] = [&mut a];

    let mut lc = lc::LightingController::new(strip, animation_array, 60_u32.Hz(), &mut timer_ch1);

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

fn init_clocks(config: &mut ClkCfg) -> Clocks {
    Strict::new()
        .use_pll(40_000_000u32.Hz())
        .sys_clk(SysclkFreq::Pll160Mhz)
        .uart_clk(UART_PLL_FREQ.Hz())
        .freeze(config)
}

fn init_timers(
    timer: pac::TIMER,
    clocks: &Clocks,
) -> (ConfiguredTimerChannel0, ConfiguredTimerChannel1) {
    let timers = timer.split();
    let timer_ch0 = timers
        .channel0
        .set_clock_source(ClockSource::Fclk(&clocks), 160_000_000_u32.Hz());

    let timer_ch1 = timers
        .channel1
        .set_clock_source(ClockSource::Fclk(&clocks), 160_000_000_u32.Hz());

    (timer_ch0, timer_ch1)
}

fn init_usb_serial<MODE>(
    uart: pac::UART,
    clocks: Clocks,
    baud_rate: Baud,
    tx_pin: Pin16<MODE>,
    rx_pin: Pin7<MODE>,
    tx_mux: UartMux0<Uart0Cts>,
    rx_mux: UartMux7<Uart0Cts>,
) -> impl Write {
    let tx = tx_pin.into_uart_sig0();
    let rx = rx_pin.into_uart_sig7();
    let tx_mux = tx_mux.into_uart0_tx();
    let rx_mux = rx_mux.into_uart0_rx();

    Serial::uart0(
        uart,
        Config::default().baudrate(baud_rate),
        ((tx, tx_mux), (rx, rx_mux)),
        clocks,
    )
}
