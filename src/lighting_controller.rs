use crate::colors as c;
use crate::NUM_LEDS;
use embedded_time::duration::Nanoseconds;
use embedded_time::rate::Hertz;

/// Adjust MAX_NUM_* consts depending on RAM requirements:
const MAX_NUM_ANIMATIONS: usize = 6;
const MAX_NUM_ACTIVE_TRIGGERS: usize = 10;

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
    /// at a constant rate.
    SolidFade,

    /// This will populate a rainbow's colors evenly across the LED in the animation in order. It
    /// does not animate once drawn.
    Rainbow,

    /// This will populate a rainbow like above, but it will animate it by slowly offsetting the
    /// color pattern over time.
    RainbowRotate,
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

/// This holds the parameters that define everything needed to set up an animation. It's a struct
/// holding the parameters for the foreground animation, the background animation, and the global
/// information for trigger animations (such as the trigger Rainbow)
pub struct AnimationParameters<const N: usize> {
    bg: AnimationBackgroundParameters<N>,
    fg: AnimationForegroundParameters<N>,
    trigger: AnimationTriggerParameters<N>,
}

/// This contains all the information necessary to set up and run a background animation. All
/// aspects of the animation can be derived from these parameters.
pub struct AnimationBackgroundParameters<const N: usize> {
    mode: BackgroundMode,
    rainbow: c::Rainbow<N>,
    direction: Direction,
    is_rainbow_reversed: bool,
    duration: Nanoseconds<u64>,
    subdivisions: usize,
}

/// This contains all the information necessary to set up and run a foreground animation. All
/// aspects of the animation can be derived from these parameters.
pub struct AnimationForegroundParameters<const N: usize> {
    mode: ForegroundMode,
    rainbow: c::Rainbow<N>,
    direction: Direction,
    is_rainbow_reversed: bool,
    duration: Nanoseconds<u64>,
    step_time: Nanoseconds<u64>,
    subdivisions: usize,
}

/// This contains all the information necessary to set up and run a trigger animation. All
/// aspects of the animation can be derived from these parameters. Some parameters will not have an
/// effect depending on the mode.
pub struct AnimationTriggerParameters<const N: usize> {
    mode: TriggerMode,
    rainbow: c::Rainbow<N>,
    direction: Direction,
    is_rainbow_reversed: bool,
    duration: Nanoseconds<u64>,
    step_time: Nanoseconds<u64>,
    fade_in_time: Nanoseconds<u64>,
    fade_out_time: Nanoseconds<u64>,
    starting_offset: u16,
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
    is_complete: bool,
}

/// This contains all the information needed to keep track of the current state of a foreground or
/// background animation. It is updated every frame to match the current state of the animation.
struct AnimationState {
    offset: u16,
    current_frame: u32,
    total_frames: u32,
    current_rainbow_color_index: usize,
}

/// This struct contains all the fixed parameters of an animation, as well as the state of the
/// foreground, background, and active trigger animations. It is updated by the LightingController
/// that it is attached to at the LightingController's frame rate based on the parameters provided.
/// To make a new animation,
struct Animation<const N: usize, const M: usize> {
    parameters: AnimationParameters<N>,
    translation_array: [usize; M],
    led_position_array: [u16; NUM_LEDS],
    fg_state: AnimationState,
    bg_state: AnimationState,
    active_triggers: [AnimationTriggerState; MAX_NUM_ACTIVE_TRIGGERS],
}

pub struct LightingController<'a, const N: usize, const M: usize> {
    logical_strip: [&'a mut c::Color; NUM_LEDS],
    animations: [&'a mut Animation<N, M>; MAX_NUM_ANIMATIONS],
    frame_rate: Hertz,
}
