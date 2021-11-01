use crate::colors as c;
use crate::leds::ws28xx::LogicalStrip;
use embedded_time::rate::*;

/// This value is used as a default value for the number of subdivisions on the const animations at
/// the end of the file. Typically this number should be 0 for shorter strips, and higher as you add
/// more LEDs. It controls how often the palette colors repeat over the entire length of LEDs.
pub const DEFAULT_NUMBER_OF_SUBDIVISIONS: usize = 0;

/// Adjust MAX_NUM_* consts depending on RAM requirements:
const MAX_NUM_ACTIVE_TRIGGERS: usize = 10;

/// Background Modes are rendered onto the animation LEDs first before any Foreground or Trigger
/// animations. The other types of animation will overwrite any pixel data from the background that
/// is effected by their animation.
pub enum BackgroundMode {
    /// This turns off all the leds in the animation for the background layer.
    NoBackground,

    /// This shows a solid unchanging color on all the leds in the background. The color will be the
    /// first in the palette. You can step to other colors in the palette by external trigger,
    /// otherwise it does not change.
    Solid,

    /// This will slowly fade all the leds as a single color fading through the colors of a palette.
    /// Color offset can be externally triggered to the next color in the palette, or will move
    /// at a constant rate, changing one color per `duration_ns` sized time step.
    SolidFade,

    /// This will populate a palette's colors evenly across the LED in the animation in order. It
    /// does not animate once drawn.
    /// When externally triggered, it moves to a random offset.
    FillPalette,

    /// This will populate a palette like above, but it will animate it by slowly offsetting the
    /// color pattern over time.
    /// When externally triggered, it moves to a random offset.
    FillPaletteRotate,
}

/// Foreground modes are rendered second, and will animate over the background animation layer but
/// below the trigger animations. Any trigger animations will overwrite the pixel data from the
/// foreground that is effected by their animation.
pub enum ForegroundMode {
    /// This is a mode that has no additional foreground animation over the background animation.
    NoForeground,

    /// This will display a single-color marquee style pixel chase animation using palette
    /// colors. The foreground trigger will advance to the next color of the palette.
    MarqueeSolid,

    /// This will display a fixed pattern the same as a marquee chase animation that will only
    /// move if the offset is changed manually where the color is always a solid constant color.
    /// The foreground trigger will advance to the next color of the palette.
    MarqueeSolidFixed,

    /// This will display a marquee style animation where the color of all the LEDs slowly fades
    /// through the colors of a palette. It will advance to the next color if externally triggered.
    MarqueeFade,

    /// This will display a fixed pattern the same as a marquee chase animation that will only move
    /// if the offset is changed manually where the color of all the LEDs slowly fades through the
    /// colors of a palette. It will advance to the next color if externally triggered.
    MarqueeFadeFixed,

    /// This will render the foreground palette based on the offset value, and leave LEDs below
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
    /// Each pulse will be a new color in the order of the palette.
    /// fade in and out times can be adjusted separately.
    ColorPulsePalette,

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
    /// of a palette.
    ColorShotPalette,

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
    /// Each flash will be a new color in the order of the palette.
    FlashPalette,
}

/// Denotes the direction of animations, effects vary depending on animation modes:
pub enum Direction {
    Positive,
    Stopped,
    Negative,
}

/// This holds the parameters that define everything needed to set up an animation. It's a struct
/// holding the parameters for the foreground animation, the background animation, and the global
/// information for trigger animations (such as the trigger Palette)
pub struct AnimationParameters<const N_BG: usize, const N_FG: usize, const N_TG: usize> {
    pub bg: AnimationBackgroundParameters<N_BG>,
    pub fg: AnimationForegroundParameters<N_FG>,
    pub trigger: AnimationGlobalTriggerParameters<N_TG>,
}

/// This contains all the information necessary to set up and run a background animation. All
/// aspects of the animation can be derived from these parameters.
pub struct AnimationBackgroundParameters<const N: usize> {
    pub mode: BackgroundMode,
    pub palette: c::Palette<N>,
    pub direction: Direction,
    pub is_palette_reversed: bool,
    pub duration_ns: u64,
    pub subdivisions: usize,
}

