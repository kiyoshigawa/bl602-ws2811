pub mod ws28xx {
    use crate::colors as c;

    use bitvec::prelude::*;
    use bl602_hal::gpio::{Output, PullDown};
    use embedded_hal::digital::blocking::OutputPin;
    use embedded_time::duration::*;

    pub struct StripTimings {
        pub zero_h: u32,
        pub one_h: u32,
        pub full_cycle: u32,
    }

    impl StripTimings {
        pub const _WS2811_ADAFRUIT: StripTimings =
            StripTimings { zero_h: 500_u32, one_h: 1200_u32, full_cycle: 2500_u32 };
        pub const WS2812_ADAFRUIT: StripTimings =
            StripTimings { zero_h: 400_u32, one_h: 800_u32, full_cycle: 1250_u32 };

        pub const _WS2811_TIM_800KHZ: StripTimings =
            StripTimings { zero_h: 200_u32, one_h: 600_u32, full_cycle: 1250_u32 };
        pub const _WS2811_TIM_400KHZ: StripTimings =
            StripTimings { zero_h: 500_u32, one_h: 1200_u32, full_cycle: 2500_u32 };
    }
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
        pub pin: u8,
        pub led_count: usize,
        pub reversed: bool,
        pub color_order: ColorOrder,
    }

    impl PhysicalStrip {
        pub fn send_bits<'a>(
            &self,
            pin: &mut impl OutputPin,
            bits: impl IntoIterator<Item = &'a bool>,
        ) {
            // Put the bits in the global timer interrupt array

            // Set a global flag to indicate that the bits can be sent

            // Loop until the global flag is cleared by the interrupt
        }

        fn colors_to_bytes<'a>(
            &self,
            colors: impl Iterator<Item = &'a c::Color>,
        ) -> [u8; crate::MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH] {
            let mut byte_buffer = [0_u8; crate::MAX_SINGLE_STRIP_BYTE_BUFFER_LENGTH];

            // Set the bytes in the RGB order for this strip
            let offsets = self.color_order.offsets();

            for (i, color) in colors.enumerate() {
                let base = i * 3;
                byte_buffer[base + offsets[0]] = color.r;
                byte_buffer[base + offsets[1]] = color.g;
                byte_buffer[base + offsets[2]] = color.b;
            }

            byte_buffer
        }
    }

    pub struct LogicalStrip<'a, const NUM_LEDS: usize> {
        color_buffer: [c::Color; NUM_LEDS],
        strips: &'a [PhysicalStrip],
    }

    impl<'a, const NUM_LEDS: usize> LogicalStrip<'a, NUM_LEDS> {
        pub fn new(strips: &'a [PhysicalStrip]) -> Self {
            LogicalStrip::<NUM_LEDS> { color_buffer: [c::Color::default(); NUM_LEDS], strips }
        }

        //this sets the color value in the color array at index:
        pub fn set_color_at_index(&mut self, index: usize, color: c::Color) {
            self.color_buffer[index] = color;
        }

        // this fills the entire strip with a single color:
        pub fn set_strip_to_solid_color(&mut self, color: c::Color) {
            self.color_buffer = [color; NUM_LEDS];
        }

        fn bytes_as_bit_slice(byte_buffer: &[u8]) -> &BitSlice<Msb0, u8> {
            byte_buffer.view_bits::<Msb0>()
        }

        // this will iterate over all the strips and send the led data in series:
        pub fn send_all_sequential(&self) {
            let mut start_index = 0;

            for strip in self.strips {
                let end_index = start_index + strip.led_count;

                let current_strip_colors = &self.color_buffer[start_index..end_index];

                let byte_count = strip.led_count * 3;

                let byte_buffer = match strip.reversed {
                    true => strip.colors_to_bytes(current_strip_colors.iter().rev()),
                    false => strip.colors_to_bytes(current_strip_colors.iter()),
                };

                let bit_slice = Self::bytes_as_bit_slice(&byte_buffer[..byte_count]);

                // call send bits and send the timing array
                //strip.send_bits(pins, bit_slice);

                start_index = end_index;
            }
        }
    }
}
