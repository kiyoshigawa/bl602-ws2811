use embedded_time::{fixed_point::FixedPoint, rate::Hertz};

use crate::c::{Color, Rainbow};

pub fn convert_ns_to_frames(nanos: u64, frame_rate: Hertz) -> u32 {
    (nanos * frame_rate.integer() as u64 / 1_000_000_000_u64) as u32
}

pub fn get_random_offset() -> u16 {
    0
}

pub struct TimedRainbow<'a> {
    pub rainbow: StatefulRainbow<'a>,
    pub frames: Progression,
}
impl<'a> TimedRainbow<'a> {
    pub fn get_slow_fade_color(&self) -> Color {
        let current_color = self.rainbow.current_color();
        if self.frames.total == 0 {
            return current_color;
        }
        let next_color = self.rainbow.peek_next_color();
        current_color.lerp_with(next_color, self.frames)
    }

    pub fn get_current_color(&self) -> Color {
        self.rainbow.current_color()
    }

    pub fn increment_frame(&mut self) {
        let did_roll = self.frames.checked_increment();
        if did_roll {
            self.rainbow.increment();
        }
    }
    pub fn increment_color(&mut self) {
        self.rainbow.increment();
        self.frames.reset();
    }
}

pub struct StatefulRainbow<'a> {
    pub backer: Rainbow<'a>,
    pub position: Progression,
}

impl<'a> StatefulRainbow<'a> {
    pub fn new(rainbow: &'a [Color], is_forward: bool) -> StatefulRainbow<'a> {
        let mut position = Progression::new(rainbow.len() as u32);
        if !is_forward {
            position.current = position.total - 1;
            position.reverse_direction();
        }
        Self { backer: rainbow, position }
    }

    pub fn current_color(&self) -> Color {
        self.backer[self.position.current as usize]
    }

    fn decrement(&mut self) {
        self.position.decrement();
    }

    pub fn increment(&mut self) {
        self.position.increment();
    }

    fn prev_color(&mut self) -> Color {
        self.increment();
        self.current_color()
    }

    fn next_color(&mut self) -> Color {
        self.increment();
        self.current_color()
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
    pub current: u32,
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

    pub fn decrement(&mut self) {
        if self.total == 0 { return; }
        match self.is_forward {
            true => self.current = self.down_one(),
            false => self.current = self.up_one(),
        }
    }

    pub fn checked_decrement(&mut self) -> bool {
        self.decrement();
        self.current == self.total - 1
    }

    pub fn increment(&mut self) {
        if self.total == 0 { return; }
        match self.is_forward {
            true => self.current = self.up_one(),
            false => self.current = self.down_one(),
        }
    }

    pub fn checked_increment(&mut self) -> bool {
        self.increment();
        self.current == 0
    }

    pub fn peek_next(&self) -> u32 {
        match self.is_forward {
            true => self.up_one(),
            false => self.down_one(),
        }
    }

    pub fn peek_prev(&self) -> u32 {
        match self.is_forward {
            true => self.down_one(),
            false => self.up_one(),
        }
    }

    pub fn up_one(&self) -> u32 {
        if self.total == 0 { return 0; }
        (self.current + 1) % self.total
    }

    pub fn down_one(&self) -> u32 {
        if self.total == 0 { return 0; }
        (self.current + self.total -1 ) % self.total
    }

    pub fn reset(&mut self) {
        self.current = match self.is_forward {
            true => 0,
            false => self.total - 1,
        }
    }
}

impl Color {
    pub fn lerp_with(&self, to_color: Color, factor: Progression) -> Color {
        Color::color_lerp(
            factor.current as i32,
            0,
            factor.total as i32,
            *self,
            to_color,
        )
    }
}