use crate::{
    animations::{Direction, MAX_OFFSET},
    colors,
    colors::{Color, Rainbow},
    utility::{
        convert_ns_to_frames, MarchingRainbow, MarchingRainbowMut, Progression, SlowFadeRainbow,
        StatefulRainbow,
    },
};
use embedded_time::rate::Hertz;

type FgUpdater = fn(&mut Foreground, &mut [Color]);

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

    /// This will use the function provided with the enum to do the update
    Custom(FgUpdater),
}

impl Mode {
    fn get_updater(&self) -> Option<FgUpdater> {
        match *self {
            Mode::NoForeground => None,
            Mode::MarqueeSolid => Some(marquee_solid),
            Mode::MarqueeSolidFixed => Some(marquee_solid_fixed),
            Mode::MarqueeFade => Some(marquee_fade),
            Mode::MarqueeFadeFixed => Some(marquee_fade_fixed),
            Mode::VUMeter => Some(vu_meter),
            Mode::Custom(u) => Some(u),
        }
    }
}

fn marquee_solid(fg: &mut Foreground, segment: &mut [Color]) {
    handle_marquee_trigger(fg);
    fg.increment_marquee_step();
    fg.fill_marquee(fg.current_slow_fade_color(), segment);
}

fn marquee_solid_fixed(fg: &mut Foreground, segment: &mut [Color]) {
    handle_marquee_trigger(fg);
    set_marquee_toggle(fg, segment.len());
    fg.fill_marquee(fg.current_slow_fade_color(), segment);
}

fn marquee_fade(fg: &mut Foreground, segment: &mut [Color]) {
    handle_marquee_trigger(fg);
    fg.increment_marquee_step();
    let color = fg.calculate_slow_fade_color();
    fg.fill_marquee(color, segment);
}

fn marquee_fade_fixed(fg: &mut Foreground, segment: &mut [Color]) {
    handle_marquee_trigger(fg);
    set_marquee_toggle(fg, segment.len());
    let color = fg.calculate_slow_fade_color();
    fg.fill_marquee(color, segment);
}

fn vu_meter(fg: &mut Foreground, segment: &mut [Color]) {
    fg.current_slow_fade_color();
    let led_count = segment.len();
    let last_on_led = fg.offset as usize / led_count;
    for led in &mut segment[last_on_led..] {
        *led = colors::C_OFF;
    }
}

fn set_marquee_toggle(fg: &mut Foreground, led_count: usize) {
    let pip_distance = (MAX_OFFSET as usize / led_count) * fg.pixels_per_pixel_group.max(1);
    let led_bucket = fg.offset as usize / pip_distance.max(1);
    fg.marquee_position_toggle = led_bucket % 2 == 0;
}

fn handle_marquee_trigger(fg: &mut Foreground) {
    if fg.has_been_triggered {
        fg.advance_rainbow_color();
        fg.reset_trigger();
    }
}

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

pub struct Foreground<'a> {
    // state
    pub offset: u16,
    pub frames: Progression,
    pub step_frames: Progression,
    marquee_position_toggle: bool,
    pub has_been_triggered: bool,

    // parameters
    pub rainbow: StatefulRainbow<'a>,
    direction: Direction,
    subdivisions: usize,
    pixels_per_pixel_group: usize,
    updater: Option<FgUpdater>,
}

impl<'a> Foreground<'a> {
    pub fn new(init: &Parameters<'a>, frame_rate: Hertz) -> Self {
        let frame_count = convert_ns_to_frames(init.duration_ns, frame_rate);
        let step_frame_count = convert_ns_to_frames(init.step_time_ns, frame_rate);

        Self {
            offset: 0,
            frames: Progression::new(frame_count),
            step_frames: Progression::new(step_frame_count),
            marquee_position_toggle: false,
            has_been_triggered: false,
            rainbow: StatefulRainbow::new(init.rainbow, init.is_rainbow_forward),
            direction: init.direction,
            subdivisions: init.subdivisions,
            pixels_per_pixel_group: init.pixels_per_pixel_group,
            updater: init.mode.get_updater(),
        }
    }

    pub fn update(&mut self, segment: &mut [Color]) {
        if let Some(f) = self.updater {
            f(self, segment);
        }
        let did_roll = self.frames.checked_increment();
        if did_roll {
            self.rainbow.increment();
        }
    }

    pub fn reset_trigger(&mut self) {
        self.has_been_triggered = false;
    }

    fn increment_marquee_step(&mut self) {
        // Increment and check to see if the color rolls over:
        let did_roll = self.step_frames.checked_increment();
        if did_roll {
            // toggle whether even or odd sub-pips are showing the marquee color:
            self.marquee_position_toggle = !self.marquee_position_toggle;
        }
    }

    fn fill_marquee(&mut self, color: Color, segment: &mut [Color]) {
        for (led_index, led) in segment.iter_mut().enumerate() {
            // every time the index is evenly divisible by the number of subpixels, toggle the state
            // that the pixels should be set to:
            let px_per_pip = self.pixels_per_pixel_group;
            let toggle = self.marquee_position_toggle;
            let subpip_number = led_index % (px_per_pip * 2);

            if subpip_number < px_per_pip && toggle {
                *led = color;
            }
            if subpip_number >= px_per_pip && !toggle {
                *led = color;
            }
        }
    }
}

impl<'a> MarchingRainbow for Foreground<'a> {
    fn rainbow(&self) -> &StatefulRainbow {
        &self.rainbow
    }
    fn frames(&self) -> &Progression {
        &self.frames
    }
}

impl<'a> MarchingRainbowMut for Foreground<'a> {
    fn rainbow_mut(&mut self) -> &'a mut StatefulRainbow {
        &mut self.rainbow
    }
    fn frames_mut(&mut self) -> &mut Progression {
        &mut self.frames
    }
}

impl<'a> SlowFadeRainbow for Foreground<'a> {
    fn rainbow(&self) -> &StatefulRainbow {
        &self.rainbow
    }
    fn frames(&self) -> &Progression {
        &self.frames
    }
}
