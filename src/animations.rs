use crate::{background, foreground, trigger};
use crate::colors::Color;
use embedded_time::rate::*;
use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};

/// Adjust MAX_NUM_* consts depending on RAM requirements:
pub(crate) const MAX_NUM_ACTIVE_TRIGGERS: usize = 10;

/// This is the maximum offset value for rotating animations. It's basically the supersampled
/// resolution of the animation over the entire translation_array of leds.
pub const MAX_OFFSET: u16 = u16::MAX;

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
    pub bg: background::Parameters<'a>,
    pub fg: foreground::Parameters<'a>,
    pub trigger: trigger::GlobalParameters<'a>,
}

/// This struct contains all the fixed parameters of an animation, as well as the state of the
/// foreground, background, and active trigger animations. It is updated by the LightingController
/// that it is attached to at the LightingController's frame rate based on the parameters provided.
/// To make a new animation,
pub struct Animation<'a, const N_LED: usize> {
    translation_array: [usize; N_LED],
    segment: [Color; N_LED],
    fg_state: foreground::Foreground<'a>,
    bg_state: background::Background<'a>,
    triggers: trigger::TriggerCollection::<'a, MAX_NUM_ACTIVE_TRIGGERS>,
    random_number_generator: SmallRng,
}

pub trait Animatable<'a> {
    fn update(&mut self);
    fn set_offset(&mut self, a_type: AnimationType, offset: u16);
    fn trigger(&mut self, params: &trigger::Parameters, frame_rate: Hertz);
    fn segment(&self) -> &[Color];
    fn translation_array(&self) -> &[usize];
}

impl<'a, const N_LED: usize> Animatable<'a> for Animation<'a, N_LED> {
    fn update(&mut self) {
        // Update all three states
        self.bg_state.update(&mut self.segment);
        self.fg_state.update(&mut self.segment);
        self.triggers.update(&mut self.segment);
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

    fn trigger(&mut self, params: &trigger::Parameters, frame_rate: Hertz) {
        let random_offset = (self.random_number_generator.next_u32() % MAX_OFFSET as u32) as u16;
        let starting_color_bucket = params.starting_offset / Self::OFFSET_BETWEEN_LEDS;
        let starting_color_offset = starting_color_bucket * Self::OFFSET_BETWEEN_LEDS;

        // let init_color_shot_trigger = |color| AnimationTriggerState {
        //     mode: params.mode,
        //     current_frame: 0,
        //     total_fade_in_frames: convert_ns_to_frames(params.step_time_ns, frame_rate),
        //     total_fade_out_frames: 0, // not used with color shots, fade in represents step duration
        //     direction: params.direction,
        //     current_offset: starting_color_offset,
        //     color,
        // };

        match params.mode {
            trigger::Mode::NoTrigger => { }
            trigger::Mode::Background => {
                self.bg_state.has_been_triggered = true;
            }
            trigger::Mode::Foreground => {
                self.fg_state.has_been_triggered = true;
            }
            _ => self.triggers.add_trigger(params, frame_rate),

        }
    }

    fn segment(&self) -> &[Color] {
        &self.segment[..]
    }

    fn translation_array(&self) -> &[usize] {
        &self.translation_array[..]
    }


}

impl<'a, const N_LED: usize> Animation<'a, N_LED> {
    const OFFSET_BETWEEN_LEDS: u16 = MAX_OFFSET / N_LED as u16;

    pub fn new(
        parameters: AnimationParameters<'a>,
        translation_array: [usize; N_LED],
        frame_rate: Hertz,
        random_seed: u64,
    ) -> Self {
        let segment = [Color::default(); N_LED];
        let fg_state = foreground::Foreground::new(&parameters.fg, frame_rate);
        let bg_state = background::Background::new(&parameters.bg, frame_rate);
        let triggers = trigger::TriggerCollection::new(&parameters.trigger, frame_rate);
        let random_number_generator = SmallRng::seed_from_u64(random_seed);

        Animation {
            translation_array,
            segment,
            fg_state,
            bg_state,
            triggers,
            random_number_generator,
        }
    }
}
