use crate::colors as c;
use crate::colors::Color;
use crate::leds::ws28xx::LogicalStrip;
use arrayvec::ArrayVec as Vec;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::rate::*;
use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};

/// Adjust MAX_NUM_* consts depending on RAM requirements:
const MAX_NUM_ACTIVE_TRIGGERS: usize = 10;

/// This is the maximum offset value for rotating animations. It's basically the supersampled
/// resolution of the animation over the entire translation_array of leds.
pub const MAX_OFFSET: u16 = u16::MAX;

/// Background Modes are rendered onto the animation LEDs first before any Foreground or Trigger
/// animations. The other types of animation will overwrite any pixel data from the background that
/// is effected by their animation.
pub enum BackgroundMode {
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
}

/// Foreground modes are rendered second, and will animate over the background animation layer but
/// below the trigger animations. Any trigger animations will overwrite the pixel data from the
/// foreground that is effected by their animation.
pub enum ForegroundMode {
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

/// These are the types of triggered animation effects that are possible with an animation. They can
/// be mixed and matched at any time over any combination of foreground and background animations.
/// The trigger animation colors will override any foreground or background pixel data on the pixels
/// it effects.
#[derive(Copy, Clone)]
pub enum TriggerMode {
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
}

/// Denotes the direction of animations, effects vary depending on animation modes:
#[derive(Copy, Clone)]
pub enum Direction {
    Positive,
    Stopped,
    Negative,
}

/// Denotes the main types of animations, e.g. Foreground, Background, or Trigger:
#[derive(Clone, Copy)]
pub enum AnimationType {
    Background,
    Foreground,
    Trigger,
}

/// This holds the parameters that define everything needed to set up an animation. It's a struct
/// holding the parameters for the foreground animation, the background animation, and the global
/// information for trigger animations (such as the trigger Rainbow)
pub struct AnimationParameters<'a> {
    pub bg: AnimationBackgroundParameters<'a>,
    pub fg: AnimationForegroundParameters<'a>,
    pub trigger: AnimationGlobalTriggerParameters<'a>,
}

/// This contains all the information necessary to set up and run a background animation. All
/// aspects of the animation can be derived from these parameters.
pub struct AnimationBackgroundParameters<'a> {
    pub mode: BackgroundMode,
    pub rainbow: c::Rainbow<'a>,
    pub direction: Direction,
    pub is_rainbow_reversed: bool,
    pub duration_ns: u64,
    pub subdivisions: usize,
}

/// This contains all the information necessary to set up and run a foreground animation. All
/// aspects of the animation can be derived from these parameters.
pub struct AnimationForegroundParameters<'a> {
    pub mode: ForegroundMode,
    pub rainbow: c::Rainbow<'a>,
    pub direction: Direction,
    pub is_rainbow_reversed: bool,
    pub duration_ns: u64,
    pub step_time_ns: u64,
    pub subdivisions: usize,
    pub num_pixels_per_marquee_pip: usize,
}

/// All triggers share a single rainbow / slow fade speed, which is configured in this struct
pub struct AnimationGlobalTriggerParameters<'a> {
    pub rainbow: c::Rainbow<'a>,
    pub is_rainbow_reversed: bool,
    pub duration_ns: u64,
}

/// This contains all the information necessary to set up and run a trigger animation. All
/// aspects of the animation can be derived from these parameters and the
/// AnimationGlobalTriggerParameters struct's parameters. Some parameters will not have an
/// effect depending on the mode.
pub struct AnimationTriggerParameters {
    pub mode: TriggerMode,
    pub direction: Direction,
    pub step_time_ns: u64,
    pub fade_in_time_ns: u64,
    pub fade_out_time_ns: u64,
    pub starting_offset: u16,
}

/// This contains all the information needed to keep track of the current state of a foreground or
/// background animation. It is updated every frame to match the current state of the animation.
#[derive(Default)]
struct AnimationState {
    offset: u16,
    frames: Progression,
    step_frames: Progression,
    current_rainbow_color_index: usize,
    has_been_triggered: bool,
    marquee_position_toggle: bool,
}

