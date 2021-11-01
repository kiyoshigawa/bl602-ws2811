use crate::animations as a;
use crate::hardware::{HardwareController, PeriodicTimer};
use crate::leds::ws28xx::LogicalStrip;
use embedded_time::rate::Hertz;

pub struct LightingController<
    'a,
    const N_FG: usize,
    const N_BG: usize,
    const N_TG: usize,
    const N_LED: usize,
    const N_ANI: usize,
> {
    logical_strip: LogicalStrip<'a, N_LED>,
    animations: [a::Animation<N_BG, N_FG, N_TG, N_LED>; N_ANI],
    frame_rate: Hertz,
}

impl<
        'a,
        const N_FG: usize,
        const N_BG: usize,
        const N_TG: usize,
        const N_LED: usize,
        const N_ANI: usize,
    > LightingController<'a, N_FG, N_BG, N_TG, N_LED, N_ANI>
{
    pub fn new(
        logical_strip: LogicalStrip<'a, N_LED>,
        animations: [a::Animation<N_BG, N_FG, N_TG, N_LED>; N_ANI],
        frame_rate: impl Into<Hertz>,
    ) -> Self {
        let frame_rate = frame_rate.into();
        let mut lc = LightingController { logical_strip, animations, frame_rate };
        for animation in lc.animations.iter_mut() {
            animation.init_total_frames(lc.frame_rate);
        }
        lc
    }

    pub fn update<T>(&mut self, hc: &mut HardwareController<T>)
    where
        T: PeriodicTimer,
    {
        for animation in self.animations.iter_mut() {
            animation.update(&mut self.logical_strip);
        }
        self.logical_strip.send_all_sequential(hc);
    }
}
