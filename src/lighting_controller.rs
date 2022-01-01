use crate::trigger;
use crate::animations::{Animatable, AnimationType};
use crate::hardware::{HardwareController, PeriodicTimer};
use crate::leds::ws28xx::LogicalStrip;
use embedded_time::duration::Nanoseconds;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::rate::Hertz;

pub struct LightingController<'a, Timer, const N_ANI: usize>
where
    Timer: PeriodicTimer,
{
    logical_strip: LogicalStrip<'a>,
    animations: [&'a mut dyn Animatable<'a>; N_ANI],
    frame_rate: Hertz,
    timer: &'a mut Timer,
}

impl<'a, Timer, const N_ANI: usize> LightingController<'a, Timer, N_ANI>
where
    Timer: PeriodicTimer,
{
    pub fn new(
        logical_strip: LogicalStrip<'a>,
        animations: [&'a mut dyn Animatable<'a>; N_ANI],
        frame_rate: impl Into<Hertz>,
        timer: &'a mut Timer,
    ) -> Self {
        let frame_rate = frame_rate.into();
        let lc = LightingController { logical_strip, animations, frame_rate, timer };
        // calculate the period of the frame rate in nanoseconds
        let frame_period = 1_000_000_000_u64 / frame_rate.integer() as u64; // 1E9 Nanoseconds / Hz = Period in ns

        // start the frame rate timer:
        lc.timer.periodic_start(Nanoseconds::<u64>(frame_period));
        lc
    }

    pub fn update(&mut self, hc: &mut HardwareController<impl PeriodicTimer>) {
        // Only update if it's been longer than the frame rate period since the last update:
        if self.timer.periodic_check_timeout().is_ok() {
            for animation in self.animations.iter_mut() {
                animation.update();

                let segment = animation.segment();
                let translater = animation.translation_array();
                let translated = translater.iter().zip(segment.iter());

                for (&index, &color) in translated {
                    self.logical_strip.set_color_at_index(index, color);
                }
            }
            self.logical_strip.send_all_sequential(hc);
        }
    }

    pub fn trigger(&mut self, animation_index: usize, params: &trigger::Parameters) {
        self.animations[animation_index].trigger(params, self.frame_rate);
    }

    pub fn set_offset(&mut self, animation_index: usize, a_type: AnimationType, offset: u16) {
        self.animations[animation_index].set_offset(a_type, offset);
    }

    pub fn replace_animation(&mut self, index: usize, new_anim: &'a mut dyn Animatable<'a>) {
        self.animations[index] = new_anim;
    }
}