/// This contains the variables that apply to all triggers simultaneously, and not just to
/// individual running triggers.
#[derive(Default)]
struct AnimationGlobalTriggerState {
    current_rainbow_color_index: usize,
    frames: Progression,
}

/// This contains all the information needed to keep track of the current state of a trigger
/// animation. It is updated every frame to match the current state of the animation.
struct AnimationTriggerState {
    mode: TriggerMode,
    current_frame: u32,
    total_fade_in_frames: u32,
    total_fade_out_frames: u32,
    direction: Direction,
    current_offset: u16,
    color: c::Color,
}

/// This struct contains all the fixed parameters of an animation, as well as the state of the
/// foreground, background, and active trigger animations. It is updated by the LightingController
/// that it is attached to at the LightingController's frame rate based on the parameters provided.
/// To make a new animation,
pub struct Animation<'a, const N_LED: usize> {
    parameters: AnimationParameters<'a>,
    translation_array: [usize; N_LED],
    led_position_array: [u16; N_LED],
    fg_state: AnimationState,
    bg_state: AnimationState,
    trigger_state: AnimationGlobalTriggerState,
    triggers: Vec<AnimationTriggerState, MAX_NUM_ACTIVE_TRIGGERS>,
    random_number_generator: SmallRng,
}

pub trait Animatable<'a> {
    fn update(&mut self, logical_strip: &mut LogicalStrip);
    fn set_offset(&mut self, a_type: AnimationType, offset: u16);
    fn trigger(&mut self, params: &AnimationTriggerParameters, frame_rate: Hertz);
    fn init_total_animation_duration_frames(&mut self, frame_rate: Hertz);
    fn init_total_animation_step_frames(&mut self, frame_rate: Hertz);
}

