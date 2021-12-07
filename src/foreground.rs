use crate::{
    a::MAX_OFFSET,
    animations::{Direction, ForegroundMode, Progression},
    c::{Color, Rainbow},
    leds::ws28xx::LogicalStrip,
};

fn marquee_solid(fg: &mut Foreground) -> Color {
    if fg.base_state.has_been_triggered {
        fg.advance_rainbow_index();
        fg.base_state.has_been_triggered = false;
    }
    fg.increment_marquee_step();
    fg.current_rainbow_color()
}

fn marquee_solid_fade(fg: &mut Foreground) -> Color {
    if fg.base_state.has_been_triggered {
        fg.advance_rainbow_index();
        fg.base_state.has_been_triggered = false;
    }

    let led_count = fg.translation_array.len();
    // calculate the marquee_position_toggle based on the set offset value:
    let pip_distance =
        (MAX_OFFSET as usize / led_count) * fg.fg_params.num_pixels_per_marquee_pip.max(1);
    let led_bucket = fg.base_state.offset as usize / pip_distance.max(1);
    fg.marquee_position_toggle = led_bucket % 2 == 0;

    fg.current_rainbow_color()
}

fn marquee_fade(fg: &mut Foreground) -> Color {
    if fg.base_state.has_been_triggered {
        fg.advance_rainbow_index();
        fg.base_state.frames.reset();
        fg.base_state.has_been_triggered = false;
    }
    fg.increment_marquee_step();

    fg.base_state.calculate_slow_fade_color()
}

fn marquee_fade_fixed(fg: &mut Foreground) -> Color {
    if fg.base_state.has_been_triggered {
        fg.advance_rainbow_index();
        fg.base_state.frames.current = 0;
        fg.base_state.has_been_triggered = false;
    }

    let led_count = fg.translation_array.len();
    // calculate the marquee_position_toggle based on the set offset value:
    let pip_distance =
        (MAX_OFFSET as usize / led_count) * fg.fg_params.num_pixels_per_marquee_pip.max(1);
    let led_bucket = fg.base_state.offset as usize / pip_distance.max(1);
    fg.marquee_position_toggle = led_bucket % 2 == 0;

    fg.base_state.calculate_slow_fade_color()
}

fn vu_meter(fg: &mut Foreground) -> Color {
    fg.current_rainbow_color();
    todo!()
}

pub struct StatefulRainbow<'a> {
    rainbow: Rainbow<'a>,
    position: Progression,
}

impl<'a> StatefulRainbow<'a> {
    fn new(rainbow: &'a [Color], is_forward: bool) -> StatefulRainbow<'a> {
        let mut position = Progression::new(rainbow.len() as u32);
        if !is_forward {
            position.current = position.total - 1;
            position.reverse_direction();
        }
        Self { rainbow, position }
    }

    fn current_color(&self) -> Color {
        self.rainbow[self.position.current as usize]
    }

    fn decrement(&mut self) {
        self.position.decrement();
    }

    fn increment(&mut self) {
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

    fn peek_next_color(&self) -> Color {
        self.rainbow[self.position.peek_next() as usize]
    }

    fn peek_last_color(&self) -> Color {
        self.rainbow[self.position.peek_prev() as usize]
    }
}

pub struct ForegroundParameters<'a> {
    pub mode: ForegroundMode,
    pub rainbow: Rainbow<'a>,
    pub direction: Direction,
    pub is_rainbow_reversed: bool,
    pub duration_ns: u64,
    pub step_time_ns: u64,
    pub subdivisions: usize,
    pub num_pixels_per_marquee_pip: usize,
}
struct AnimationState<'a> {
    offset: u16,
    frames: Progression,
    rainbow: StatefulRainbow<'a>,
    has_been_triggered: bool,
}

impl<'a> AnimationState<'a> {
    fn new(rainbow: Rainbow<'a>, frame_count: u32) -> Self {
        Self {
            offset: 0,
            frames: Progression::new(frame_count),
            rainbow: StatefulRainbow::new(rainbow, true),
            has_been_triggered: false,
        }
    }

    fn calculate_slow_fade_color(&mut self) -> Color {
        let frames = &mut self.frames;
        if frames.total == 0 {
            return self.rainbow.current_color();
        }

        let did_roll = frames.checked_increment();
        let progress = *frames;

        if did_roll {
            self.rainbow.position.increment();
        }

        let current_color = self.rainbow.current_color();
        let next_color = self.rainbow.peek_next_color();
        current_color.lerp_with(next_color, progress)
    }
}

pub(crate) struct Foreground<'a> {
    base_state: AnimationState<'a>,
    fg_params: ForegroundParameters<'a>,
    step_frames: Progression,
    marquee_position_toggle: bool,
    translation_array: &'a [usize],
    mode: Option<fn(&mut Foreground) -> Color>,
}

impl<'a> Foreground<'a> {
    fn new(init: ForegroundParameters) -> Self {
        todo!()
    }

    fn update(&mut self, logical_strip: &mut LogicalStrip) {
        if let Some(f) = self.mode {
            let color = f(self);
            self.fill_marquee(color, logical_strip);
        }
    }

    fn current_rainbow_color(&self) -> Color {
        self.base_state.rainbow.current_color()
    }

    fn advance_rainbow_index(&mut self) {
        self.base_state.rainbow.position.increment();
    }

    fn increment_marquee_step(&mut self) {
        // Increment and check to see if the color rolls over:
        let did_roll = self.step_frames.checked_increment();
        if did_roll {
            // toggle whether even or odd sub-pips are showing the marquee color:
            self.marquee_position_toggle = !self.marquee_position_toggle;
        }
    }

    fn fill_marquee(&mut self, color: Color, logical_strip: &mut LogicalStrip) {
        for &led_index in self.translation_array {
            // every time the index is evenly divisible by the number of subpixels, toggle the state
            // that the pixels should be set to:
            let px_per_pip = self.fg_params.num_pixels_per_marquee_pip;
            let subpip_number = led_index % (px_per_pip * 2);

            if subpip_number < px_per_pip && self.marquee_position_toggle {
                logical_strip.set_color_at_index(led_index, color);
            }
            if subpip_number >= px_per_pip && !self.marquee_position_toggle {
                logical_strip.set_color_at_index(led_index, color);
            }
        }
    }
}
