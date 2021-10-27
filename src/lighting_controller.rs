use crate::animations as a;
use crate::colors as c;
use embedded_time::rate::Hertz;

/// Adjust MAX_NUM_* consts depending on RAM requirements:
const MAX_NUM_ANIMATIONS: usize = 6;

pub struct LightingController<
    'a,
    const N_FG: usize,
    const N_BG: usize,
    const N_TG: usize,
    const N_LED: usize,
    const N_ANI: usize,
> {
    logical_strip: &'a mut [c::Color],
    animations: [&'a mut a::Animation<N_BG, N_FG, N_TG, N_LED>; N_ANI],
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
        logical_strip: &'a mut [c::Color],
        animations: [&'a mut a::Animation<N_BG, N_FG, N_TG, N_LED>; N_ANI],
        frame_rate: impl Into<Hertz>,
    ) -> Self {
        LightingController { logical_strip, animations, frame_rate: frame_rate.into() }
    }
}