impl<'a, const N_LED: usize> Animatable<'a> for Animation<'a, N_LED> {
    fn update(&mut self, logical_strip: &mut LogicalStrip) {
        // Update BG:
        self.update_bg(logical_strip);
        // Update FG:
        self.update_fg(logical_strip);
        // Update Triggers:
        self.update_triggers(logical_strip);
    }

    fn set_offset(&mut self, a_type: AnimationType, offset: u16) {
        match a_type {
            AnimationType::Background => {
                self.bg_state.offset = offset;
            }
            AnimationType::Foreground => {
                self.fg_state.offset = offset;
            }
            AnimationType::Trigger => {
                // Triggers don't use offsets, so do nothing until they need to.
            }
        }
    }

    fn trigger(&mut self, params: &AnimationTriggerParameters, frame_rate: Hertz) {
        let random_offset = (self.random_number_generator.next_u32() % MAX_OFFSET as u32) as u16;
        let starting_color_bucket = params.starting_offset / Self::OFFSET_BETWEEN_LEDS;
        let starting_color_offset = starting_color_bucket * Self::OFFSET_BETWEEN_LEDS;
        let init_color_pulse_trigger = |color| AnimationTriggerState {
            mode: params.mode,
            current_frame: 0,
            total_fade_in_frames: Self::convert_ns_to_frames(params.fade_in_time_ns, frame_rate),
            total_fade_out_frames: Self::convert_ns_to_frames(params.fade_out_time_ns, frame_rate),
            direction: Direction::Stopped,
            current_offset: random_offset,
            color,
        };
        let init_color_shot_trigger = |color| AnimationTriggerState {
            mode: params.mode,
            current_frame: 0,
            total_fade_in_frames: Self::convert_ns_to_frames(params.step_time_ns, frame_rate),
            total_fade_out_frames: 0, // not used with color shots, fade in represents step duration
            direction: params.direction,
            current_offset: starting_color_offset,
            color,
        };
        let init_flash_trigger = |color| AnimationTriggerState {
            mode: params.mode,
            current_frame: 0,
            total_fade_in_frames: Self::convert_ns_to_frames(params.fade_in_time_ns, frame_rate),
            total_fade_out_frames: Self::convert_ns_to_frames(params.fade_out_time_ns, frame_rate),
            direction: Direction::Stopped,
            current_offset: 0,
            color,
        };

        use TriggerMode::*;
        let current_color = self.current_rainbow_color(AnimationType::Trigger);
        let new_trigger_state = match params.mode {
            NoTrigger => { None }
            Background => {
                self.bg_state.has_been_triggered = true;
                None
            }
            Foreground => {
                self.fg_state.has_been_triggered = true;
                None
            }
            ColorPulse => {
                Some(init_color_pulse_trigger(current_color))
            }
            ColorPulseSlowFade => {
                Some(init_color_pulse_trigger(self.calculate_slow_fade_color(AnimationType::Trigger)))
            }
            ColorPulseRainbow => {
                self.advance_rainbow_index(AnimationType::Trigger);
                Some(init_color_pulse_trigger(current_color))
            }
            ColorShot => {
                Some(init_color_shot_trigger(current_color))
            }
            ColorShotSlowFade => {
                Some(init_color_shot_trigger(self.calculate_slow_fade_color(AnimationType::Trigger)))
            }
            ColorShotRainbow => {
                self.advance_rainbow_index(AnimationType::Trigger);
                Some(init_color_shot_trigger(current_color))
            }
            Flash => {
                Some(init_flash_trigger(current_color))
            }
            FlashSlowFade => {
                Some(init_flash_trigger(self.calculate_slow_fade_color(AnimationType::Trigger)))
            }
            FlashRainbow => {
                self.advance_rainbow_index(AnimationType::Trigger);
                Some(init_flash_trigger(current_color))
            }
        };

        if let Some(trigger_state) = new_trigger_state {
            self.add_trigger(trigger_state);
        }
    }

    fn init_total_animation_duration_frames(&mut self, frame_rate: Hertz) {
        self.bg_state.frames.total =
            Self::convert_ns_to_frames(self.parameters.bg.duration_ns, frame_rate);
        self.fg_state.frames.total =
            Self::convert_ns_to_frames(self.parameters.fg.duration_ns, frame_rate);
        self.trigger_state.frames.total =
            Self::convert_ns_to_frames(self.parameters.trigger.duration_ns, frame_rate);
    }

    fn init_total_animation_step_frames(&mut self, frame_rate: Hertz) {
        // Background animations don't use steps, this can be set to 0 and ignored:
        self.bg_state.step_frames.total = 0;
        self.fg_state.step_frames.total =
            Self::convert_ns_to_frames(self.parameters.fg.step_time_ns, frame_rate);
    }
}

impl<'a, const N_LED: usize> Animation<'a, N_LED> {
    const OFFSET_BETWEEN_LEDS: u16 = MAX_OFFSET / N_LED as u16;

    pub fn new(
        parameters: AnimationParameters<'a>,
        translation_array: [usize; N_LED],
        random_seed: u64,
    ) -> Self {
        // Generate the LED Position Array. This is constant for every Animation based on the
        // number of LEDs <N_LED> in the animation. The LED positions are distributed evenly over
        // the entire range from 0..MAX_OFFSET, to increase the effective supersampling resolution of
        // the animation.
        let mut current_led_offset = 0;
        let mut led_position_array = [0_u16; N_LED];
        for led in led_position_array.iter_mut() {
            *led = current_led_offset;
            current_led_offset += Self::OFFSET_BETWEEN_LEDS;
        }
        let random_number_generator = SmallRng::seed_from_u64(random_seed);
        Animation {
            parameters,
            translation_array,
            led_position_array,
            fg_state: AnimationState::default(),
            bg_state: AnimationState::default(),
            trigger_state: AnimationGlobalTriggerState::default(),
            triggers: Vec::new(),
            random_number_generator,
        }
    }

    fn update_bg(&mut self, logical_strip: &mut LogicalStrip) {
        match self.parameters.bg.mode {
            BackgroundMode::NoBackground => self.update_bg_no_background(logical_strip),
            BackgroundMode::Solid => self.update_bg_solid(logical_strip),
            BackgroundMode::SolidFade => self.update_bg_solid_fade(logical_strip),
            BackgroundMode::FillRainbow => self.update_bg_fill_rainbow(logical_strip),
            BackgroundMode::FillRainbowRotate => self.update_bg_fill_rainbow_rotate(logical_strip),
        }
    }

