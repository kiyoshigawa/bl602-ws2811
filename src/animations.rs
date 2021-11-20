use crate::colors as c;
use crate::colors::Color;
use crate::leds::ws28xx::LogicalStrip;
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
pub enum Direction {
    Positive,
    Stopped,
    Negative,
}

/// Denotes the main types of animations, e.g. Foreground, Background, or Trigger:
pub enum AnimationType {
    Background,
    Foreground,
    Trigger,
}

/// This holds the parameters that define everything needed to set up an animation. It's a struct
/// holding the parameters for the foreground animation, the background animation, and the global
/// information for trigger animations (such as the trigger Rainbow)
pub struct AnimationParameters<const N_BG: usize, const N_FG: usize, const N_TG: usize> {
    pub bg: AnimationBackgroundParameters<N_BG>,
    pub fg: AnimationForegroundParameters<N_FG>,
    pub trigger: AnimationGlobalTriggerParameters<N_TG>,
}

/// This contains all the information necessary to set up and run a background animation. All
/// aspects of the animation can be derived from these parameters.
pub struct AnimationBackgroundParameters<const N: usize> {
    pub mode: BackgroundMode,
    pub rainbow: c::Rainbow<N>,
    pub direction: Direction,
    pub is_rainbow_reversed: bool,
    pub duration_ns: u64,
    pub subdivisions: usize,
}

/// This contains all the information necessary to set up and run a foreground animation. All
/// aspects of the animation can be derived from these parameters.
pub struct AnimationForegroundParameters<const N: usize> {
    pub mode: ForegroundMode,
    pub rainbow: c::Rainbow<N>,
    pub direction: Direction,
    pub is_rainbow_reversed: bool,
    pub duration_ns: u64,
    pub step_time_ns: u64,
    pub subdivisions: usize,
    pub num_pixels_per_marquee_pip: usize,
}

