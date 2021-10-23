use crate::{PeriodicTimer, CLOSET_STRIP_PIN, DOOR_STRIP_PIN, WINDOW_STRIP_PIN};
use bl602_hal::timer::Preload;
use embedded_hal::digital::blocking::OutputPin;
use embedded_time::duration::*;

// Struct to hold the actual pins.
// All pins must have the OutputPin trait. The OutputPin trait allows
// them to be used with set_low() and set_high() even though they are
// technically different types.
pub struct PinControl<P1: OutputPin, P2: OutputPin, P3: OutputPin> {
    pub p1: P1,
    pub p2: P2,
    pub p3: P3,
    pub timer: PeriodicTimer,
}

impl<P1, P2, P3> PinControl<P1, P2, P3>
where
    P1: OutputPin,
    P2: OutputPin,
    P3: OutputPin,
{
    //noinspection RsSelfConvention
    // This allows us to use the pin number in a match statement to call the set_low() function.
    pub fn set_pin_low(pin: u8, pins: &mut PinControl<P1, P2, P3>) {
        match pin {
            CLOSET_STRIP_PIN => pins.p1.set_low().ok(),
            WINDOW_STRIP_PIN => pins.p2.set_low().ok(),
            DOOR_STRIP_PIN => pins.p3.set_low().ok(),
            _ => unreachable!(),
        };
    }
    //noinspection RsSelfConvention
    // This allows us to use the pin number in a match statement to call the set_high() function.
    pub fn set_pin_high(pin: u8, pins: &mut PinControl<P1, P2, P3>) {
        match pin {
            CLOSET_STRIP_PIN => pins.p1.set_high().ok(),
            WINDOW_STRIP_PIN => pins.p2.set_high().ok(),
            DOOR_STRIP_PIN => pins.p3.set_high().ok(),
            _ => unreachable!(),
        };
    }

    pub fn periodic_start(pins: &mut PinControl<P1, P2, P3>, time: impl Into<Nanoseconds<u64>>) {
        let time: Nanoseconds<u64> = time.into();
        let timer = &mut pins.timer;
        timer.set_match2(time);
        timer.enable_match2_interrupt();
        timer.set_preload_value(0.nanoseconds());
        timer.set_preload(Preload::PreloadMatchComparator2);
        timer.enable();
    }

    pub fn periodic_wait(pins: &mut PinControl<P1, P2, P3>) {
        let timer = &mut pins.timer;
        loop {
            if timer.is_match2() {
                timer.clear_match2_interrupt();
                break;
            }
        }
    }
}