    fn update_fg(&mut self, logical_strip: &mut LogicalStrip) {
        match self.parameters.fg.mode {
            ForegroundMode::NoForeground => self.update_fg_no_foreground(logical_strip),
            ForegroundMode::MarqueeSolid => self.update_fg_marquee_solid(logical_strip),
            ForegroundMode::MarqueeSolidFixed => self.update_fg_marquee_solid_fixed(logical_strip),
            ForegroundMode::MarqueeFade => self.update_fg_marquee_fade(logical_strip),
            ForegroundMode::MarqueeFadeFixed => self.update_fg_marquee_fade_fixed(logical_strip),
            ForegroundMode::VUMeter => self.update_fg_vu_meter(logical_strip),
        }
    }

    fn update_triggers(&mut self, logical_strip: &mut LogicalStrip) {
        // iterate the slow fade frames for slow fading color animations:
        self.trigger_state.frames.increment();

        // then iterate over the triggers in the vec. Note, these have to go in reverse, because
        // if a trigger completes its animation and is removed, it would reduce the maximum index
        // value resulting in panics when trying to access out of bounds.
        for trigger_index in (0..self.triggers.len()).rev() {
            use TriggerMode::*;
            match self.triggers[trigger_index].mode {
                NoTrigger => self.update_tg_no_trigger(logical_strip, trigger_index),
                Background | Foreground => (), // This is handled in the update_fg() functions
                ColorPulse | ColorPulseSlowFade | ColorPulseRainbow => {
                    self.update_tg_color_pulse(logical_strip, trigger_index)
                }
                ColorShot | ColorShotSlowFade | ColorShotRainbow => {
                    self.update_tg_color_shot(logical_strip, trigger_index)
                }
                Flash | FlashSlowFade | FlashRainbow => {
                    self.update_tg_flash(logical_strip, trigger_index)
                }
            }
        }
    }

    fn increment_marquee_step(&mut self) {
        // Increment and check to see if the color rolls over:
        let did_roll = self.fg_state.step_frames.checked_increment();
        if did_roll {
            // toggle whether even or odd sub-pips are showing the marquee color:
            self.fg_state.marquee_position_toggle = !self.fg_state.marquee_position_toggle;
        }
    }

    // Slow Fade Intermediate Color Calculators:
    fn calculate_slow_fade_color(&mut self, anim_type: AnimationType) -> Color {
        let frames = match anim_type {
            AnimationType::Background => &mut self.bg_state.frames,
            AnimationType::Foreground => &mut self.fg_state.frames,
            AnimationType::Trigger => &mut self.trigger_state.frames,
        };

        if frames.total == 0 {
            return self.current_rainbow_color(anim_type);
        }

        let did_roll = frames.checked_increment();
        let progress = *frames;

        if did_roll {
            self.advance_rainbow_index(anim_type);
        }

        let current_color = self.current_rainbow_color(anim_type);
        let next_color = self.next_rainbow_color(anim_type);
        current_color.lerp_with(next_color, progress)
    }

    // Rainbow Position Controls:

    fn next_rainbow_index(&mut self, ani_type: AnimationType) -> usize {
        match ani_type {
            AnimationType::Background => {
                let increment: i32 = match self.parameters.bg.is_rainbow_reversed {
                    true => 1,
                    false => -1,
                };
                let bg_length = self.parameters.bg.rainbow.len();
                (self.bg_state.current_rainbow_color_index as i32 + bg_length as i32 + increment)
                    as usize
                    % bg_length
            }
            AnimationType::Foreground => {
                let increment: i32 = match self.parameters.fg.is_rainbow_reversed {
                    true => 1,
                    false => -1,
                };
                let fg_length = self.parameters.fg.rainbow.len();
                (self.fg_state.current_rainbow_color_index as i32 + fg_length as i32 + increment)
                    as usize
                    % fg_length
            }
            AnimationType::Trigger => {
                let increment: i32 = match self.parameters.trigger.is_rainbow_reversed {
                    true => 1,
                    false => -1,
                };
                let trigger_length = self.parameters.trigger.rainbow.len();
                (self.trigger_state.current_rainbow_color_index as i32
                    + trigger_length as i32
                    + increment) as usize
                    % trigger_length
            }
        }
    }

