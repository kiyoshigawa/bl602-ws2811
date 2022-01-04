use core::ops::Index;

use crate::{
    animations::{Direction, MAX_OFFSET},
    colors::{Color, Rainbow},
};

use bl602_hal as hal;
use core::fmt::Write;
use embedded_time::rate::*;
use hal::{
    clock::{Clocks, Strict, SysclkFreq, UART_PLL_FREQ},
    gpio::*,
    pac,
    serial::*,
    timer::*,
};

pub fn init_clocks(config: &mut ClkCfg) -> Clocks {
    Strict::new()
        .use_pll(40_000_000u32.Hz())
        .sys_clk(SysclkFreq::Pll160Mhz)
        .uart_clk(UART_PLL_FREQ.Hz())
        .freeze(config)
}

pub fn init_timers(
    timer: pac::TIMER,
    clocks: &Clocks,
) -> (ConfiguredTimerChannel0, ConfiguredTimerChannel1) {
    let timers = timer.split();
    let timer_ch0 = timers
        .channel0
        .set_clock_source(ClockSource::Fclk(clocks), 160_000_000_u32.Hz());

    let timer_ch1 = timers
        .channel1
        .set_clock_source(ClockSource::Fclk(clocks), 160_000_000_u32.Hz());

    (timer_ch0, timer_ch1)
}

pub fn init_usb_serial<MODE>(
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

    Serial::uart0(uart, Config::default().baudrate(baud_rate), ((tx, tx_mux), (rx, rx_mux)), clocks)
}

pub fn convert_ns_to_frames(nanos: u64, frame_rate: Hertz) -> usize {
    (nanos * frame_rate.integer() as u64 / 1_000_000_000_u64) as usize
}

pub fn convert_ms_to_frames(millis: u64, frame_rate: Hertz) -> usize {
    (millis * frame_rate.integer() as u64 / 1_000_u64) as usize
}

/// Returns a translation array beginning with index `start_at` and
/// incrementing until reaching the desired `SIZE`
pub fn default_translation_array<const SIZE: usize>(start_at: usize) -> [usize; SIZE] {
    let mut result: [usize; SIZE] = [0; SIZE];
    for (index, value) in result.iter_mut().enumerate() {
        *value = start_at + index;
    }
    result
}

pub fn get_random_offset() -> u16 {
    riscv::register::mcycle::read64() as u16
}

pub fn shift_offset(starting_offset: u16, frames: Progression, direction: Direction) -> u16 {
    if frames.total == 0 {
        return starting_offset;
    }
    let max_offset = MAX_OFFSET as usize;
    let starting_offset = starting_offset as usize;
    let offset_shift = match direction {
        Direction::Positive => max_offset * frames.get_current() / frames.total,
        Direction::Negative => max_offset * (frames.total - frames.get_current()) / frames.total,
        Direction::Stopped => 0,
    };
    (starting_offset + offset_shift) as u16
}

pub struct ReversibleRainbow<'a> {
    backer: Rainbow<'a>,
    is_forward: bool,
}

impl<'a> ReversibleRainbow<'a> {
    pub fn len(&self) -> usize {
        self.backer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.backer.is_empty()
    }
}

impl<'a> Index<usize> for ReversibleRainbow<'a> {
    type Output = Color;

    fn index(&self, index: usize) -> &Self::Output {
        match self.is_forward {
            true => &self.backer[index],
            false => &self.backer[self.backer.len() - 1 - index],
        }
    }
}

pub trait FadeRainbow {
    fn rainbow(&self) -> &StatefulRainbow;
    fn frames(&self) -> &Progression;

    fn calculate_fade_color(&self) -> Color {
        let (rainbow, frames) = (self.rainbow(), self.frames());

        let current_color = rainbow.current_color();
        if frames.total == 0 {
            return current_color;
        }
        let next_color = rainbow.peek_next_color();
        current_color.lerp_with(next_color, *frames)
    }

    fn current_fade_color(&self) -> Color {
        self.rainbow().current_color()
    }
}

pub trait MarchingRainbow {
    fn rainbow(&self) -> &StatefulRainbow;
    fn frames(&self) -> &Progression;

    fn current_rainbow_color(&self) -> Color {
        self.rainbow().current_color()
    }
}

pub trait MarchingRainbowMut {
    fn rainbow_mut(&mut self) -> &mut StatefulRainbow;
    fn frames_mut(&mut self) -> &mut Progression;

