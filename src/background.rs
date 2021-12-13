use embedded_time::rate::Hertz;
use crate::{a, a::{Direction, MAX_OFFSET}, c::{self, Color, Rainbow}, foreground::AnimationState};
type BgUpdater = fn(&mut Background, &mut [Color]);

/// Background Modes are rendered onto the animation LEDs first before any Foreground or Trigger
/// animations. The other types of animation will overwrite any pixel data from the background that
/// is effected by their animation.
pub enum Mode {
    /// This turns off all the leds in the animation for the background layer.
    NoBackground,

    /// This shows a solid unchanging color on all the leds in the background. The color will be the
    /// first in the rainbow. You can step to other colors in the rainbow by external trigger,
    /// otherwise it does not change.
    Solid,

    /// This will slowly fade all the leds as a single color fading through the colors of a rainbow.
    /// Color offset can be externally triggered to the next color in the rainbow, or will move
    /// at a constant rate, changing one color per `duration_ns` sized time step.
    SolidFade,

    /// This will populate a rainbow's colors evenly across the LED in the animation in order. It
    /// does not animate once drawn.
    /// When externally triggered, it moves to a random offset.
    FillRainbow,

    /// This will populate a rainbow like above, but it will animate it by slowly offsetting the
    /// color pattern over time.
    /// When externally triggered, it moves to a random offset.
    FillRainbowRotate,

    /// This will use the function provided with the enum to do the update
    Custom(BgUpdater),
}

impl Mode {
    fn get_updater(&self) -> Option<BgUpdater> {
        match *self {
            Mode::NoBackground => Some(no_background),
            Mode::Solid => Some(solid),
            Mode::SolidFade => Some(solid_fade),
            Mode::FillRainbow => Some(fill_rainbow),
            Mode::FillRainbowRotate => Some(fill_rainbow_rotate),
            Mode::Custom(u) => Some(u),
        }
    }
}

pub struct Parameters<'a> {
    pub mode: Mode,
    pub rainbow: Rainbow<'a>,
    pub direction: Direction,
    pub is_rainbow_forward: bool,
    pub duration_ns: u64,
    pub subdivisions: usize,
}

pub struct Background<'a> {
    // state
    pub base_state: AnimationState<'a>,

    // parameters
    direction: Direction,
    subdivisions: usize,
    updater: Option<BgUpdater>,
}

impl<'a> Background<'a> {
    pub fn new(init: Parameters<'a>, frame_rate: Hertz) -> Self {
        let frames = a::convert_ns_to_frames(init.duration_ns, frame_rate);
        let base_state = AnimationState::new(init.rainbow, init.is_rainbow_forward, frames);

        let updater = init.mode.get_updater();

        Self {
            base_state,
            direction: init.direction,
            subdivisions: init.subdivisions,
            updater,
        }
    }

    pub fn update(&mut self, segment: &mut [Color]) {
        if let Some(f) = self.updater {
            f(self, segment);
        }
    }

    pub fn current_rainbow_color(&self) -> Color {
        self.base_state.rainbow.current_color()
    }

    pub fn advance_rainbow_index(&mut self) {
        self.base_state.rainbow.position.increment();
    }

    fn fill_solid(&mut self, color: Color, segment: &mut[Color]) {
        segment.iter_mut().for_each(|led| *led = color);
    }

    fn fill_rainbow(
        &mut self,
        start_offset: u16,
        rainbow: &[c::Color],
        segment: &mut [Color],
    ) {
        let start_offset = start_offset as usize;
        let max_offset = MAX_OFFSET as usize;
        let led_count = segment.len();
        let get_position = |led_index| led_index * (max_offset / led_count);

        // Always start with the first color of the rainbow:
        self.base_state.rainbow.reset();

        let rainbow_length = rainbow.len();

        // We will need to know the distance between each color of the rainbow, and this will need
        // to take into account that the rainbow will be repeated by the number of subdivisions in
        // the bg parameters:
        let total_num_rainbow_colors = rainbow_length * 1.max(self.subdivisions);
        let distance_between_colors = max_offset / total_num_rainbow_colors;

        let led_iterator = segment
            .iter_mut()
            .enumerate()
            .map(|(i, c)| (get_position(i), c) );

        for (led_position, led) in led_iterator {
            // move the led position by offset rather than the rainbow itself
            let shifted_position = (led_position as usize + max_offset - start_offset) % max_offset;

            // all positions from one color to just before the next map to a rainbow bucket index
            let rainbow_bucket = shifted_position / distance_between_colors;
            let bucket_start = rainbow_bucket * distance_between_colors;

            let factor = shifted_position - bucket_start;

            let start_color_index = rainbow_bucket % rainbow_length;
            let start_color = rainbow[start_color_index];

            let end_color_index = (rainbow_bucket + 1) % rainbow_length;
            let end_color = rainbow[end_color_index];

            let mid_color = c::Color::color_lerp(
                factor as i32,
                0,
                distance_between_colors as i32,
                start_color,
                end_color,
            );

            *led = mid_color;
        }
    }

}

fn get_random_offset() -> u16 {
    // (self.random_number_generator.next_u32() % MAX_OFFSET as u32) as u16;
    todo!()
}

fn no_background(bg: &mut Background, segment: &mut [Color]) {
    bg.fill_solid(c::C_OFF, segment);
}

fn solid(bg: &mut Background, segment: &mut [Color]) {
    if bg.base_state.has_been_triggered {
        bg.advance_rainbow_index();
        bg.base_state.reset_trigger();
    }
    // Set all LEDs to the current rainbow color. Note that in this mode the color will only
    // change when an external trigger of type `Background` is received.
    bg.fill_solid(bg.current_rainbow_color(), segment);
}

fn solid_fade(bg: &mut Background, segment: &mut [Color]) {
    if bg.base_state.has_been_triggered {
        bg.advance_rainbow_index();
        bg.base_state.frames.reset();
        bg.base_state.reset_trigger();
    }
    for led in segment {
        *led = bg.base_state.calculate_slow_fade_color();
    }
}

fn fill_rainbow(bg: &mut Background, segment: &mut [Color]) {
    // handle trigger condition:
    if bg.base_state.has_been_triggered {
        bg.base_state.offset = get_random_offset();
        bg.base_state.reset_trigger();
    }
    // This mode only fills the rainbow to whatever value the offset is currently set to:
    bg.fill_rainbow(bg.base_state.offset, bg.base_state.rainbow.backer, segment);
}

fn fill_rainbow_rotate(bg: &mut Background, segment: &mut [Color]) {
    bg.base_state.rainbow.increment();
    // handle trigger condition:
    if bg.base_state.has_been_triggered {
        bg.base_state.offset = get_random_offset();
        bg.base_state.reset_trigger();
    }

    // This mode will take the value that the offset is set to and then adjust based on the
    // current frame / total frames ratio to decide where to begin the rainbow. Need to do the
    // addition of the set offset plus the frame offset as u32s to avoid going over u16::MAX,
    // then modulo back to a u16 value using MAX_OFFSET when done.
    let mut color_start_offset = bg.base_state.offset;

    if bg.base_state.frames.total != 0 {
        color_start_offset += (MAX_OFFSET as u32 * bg.base_state.frames.current
            / bg.base_state.frames.total) as u16;
    }
    color_start_offset %= MAX_OFFSET;

    bg.fill_rainbow(color_start_offset, bg.base_state.rainbow.backer, segment);
}