    fn current_rainbow_color(&mut self, ani_type: AnimationType) -> Color {
        match ani_type {
            AnimationType::Background => {
                self.parameters.bg.rainbow[self.bg_state.current_rainbow_color_index]
            }
            AnimationType::Foreground => {
                self.parameters.fg.rainbow[self.fg_state.current_rainbow_color_index]
            }
            AnimationType::Trigger => {
                self.parameters.trigger.rainbow[self.trigger_state.current_rainbow_color_index]
            }
        }
    }

    fn next_rainbow_color(&mut self, ani_type: AnimationType) -> Color {
        match ani_type {
            AnimationType::Background => {
                self.parameters.bg.rainbow[self.next_rainbow_index(ani_type)]
            }
            AnimationType::Foreground => {
                self.parameters.fg.rainbow[self.next_rainbow_index(ani_type)]
            }
            AnimationType::Trigger => {
                self.parameters.trigger.rainbow[self.next_rainbow_index(ani_type)]
            }
        }
    }

    fn advance_rainbow_index(&mut self, ani_type: AnimationType) {
        match ani_type {
            AnimationType::Background => {
                if self.parameters.bg.rainbow.len() != 0 {
                    self.bg_state.current_rainbow_color_index = self.next_rainbow_index(ani_type);
                } else {
                    self.bg_state.current_rainbow_color_index = 0;
                }
            }
            AnimationType::Foreground => {
                if self.parameters.fg.rainbow.len() != 0 {
                    self.fg_state.current_rainbow_color_index = self.next_rainbow_index(ani_type);
                } else {
                    self.fg_state.current_rainbow_color_index = 0;
                }
            }
            AnimationType::Trigger => {
                if self.parameters.trigger.rainbow.len() != 0 {
                    self.trigger_state.current_rainbow_color_index =
                        self.next_rainbow_index(ani_type);
                } else {
                    self.trigger_state.current_rainbow_color_index = 0;
                }
            }
        }
    }

    // Fills:

    fn fill_solid(&mut self, color: Color, logical_strip: &mut LogicalStrip) {
        for led_index in self.translation_array {
            logical_strip.set_color_at_index(led_index, color);
        }
    }

    fn fill_rainbow(
        &mut self,
        start_offset: u16,
        rainbow: &[c::Color],
        logical_strip: &mut LogicalStrip,
    ) {
        let max_offset = MAX_OFFSET as usize;
        let start_offset = start_offset as usize;
        // Always start with the first color of the rainbow:
        self.bg_state.current_rainbow_color_index = 0;

        let rainbow_length = rainbow.len();

        // We will need to know the distance between each color of the rainbow, and this will need
        // to take into account that the rainbow will be repeated by the number of subdivisions in
        // the bg parameters:
        let total_num_rainbow_colors = rainbow_length * self.parameters.bg.subdivisions.max(1);
        let distance_between_colors = max_offset / total_num_rainbow_colors;

        for (led_index, &led_position) in self.led_position_array.iter().enumerate() {
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

            let translated_led_index = self.translation_array[led_index];

            logical_strip.set_color_at_index(translated_led_index, mid_color);
        }
    }

    fn fill_marquee(&mut self, color: c::Color, logical_strip: &mut LogicalStrip) {
        for led_index in 0..N_LED {
            // every time the index is evenly divisible by the number of subpixels, toggle the state
            // that the pixels should be set to:
            let subpip_number = led_index % (self.parameters.fg.num_pixels_per_marquee_pip * 2);

            if subpip_number < self.parameters.fg.num_pixels_per_marquee_pip
                && self.fg_state.marquee_position_toggle
            {
                logical_strip.set_color_at_index(self.translation_array[led_index], color);
            }
            if subpip_number >= self.parameters.fg.num_pixels_per_marquee_pip
                && !self.fg_state.marquee_position_toggle
            {
                logical_strip.set_color_at_index(self.translation_array[led_index], color);
            }
        }
    }

