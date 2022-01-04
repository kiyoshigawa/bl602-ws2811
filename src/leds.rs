pub mod ws28xx {
    use crate::{
        colors as c,
        hardware::{HardwareController, PeriodicTimer},
    };
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
            bit_buffer: impl IntoIterator<Item = bool>,
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
            let mut bit_iter = bit_buffer.into_iter();
            let mut next_bit = bit_iter.next();

            // load the next bit during the longest pause
            while let Some(bit) = next_bit {
                hc.set_high(pin_index);
                match bit {
                    true => {
                        // on for 2/3 of the total time:
                        next_bit = bit_iter.next();
                        while hc.periodic_check_timeout().is_err() {}
                        hc.periodic_wait();
                        hc.set_low(pin_index);
                        hc.periodic_wait();
                    }
                    false => {
                        // on for 1/3 of the total time:
                        hc.periodic_wait();
                        hc.set_low(pin_index);
                        next_bit = bit_iter.next();
                        while hc.periodic_check_timeout().is_err() {}
                        hc.periodic_wait();
                    }
                }
            }
        }
    }

    pub struct LogicalStrip<'a> {
        _byte_buffer: &'a mut [u8],
        color_buffer: &'a mut [c::Color],
        strips: &'a [PhysicalStrip],
    }

    impl<'a> LogicalStrip<'a> {
        pub fn new(
            byte_buffer: &'a mut [u8],
            color_buffer: &'a mut [c::Color],
            strips: &'a [PhysicalStrip],
        ) -> Self {
            LogicalStrip { color_buffer, strips, _byte_buffer: byte_buffer }
        }

        pub fn get_color_at_index(&self, index: usize) -> c::Color {
            self.color_buffer[index]
        }

        // this sets the color value in the color array at index:
        pub fn set_color_at_index(&mut self, index: usize, color: c::Color) {
            self.color_buffer[index].set_color(color);

            let mut index = index;
            let (belongs_to, start) = self.belongs_to(index);

            let [r, g, b] = belongs_to.color_order.offsets();

            let mut as_bytes = [0; 3];
            as_bytes[r] = color.r;
            as_bytes[g] = color.g;
            as_bytes[b] = color.b;

            if belongs_to.reversed {
                let index_offset = index - start;
                let reversed_index_offset = belongs_to.led_count - 1 - index_offset;
                index = start + reversed_index_offset;
            }

            for i in 0..as_bytes.len() {
                self._byte_buffer[(3 * index) + i] = as_bytes[i];
            }
        }

        fn belongs_to(&self, index: usize) -> (&PhysicalStrip, usize) {
            let (mut start, mut end) = (0, 0);

            for strip in self.strips {
                end += strip.led_count;

                if index < end {
                    return (strip, start);
                };

                start = end;
            }
            panic!("Index out of bounds");
        }

        // this fills the entire strip with a single color:
        pub fn set_strip_to_solid_color(&mut self, color: c::Color) {
            for c in &mut self.color_buffer.iter_mut() {
                c.set_color(color);
            }
        }

        // this will iterate over all the strips and send the led data in series:
        pub fn send_all_sequential<T>(&self, hc: &mut HardwareController<T>)
        where
            T: PeriodicTimer,
        {
            let mut start_index = 0;

            for (pin_index, strip) in self.strips.iter().enumerate() {
                let end_index = start_index + strip.led_count;

                let start_byte_index = start_index * 3;
                let end_byte_index = end_index * 3;
                let bit_slice =
                    Self::bytes_as_bit_slice(&self._byte_buffer[start_byte_index..end_byte_index]);

                strip.send_bits(hc, pin_index, bit_slice.iter().by_val());

                start_index = end_index;
            }
        }

        // this takes an array of u8 color data and converts it into an array of bools
        pub fn bytes_as_bit_slice(byte_buffer: &[u8]) -> &BitSlice<Msb0, u8> {
            byte_buffer.view_bits::<Msb0>()
        }
    }
}
