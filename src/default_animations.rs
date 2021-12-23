use crate::animations::*;
use crate::background;
use crate::colors as c;
use crate::foreground;
use crate::trigger;

/// This value is used as a default value for the number of subdivisions on the const animations at
/// the end of the file. Typically this number should be 1 for shorter strips, and higher as you add
/// more LEDs. It controls how often the rainbow colors repeat over the entire length of LEDs. If 0
/// is used, it will automatically be upsized to 1 subdivision, as you can't divide a rainbow by 0
pub const DEFAULT_NUMBER_OF_SUBDIVISIONS: usize = 1;

/// This value is used to determine the default value for how many leds in a row are used as a
/// single pip for a marquee animation. If it is set to 0, it will automatically be upsized to 1,
/// as division by 0 is generally considered bad form.
pub const DEFAULT_NUMBER_OF_PIXELS_PER_MARQUEE_PIP: usize = 1;

// Below here are some pre-configured const AnimationParameters variables that can be references and used
// in animations. They're also good examples of the kinds of things you can do with animations.

/// This background parameter struct can be used to turn off all background effects
pub const BG_OFF: background::Parameters = background::Parameters {
    mode: background::Mode::NoBackground,
    rainbow: c::R_OFF,
    direction: Direction::Stopped,
    is_rainbow_forward: true,
    duration_ns: 0,
    subdivisions: DEFAULT_NUMBER_OF_SUBDIVISIONS,
};

/// This foreground parameter struct can be used to turn off all foreground effects
pub const FG_OFF: foreground::Parameters = foreground::Parameters {
    mode: foreground::Mode::NoForeground,
    rainbow: c::R_OFF,
    direction: Direction::Stopped,
    is_rainbow_forward: true,
    duration_ns: 0,
    step_time_ns: 0,
    subdivisions: DEFAULT_NUMBER_OF_SUBDIVISIONS,
    pixels_per_pixel_group: DEFAULT_NUMBER_OF_PIXELS_PER_MARQUEE_PIP,
};

/// This global trigger parameter struct can be used to turn off all trigger effects.
pub const TRIGGER_OFF: trigger::GlobalParameters =
    trigger::GlobalParameters { rainbow: c::R_OFF, is_rainbow_forward: true, duration_ns: 0 };

/// This animation parameter struct will turn off ALL animations: fg, bg, and trigger.
pub const ANI_ALL_OFF: AnimationParameters =
    AnimationParameters { bg: BG_OFF, fg: FG_OFF, trigger: TRIGGER_OFF };

/// This is an animation background struct used for testing
pub const BG_TEST: background::Parameters = background::Parameters {
    mode: background::Mode::FillRainbowRotate,
    rainbow: c::R_ROYGBIV,
    direction: Direction::Positive,
    is_rainbow_forward: true,
    duration_ns: 20_000_000_000,
    subdivisions: 0,
};

/// This is an animation foreground struct used for testing
pub const FG_TEST: foreground::Parameters = foreground::Parameters {
    mode: foreground::Mode::NoForeground,
    rainbow: c::R_OFF,
    direction: Direction::Positive,
    is_rainbow_forward: true,
    duration_ns: 10_000_000_000,
    step_time_ns: 1_000_000_000,
    subdivisions: DEFAULT_NUMBER_OF_SUBDIVISIONS,
    pixels_per_pixel_group: 1,
};

/// This is an animation trigger struct used for testing
pub const TRIGGER_TEST: trigger::GlobalParameters = trigger::GlobalParameters {
    rainbow: c::R_ROYGBIV,
    is_rainbow_forward: true,
    duration_ns: 10_000_000_000,
};

/// This animation parameter struct will turn off ALL animations: fg, bg, and trigger.
pub const ANI_TEST: AnimationParameters =
    AnimationParameters { bg: BG_TEST, fg: FG_TEST, trigger: TRIGGER_TEST };
