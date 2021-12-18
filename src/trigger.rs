use crate::a::{self, Direction};
use crate::c::{self, Color};
use crate::utility::{MarchingRainbow, MarchingRainbowMut, Progression, StatefulRainbow, TimedRainbow, convert_ns_to_frames, get_random_offset};
use arrayvec::ArrayVec;
use embedded_time::rate::Hertz;

pub type TriggerInit = fn(&mut Trigger, &mut TimedRainbow);
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

    /// This will cause a pulse that slow fades to appear somewhere randomly along the led array.
    /// It will fade in, then fade back out one time per trigger, and its color will match the
    /// animation's global trigger slow fade speed setting.
    /// All pulses will be the same color, and the color will change over time.
    /// Fade in and out times can be adjusted separately.
    ColorPulseSlowFade,

    /// This will cause a pulse of to appear somewhere randomly along the led array.
    /// It will fade in, then fade back out one time per trigger.
    /// Each pulse will be a new color in the order of the rainbow.
    /// fade in and out times can be adjusted separately.
    ColorPulseRainbow,

    /// This will cause colored pulses of a single color to run down the LED strip.
    /// The starting offset and direction can be specified manually.
    ColorShot,

    /// This will cause colored pulses of a single slow-fading color to run down the LED strip.
    /// It will fade in, then fade back out one time per trigger, and its color will match the
    /// animation's global trigger slow fade speed setting.
    /// All pulses will be the same color, and the color will change over time.
    /// Fade in and out times can be adjusted separately.
    ColorShotSlowFade,

    /// This will fire off color pulses with a new color for each pulse, in the order of the colors
    /// of a rainbow.
    ColorShotRainbow,

    /// This will flash all the LEDs to a single color for a short time.
    /// Fade in and out times can be adjusted separately.
    Flash,

    /// This will flash all the LEDs to a single slow-fading color for a short time.
    /// It will fade in, then fade back out one time per trigger, and its color will match the
    /// animation's global trigger slow fade speed setting.
    /// All pulses will be the same color, and the color will change over time.
    /// Fade in and out times can be adjusted separately.
    FlashSlowFade,

    /// This will flash all the LEDs to a single slow-fading color for a short time.
    /// Each flash will be a new color in the order of the rainbow.
    FlashRainbow,

    Custom(TriggerBehavior)
}

impl Mode {
    pub fn get_behavior(&self) -> TriggerBehavior {
        match *self {
            Mode::NoTrigger  => (None, None),
            Mode::Background => (None, None),
            Mode::Foreground => (None, None),
            Mode::ColorPulse         => (Some(init_color_pulse), Some(color_pulse)),
            Mode::ColorPulseSlowFade => (Some(init_color_pulse_slow_fade), Some(color_pulse)),
            Mode::ColorPulseRainbow  => (Some(init_color_pulse_rainbow), Some(color_pulse)),
            Mode::ColorShot         => (Some(init_color_shot), Some(color_shot)),
            Mode::ColorShotSlowFade => (Some(init_color_shot_slow_fade), Some(color_shot)),
            Mode::ColorShotRainbow  => (Some(init_color_shot_rainbow), Some(color_shot)),
            Mode::Flash         => (Some(init_flash), Some(flash)),
            Mode::FlashSlowFade => (Some(init_flash_slow_fade), Some(flash)),
            Mode::FlashRainbow  => (Some(init_flash_rainbow), Some(flash)),
            Mode::Custom((i, u)) => (i, u),
        }
    }
}

/// All triggers share a single rainbow / slow fade speed, which is configured in this struct
pub struct GlobalParameters<'a> {
    pub rainbow: c::Rainbow<'a>,
    pub is_rainbow_forward: bool,
    pub duration_ns: u64,
}

/// This holds all triggers and contains the variables that apply to all triggers simultaneously, and not just to
/// individual running triggers.
pub struct TriggerCollection<'a, const N: usize> {
    pub rainbow: StatefulRainbow<'a>,
    pub frames: Progression,
    triggers: ArrayVec<Trigger, N>,
}

impl<'a, const N: usize> TriggerCollection<'a, N> {
    pub fn new(init: &GlobalParameters<'a>, frame_rate: Hertz) -> Self {
        let rainbow = StatefulRainbow::new(init.rainbow, init.is_rainbow_forward);
        let frames = Progression::new(convert_ns_to_frames(init.duration_ns, frame_rate));
        let triggers = ArrayVec::new();

        Self {
            rainbow,
            frames,
            triggers,
        }
    }

