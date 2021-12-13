use embedded_time::rate::Hertz;
use crate::{
    animations as a,
    a::MAX_OFFSET,
    animations::{Direction, Progression},
    c::{Color, Rainbow},
    leds::ws28xx::LogicalStrip,
};

/// This contains all the information necessary to set up and run a foreground animation. All
/// aspects of the animation can be derived from these parameters.
pub struct Parameters<'a> {
    pub mode: Mode,
    pub rainbow: Rainbow<'a>,
    pub direction: Direction,
    pub is_rainbow_forward: bool,
    pub duration_ns: u64,
    pub step_time_ns: u64,
    pub subdivisions: usize,
    pub pixels_per_pixel_group: usize,
}

/// Foreground modes are rendered second, and will animate over the background animation layer but
/// below the trigger animations. Any trigger animations will overwrite the pixel data from the
/// foreground that is effected by their animation.
pub enum Mode {
    /// This is a mode that has no additional foreground animation over the background animation.
    NoForeground,

    /// This will display a single-color marquee style pixel chase animation using rainbow
    /// colors. The foreground trigger will advance to the next color of the rainbow.
    MarqueeSolid,

    /// This will display a fixed pattern the same as a marquee chase animation that will only
    /// move if the offset is changed manually where the color is always a solid constant color.
    /// The foreground trigger will advance to the next color of the rainbow.
    MarqueeSolidFixed,

    /// This will display a marquee style animation where the color of all the LEDs slowly fades
    /// through the colors of a rainbow. It will advance to the next color if externally triggered.
    MarqueeFade,

    /// This will display a fixed pattern the same as a marquee chase animation that will only move
    /// if the offset is changed manually where the color of all the LEDs slowly fades through the
    /// colors of a rainbow. It will advance to the next color if externally triggered.
    MarqueeFadeFixed,

    /// This will render the foreground rainbow based on the offset value, and leave LEDs below
    /// the offset value alone.
    VUMeter,
}

impl Mode {
    fn get_updater(&self) -> Option<fn(&mut Foreground)->Color> {
        match self {
            Mode::NoForeground => None,
            Mode::MarqueeSolid => Some(marquee_solid),
            Mode::MarqueeSolidFixed => Some(marquee_solid_fixed),
            Mode::MarqueeFade => Some(marquee_fade),
            Mode::MarqueeFadeFixed => Some(marquee_fade_fixed),
            Mode::VUMeter => Some(vu_meter),
        }
    }
}


fn marquee_solid(fg: &mut Foreground) -> Color {
    handle_marquee_trigger(fg);
    fg.increment_marquee_step();
    fg.current_rainbow_color()
}

fn marquee_solid_fixed(fg: &mut Foreground) -> Color {
    handle_marquee_trigger(fg);
    set_marquee_toggle(fg);
    fg.current_rainbow_color()
}

fn marquee_fade(fg: &mut Foreground) -> Color {
    fg.base_state.frames.increment();
    handle_marquee_trigger(fg);
    fg.increment_marquee_step();
    fg.base_state.calculate_slow_fade_color()
}

fn marquee_fade_fixed(fg: &mut Foreground) -> Color {
    fg.base_state.frames.increment();
    handle_marquee_trigger(fg);
    set_marquee_toggle(fg);
    fg.base_state.calculate_slow_fade_color()
}


fn vu_meter(fg: &mut Foreground) -> Color {
    fg.current_rainbow_color();
    todo!()
}

fn set_marquee_toggle(fg: &mut Foreground) {
    let led_count = fg.segment.len();
    let pip_distance =
        (MAX_OFFSET as usize / led_count) * fg.pixels_per_pixel_group.max(1);
    let led_bucket = fg.base_state.offset as usize / pip_distance.max(1);
    fg.marquee_position_toggle = led_bucket % 2 == 0;
}

fn handle_marquee_trigger(fg: &mut Foreground) {
    if fg.base_state.has_been_triggered {
        fg.advance_rainbow_index();
        fg.base_state.frames.reset();
        fg.base_state.has_been_triggered = false;
    }
}


pub struct StatefulRainbow<'a> {
    rainbow: Rainbow<'a>,
    position: Progression,
}

