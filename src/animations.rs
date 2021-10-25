use crate::colors as c;

/// Background Modes are rendered onto the animation LEDs first before any Foreground or Trigger
/// animations. The other types of animation will overwrite any pixel data from the background that
/// is effected by their animation.
pub enum BackgroundType {
    /// This turns off all the leds in the animation. It does not change once drawn:
    NoBackground,

    /// This shows a solid unchanging color on all the leds in the background. The color will be the
    /// first in the rainbow. You can step to other colors in the rainbow by external trigger,
    /// otherwise it does not change once drawn.
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
/// below the trigger animations. Ani trigger animations will overwrite the pixel data from the
/// foreground that is effected by their animation.
pub enum ForegroundType {
    /// This is a mode that has no additional foreground animation over the background animation.
    NoForeground,

    /// This will display a fixed pattern the same as a marquee chase animation that will only
    /// move if the offset is changed manually where the color is always a solid constant color.
    MarqueeSolidFixed,

    /// This will display a single-color marquee style pixel chase animation.
    MarqueeSolidFade,

    /// This will display a fixed pattern the same as a marquee chase animation that will only move
    /// if the offset is changed manually where the color of all the LEDs slowly fades through the
    /// colors of a rainbow.
    MarqueeFadeFixed,

    /// This will display a marquee style animation where the color of all the LEDs slowly fades
    /// through the colors of a rainbow.
    MarqueeFade,

    /// This will render the foreground rainbow based on the offset value, and leave LEDs below
    /// the offset value alone.
    VUMeter,
}

/// These are the types of triggered animation effects that are possible with an animation. They can
/// be mixed and matched at any time over any combination of foreground and background animations.
/// The trigger animation colors will override any foreground or background pixel data on the pixels
/// it effects.
pub enum TriggerType {
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

/// An enum to make the code easier to read:
pub enum Direction {
    Positive,
    Stopped,
    Negative,
}

pub struct Animation {
    buffer_size: usize,
}

impl Animation {
    pub fn new(buffer_size: usize) -> Self {
        Animation { buffer_size }
    }
}