    pub fn add_trigger(&mut self, init: &Parameters, frame_rate: Hertz) {
        let (initializer, updater) = init.mode.get_behavior();
        let mut new_trigger = Trigger::new(init, self.current_rainbow_color(), frame_rate);

        if let Some(initialize) = initializer {
            initialize(&mut new_trigger, &mut TimedRainbow { rainbow: &mut self.rainbow, frames: &mut self.frames });
        }
        new_trigger.updater = updater;

        let _ = self.triggers.try_push(new_trigger);
    }

    pub fn update(&mut self, segment: &mut [Color]) {
        for trigger in self.triggers.iter_mut() {
            trigger.update(segment)
        }

        self.triggers.retain(|t| t.frames.get_current() < t.frames.total - 1);
        self.frames.increment();
    }
}

/// This contains all the information necessary to set up and run a trigger animation. All
/// aspects of the animation can be derived from these parameters and the
/// AnimationGlobalTriggerParameters struct's parameters. Some parameters will not have an
/// effect depending on the mode.
pub struct Parameters {
    pub mode: Mode,
    pub direction: Direction,
    pub step_time_ns: u64,
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
    transition_frame: u32,
    direction: Direction,
    color: Color,
    updater: Option<TriggerUpdater>,
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

        Self {
            offset,
            frames,
            transition_frame,
            direction,
            color,
            updater,
        }
    }

    pub fn update(&mut self, segment: &mut [Color]) {
        if let Some(f) = self.updater {
            f(self, segment);
        }
        self.frames.increment();
    }
}

fn flash(trigger: &mut Trigger, segment: &mut [Color]) {
    let is_fade_in = trigger.frames.get_current() < trigger.transition_frame;

    let mut progress;
    if is_fade_in {
        progress = Progression::new(trigger.transition_frame);
    } else {
        progress = Progression::new(trigger.frames.total - trigger.transition_frame);
        progress.reverse_direction();
    }

    progress.set_current(trigger.frames.get_current() % trigger.transition_frame);

    for led in segment {
        *led = led.lerp_with(trigger.color, progress);
    }
}

impl<'a, const N: usize> MarchingRainbow for TriggerCollection<'a, N> {
    fn rainbow(&self) -> &StatefulRainbow { &self.rainbow }
    fn frames(&self) -> &Progression { &self.frames }
}

fn color_pulse(trigger: &mut Trigger, segment: &mut [Color]) {
    todo!()
}

fn color_shot(trigger: &mut Trigger, segment: &mut [Color]) {
    todo!()
}

fn init_color_pulse(trigger: &mut Trigger, global: &mut TimedRainbow) {
    trigger.direction = Direction::Stopped;
    trigger.offset = get_random_offset();
}

fn init_color_pulse_slow_fade(trigger: &mut Trigger, global: &mut TimedRainbow) {
    trigger.color = global.calculate_slow_fade_color();
    init_color_pulse(trigger, global);
}

fn init_color_pulse_rainbow(trigger: &mut Trigger, global: &mut TimedRainbow) {
    global.advance_rainbow_color_hard();
    trigger.color = global.current_rainbow_color();
    init_color_pulse(trigger, global);
}

fn init_color_shot(trigger: &mut Trigger, global: &mut TimedRainbow) {
    trigger.frames.total = trigger.transition_frame;
    trigger.offset = 0;
}

fn init_color_shot_slow_fade(trigger: &mut Trigger, global: &mut TimedRainbow) {
    trigger.color = global.calculate_slow_fade_color();
    init_color_shot(trigger, global);
}

fn init_color_shot_rainbow(trigger: &mut Trigger, global: &mut TimedRainbow) {
    global.advance_rainbow_color_hard();
    trigger.color = global.current_rainbow_color();
    init_color_shot(trigger, global);
}

fn init_flash(trigger: &mut Trigger, global: &mut TimedRainbow) {
    trigger.direction = Direction::Stopped;
}

fn init_flash_slow_fade(trigger: &mut Trigger, global: &mut TimedRainbow) {
    trigger.color = global.calculate_slow_fade_color();
    init_flash(trigger, global);
}

fn init_flash_rainbow(trigger: &mut Trigger, global: &mut TimedRainbow) {
    global.advance_rainbow_color_hard();
    trigger.color = global.current_rainbow_color();
    init_flash(trigger, global);
}
