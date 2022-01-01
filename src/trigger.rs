use crate::animations::{Direction, MAX_OFFSET};
use crate::colors;
use crate::colors::Color;
use crate::utility::{
    convert_ns_to_frames, get_random_offset, shift_offset, FadeRainbow, MarchingRainbow,
    MarchingRainbowMut, Progression, StatefulRainbow, TimedRainbows,
};
use arrayvec::ArrayVec;
use embedded_time::rate::Hertz;

pub type TriggerInit = fn(&mut Trigger, &mut TimedRainbows);
pub type TriggerUpdater = fn(&mut Trigger, &mut [Color]);
pub type TriggerBehavior = (Option<TriggerInit>, Option<TriggerUpdater>);

/// These are the types of triggered animation effects that are possible with an animation. They can
/// be mixed and matched at any time over any combination of foreground and background animations.
/// The trigger animation colors will override any foreground or background pixel data on the pixels
/// it effects.
#[derive(Copy, Clone)]
pub enum Mode {
    /// This is a fallback value that doesn't have any trigger effect.
    NoTrigger,

    /// This will trigger a change in the background lighting, depending on the mode.
    Background,

    /// This will trigger a change in the foreground lighting, depending on the mode.
    Foreground,

    /// This will cause a pulse of a single color to appear somewhere randomly along the led array.
    /// It will fade in, then fade back out one time per trigger.
    /// Fade in and out times can be adjusted separately.
    ColorPulse,

    /// This will cause a pulse that fades to appear somewhere randomly along the led array.
    /// It will fade in, then fade back out one time per trigger, and its color will match the
    /// animation's global trigger fade speed setting.
    /// All pulses will be the same color, and the color will change over time.
    /// Fade in and out times can be adjusted separately.
    ColorPulseFade,

    /// This will cause a pulse of to appear somewhere randomly along the led array.
    /// It will fade in, then fade back out one time per trigger.
    /// Each pulse will be a new color in the order of the rainbow.
    /// fade in and out times can be adjusted separately.
    ColorPulseRainbow,

    /// This will cause colored pulses of a single color to run down the LED strip.
    /// The starting offset and direction can be specified manually.
    ColorShot,

    /// This will cause colored pulses of a single fading color to run down the LED strip.
    /// It will fade in, then fade back out one time per trigger, and its color will match the
    /// animation's global trigger fade speed setting.
    /// All pulses will be the same color, and the color will change over time.
    /// Fade in and out times can be adjusted separately.
    ColorShotFade,

    /// This will fire off color pulses with a new color for each pulse, in the order of the colors
    /// of a rainbow.
    ColorShotRainbow,

    /// This will flash all the LEDs to a single color for a short time.
    /// Fade in and out times can be adjusted separately.
    Flash,

    /// This will flash all the LEDs to a single fading color for a short time.
    /// It will fade in, then fade back out one time per trigger, and its color will match the
    /// animation's global trigger fade speed setting.
    /// All pulses will be the same color, and the color will change over time.
    /// Fade in and out times can be adjusted separately.
    FlashFade,

    /// This will flash all the LEDs to a single fading color for a short time.
    /// Each flash will be a new color in the order of the rainbow.
    FlashRainbow,

    Custom(TriggerBehavior),
}

impl Mode {
    pub fn get_behavior(&self) -> TriggerBehavior {
        match *self {
            Mode::NoTrigger => (None, None),
            Mode::Background => (None, None),
            Mode::Foreground => (None, None),
            Mode::ColorPulse => (Some(init_color_pulse), Some(color_pulse)),
            Mode::ColorPulseFade => (Some(init_color_pulse_fade), Some(color_pulse)),
            Mode::ColorPulseRainbow => (Some(init_color_pulse_rainbow), Some(color_pulse)),
            Mode::ColorShot => (Some(init_color_shot), Some(color_shot)),
            Mode::ColorShotFade => (Some(init_color_shot_fade), Some(color_shot)),
            Mode::ColorShotRainbow => (Some(init_color_shot_rainbow), Some(color_shot)),
            Mode::Flash => (Some(init_flash), Some(flash)),
            Mode::FlashFade => (Some(init_flash_fade), Some(flash)),
            Mode::FlashRainbow => (Some(init_flash_rainbow), Some(flash)),
            Mode::Custom((i, u)) => (i, u),
        }
    }
}

