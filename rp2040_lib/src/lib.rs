#![no_std]

pub mod bme280;
pub mod my_macro;
pub mod panic;
pub mod rtc8564;
pub mod sc2004;

pub enum ScreenState {
    Top,
    Elements,
    SetDateTime,
}

pub enum SW {
    Center,
    Down,
    Left,
    Right,
    Up,
    None,
}

pub static mut SWITCH: SW = SW::None;
