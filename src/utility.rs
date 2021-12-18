use crate::{c::{Color, Rainbow}};
use embedded_time::{fixed_point::FixedPoint, rate::Hertz};

pub fn convert_ns_to_frames(nanos: u64, frame_rate: Hertz) -> u32 {
    (nanos * frame_rate.integer() as u64 / 1_000_000_000_u64) as u32
}

pub fn get_random_offset() -> u16 {
    0
}

pub trait MarchingRainbow {
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

    fn current_rainbow_color(&self) -> Color {
        self.rainbow().current_color()
    }
}

pub trait MarchingRainbowMut {
    fn rainbow_mut(&mut self) -> &mut StatefulRainbow;
    fn frames_mut(&mut self) -> &mut Progression;

    /// Advances the rainbow color and resets the frame count
    fn advance_rainbow_color_hard(&mut self) {
        self.rainbow_mut().increment();
        self.frames_mut().reset();
    }

    /// Advances the rainbow while keeping the frame count, currenty unused
    fn advance_rainbow_color_soft(&mut self) {
        self.rainbow_mut().increment();
    }
}

pub struct TimedRainbow<'a, 'b> {
    pub rainbow: &'b mut StatefulRainbow<'a>,
    pub frames: &'b mut Progression,
}

impl<'a, 'b> MarchingRainbow for TimedRainbow<'a, 'b> {
    fn rainbow(&self) -> &StatefulRainbow { self.rainbow }
    fn frames(&self) -> &Progression { self.frames }
}

impl<'a, 'b> MarchingRainbowMut for TimedRainbow<'a, 'b> {
    fn rainbow_mut(&mut self) -> &'a mut StatefulRainbow { self.rainbow }
    fn frames_mut(&mut self) -> &mut Progression { self.frames }
}


pub struct StatefulRainbow<'a> {
    pub backer: Rainbow<'a>,
    pub position: Progression,
}

impl<'a> StatefulRainbow<'a> {
    pub fn new(rainbow: &'a [Color], is_forward: bool) -> StatefulRainbow<'a> {
        let mut position = Progression::new(rainbow.len() as u32);
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
    current: u32,
    pub total: u32,
    pub is_forward: bool,
}

impl Progression {
    pub fn new(total: u32) -> Self {
        Self { current: 0, total, is_forward: true }
    }

    pub fn reverse_direction(&mut self) {
        self.is_forward = !self.is_forward;
    }

    fn is_mono(&self) -> bool {
        self.total <= 1
    }

    pub fn get_current(&self) -> u32 {
        if self.is_mono() { return 0; }
        match self.is_forward {
            true => self.current,
            false => self.total - 1 - self.current,
        }
    }

    pub fn set_current(&mut self, value: u32) {
        if self.is_mono() { return }
        let value = value % self.total;
        self.current = match self.is_forward {
            true  => value,
            false => self.total - 1 - value,
        }
    }

    pub fn decrement(&mut self) {
        if self.is_mono() { return; }
        self.current = self.peek_prev();
    }

    pub fn checked_decrement(&mut self) -> bool {
        if self.is_mono() { return false; }
        self.decrement();
        self.current == self.total - 1
    }

    pub fn increment(&mut self) {
        if self.is_mono() { return; }
        self.current = self.peek_next();
    }

    pub fn checked_increment(&mut self) -> bool {
        if self.is_mono() { return false; }
        self.increment();
        self.current == 0
    }

    pub fn peek_next(&self) -> u32 {
        self.up_one()
    }

    pub fn peek_prev(&self) -> u32 {
        self.down_one()
    }

    fn up_one(&self) -> u32 {
        if self.is_mono() { return 0; }
        (self.current + 1) % self.total
    }

    fn down_one(&self) -> u32 {
        if self.is_mono() { return 0; }
        (self.current + self.total -1 ) % self.total
    }

    pub fn reset(&mut self) {
        self.current = 0
    }
}

impl Color {
    pub fn lerp_with(&self, to_color: Color, factor: Progression) -> Color {
        Color::color_lerp(
            factor.get_current() as i32,
            0,
            factor.total as i32,
            *self,
            to_color,
        )
    }
}