use crate::leds::ws28xx::PhysicalStrip;
use crate::BL602_NUM_PINS;
use bl602_hal::gpio::Parts;
use bl602_hal::timer::{ConfiguredTimerChannel0, Preload};
use core::convert::Infallible;
use embedded_hal::digital::blocking::OutputPin;
use embedded_time::duration::*;

pub type DynamicPin<'a> = &'a mut dyn OutputPin<Error = Infallible>;
type Timer = ConfiguredTimerChannel0;
pub struct HardwareController<'a> {
    pins: [DynamicPin<'a>; 4],
    timer: Timer,
}

#[derive(Copy, Clone)]
pub struct NoPin;
impl OutputPin for NoPin {
    type Error = Infallible;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a> HardwareController<'a> {
    pub fn new(pins: [DynamicPin<'a>; 4], timer: Timer) -> Self {
        let mut hc = HardwareController { pins, timer };
        // hc.setup_strip_pins(gpio_pins, strips);
        hc
    }

    pub fn set_low(&mut self, pin: usize) {
        self.pins[pin].set_low();
    }

    pub fn set_high(&mut self, pin: usize) {
        self.pins[pin].set_high();
    }

    pub fn periodic_start(&mut self, time: impl Into<Nanoseconds<u64>>) {
        self.timer.periodic_start(time);
    }

    pub fn periodic_wait(&mut self) {
        self.timer.periodic_wait();
    }

    // /// This sets up output pins used by the logical strip and gives the logical strip access
    // /// to them for sending data. It's long and ugly, and only works for the BL602, but it works,
    // /// and it means you don't need to worry about tracking down any pin changes throughout the
    // /// codebase.
    // fn setup_strip_pins(&mut self, pins: &mut Parts, strips: &[PhysicalStrip]) {
    //     for strip in strips {
    //         match strip.pin {
    //             0 => {
    //                 self.pins[strip.pin] = &mut pins.pin0.into_pull_down_output();
    //             }
    //             1 => {
    //                 self.pins[strip.pin] = &mut pins.pin1.into_pull_down_output();
    //             }
    //             // 2 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin2.into_pull_down_output());
    //             // }
    //             // 3 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin3.into_pull_down_output());
    //             // }
    //             // 4 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin4.into_pull_down_output());
    //             // }
    //             // 5 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin5.into_pull_down_output());
    //             // }
    //             // 6 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin6.into_pull_down_output());
    //             // }
    //             // 7 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin7.into_pull_down_output());
    //             // }
    //             // 8 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin8.into_pull_down_output());
    //             // }
    //             // 9 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin9.into_pull_down_output());
    //             // }
    //             // 10 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin10.into_pull_down_output());
    //             // }
    //             // 11 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin11.into_pull_down_output());
    //             // }
    //             // 12 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin12.into_pull_down_output());
    //             // }
    //             // 13 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin13.into_pull_down_output());
    //             // }
    //             // 14 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin14.into_pull_down_output());
    //             // }
    //             // 15 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin15.into_pull_down_output());
    //             // }
    //             // 16 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin16.into_pull_down_output());
    //             // }
    //             // 17 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin17.into_pull_down_output());
    //             // }
    //             // 18 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin18.into_pull_down_output());
    //             // }
    //             // 19 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin19.into_pull_down_output());
    //             // }
    //             // 20 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin20.into_pull_down_output());
    //             // }
    //             // 21 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin21.into_pull_down_output());
    //             // }
    //             // 22 => {
    //             //     self.pins[strip.pin] = Some(&mut pins.pin22.into_pull_down_output());
    //             // }
    //             _ => {
    //                 self.pins[strip.pin] = &mut NoPin;
    //             }
    //         };
    //     }
    // }
}

pub trait PeriodicTimer {
    fn periodic_start(&mut self, time: impl Into<Nanoseconds<u64>>);
    fn periodic_wait(&mut self);
}

impl PeriodicTimer for ConfiguredTimerChannel0 {
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
}