/// This contains all the information necessary to set up and run a foreground animation. All
/// aspects of the animation can be derived from these parameters.
pub struct AnimationForegroundParameters<const N: usize> {
    pub mode: ForegroundMode,
    pub palette: c::Palette<N>,
    pub direction: Direction,
    pub is_palette_reversed: bool,
    pub duration_ns: u64,
    pub step_time_ns: u64,
    pub subdivisions: usize,
}

/// All triggers share a single palette / slow fade speed, which is configured in this struct
pub struct AnimationGlobalTriggerParameters<const N: usize> {
    pub palette: c::Palette<N>,
    pub is_palette_reversed: bool,
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
    current_palette_color_index: usize,
}

impl AnimationTriggerState {
    fn default() -> Self {
        AnimationTriggerState {
            mode: TriggerMode::NoTrigger,
            current_frame: 0,
            total_frames: 0,
            last_direction: Direction::Positive,
            color: c::C_OFF,
            is_running: false,
            current_palette_color_index: 0,
        }
    }
}

/// This contains all the information needed to keep track of the current state of a foreground or
/// background animation. It is updated every frame to match the current state of the animation.
#[derive(Default)]
struct AnimationState {
    offset: u16,
    current_frame: u32,
    total_frames: u32,
    current_palette_color_index: usize,
    has_been_triggered: bool,
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
    active_triggers: [AnimationTriggerState; MAX_NUM_ACTIVE_TRIGGERS],
}

