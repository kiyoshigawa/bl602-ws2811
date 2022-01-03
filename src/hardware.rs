use bl602_hal::timer::{ConfiguredTimerChannel0, ConfiguredTimerChannel1, Preload};
use core::convert::Infallible;
use embedded_hal::digital::blocking::OutputPin;
use embedded_time::duration::*;

pub type DynamicPin<'a> = &'a mut dyn OutputPin<Error = Infallible>;

pub struct HardwareController<'a, T>
{
    pins: &'a mut [DynamicPin<'a>],
    timer: T,
}

impl <'a, T> HardwareController<'a, T> {
    pub fn new(pins: &'a mut [DynamicPin<'a>], timer: T) -> Self {
        HardwareController { pins, timer }
    }

    pub fn set_low(&mut self, pin: usize) {
        self.pins[pin].set_low().ok();
    }

    pub fn set_high(&mut self, pin: usize) {
        self.pins[pin].set_high().ok();
    }

}

impl<'a, T> PeriodicTimer for HardwareController<'a, T>
where
    T: PeriodicTimer,
{
    fn periodic_start(&mut self, time: impl Into<Nanoseconds<u64>>) {
        self.timer.periodic_start(time);
    }

    fn periodic_wait(&mut self) {
        self.timer.periodic_wait();
    }

    fn periodic_check_timeout(&mut self) -> Result<(), TimerError> {
        self.timer.periodic_check_timeout()
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