/// All triggers share a single rainbow / fade speed, which is configured in this struct
pub struct GlobalParameters<'a> {
    pub rainbow: colors::Rainbow<'a>,
    pub is_rainbow_forward: bool,
    pub duration_ns: u64,
}

/// This holds all triggers and contains the variables that apply to all triggers simultaneously, and not just to
/// individual running triggers.
pub struct TriggerCollection<'a, const N: usize> {
    pub fade_rainbow: StatefulRainbow<'a>,
    pub incremental_rainbow: StatefulRainbow<'a>,
    pub frames: Progression,
    triggers: ArrayVec<Trigger, N>,
}

impl<'a, const N: usize> TriggerCollection<'a, N> {
    pub fn new(init: &GlobalParameters<'a>, frame_rate: Hertz) -> Self {
        let fade_rainbow = StatefulRainbow::new(init.rainbow, init.is_rainbow_forward);
        let incremental_rainbow = StatefulRainbow::new(init.rainbow, init.is_rainbow_forward);
        let frames = Progression::new(convert_ns_to_frames(init.duration_ns, frame_rate));
        let triggers = ArrayVec::new();

        Self { fade_rainbow, incremental_rainbow, frames, triggers }
    }

    pub fn add_trigger(&mut self, init: &Parameters, frame_rate: Hertz) {
        let (initializer, updater) = init.mode.get_behavior();
        let mut new_trigger = Trigger::new(init, self.current_rainbow_color(), frame_rate);

        if let Some(initialize) = initializer {
            initialize(
                &mut new_trigger,
                &mut TimedRainbows {
                    fade_rainbow: &mut self.fade_rainbow,
                    incremental_rainbow: &mut self.incremental_rainbow,
                    frames: &mut self.frames,
                },
            );
        }
        new_trigger.updater = updater;

        let _ = self.triggers.try_push(new_trigger);
    }

    pub fn update(&mut self, segment: &mut [Color]) {
        for trigger in self.triggers.iter_mut() {
            trigger.update(segment)
        }

        self.triggers
            .retain(|t| t.frames.get_current() < t.frames.total - 1);
        let did_roll = self.frames.checked_increment();
        if did_roll {
            self.fade_rainbow.increment();
        }
    }
}

/// This contains all the information necessary to set up and run a trigger animation. All
/// aspects of the animation can be derived from these parameters and the
/// AnimationGlobalTriggerParameters struct's parameters. Some parameters will not have an
/// effect depending on the mode.
pub struct Parameters {
    pub mode: Mode,
    pub direction: Direction,
    pub fade_in_time_ns: u64,
    pub fade_out_time_ns: u64,
    pub starting_offset: u16,
    pub pixels_per_pixel_group: usize,
}

/// This contains all the information needed to keep track of the current state of a trigger
/// animation. It is updated every frame to match the current state of the animation.
pub struct Trigger {
    offset: u16,
    frames: Progression,
    transition_frame: usize,
    direction: Direction,
    color: Color,
    updater: Option<TriggerUpdater>,
    pixels_per_pixel_group: usize,
}

impl Trigger {
    pub fn new(init: &Parameters, color: Color, frame_rate: Hertz) -> Self {
        let offset = init.starting_offset;
        let total_duration_ns = init.fade_in_time_ns + init.fade_out_time_ns;

        let frames = convert_ns_to_frames(total_duration_ns, frame_rate);
        let frames = Progression::new(frames);

        let transition_frame = convert_ns_to_frames(init.fade_in_time_ns, frame_rate);
        let direction = init.direction;
        let updater = None;

        let pixels_per_pixel_group = init.pixels_per_pixel_group;

        Self { offset, frames, transition_frame, direction, color, updater, pixels_per_pixel_group }
    }

