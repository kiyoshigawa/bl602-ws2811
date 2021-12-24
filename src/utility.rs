use core::ops::Index;

use crate::{
    animations::MAX_OFFSET,
    colors::{Color, Rainbow},
};
use embedded_time::{fixed_point::FixedPoint, rate::Hertz};

pub fn convert_ns_to_frames(nanos: u64, frame_rate: Hertz) -> usize {
    (nanos * frame_rate.integer() as u64 / 1_000_000_000_u64) as usize
}

pub fn convert_ms_to_frames(millis: u64, frame_rate: Hertz) -> usize {
    (millis * frame_rate.integer() as u64 / 1_000_u64) as usize
}

pub fn get_random_offset() -> u16 {
    riscv::register::mcycle::read64() as u16
}

fn get_color_at_offset(rainbow: &ModdedRainbow, subdivisions: usize, offset: u16) -> Color {
    let rainbow_length = rainbow.len();
    let full_color_count = rainbow_length * subdivisions;
    let next_color_distance = MAX_OFFSET as usize / full_color_count;

    let offset = offset as usize;
    let factor = offset % next_color_distance;

    let start_color_index = (offset / next_color_distance) % rainbow_length;
    let start_color = rainbow[start_color_index];

    let end_color_index = (start_color_index + 1) % rainbow_length;
    let end_color = rainbow[end_color_index];

    Color::color_lerp(factor as i32, 0, next_color_distance as i32, start_color, end_color)
}

pub fn shift_offset(starting_offset: u16, frames: Progression) -> u16 {
    if frames.total == 0 {
        return starting_offset;
    }

    let starting_offset = starting_offset as usize;
    let offset_shift = MAX_OFFSET as usize * frames.get_current() / frames.total;
    (starting_offset + offset_shift) as u16
}

struct ModdedRainbow<'a> {
    backer: Rainbow<'a>,
    is_forward: bool,
}

impl<'a> ModdedRainbow<'a> {
    pub fn len(&self) -> usize {
        self.backer.len()
    }
}

impl<'a> Index<usize> for ModdedRainbow<'a> {
    type Output = Color;

    fn index(&self, index: usize) -> &Self::Output {
        match self.is_forward {
            true => &self.backer[index],
            false => &self.backer[self.backer.len() - 1 - index],
        }
    }
}

pub trait SlowFadeRainbow {
    fn rainbow(&self) -> &StatefulRainbow;
    fn frames(&self) -> &Progression;

    fn calculate_slow_fade_color(&self) -> Color {
        let (rainbow, frames) = (self.rainbow(), self.frames());

        let current_color = rainbow.current_color();
        if frames.total == 0 {
            return current_color;
        }
        let next_color = rainbow.peek_next_color();
        current_color.lerp_with(next_color, *frames)
    }

    fn current_slow_fade_color(&self) -> Color {
        self.rainbow().current_color()
    }
}

pub trait MarchingRainbow {
    fn rainbow(&self) -> &StatefulRainbow;
    fn frames(&self) -> &Progression;

    // fn calculate_slow_fade_color(&self) -> Color {
    //     let (rainbow, frames) = (self.rainbow(), self.frames());

    //     let current_color = rainbow.current_color();
    //     if frames.total == 0 {
    //         return current_color;
    //     }
    //     let next_color = rainbow.peek_next_color();
    //     current_color.lerp_with(next_color, *frames)
    // }

    fn current_rainbow_color(&self) -> Color {
        self.rainbow().current_color()
    }
}

pub trait MarchingRainbowMut {
    fn rainbow_mut(&mut self) -> &mut StatefulRainbow;
    fn frames_mut(&mut self) -> &mut Progression;

    /// Advances the rainbow color and resets the frame count
    fn advance_rainbow_color(&mut self) {
        self.rainbow_mut().increment();
        self.frames_mut().reset();
    }
}

pub struct TimedRainbows<'a, 'b> {
    pub slow_fade_rainbow: &'b mut StatefulRainbow<'a>,
    pub incremental_rainbow: &'b mut StatefulRainbow<'a>,
    pub frames: &'b mut Progression,
}