/// All triggers share a single rainbow / slow fade speed, which is configured in this struct
pub struct AnimationGlobalTriggerParameters<const N: usize> {
    pub rainbow: c::Rainbow<N>,
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

/// This contains all the information needed to keep track of the current state of a trigger
/// animation. It is updated every frame to match the current state of the animation. When complete,
/// the trigger animation will be set such that `is_complete = true` so that it can be removed from
/// the active trigger array.
struct AnimationTriggerState {
    mode: TriggerMode,
    current_frame: u32,
    total_frames: u32,
    last_direction: Direction,
    color: c::Color,
    is_running: bool,
}

/// Used to initialize the array of trigger states to the default value.
const DEFAULT_TRIGGER: AnimationTriggerState = AnimationTriggerState {
    mode: TriggerMode::NoTrigger,
    current_frame: 0,
    total_frames: 0,
    last_direction: Direction::Positive,
    color: c::C_OFF,
    is_running: false,
};

/// This contains all the information needed to keep track of the current state of a foreground or
/// background animation. It is updated every frame to match the current state of the animation.
#[derive(Default)]
struct AnimationState {
    offset: u16,
    current_duration_frame: u32,
    total_duration_frames: u32,
    current_step_frame: u32,
    total_step_frames: u32,
    current_rainbow_color_index: usize,
    has_been_triggered: bool,
    marquee_position_toggle: bool,
}

/// This contains the variables that apply to all triggers simultaneously, and not just to
/// individual running triggers.
#[derive(Default)]
struct AnimationGlobalTriggerState {
    current_rainbow_color_index: usize,
}

/// This struct contains all the fixed parameters of an animation, as well as the state of the
/// foreground, background, and active trigger animations. It is updated by the LightingController
/// that it is attached to at the LightingController's frame rate based on the parameters provided.
/// To make a new animation,
pub struct Animation<const N_BG: usize, const N_FG: usize, const N_TG: usize, const N_LED: usize> {
    parameters: AnimationParameters<N_BG, N_FG, N_TG>,
    translation_array: [usize; N_LED],
    led_position_array: [u16; N_LED],
    fg_state: AnimationState,
    bg_state: AnimationState,
    trigger_state: AnimationGlobalTriggerState,
    active_triggers: [AnimationTriggerState; MAX_NUM_ACTIVE_TRIGGERS],
    random_number_generator: SmallRng,
}

impl<const N_BG: usize, const N_FG: usize, const N_TG: usize, const N_LED: usize>
    Animation<N_BG, N_FG, N_TG, N_LED>
{
    pub fn new(
        parameters: AnimationParameters<N_BG, N_FG, N_TG>,
        translation_array: [usize; N_LED],
        random_seed: u64,
    ) -> Self {
        // Generate the LED Position Array. This is constant for every Animation based on the
        // number of LEDs <N_LED> in the animation. The LED positions are distributed evenly over
        // the entire range from 0..MAX_OFFSET, to increase the effective supersampling resolution of
        // the animation.
        let single_led_offset = MAX_OFFSET / N_LED as u16;
        let mut current_led_offset = 0;
        let mut led_position_array = [0_u16; N_LED];
        for led in led_position_array.iter_mut() {
            *led = current_led_offset;
            current_led_offset += single_led_offset;
        }
        let random_number_generator = SmallRng::seed_from_u64(random_seed);
        Animation {
            parameters,
            translation_array,
            led_position_array,
            fg_state: AnimationState::default(),
            bg_state: AnimationState::default(),
            trigger_state: AnimationGlobalTriggerState::default(),
            active_triggers: [DEFAULT_TRIGGER; MAX_NUM_ACTIVE_TRIGGERS],
            random_number_generator,
        }
    }

    pub fn init_total_duration_frames(&mut self, framerate: impl Into<Hertz>) {
        let framerate = framerate.into();
        self.bg_state.total_duration_frames = (self.parameters.bg.duration_ns
            * framerate.integer() as u64
            / 1_000_000_000_u64) as u32;
        self.fg_state.total_duration_frames = (self.parameters.fg.duration_ns
            * framerate.integer() as u64
            / 1_000_000_000_u64) as u32;
    }

    pub fn init_total_step_frames(&mut self, framerate: impl Into<Hertz>) {
        let framerate = framerate.into();
        // Background animations don't use steps, this can be set to 0 and ignored:
        self.bg_state.total_step_frames = 0;
        self.fg_state.total_step_frames = (self.parameters.fg.step_time_ns
            * framerate.integer() as u64
            / 1_000_000_000_u64) as u32;
    }

    pub fn update(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        // Update BG:
        self.update_bg(logical_strip);
        // Update FG:
        self.update_fg(logical_strip);
        // Update Triggers:
        // self.update_triggers(logical_strip);
    }

    pub fn trigger(&mut self, params: &AnimationTriggerParameters) {
        match params.mode {
            TriggerMode::NoTrigger => {
                // Do Nothing
            }
            TriggerMode::Background => {
                self.bg_state.has_been_triggered = true;
            }
            TriggerMode::Foreground => {
                self.fg_state.has_been_triggered = true;
            }
            TriggerMode::ColorPulse => {
                todo!()
            }
            TriggerMode::ColorPulseSlowFade => {
                todo!()
            }
            TriggerMode::ColorPulseRainbow => {
                todo!()
            }
            TriggerMode::ColorShot => {
                todo!()
            }
            TriggerMode::ColorShotSlowFade => {
                todo!()
            }
            TriggerMode::ColorShotRainbow => {
                todo!()
            }
            TriggerMode::Flash => {
                todo!()
            }
            TriggerMode::FlashSlowFade => {
                todo!()
            }
            TriggerMode::FlashRainbow => {
                todo!()
            }
        }
    }

    pub fn set_offset(&mut self, a_type: AnimationType, offset: u16) {
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

    fn update_bg(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        match self.parameters.bg.mode {
            BackgroundMode::NoBackground => self.update_bg_no_background(logical_strip),
            BackgroundMode::Solid => self.update_bg_solid(logical_strip),
            BackgroundMode::SolidFade => self.update_bg_solid_fade(logical_strip),
            BackgroundMode::FillRainbow => self.update_bg_fill_rainbow(logical_strip),
            BackgroundMode::FillRainbowRotate => self.update_bg_fill_rainbow_rotate(logical_strip),
        }
    }

    fn update_fg(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        match self.parameters.fg.mode {
            ForegroundMode::NoForeground => self.update_fg_no_foreground(logical_strip),
            ForegroundMode::MarqueeSolid => self.update_fg_marquee_solid(logical_strip),
            ForegroundMode::MarqueeSolidFixed => self.update_fg_marquee_solid_fixed(logical_strip),
            ForegroundMode::MarqueeFade => self.update_fg_marquee_fade(logical_strip),
            ForegroundMode::MarqueeFadeFixed => self.update_fg_marquee_fade_fixed(logical_strip),
            ForegroundMode::VUMeter => self.update_fg_vu_meter(logical_strip),
        }
    }

    fn update_triggers(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        for trigger_index in 0..MAX_NUM_ACTIVE_TRIGGERS {
            match self.active_triggers[trigger_index].mode {
                TriggerMode::NoTrigger => self.update_tg_no_trigger(logical_strip),
                TriggerMode::Background => (), // This is handled in the update_bg() functions
                TriggerMode::Foreground => (), // This is handled in the update_fg() functions
                TriggerMode::ColorPulse => self.update_tg_color_pulse(logical_strip),
                TriggerMode::ColorPulseSlowFade => {
                    self.update_tg_color_pulse_slow_fade(logical_strip)
                }
                TriggerMode::ColorPulseRainbow => self.update_tg_color_pulse_rainbow(logical_strip),
                TriggerMode::ColorShot => self.update_tg_color_shot(logical_strip),
                TriggerMode::ColorShotSlowFade => {
                    self.update_tg_color_shot_slow_fade(logical_strip)
                }
                TriggerMode::ColorShotRainbow => self.update_tg_color_shot_rainbow(logical_strip),
                TriggerMode::Flash => self.update_tg_flash(logical_strip),
                TriggerMode::FlashSlowFade => self.update_tg_flash_slow_fade(logical_strip),
                TriggerMode::FlashRainbow => self.update_tg_flash_rainbow(logical_strip),
            }
        }
    }

    // Incrementers:

    fn increment_bg_duration_frames(&mut self) {
        if self.bg_state.total_duration_frames != 0 {
            self.bg_state.current_duration_frame =
                (self.bg_state.current_duration_frame + 1) % self.bg_state.total_duration_frames;
        } else {
            self.bg_state.current_duration_frame = 0
        }
    }

    fn increment_fg_duration_frames(&mut self) {
        if self.fg_state.total_duration_frames != 0 {
            self.fg_state.current_duration_frame =
                (self.fg_state.current_duration_frame + 1) % self.fg_state.total_duration_frames;
        } else {
            self.fg_state.current_duration_frame = 0
        }
    }

    fn increment_fg_step_frames(&mut self) {
        if self.fg_state.total_step_frames != 0 {
            self.fg_state.current_step_frame =
                (self.fg_state.current_step_frame + 1) % self.fg_state.total_step_frames;
        } else {
            self.fg_state.current_step_frame = 0
        }
    }

    fn increment_marquee_step(&mut self) {
        let previous_frame = self.fg_state.current_step_frame;
        self.increment_fg_step_frames();
        // Check to see when the color rolls over:
        if self.fg_state.current_step_frame < previous_frame {
            // toggle whether even or odd sub-pips are showing the marquee color:
            self.fg_state.marquee_position_toggle = !self.fg_state.marquee_position_toggle;
        }
    }

    // Slow Fade Intermediate Color Calculators:

    fn calculate_bg_slow_fade_color(&mut self) -> Color {
        let previous_frame = self.bg_state.current_duration_frame;
        self.increment_fg_duration_frames();
        // Check to see when the color rolls over:
        if self.bg_state.current_duration_frame < previous_frame {
            self.advance_rainbow_index(AnimationType::Background);
        }
        let current_color = self.current_rainbow_color(AnimationType::Background);
        let next_color = self.next_rainbow_color(AnimationType::Background);
        c::Color::color_lerp(
            self.bg_state.current_duration_frame as i32,
            0,
            self.bg_state.total_duration_frames as i32,
            current_color,
            next_color,
        )
    }

    fn calculate_fg_slow_fade_color(&mut self) -> Color {
        let previous_frame = self.fg_state.current_duration_frame;
        self.increment_fg_duration_frames();
        // Check to see when the color rolls over:
        if self.fg_state.current_duration_frame < previous_frame {
            self.advance_rainbow_index(AnimationType::Foreground);
        }
        let current_color = self.current_rainbow_color(AnimationType::Foreground);
        let next_color = self.next_rainbow_color(AnimationType::Foreground);
        c::Color::color_lerp(
            self.fg_state.current_duration_frame as i32,
            0,
            self.fg_state.total_duration_frames as i32,
            current_color,
            next_color,
        )
    }

    // Rainbow Position Controls:

    fn next_rainbow_index(&mut self, ani_type: AnimationType) -> usize {
        match ani_type {
            AnimationType::Background => {
                let increment: i32 = match self.parameters.bg.is_rainbow_reversed {
                    true => 1,
                    false => -1,
                };
                (self.bg_state.current_rainbow_color_index as i32 + N_BG as i32 + increment)
                    as usize
                    % N_BG
            }
            AnimationType::Foreground => {
                let increment: i32 = match self.parameters.fg.is_rainbow_reversed {
                    true => 1,
                    false => -1,
                };
                (self.fg_state.current_rainbow_color_index as i32 + N_FG as i32 + increment)
                    as usize
                    % N_FG
            }
            AnimationType::Trigger => {
                let increment: i32 = match self.parameters.trigger.is_rainbow_reversed {
                    true => 1,
                    false => -1,
                };
                (self.trigger_state.current_rainbow_color_index as i32 + N_TG as i32 + increment)
                    as usize
                    % N_TG
            }
        }
    }

    fn current_rainbow_color(&mut self, ani_type: AnimationType) -> Color {
        match ani_type {
            AnimationType::Background => {
                self.parameters.bg.rainbow.colors[self.bg_state.current_rainbow_color_index]
            }
            AnimationType::Foreground => {
                self.parameters.fg.rainbow.colors[self.fg_state.current_rainbow_color_index]
            }
            AnimationType::Trigger => {
                self.parameters.trigger.rainbow.colors
                    [self.trigger_state.current_rainbow_color_index]
            }
        }
    }

    fn next_rainbow_color(&mut self, ani_type: AnimationType) -> Color {
        match ani_type {
            AnimationType::Background => {
                self.parameters.bg.rainbow.colors[self.next_rainbow_index(ani_type)]
            }
            AnimationType::Foreground => {
                self.parameters.fg.rainbow.colors[self.next_rainbow_index(ani_type)]
            }
            AnimationType::Trigger => {
                self.parameters.trigger.rainbow.colors[self.next_rainbow_index(ani_type)]
            }
        }
    }

    fn advance_rainbow_index(&mut self, ani_type: AnimationType) {
        match ani_type {
            AnimationType::Background => {
                if N_BG != 0 {
                    self.bg_state.current_rainbow_color_index = self.next_rainbow_index(ani_type);
                } else {
                    self.bg_state.current_rainbow_color_index = 0;
                }
            }
            AnimationType::Foreground => {
                if N_FG != 0 {
                    self.fg_state.current_rainbow_color_index = self.next_rainbow_index(ani_type);
                } else {
                    self.fg_state.current_rainbow_color_index = 0;
                }
            }
            AnimationType::Trigger => {
                if N_TG != 0 {
                    self.trigger_state.current_rainbow_color_index =
                        self.next_rainbow_index(ani_type);
                } else {
                    self.trigger_state.current_rainbow_color_index = 0;
                }
            }
        }
    }

    // Fills:

    fn fill_solid(&mut self, color: Color, logical_strip: &mut LogicalStrip<{ N_LED }>) {
        for led_index in self.translation_array {
            logical_strip.set_color_at_index(led_index, color);
        }
    }

    fn fill_rainbow<const N_R: usize>(
        &mut self,
        start_offset: u16,
        rainbow: c::Rainbow<N_R>,
        logical_strip: &mut LogicalStrip<N_LED>,
    ) {
        const MAX_OFFSET: usize = u16::MAX as usize;
        let start_offset = start_offset as usize;
        // Always start with the first color of the rainbow:
        self.bg_state.current_rainbow_color_index = 0;

        // We will need to know the distance between each color of the rainbow, and this will need
        // to take into account that the rainbow will be repeated by the number of subdivisions in
        // the bg parameters:
        let total_num_rainbow_colors = N_R * self.parameters.bg.subdivisions.max(1);
        let distance_between_colors = MAX_OFFSET / total_num_rainbow_colors;

        for (led_index, &led_position) in self.led_position_array.iter().enumerate() {
            // move the led position by offset rather than the rainbow itself
            let shifted_position = (led_position as usize + MAX_OFFSET - start_offset) % MAX_OFFSET;

            // all positions from one color to just before the next map to a rainbow bucket index
            let rainbow_bucket = shifted_position / distance_between_colors;
            let bucket_start = rainbow_bucket * distance_between_colors;

            let factor = shifted_position - bucket_start;

            let start_color_index = rainbow_bucket % N_R;
            let start_color = rainbow.colors[start_color_index];

            let end_color_index = (rainbow_bucket + 1) % N_R;
            let end_color = rainbow.colors[end_color_index];

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

    fn fill_marquee(&mut self, color: c::Color, logical_strip: &mut LogicalStrip<N_LED>) {
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

    fn update_bg_no_background(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        for led_index in self.translation_array {
            logical_strip.set_color_at_index(led_index, c::C_OFF);
        }
    }

    fn update_bg_solid(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        if self.bg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Background);
            self.bg_state.has_been_triggered = false;
        }
        // Set all LEDs to the current rainbow color. Note that in this mode the color will only
        // change when an external trigger of type `Background` is received.
        let color_index = self.bg_state.current_rainbow_color_index;
        let color = self.parameters.bg.rainbow.colors[color_index];
        self.fill_solid(color, logical_strip)
    }

    fn update_bg_solid_fade(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        if self.bg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Background);
            self.bg_state.current_duration_frame = 0;
            self.bg_state.has_been_triggered = false;
        }
        for led_index in self.translation_array {
            logical_strip.set_color_at_index(led_index, self.calculate_bg_slow_fade_color());
        }
    }

    fn update_bg_fill_rainbow(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        // handle trigger condition:
        if self.bg_state.has_been_triggered {
            self.bg_state.offset =
                (self.random_number_generator.next_u32() % MAX_OFFSET as u32) as u16;
            self.bg_state.has_been_triggered = false;
        }
        // This mode only fills the rainbow to whatever value the offset is currently set to:
        self.fill_rainbow(self.bg_state.offset, self.parameters.bg.rainbow, logical_strip);
    }

    fn update_bg_fill_rainbow_rotate(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        self.increment_bg_duration_frames();
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

        if self.bg_state.total_duration_frames != 0 {
            color_start_offset += (MAX_OFFSET as u32 * self.bg_state.current_duration_frame
                / self.bg_state.total_duration_frames) as u16;
        }
        color_start_offset %= MAX_OFFSET;

        self.fill_rainbow(color_start_offset, self.parameters.bg.rainbow, logical_strip);
    }

    // Foregrounds:

    fn update_fg_no_foreground(&mut self, _: &mut LogicalStrip<N_LED>) {
        // Do Nothing
    }

    fn update_fg_marquee_solid(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        if self.fg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Foreground);
            self.fg_state.has_been_triggered = false;
        }
        let color = self.current_rainbow_color(AnimationType::Foreground);
        self.increment_marquee_step();
        self.fill_marquee(color, logical_strip);
    }

    fn update_fg_marquee_solid_fixed(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        if self.fg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Foreground);
            self.fg_state.has_been_triggered = false;
        }
        let color = self.current_rainbow_color(AnimationType::Foreground);
        self.fill_marquee(color, logical_strip);
    }

    fn update_fg_marquee_fade(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        if self.fg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Foreground);
            self.fg_state.current_duration_frame = 0;
            self.fg_state.has_been_triggered = false;
        }
        self.increment_marquee_step();
        let intermediate_color = self.calculate_fg_slow_fade_color();
        self.fill_marquee(intermediate_color, logical_strip);
    }

    fn update_fg_marquee_fade_fixed(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        if self.fg_state.has_been_triggered {
            self.advance_rainbow_index(AnimationType::Foreground);
            self.fg_state.current_duration_frame = 0;
            self.fg_state.has_been_triggered = false;
        }
        let intermediate_color = self.calculate_fg_slow_fade_color();
        self.fill_marquee(intermediate_color, logical_strip);
    }

    fn update_fg_vu_meter(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        // TODO
    }

    // Triggers:

    fn update_tg_no_trigger(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        // Do Nothing
    }

    fn update_tg_color_pulse(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_color_pulse_slow_fade(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_color_pulse_rainbow(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_color_shot(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_color_shot_slow_fade(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_color_shot_rainbow(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_flash(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_flash_slow_fade(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_flash_rainbow(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }
}
