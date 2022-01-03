#![no_std]
#![no_main]

use bl602_ws2811::*;

use animations::{AnimationParameters, Animatable, Animation};
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

    // set aside memory for a logical strip
    let mut memory_buffer = [0; NUM_LEDS * 3];
    let mut color_buffer: [c::Color; NUM_LEDS] = [c::Color::default(); NUM_LEDS];
    let office_strip = strip::LogicalStrip::new(&mut memory_buffer, &mut color_buffer, &ALL_STRIPS);

    // The order of pins here needs to match the array of strips passed into LogicalStrip::new()
    let mut pins = [
        &mut gpio.pin0.into_pull_down_output() as DynamicPin,
        &mut gpio.pin3.into_pull_down_output(),
        &mut gpio.pin1.into_pull_down_output(),
    ];

    let mut hc = HardwareController::new(&mut pins, timer_ch0);

    // initialize translation array
    let s_ta = utility::default_translation_array::<NUM_LEDS_SOUTH_WALL>(0);
    let s_ta_off = utility::default_translation_array::<NUM_LEDS_SOUTH_WALL>(0);
    //Frame Rate to run at:
    let frame_rate = 60.Hz();

    // Make a single animation operating on the whole strip:
    let ani_bg_test = AnimationParameters { bg: da::BG_TEST, fg: da::FG_OFF, trigger: da::TRIGGER_OFF };

    let s_a = &mut Animation::new(ani_bg_test, s_ta,  frame_rate);
    let s_a_off = &mut Animation::new(da::ANI_ALL_OFF, s_ta_off, frame_rate);

    let animation_array: [&mut dyn Animatable; 1] = [s_a];

    let mut lc =
        lc::LightingController::new(office_strip, animation_array, frame_rate, &mut timer_ch1);

    let mut last_time = riscv::register::mcycle::read64();
    loop {
        lc.update(&mut hc);
        if riscv::register::mcycle::read64() - last_time > 160_000_000 {
            last_time = riscv::register::mcycle::read64();
            writeln!(serial, "average time(cycles): {:?}", crate::profile_average()).ok();
        }
    }
}


fn init_clocks(config: &mut ClkCfg) -> Clocks {
    Strict::new()
        .use_pll(40_000_000u32.Hz())
        .sys_clk(SysclkFreq::Pll160Mhz)
        .uart_clk(UART_PLL_FREQ.Hz())
        .freeze(config)
}

fn init_timers(timer: pac::TIMER, clocks: &Clocks) -> (ConfiguredTimerChannel0, ConfiguredTimerChannel1) {
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

