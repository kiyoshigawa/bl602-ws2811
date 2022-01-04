#![no_std]

pub mod animations;
pub mod background;
pub mod colors;
pub mod default_animations;
pub mod foreground;
pub mod hardware;
pub mod leds;
pub mod lighting_controller;
pub mod trigger;
pub mod utility;

use leds::ws28xx as strip;

pub static mut PROFILE: arrayvec::ArrayVec<usize, 512> = arrayvec::ArrayVec::new_const();

pub fn measure(start: usize) {
    unsafe {
        PROFILE
            .try_push(riscv::register::mcycle::read() - start)
            .ok();
    }
}

pub fn profile_average() -> usize {
    unsafe { PROFILE.iter().sum::<usize>() / 1.max(PROFILE.len()) }
}

pub const fn get_total_num_leds(strips: &[strip::PhysicalStrip]) -> usize {
    let mut index = 0;
    let mut total = 0;
    while index < strips.len() {
        total += strips[index].led_count;
        index += 1;
    }
    total
}