impl<'a, 'b> SlowFadeRainbow for TimedRainbows<'a, 'b> {
    fn rainbow(&self) -> &StatefulRainbow {
        self.slow_fade_rainbow
    }
    fn frames(&self) -> &Progression {
        self.frames
    }
}

impl<'a, 'b> MarchingRainbow for TimedRainbows<'a, 'b> {
    fn rainbow(&self) -> &StatefulRainbow {
        self.incremental_rainbow
    }
    fn frames(&self) -> &Progression {
        self.frames
    }
}

impl<'a, 'b> MarchingRainbowMut for TimedRainbows<'a, 'b> {
    fn rainbow_mut(&mut self) -> &'a mut StatefulRainbow {
        self.incremental_rainbow
    }
    fn frames_mut(&mut self) -> &mut Progression {
        self.frames
    }
}

pub struct StatefulRainbow<'a> {
    pub backer: Rainbow<'a>,
    pub position: Progression,
}

impl<'a> StatefulRainbow<'a> {
    pub fn new(rainbow: &'a [Color], is_forward: bool) -> StatefulRainbow<'a> {
        let mut position = Progression::new(rainbow.len());
        position.is_forward = is_forward;

        Self { backer: rainbow, position }
    }

    pub fn current_color(&self) -> Color {
        self.backer[self.position.get_current() as usize]
    }

    fn decrement(&mut self) {
        self.position.decrement();
    }

    pub fn increment(&mut self) {
        self.position.increment();
    }

    pub fn peek_next_color(&self) -> Color {
        self.backer[self.position.peek_next() as usize]
    }

    pub fn peek_last_color(&self) -> Color {
        self.backer[self.position.peek_prev() as usize]
    }

    pub fn reset(&mut self) {
        self.position.reset();
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct Progression {
    current: usize,
    pub total: usize,
    pub is_forward: bool,
}

impl Progression {
    pub fn new(total: usize) -> Self {
        Self { current: 0, total, is_forward: true }
    }

    pub fn reverse_direction(&mut self) {
        self.is_forward = !self.is_forward;
    }

    fn is_mono(&self) -> bool {
        self.total <= 1
    }

    pub fn is_first_frame(&self) -> bool {
        self.current == 0
    }

    pub fn get_current(&self) -> usize {
        if self.is_mono() {
            return 0;
        }
        match self.is_forward {
            true => self.current,
            false => self.total - 1 - self.current,
        }
    }

    pub fn set_current(&mut self, value: usize) {
        if self.is_mono() {
            return;
        }
        let value = value % self.total;
        self.current = value;
    }

    pub fn decrement(&mut self) {
        if self.is_mono() {
            return;
        }
        self.current = self.peek_prev();
    }

    pub fn checked_decrement(&mut self) -> bool {
        if self.is_mono() {
            return false;
        }
        self.decrement();
        self.current == self.total - 1
    }

    pub fn increment(&mut self) {
        if self.is_mono() {
            return;
        }
        self.current = self.peek_next();
    }

    pub fn checked_increment(&mut self) -> bool {
        if self.is_mono() {
            return false;
        }
        self.increment();
        self.current == 0
    }

    pub fn peek_next(&self) -> usize {
        self.up_one()
    }

    pub fn peek_prev(&self) -> usize {
        self.down_one()
    }

    fn up_one(&self) -> usize {
        if self.is_mono() {
            return 0;
        }
        (self.current + 1) % self.total
    }

    fn down_one(&self) -> usize {
        if self.is_mono() {
            return 0;
        }
        (self.current + self.total - 1) % self.total
    }

    pub fn reset(&mut self) {
        self.current = 0
    }
}

impl Color {
    pub fn lerp_with(&self, to_color: Color, factor: Progression) -> Color {
        Color::color_lerp(factor.get_current() as i32, 0, factor.total as i32, *self, to_color)
    }
}