impl<const N_BG: usize, const N_FG: usize, const N_TG: usize, const N_LED: usize>
    Animation<N_BG, N_FG, N_TG, N_LED>
{
    pub fn new(
        parameters: AnimationParameters<N_BG, N_FG, N_TG>,
        translation_array: [usize; N_LED],
    ) -> Self {
        // Generate the LED Position Array. This is constant for every Animation based on the
        // number of LEDs <N_LED> in the animation. The LED positions are distributed evenly over
        // the entire range from 0..u16:MAX, to increase the effective supersampling resolution of
        // the animation.
        let single_led_offset = u16::MAX / N_LED as u16;
        let mut current_led_offset = 0;
        let mut led_position_array = [0_u16; N_LED];
        for led in led_position_array.iter_mut() {
            *led = current_led_offset;
            current_led_offset += single_led_offset;
        }
        Animation {
            parameters,
            translation_array,
            led_position_array,
            fg_state: AnimationState::default(),
            bg_state: AnimationState::default(),
            // Figure out how to implement Clone for this later:
            active_triggers: [
                AnimationTriggerState::default(),
                AnimationTriggerState::default(),
                AnimationTriggerState::default(),
                AnimationTriggerState::default(),
                AnimationTriggerState::default(),
                AnimationTriggerState::default(),
                AnimationTriggerState::default(),
                AnimationTriggerState::default(),
                AnimationTriggerState::default(),
                AnimationTriggerState::default(),
            ],
        }
    }

    pub fn init_total_frames(&mut self, framerate: impl Into<Hertz>) {
        let framerate = framerate.into();
        self.bg_state.total_frames = (self.parameters.bg.duration_ns * framerate.integer() as u64
            / 1_000_000_000_u64) as u32;
        self.fg_state.total_frames = (self.parameters.fg.duration_ns * framerate.integer() as u64
            / 1_000_000_000_u64) as u32;
    }

    pub fn update(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        // Update BG:
        self.update_bg(logical_strip);
        // Update FG:
        // self.update_fg(logical_strip);
        // Update Triggers:
        // self.update_triggers(logical_strip);
    }

    fn update_bg(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        match self.parameters.bg.mode {
            BackgroundMode::NoBackground => self.update_bg_no_background(logical_strip),
            BackgroundMode::Solid => self.update_bg_solid(logical_strip),
            BackgroundMode::SolidFade => self.update_bg_solid_fade(logical_strip),
            BackgroundMode::FillPalette => self.update_bg_fill_palette(logical_strip),
            BackgroundMode::FillPaletteRotate => self.update_bg_fill_palette_rotate(logical_strip),
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
                TriggerMode::Background => self.update_tg_background(logical_strip),
                TriggerMode::Foreground => self.update_tg_foreground(logical_strip),
                TriggerMode::ColorPulse => self.update_tg_color_pulse(logical_strip),
                TriggerMode::ColorPulseSlowFade => {
                    self.update_tg_color_pulse_slow_fade(logical_strip)
                }
                TriggerMode::ColorPulsePalette => self.update_tg_color_pulse_palette(logical_strip),
                TriggerMode::ColorShot => self.update_tg_color_shot(logical_strip),
                TriggerMode::ColorShotSlowFade => {
                    self.update_tg_color_shot_slow_fade(logical_strip)
                }
                TriggerMode::ColorShotPalette => self.update_tg_color_shot_palette(logical_strip),
                TriggerMode::Flash => self.update_tg_flash(logical_strip),
                TriggerMode::FlashSlowFade => self.update_tg_flash_slow_fade(logical_strip),
                TriggerMode::FlashPalette => self.update_tg_flash_palette(logical_strip),
            }
        }
    }

    pub fn trigger(&mut self, params: &AnimationTriggerParameters) {
        match params.mode {
            TriggerMode::NoTrigger => {}
            TriggerMode::Background => {
                self.bg_state.has_been_triggered = true;
            }
            TriggerMode::Foreground => {
                self.fg_state.has_been_triggered = true;
            }
            _ => todo!(),
        }
    }

    // Incrementers:

    fn increment_bg_frames(&mut self) {
        if self.bg_state.total_frames != 0 {
            self.bg_state.current_frame =
                (self.bg_state.current_frame + 1) % self.bg_state.total_frames;
        } else {
            self.bg_state.current_frame = 0
        }
    }

    fn increment_fg_frames(&mut self) {
        if self.fg_state.total_frames != 0 {
            self.fg_state.current_frame =
                (self.fg_state.current_frame + 1) % self.fg_state.total_frames;
        } else {
            self.fg_state.current_frame = 0
        }
    }

    fn increment_bg_palette_index(&mut self) {
        if N_BG != 0 {
            self.bg_state.current_palette_color_index =
                (self.bg_state.current_palette_color_index + 1) % N_BG;
        } else {
            self.bg_state.current_palette_color_index = 0
        }
    }

    fn increment_fg_palette_index(&mut self) {
        if N_FG != 0 {
            self.fg_state.current_palette_color_index =
                (self.fg_state.current_palette_color_index + 1) % N_FG;
        } else {
            self.fg_state.current_palette_color_index = 0
        }
    }

    fn increment_trigger_palette_index(&mut self, trigger_index: usize) {
        if N_TG != 0 {
            self.active_triggers[trigger_index].current_palette_color_index =
                (self.active_triggers[trigger_index].current_palette_color_index + 1) % N_TG;
        } else {
            self.active_triggers[trigger_index].current_palette_color_index = 0
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
            self.increment_bg_palette_index();
            self.bg_state.has_been_triggered = false;
        }
        // Set all LEDs to the current rainbow color. Note that in this mode the color will only
        // change when an external trigger of type `Background` is received.
        let color_index = self.bg_state.current_palette_color_index;
        let color = self.parameters.bg.palette.colors[color_index];
        for led_index in self.translation_array {
            logical_strip.set_color_at_index(led_index, color);
        }
    }

    fn update_bg_solid_fade(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        if self.bg_state.has_been_triggered {
            self.increment_bg_palette_index();
            self.bg_state.current_frame = 0;
            self.bg_state.has_been_triggered = false;
        }
        let previous_frame = self.bg_state.current_frame;
        self.increment_bg_frames();
        // Check to see when the color rolls over:
        if self.bg_state.current_frame < previous_frame {
            self.increment_bg_palette_index();
        }
        let current_color =
            self.parameters.bg.palette.colors[self.bg_state.current_palette_color_index];
        let next_color = self.parameters.bg.palette.colors
            [(self.bg_state.current_palette_color_index + 1) % N_BG];
        let intermediate_color = c::Color::color_lerp(
            self.bg_state.current_frame as i32,
            0,
            self.bg_state.total_frames as i32,
            current_color,
            next_color,
        );
        for led_index in self.translation_array {
            logical_strip.set_color_at_index(led_index, intermediate_color);
        }
    }

    fn update_bg_fill_palette(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_bg_fill_palette_rotate(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    // Foregrounds:

    fn update_fg_no_foreground(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        // Do Nothing
    }

    fn update_fg_marquee_solid(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_fg_marquee_solid_fixed(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_fg_marquee_fade(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_fg_marquee_fade_fixed(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_fg_vu_meter(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    // Triggers:

    fn update_tg_no_trigger(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        // Do Nothing
    }

    fn update_tg_background(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_foreground(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_color_pulse(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_color_pulse_slow_fade(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_color_pulse_palette(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_color_shot(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_color_shot_slow_fade(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_color_shot_palette(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_flash(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_flash_slow_fade(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }

    fn update_tg_flash_palette(&mut self, logical_strip: &mut LogicalStrip<N_LED>) {
        todo!()
    }
}

// Below here are some pre-configured const AnimationParameters variables that can be references and used
// in animations. They're also good examples of the kinds of things you can do with animations.

/// This background parameter struct can be used to turn off all background effects
pub const BG_OFF: AnimationBackgroundParameters<1> = AnimationBackgroundParameters {
    mode: BackgroundMode::NoBackground,
    palette: c::P_OFF,
    direction: Direction::Stopped,
    is_palette_reversed: false,
    duration_ns: 0,
    subdivisions: DEFAULT_NUMBER_OF_SUBDIVISIONS,
};

/// This foreground parameter struct can be used to turn off all foreground effects
pub const FG_OFF: AnimationForegroundParameters<1> = AnimationForegroundParameters {
    mode: ForegroundMode::NoForeground,
    palette: c::P_OFF,
    direction: Direction::Stopped,
    is_palette_reversed: false,
    duration_ns: 0,
    step_time_ns: 0,
    subdivisions: DEFAULT_NUMBER_OF_SUBDIVISIONS,
};

/// This global trigger parameter struct can be used to turn off all trigger effects.
pub const TRIGGER_OFF: AnimationGlobalTriggerParameters<1> = AnimationGlobalTriggerParameters {
    palette: c::P_OFF,
    is_palette_reversed: false,
    duration_ns: 0,
};

/// This animation parameter struct will turn off ALL animations: fg, bg, and trigger.
pub const ANI_ALL_OFF: AnimationParameters<1, 1, 1> =
    AnimationParameters { bg: BG_OFF, fg: FG_OFF, trigger: TRIGGER_OFF };

/// This is an animation background struct used for testing
pub const BG_TEST: AnimationBackgroundParameters<3> = AnimationBackgroundParameters {
    mode: BackgroundMode::SolidFade,
    palette: c::P_ROYGBIV,
    direction: Direction::Positive,
    is_palette_reversed: false,
    duration_ns: 10_000_000_000,
    subdivisions: DEFAULT_NUMBER_OF_SUBDIVISIONS,
};

/// This is an animation foreground struct used for testing
pub const FG_TEST: AnimationForegroundParameters<1> = AnimationForegroundParameters {
    mode: ForegroundMode::NoForeground,
    palette: c::P_OFF,
    direction: Direction::Stopped,
    is_palette_reversed: false,
    duration_ns: 0,
    step_time_ns: 0,
    subdivisions: DEFAULT_NUMBER_OF_SUBDIVISIONS,
};

/// This is an animation trigger struct used for testing
pub const TRIGGER_TEST: AnimationGlobalTriggerParameters<1> = AnimationGlobalTriggerParameters {
    palette: c::P_OFF,
    is_palette_reversed: false,
    duration_ns: 0,
};

/// This animation parameter struct will turn off ALL animations: fg, bg, and trigger.
pub const ANI_TEST: AnimationParameters<3, 1, 1> =
    AnimationParameters { bg: BG_TEST, fg: FG_TEST, trigger: TRIGGER_TEST };
