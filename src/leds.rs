pub mod ws28xx {
    use crate::colors as c;
    use crate::hardware::{HardwareController, PeriodicTimer};
    use bitvec::prelude::*;
    use embedded_time::duration::*;

    pub struct StripTimings {
        pub zero_h: u32,
        pub one_h: u32,
        pub full_cycle: u32,
    }

    #[allow(unused_variables)]
    impl StripTimings {
        pub const WS2811_ADAFRUIT: StripTimings =
            StripTimings { zero_h: 500_u32, one_h: 1200_u32, full_cycle: 2500_u32 };
        pub const WS2812_ADAFRUIT: StripTimings =
            StripTimings { zero_h: 400_u32, one_h: 800_u32, full_cycle: 1250_u32 };
    }

    pub const WS2811_DELAY_LOOPS_BEFORE_SEND: u32 = 900;

    #[allow(clippy::upper_case_acronyms)]
    pub enum ColorOrder {
        RGB,
        RBG,
        GRB,
        GBR,
        BRG,
        BGR,
    }

    impl ColorOrder {
        pub fn offsets(&self) -> [usize; 3] {
            use ColorOrder::*;
            match self {
                RGB => [0, 1, 2],
                RBG => [0, 2, 1],
                GRB => [1, 0, 2],
                BRG => [1, 2, 0],
                GBR => [2, 0, 1],
                BGR => [2, 1, 0],
            }
        }
    }

    pub struct PhysicalStrip {
        pub led_count: usize,
        pub reversed: bool,
        pub color_order: ColorOrder,
        pub strip_timings: StripTimings,
    }

    impl PhysicalStrip {
        pub fn send_bits<'b, T>(
            &self,
            hc: &mut HardwareController<T>,
            pin_index: usize,
            bit_buffer: impl IntoIterator<Item = &'b bool>,
        ) where
            T: PeriodicTimer,
        {
            // restart the timer every time to make sure it's configured correctly and nobody has
            // changed its interrupt timing settings:
            hc.periodic_start((self.strip_timings.full_cycle / 3).nanoseconds());
            // keep the data pin low long enough for the leds to reset
            hc.set_low(pin_index);
            for _ in 0..WS2811_DELAY_LOOPS_BEFORE_SEND {
                hc.periodic_wait();
            }
            // iterate over the bits and send them to the pin with appropriate timing
            for bit in bit_buffer {
                match bit {
                    true => {
                        // on for 2/3 of the total time:
                        hc.set_high(pin_index);
                        hc.periodic_wait();
                        hc.periodic_wait();
                        hc.set_low(pin_index);
                        hc.periodic_wait();
                    }
                    false => {
                        // on for 1/3 of the total time:
                        hc.set_high(pin_index);
                        hc.periodic_wait();
                        hc.set_low(pin_index);
                        hc.periodic_wait();
                        hc.periodic_wait();
                    }
                }
            }
        }
    }

    pub struct LogicalStrip<'a> {
        pub color_buffer: &'a mut [c::Color],
        pub strips: &'a [PhysicalStrip],
    }

    impl<'a> LogicalStrip<'a> {
        pub fn new(color_buffer: &'a mut [c::Color], strips: &'a [PhysicalStrip]) -> Self {
            LogicalStrip { color_buffer, strips }
        }

        pub fn get_color_at_index(&self, index: usize) -> c::Color {
            self.color_buffer[index]
        }

        // this sets the color value in the color array at index:
        pub fn set_color_at_index(&mut self, index: usize, color: c::Color) {
            self.color_buffer[index].set_color(color);
        }

        // this fills the entire strip with a single color:
        pub fn set_strip_to_solid_color(&mut self, color: c::Color) {
            for c in &mut self.color_buffer.iter_mut() {
                c.set_color(color);
            }
        }

        pub fn colors_to_bytes(
            &self,
            colors: impl Iterator<Item = &'a c::Color>,
            color_order: &ColorOrder,
        ) -> [u8; crate::MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH] {
            let mut byte_buffer = [0_u8; crate::MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH];

            // Set the bytes in the RGB order for this strip
            let offsets = color_order.offsets();

            for (i, color) in colors.enumerate() {
                let base = i * 3;
                byte_buffer[base + offsets[0]] = color.r;
                byte_buffer[base + offsets[1]] = color.g;
                byte_buffer[base + offsets[2]] = color.b;
            }

            byte_buffer
        }

        // this takes an array of u8 color data and converts it into an array of bools
        pub fn bytes_as_bit_slice(byte_buffer: &[u8]) -> &BitSlice<Msb0, u8> {
            byte_buffer.view_bits::<Msb0>()
        }
    }
}
