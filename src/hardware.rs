use crate::{NUM_STRIPS, leds::ws28xx::LogicalStrip};
use bl602_hal::timer::{ConfiguredTimerChannel0, ConfiguredTimerChannel1, Preload};
use core::convert::Infallible;
use embedded_hal::digital::blocking::OutputPin;
use embedded_time::duration::*;

pub type DynamicPin<'a> = &'a mut dyn OutputPin<Error = Infallible>;

pub struct HardwareController<'a, T>
where
    T: PeriodicTimer,
{
    pins: [DynamicPin<'a>; NUM_STRIPS],
    timer: T,
}

impl<'a, T> HardwareController<'a, T>
where
    T: PeriodicTimer,
{
    pub fn new(pins: [DynamicPin<'a>; NUM_STRIPS], timer: T) -> Self {
        HardwareController { pins, timer }
    }

    pub fn set_low(&mut self, pin: usize) {
        self.pins[pin].set_low().ok();
    }

    pub fn set_high(&mut self, pin: usize) {
        self.pins[pin].set_high().ok();
    }

    pub fn periodic_start(&mut self, time: impl Into<Nanoseconds<u64>>) {
        self.timer.periodic_start(time);
    }

    pub fn periodic_wait(&mut self) {
        self.timer.periodic_wait();
    }

    /// this will iterate over all the strips and send the led data in series:
    pub fn send_all_sequential(&mut self, logical_strip: &mut LogicalStrip)
    {
        let mut start_index = 0;

        for (pin_index, strip) in logical_strip.strips.iter().enumerate() {
            let end_index = start_index + strip.led_count;

            let current_strip_colors = &logical_strip.color_buffer[start_index..end_index];

            let byte_count = strip.led_count * 3;

            let byte_buffer = match strip.reversed {
                true => {
                    logical_strip.colors_to_bytes(current_strip_colors.iter().rev(), &strip.color_order)
                }
                false => logical_strip.colors_to_bytes(current_strip_colors.iter(), &strip.color_order),
            };

            let bit_slice = LogicalStrip::bytes_as_bit_slice(&byte_buffer[..byte_count]);

            strip.send_bits(self, pin_index, bit_slice.iter().by_ref());

            start_index = end_index;
        }
    }
}

pub trait PeriodicTimer {
    fn periodic_start(&mut self, time: impl Into<Nanoseconds<u64>>);
    fn periodic_wait(&mut self);
    fn periodic_check_timeout(&mut self) -> Result<(), TimerError>;
}

pub enum TimerError {
    WouldBlock,
}

macro_rules! setup_periodic_timer {
    ($timer:ident) => {
        impl PeriodicTimer for $timer {
            fn periodic_start(&mut self, time: impl Into<Nanoseconds<u64>>) {
                let time: Nanoseconds<u64> = time.into();
                self.set_match2(time);
                self.enable_match2_interrupt();
                self.set_preload_value(0.nanoseconds());
                self.set_preload(Preload::PreloadMatchComparator2);
                self.enable();
            }

            fn periodic_wait(&mut self) {
                loop {
                    if self.is_match2() {
                        self.clear_match2_interrupt();
                        break;
                    }
                }
            }

            fn periodic_check_timeout(&mut self) -> Result<(), TimerError> {
                if self.is_match2() {
                    self.clear_match2_interrupt();
                    return Ok(());
                } else {
                    return Err(TimerError::WouldBlock);
                }
            }
        }
    };
}

setup_periodic_timer!(ConfiguredTimerChannel0);
setup_periodic_timer!(ConfiguredTimerChannel1);