impl<'a> StatefulRainbow<'a> {
    fn new(rainbow: &'a [Color], is_forward: bool) -> StatefulRainbow<'a> {
        let mut position = Progression::new(rainbow.len() as u32);
        if !is_forward {
            position.current = position.total - 1;
            position.reverse_direction();
        }
        Self { rainbow, position }
    }

    fn current_color(&self) -> Color {
        self.rainbow[self.position.current as usize]
    }

    fn decrement(&mut self) {
        self.position.decrement();
    }

    fn increment(&mut self) {
        self.position.increment();
    }

    fn prev_color(&mut self) -> Color {
        self.increment();
        self.current_color()
    }

    fn next_color(&mut self) -> Color {
        self.increment();
        self.current_color()
    }

    fn peek_next_color(&self) -> Color {
        self.rainbow[self.position.peek_next() as usize]
    }

    fn peek_last_color(&self) -> Color {
        self.rainbow[self.position.peek_prev() as usize]
    }
}


struct AnimationState<'a> {
    offset: u16,
    frames: Progression,
    rainbow: StatefulRainbow<'a>,
    has_been_triggered: bool,
}

impl<'a> AnimationState<'a> {
    fn new(rainbow: Rainbow<'a>, is_forward: bool, frame_count: u32) -> Self {
        Self {
            offset: 0,
            frames: Progression::new(frame_count),
            rainbow: StatefulRainbow::new(rainbow, is_forward),
            has_been_triggered: false,
        }
    }

    fn calculate_slow_fade_color(&mut self) -> Color {
        let frames = &mut self.frames;
        if frames.total == 0 {
            return self.rainbow.current_color();
        }

        let did_roll = frames.checked_increment();
        let progress = *frames;

        if did_roll {
            self.rainbow.position.increment();
        }

        let current_color = self.rainbow.current_color();
        let next_color = self.rainbow.peek_next_color();
        current_color.lerp_with(next_color, progress)
    }
}

pub struct _ForegroundParameters<'a> {
    pub mode: Mode,
    pub rainbow: Rainbow<'a>,
    pub direction: Direction,
    pub is_rainbow_reversed: bool,
    pub duration_ns: u64,
    pub step_time_ns: u64,
    pub subdivisions: usize,
    pub num_pixels_per_marquee_pip: usize,
}

pub(crate) struct Foreground<'a> {
    // state
    base_state: AnimationState<'a>,
    step_frames: Progression,
    marquee_position_toggle: bool,
    segment: &'a mut [Color],

    // parameters
    direction: Direction,
    subdivisions: usize,
    pixels_per_pixel_group: usize,
    updater: Option<fn(&mut Foreground) -> Color>,
}

impl<'a> Foreground<'a> {
    fn new(init: Parameters<'a>, frame_rate: Hertz, segment: &'a mut [Color]) -> Self {
        let frames = a::convert_ns_to_frames(init.duration_ns, frame_rate);
        let base_state = AnimationState::new(init.rainbow, init.is_rainbow_forward, frames);
        let step_frames = a::convert_ns_to_frames(init.step_time_ns, frame_rate);
        let step_frames = Progression::new(step_frames);
        let updater = init.mode.get_updater();

        Self {
            base_state,
            step_frames,
            marquee_position_toggle: false,
            segment,
            direction: init.direction,
            subdivisions: init.subdivisions,
            pixels_per_pixel_group: init.pixels_per_pixel_group,
            updater,
        }
    }

    fn update(&mut self, logical_strip: &mut LogicalStrip) {
        if let Some(f) = self.updater {
            let color = f(self);
            self.fill_marquee(color, logical_strip);
        }
    }

    fn current_rainbow_color(&self) -> Color {
        self.base_state.rainbow.current_color()
    }

    fn advance_rainbow_index(&mut self) {
        self.base_state.rainbow.position.increment();
    }

    fn increment_marquee_step(&mut self) {
        // Increment and check to see if the color rolls over:
        let did_roll = self.step_frames.checked_increment();
        if did_roll {
            // toggle whether even or odd sub-pips are showing the marquee color:
            self.marquee_position_toggle = !self.marquee_position_toggle;
        }
    }

    fn fill_marquee(&mut self, color: Color, logical_strip: &mut LogicalStrip) {
        for (led_index, led) in self.segment.iter_mut().enumerate() {
            // every time the index is evenly divisible by the number of subpixels, toggle the state
            // that the pixels should be set to:
            let px_per_pip = self.pixels_per_pixel_group;
            let subpip_number = led_index % (px_per_pip * 2);

            if subpip_number < px_per_pip && self.marquee_position_toggle {
                *led = color;
            }
            if subpip_number >= px_per_pip && !self.marquee_position_toggle {
                *led = color;
            }
        }
    }
}
