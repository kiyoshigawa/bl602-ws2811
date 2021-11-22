use crate::animations as a;
use crate::animations::AnimationType;
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
    animations: [&'a mut dyn a::Animatable<'a>; N_ANI],
    frame_rate: Hertz,
    timer: &'a mut Timer,
}

impl<'a, Timer, const N_ANI: usize> LightingController<'a, Timer, N_ANI>
where
    Timer: PeriodicTimer,
{
    pub fn new(
        logical_strip: LogicalStrip<'a>,
        animations: [&'a mut (dyn a::Animatable<'a> + 'a); N_ANI],
        frame_rate: impl Into<Hertz>,
        timer: &'a mut Timer,
    ) -> Self {
        let frame_rate = frame_rate.into();
        let mut lc = LightingController { logical_strip, animations, frame_rate, timer };
        for animation in lc.animations.iter_mut() {
            animation.init_total_duration_frames(lc.frame_rate);
            animation.init_total_step_frames(lc.frame_rate);
        }
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
                animation.update(&mut self.logical_strip);
            }
            self.logical_strip.send_all_sequential(hc);
        }
    }

    pub fn trigger(&mut self, animation_index: usize, params: &a::AnimationTriggerParameters) {
        self.animations[animation_index].trigger(params);
    }

    pub fn set_offset(&mut self, animation_index: usize, a_type: AnimationType, offset: u16) {
        self.animations[animation_index].set_offset(a_type, offset);
    }
}