    // Backgrounds:

    fn update_bg_no_background(&mut self, logical_strip: &mut LogicalStrip) {
        for led_index in self.translation_array {
            logical_strip.set_color_at_index(led_index, c::C_OFF);
        }
    }

    fn update_bg_solid(&mut self, logical_strip: &mut LogicalStrip) {
        if self.bg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Background);
            self.bg_state.has_been_triggered = false;
        }
        // Set all LEDs to the current rainbow color. Note that in this mode the color will only
        // change when an external trigger of type `Background` is received.
        let color_index = self.bg_state.current_rainbow_color_index;
        let color = self.parameters.bg.rainbow[color_index];
        self.fill_solid(color, logical_strip)
    }

    fn update_bg_solid_fade(&mut self, logical_strip: &mut LogicalStrip) {
        if self.bg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Background);
            self.bg_state.frames.current = 0;
            self.bg_state.has_been_triggered = false;
        }
        for led_index in self.translation_array {
            logical_strip.set_color_at_index(led_index, self.calculate_slow_fade_color(AnimationType::Background));
        }
    }

    fn update_bg_fill_rainbow(&mut self, logical_strip: &mut LogicalStrip) {
        // handle trigger condition:
        if self.bg_state.has_been_triggered {
            self.bg_state.offset =
                (self.random_number_generator.next_u32() % MAX_OFFSET as u32) as u16;
            self.bg_state.has_been_triggered = false;
        }
        // This mode only fills the rainbow to whatever value the offset is currently set to:
        self.fill_rainbow(self.bg_state.offset, self.parameters.bg.rainbow, logical_strip);
    }

    fn update_bg_fill_rainbow_rotate(&mut self, logical_strip: &mut LogicalStrip) {
        self.bg_state.frames.increment();
        // handle trigger condition:
        if self.bg_state.has_been_triggered {
            self.bg_state.offset =
                (self.random_number_generator.next_u32() % MAX_OFFSET as u32) as u16;
            self.bg_state.has_been_triggered = false;
        }

        // This mode will take the value that the offset is set to and then adjust based on the
        // current frame / total frames ratio to decide where to begin the rainbow. Need to do the
        // addition of the set offset plus the frame offset as u32s to avoid going over u16::MAX,
        // then modulo back to a u16 value using MAX_OFFSET when done.
        let mut color_start_offset = self.bg_state.offset;

        if self.bg_state.frames.total != 0 {
            color_start_offset += (MAX_OFFSET as u32 * self.bg_state.frames.current
                / self.bg_state.frames.total) as u16;
        }
        color_start_offset %= MAX_OFFSET;

        self.fill_rainbow(color_start_offset, self.parameters.bg.rainbow, logical_strip);
    }

    // Foregrounds:

    fn update_fg_no_foreground(&mut self, _: &mut LogicalStrip) {
        // Do Nothing
    }

    fn update_fg_marquee_solid(&mut self, logical_strip: &mut LogicalStrip) {
        if self.fg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Foreground);
            self.fg_state.has_been_triggered = false;
        }
        let color = self.current_rainbow_color(AnimationType::Foreground);
        self.increment_marquee_step();
        self.fill_marquee(color, logical_strip);
    }

    fn update_fg_marquee_solid_fixed(&mut self, logical_strip: &mut LogicalStrip) {
        if self.fg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Foreground);
            self.fg_state.has_been_triggered = false;
        }

        // calculate the marquee_position_toggle based on the set offset value:
        let pip_distance =
            (MAX_OFFSET as usize / N_LED) * self.parameters.fg.num_pixels_per_marquee_pip.max(1);
        let led_bucket = self.fg_state.offset as usize / pip_distance.max(1);
        self.fg_state.marquee_position_toggle = led_bucket % 2 == 0;

        let color = self.current_rainbow_color(AnimationType::Foreground);
        self.fill_marquee(color, logical_strip);
    }

    fn update_fg_marquee_fade(&mut self, logical_strip: &mut LogicalStrip) {
        if self.fg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Foreground);
            self.fg_state.frames.current = 0;
            self.fg_state.has_been_triggered = false;
        }
        self.increment_marquee_step();
        let intermediate_color = self.calculate_slow_fade_color(AnimationType::Foreground);
        self.fill_marquee(intermediate_color, logical_strip);
    }

    fn update_fg_marquee_fade_fixed(&mut self, logical_strip: &mut LogicalStrip) {
        if self.fg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Foreground);
            self.fg_state.frames.current = 0;
            self.fg_state.has_been_triggered = false;
        }

        // calculate the marquee_position_toggle based on the set offset value:
        let pip_distance =
            (MAX_OFFSET as usize / N_LED) * self.parameters.fg.num_pixels_per_marquee_pip.max(1);
        let led_bucket = self.fg_state.offset as usize / pip_distance.max(1);
        self.fg_state.marquee_position_toggle = led_bucket % 2 == 0;

        let intermediate_color = self.calculate_slow_fade_color(AnimationType::Foreground);
        self.fill_marquee(intermediate_color, logical_strip);
    }

    fn update_fg_vu_meter(&mut self, logical_strip: &mut LogicalStrip) {
        // TODO
    }

    // Triggers:

    fn add_trigger(&mut self, trigger_state: AnimationTriggerState) {
        // if the vector is still full, this will ignore the new trigger:
        let _ = self.triggers.try_push(trigger_state);
    }

    fn update_tg_no_trigger(&mut self, logical_strip: &mut LogicalStrip, trigger_index: usize) {
        // Do Nothing
    }

    fn update_tg_color_pulse(&mut self, logical_strip: &mut LogicalStrip, trigger_index: usize) {
        todo!()
    }

    fn update_tg_color_shot(&mut self, logical_strip: &mut LogicalStrip, trigger_index: usize) {
        todo!()
    }

    fn update_tg_flash(&mut self, logical_strip: &mut LogicalStrip, trigger_index: usize) {
        // prevent out of bounds errors if someone calls this with a bad index:
        if trigger_index >= self.triggers.len() {
            return;
        }

        let ts = &mut self.triggers[trigger_index];

        if ts.current_frame < ts.total_fade_in_frames {
            // fading in interpolation:
            for led_index in self.translation_array {
                let mid_color = Color::color_lerp(
                    ts.current_frame as i32,
                    0,
                    (ts.total_fade_in_frames - 1) as i32,
                    logical_strip.get_color_at_index(led_index),
                    ts.color,
                );
                logical_strip.set_color_at_index(led_index, mid_color);
            }
        } else {
            // fading out interpolation:
            for led_index in self.translation_array {
                let mid_color = Color::color_lerp(
                    (ts.current_frame - ts.total_fade_in_frames) as i32,
                    0,
                    (ts.total_fade_out_frames - 1) as i32,
                    ts.color,
                    logical_strip.get_color_at_index(led_index),
                );
                logical_strip.set_color_at_index(led_index, mid_color);
            }
        }
        ts.current_frame += 1;

        // if we've done all the frames, get this trigger out of here!
        if ts.current_frame >= ts.total_fade_in_frames + ts.total_fade_out_frames {
            self.triggers.remove(trigger_index);
        }
    }

    // Misc:

    fn convert_ns_to_frames(nanos: u64, frame_rate: Hertz) -> u32 {
        (nanos * frame_rate.integer() as u64 / 1_000_000_000_u64) as u32
    }
}

#[derive(Default, Debug, Copy, Clone)]
struct Progression {
    current: u32,
    total: u32,
}

impl Progression {
    fn increment(&mut self) {
        if self.total == 0 { return; }
        self.current = (self.current + 1) % self.total;
    }

    fn checked_increment(&mut self) -> bool {
        self.increment();
        self.current == 0
    }
}

impl Color {
    fn lerp_with(&self, to_color: Color, factor: Progression) -> Color {
        c::Color::color_lerp(
            factor.current as i32,
            0,
            factor.total as i32,
            *self,
            to_color,
        )
    }
}