pub mod ws28xx {
    use crate::colors::Color;
    use crate::hardware::{HardwareController, PeriodicTimer};
    use crate::{colors as c, MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH};
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
        pub fn new(byte_buffer: &'a mut [u8], color_buffer: &'a mut [c::Color], strips: &'a [PhysicalStrip]) -> Self {
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

                if index < end { return (strip, start); };

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

                let current_strip_colors = &self.color_buffer[start_index..end_index];

                // let byte_count = strip.led_count * 3;

                // let byte_buffer = match strip.reversed {
                //     true => {
                //         self.colors_to_bytes(current_strip_colors.iter().rev(), &strip.color_order)
                //     }
                //     false => self.colors_to_bytes(current_strip_colors.iter(), &strip.color_order),
                // };


                // let bit_slice = Self::bytes_as_bit_slice(&byte_buffer[..byte_count]);
                // let start = riscv::register::mcycle::read();
                // strip.send_bits(hc, pin_index, bit_slice.iter().by_ref());
                // crate::measure(start);

                let mut _forward;
                let mut _reverse;
                let colors: &mut dyn Iterator<Item = &Color> = match strip.reversed {
                    true => {
                        _reverse = current_strip_colors.iter().rev();
                        &mut _reverse
                    }
                    false => {
                        _forward = current_strip_colors.iter();
                        &mut _forward
                    }
                };

                let bits = colors
                    .into_bytes(&strip.color_order)
                    .into_bits();

                let start = riscv::register::mcycle::read();
                strip.send_bits(hc, pin_index, bits);
                crate::measure(start);

                // let bit_slice = Self::bytes_as_bit_slice(&self._byte_buffer);

                // let start = riscv::register::mcycle::read();

                // strip.send_bits(hc, pin_index, bit_slice.iter().by_ref());

                // crate::measure(start);

                start_index = end_index;
            }
        }

        fn colors_to_bytes(
            &self,
            colors: impl Iterator<Item = &'a c::Color>,
            color_order: &ColorOrder,
        ) -> [u8; MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH] {
            let mut byte_buffer = [0_u8; MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH];

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
        fn bytes_as_bit_slice(byte_buffer: &[u8]) -> &BitSlice<Msb0, u8> {
            byte_buffer.view_bits::<Msb0>()
        }
    }

    trait IntoByteIter {
        fn into_bytes(self, color_order: &ColorOrder) -> ByteIter<Self> where Self: Sized;
    }

    impl<'a, I: Iterator<Item = &'a Color> + Sized> IntoByteIter for I {
        fn into_bytes(self, color_order: &ColorOrder) -> ByteIter<Self> where Self: Sized {
            ByteIter {
                source: self,
                offsets: color_order.offsets(),
                byte_buffer: [0; 3],
                index: 3,
            }
        }
    }

    struct ByteIter<T> {
        source: T,
        offsets: [usize; 3],
        byte_buffer: [u8; 3],
        index: usize,
    }


    impl<'a, T: Iterator<Item=&'a c::Color>> Iterator for ByteIter<T> {
        type Item = u8;

        fn next(&mut self) -> Option<Self::Item> {

            if self.index > 2 {
                let color = self.source.next()?;
                self.byte_buffer[self.offsets[0]] = color.r;
                self.byte_buffer[self.offsets[1]] = color.g;
                self.byte_buffer[self.offsets[2]] = color.b;
                self.index = 0;
            }
            let result = self.byte_buffer[self.index];
            self.index += 1;

            Some(result)
        }
    }


    trait IntoBitIter {
        fn into_bits(self) -> BitIter<Self> where Self: Sized;
    }

    impl<I: Iterator<Item=u8> + Sized> IntoBitIter for I {
        fn into_bits(self) -> BitIter<Self> where Self: Sized {
            BitIter {
                source: self,
                current_byte: 0,
                mask: 0,
            }
        }
    }

    struct BitIter<T> {
        source: T,
        current_byte: u8,
        mask: u8,
    }

    impl<T: Iterator<Item=u8>> Iterator for BitIter<T> {
        type Item = bool;

        fn next(&mut self) -> Option<Self::Item> {
            if self.mask == 0 {
                self.current_byte = self.source.next()?;
                // Most signifigant bit
                self.mask = 128;
            }
            let result = self.current_byte & self.mask;
            // Most significant bit
            self.mask >>= 1;
            Some(result != 0)
        }
    }

}