    /// Advances the rainbow color and resets the frame count
    fn advance_rainbow_color(&mut self) {
        self.rainbow_mut().increment();
        self.frames_mut().reset();
    }
}

pub struct TimedRainbows<'a, 'b> {
    pub fade_rainbow: &'b mut StatefulRainbow<'a>,
    pub incremental_rainbow: &'b mut StatefulRainbow<'a>,
    pub frames: &'b mut Progression,
}

impl<'a, 'b> FadeRainbow for TimedRainbows<'a, 'b> {
    fn rainbow(&self) -> &StatefulRainbow {
        self.fade_rainbow
    }
    fn frames(&self) -> &Progression {
        self.frames
    }
}

impl<'a, 'b> MarchingRainbow for TimedRainbows<'a, 'b> {
    fn rainbow(&self) -> &StatefulRainbow {
        self.incremental_rainbow
    }
    fn frames(&self) -> &Progression {
        self.frames
    }
}

impl<'a, 'b> MarchingRainbowMut for TimedRainbows<'a, 'b> {
    fn rainbow_mut(&mut self) -> &'a mut StatefulRainbow {
        self.incremental_rainbow
    }
    fn frames_mut(&mut self) -> &mut Progression {
        self.frames
    }
}

pub struct StatefulRainbow<'a> {
    pub backer: ReversibleRainbow<'a>,
    pub position: Progression,
}

impl<'a> StatefulRainbow<'a> {
    pub fn new(rainbow: &'a [Color], is_forward: bool) -> StatefulRainbow<'a> {
        let position = Progression::new(rainbow.len());
        let backer = ReversibleRainbow { backer: rainbow, is_forward };
        Self { backer, position }
    }

    pub fn current_color(&self) -> Color {
        self.backer[self.position.get_current() as usize]
    }

    pub fn decrement(&mut self) {
        self.position.decrement();
    }

    pub fn increment(&mut self) {
        self.position.increment();
    }

    pub fn peek_next_color(&self) -> Color {
        self.backer[self.position.peek_next() as usize]
    }

    pub fn peek_last_color(&self) -> Color {
        self.backer[self.position.peek_prev() as usize]
    }

    pub fn reset(&mut self) {
        self.position.reset();
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct Progression {
    current: usize,
    pub total: usize,
    pub is_forward: bool,
}

impl Progression {
    pub fn new(total: usize) -> Self {
        Self { current: 0, total, is_forward: true }
    }

    pub fn reverse_direction(&mut self) {
        self.is_forward = !self.is_forward;
    }

    fn is_mono(&self) -> bool {
        self.total <= 1
    }

    pub fn is_first_frame(&self) -> bool {
        self.current == 0
    }

    pub fn get_current(&self) -> usize {
        if self.is_mono() {
            return 0;
        }
        match self.is_forward {
            true => self.current,
            false => self.total - 1 - self.current,
        }
    }

    pub fn set_current(&mut self, value: usize) {
        if self.is_mono() {
            return;
        }
        let value = value % self.total;
        self.current = value;
    }

    pub fn decrement(&mut self) {
        if self.is_mono() {
            return;
        }
        self.current = self.peek_prev();
    }

    pub fn checked_decrement(&mut self) -> bool {
        if self.is_mono() {
            return false;
        }
        self.decrement();
        self.current == self.total - 1
    }

    pub fn increment(&mut self) {
        if self.is_mono() {
            return;
        }
        self.current = self.peek_next();
    }

    pub fn checked_increment(&mut self) -> bool {
        if self.is_mono() {
            return false;
        }
        self.increment();
        self.current == 0
    }

    pub fn peek_next(&self) -> usize {
        self.up_one()
    }

    pub fn peek_prev(&self) -> usize {
        self.down_one()
    }

    fn up_one(&self) -> usize {
        if self.is_mono() {
            return 0;
        }
        (self.current + 1) % self.total
    }

    fn down_one(&self) -> usize {
        if self.is_mono() {
            return 0;
        }
        (self.current + self.total - 1) % self.total
    }

    pub fn reset(&mut self) {
        self.current = 0
    }
}

impl Color {
    pub fn lerp_with(&self, to_color: Color, factor: Progression) -> Color {
        Color::color_lerp(factor.get_current() as i32, 0, factor.total as i32, *self, to_color)
    }
}
