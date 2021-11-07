use crate::animations::*;
use crate::colors as c;

// Below here are some pre-configured const AnimationParameters variables that can be references and used
// in animations. They're also good examples of the kinds of things you can do with animations.

/// This background parameter struct can be used to turn off all background effects
pub const BG_OFF: AnimationBackgroundParameters<1> = AnimationBackgroundParameters {
    mode: BackgroundMode::NoBackground,
    rainbow: c::R_OFF,
    direction: Direction::Stopped,
    is_rainbow_reversed: false,
    duration_ns: 0,
    subdivisions: DEFAULT_NUMBER_OF_SUBDIVISIONS,
};

/// This foreground parameter struct can be used to turn off all foreground effects
pub const FG_OFF: AnimationForegroundParameters<1> = AnimationForegroundParameters {
    mode: ForegroundMode::NoForeground,
    rainbow: c::R_OFF,
    direction: Direction::Stopped,
    is_rainbow_reversed: false,
    duration_ns: 0,
    step_time_ns: 0,
    subdivisions: DEFAULT_NUMBER_OF_SUBDIVISIONS,
};

/// This global trigger parameter struct can be used to turn off all trigger effects.
pub const TRIGGER_OFF: AnimationGlobalTriggerParameters<1> = AnimationGlobalTriggerParameters {
    rainbow: c::R_OFF,
    is_rainbow_reversed: false,
    duration_ns: 0,
};

/// This animation parameter struct will turn off ALL animations: fg, bg, and trigger.
pub const ANI_ALL_OFF: AnimationParameters<1, 1, 1> =
    AnimationParameters { bg: BG_OFF, fg: FG_OFF, trigger: TRIGGER_OFF };

/// This is an animation background struct used for testing
pub const BG_TEST: AnimationBackgroundParameters<3> = AnimationBackgroundParameters {
    mode: BackgroundMode::FillRainbowRotate,
    rainbow: c::R_ROYGBIV,
    direction: Direction::Positive,
    is_rainbow_reversed: false,
    duration_ns: 60_000_000_000,
    subdivisions: 0,
};

/// This is an animation foreground struct used for testing
pub const FG_TEST: AnimationForegroundParameters<1> = AnimationForegroundParameters {
    mode: ForegroundMode::NoForeground,
    rainbow: c::R_OFF,
    direction: Direction::Stopped,
    is_rainbow_reversed: false,
    duration_ns: 0,
    step_time_ns: 0,
    subdivisions: DEFAULT_NUMBER_OF_SUBDIVISIONS,
};

/// This is an animation trigger struct used for testing
pub const TRIGGER_TEST: AnimationGlobalTriggerParameters<1> = AnimationGlobalTriggerParameters {
    rainbow: c::R_OFF,
    is_rainbow_reversed: false,
    duration_ns: 0,
};

/// This animation parameter struct will turn off ALL animations: fg, bg, and trigger.
pub const ANI_TEST: AnimationParameters<3, 1, 1> =
    AnimationParameters { bg: BG_TEST, fg: FG_TEST, trigger: TRIGGER_TEST };