    pub fn update(&mut self, segment: &mut [Color]) {
        if let Some(f) = self.updater {
            f(self, segment);
        }
        self.frames.increment();
    }
}

impl<'a, const N: usize> MarchingRainbow for TriggerCollection<'a, N> {
    fn rainbow(&self) -> &StatefulRainbow {
        &self.incremental_rainbow
    }
    fn frames(&self) -> &Progression {
        &self.frames
    }
}

fn get_trigger_fade_progress(trigger: &mut Trigger) -> Progression {
    let is_fade_in = trigger.frames.get_current() < trigger.transition_frame;

    let mut progress;
    let mut transition_frame = 0;
    if is_fade_in {
        progress = Progression::new(trigger.transition_frame);
    } else {
        progress = Progression::new(trigger.frames.total - trigger.transition_frame);
        progress.reverse_direction();
        transition_frame = trigger.transition_frame;
    }

    progress.set_current(trigger.frames.get_current() - transition_frame);
    progress
}

fn flash(trigger: &mut Trigger, segment: &mut [Color]) {
    let progress = get_trigger_fade_progress(trigger);

    for led in segment {
        *led = led.lerp_with(trigger.color, progress);
    }
}

fn color_pulse(trigger: &mut Trigger, segment: &mut [Color]) {
    let progress = get_trigger_fade_progress(trigger);

    // the range will be always at least 1 led, up to pixels_per_pixel_group leds:
    let first_led_index = trigger.offset as usize / segment.len();
    let shot_width = 1.max(trigger.pixels_per_pixel_group);
    let last_led_index = first_led_index + shot_width;

    for index in first_led_index..last_led_index {
        let corrected_index = index % segment.len();
        segment[corrected_index] = segment[corrected_index].lerp_with(trigger.color, progress);
    }
}

fn color_shot(trigger: &mut Trigger, segment: &mut [Color]) {
    let current_offset = shift_offset(trigger.offset, trigger.frames, trigger.direction) as usize;
    let offset_distance_between_leds = MAX_OFFSET as usize / segment.len();

    // the range will be always at least 1 led, up to pixels_per_pixel_group leds:
    let first_led_index = current_offset / offset_distance_between_leds;
    let shot_width = 1.max(trigger.pixels_per_pixel_group);
    let last_led_index = first_led_index + shot_width;

    for index in first_led_index..last_led_index {
        let corrected_index = index % segment.len();
        segment[corrected_index] = trigger.color;
    }
}

fn init_color_pulse(trigger: &mut Trigger, _: &mut TimedRainbows) {
    trigger.direction = Direction::Stopped;
    trigger.offset = get_random_offset();
}

fn init_color_pulse_fade(trigger: &mut Trigger, global: &mut TimedRainbows) {
    trigger.color = global.calculate_fade_color();
    init_color_pulse(trigger, global);
}

fn init_color_pulse_rainbow(trigger: &mut Trigger, global: &mut TimedRainbows) {
    trigger.color = global.current_rainbow_color();
    init_color_pulse(trigger, global);
    global.advance_rainbow_color();
}

fn init_color_shot(_: &mut Trigger, _: &mut TimedRainbows) {
    // Don't need to change anything until it's running.
}

fn init_color_shot_fade(trigger: &mut Trigger, global: &mut TimedRainbows) {
    trigger.color = global.calculate_fade_color();
    init_color_shot(trigger, global);
}

fn init_color_shot_rainbow(trigger: &mut Trigger, global: &mut TimedRainbows) {
    trigger.color = global.current_rainbow_color();
    init_color_shot(trigger, global);
    global.advance_rainbow_color();
}

fn init_flash(trigger: &mut Trigger, _: &mut TimedRainbows) {
    trigger.direction = Direction::Stopped;
}

fn init_flash_fade(trigger: &mut Trigger, global: &mut TimedRainbows) {
    init_flash(trigger, global);
    trigger.color = global.calculate_fade_color();
}

fn init_flash_rainbow(trigger: &mut Trigger, global: &mut TimedRainbows) {
    init_flash(trigger, global);
    trigger.color = global.current_rainbow_color();
    global.advance_rainbow_color();
}
