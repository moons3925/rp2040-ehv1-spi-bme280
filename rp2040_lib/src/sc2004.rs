use embedded_hal_0_2::blocking::delay::{DelayMs, DelayUs};
use hd44780_driver::{bus::FourBitBus, Cursor, CursorBlink, Display, DisplayMode, HD44780}; // embedded-hal ver0.2.x

use rp2040_hal::gpio::bank0::Gpio20;
use rp2040_hal::gpio::bank0::Gpio21;
use rp2040_hal::gpio::bank0::Gpio22;
use rp2040_hal::gpio::bank0::Gpio26;
use rp2040_hal::gpio::bank0::Gpio27;
use rp2040_hal::gpio::bank0::Gpio28;
use rp2040_hal::gpio::FunctionSio;
use rp2040_hal::gpio::Pin;
use rp2040_hal::gpio::PullDown;
use rp2040_hal::gpio::PullUp;
use rp2040_hal::gpio::SioOutput;

//use crate::rtc8564::Time;

use crate::rtc8564::RTC8564;
use rp2040_hal::gpio::bank0::Gpio16;
use rp2040_hal::gpio::bank0::Gpio17;
use rp2040_hal::gpio::FunctionI2c;
use rp2040_hal::pac::I2C0;
use rp2040_hal::I2C;

use crate::ScreenState;
use crate::SW;
use crate::SWITCH;

const DDRAM_ADDRESS_FIRST: u8 = 0;
const DDRAM_ADDRESS_SECOND: u8 = 0x40;
const DDRAM_ADDRESS_THIRD: u8 = 0x14;
const DDRAM_ADDRESS_FOURTH: u8 = 0x54;

const DDRAM_ADDRESS_SET_DATE_TIME_TOP: u8 = DDRAM_ADDRESS_THIRD + 2;

const POCHI_CODE: u8 = 0xdf; // °のコード

use crate::rtc8564::CONTROL1_REG;
//use crate::rtc8564::CONTROL2_REG;
use crate::rtc8564::DAYS_REG;
use crate::rtc8564::HOURS_REG;
use crate::rtc8564::MINUTES_REG;
use crate::rtc8564::MONTHS_CENTURY_REG;
use crate::rtc8564::SECONDS_REG;
use crate::rtc8564::WEEKDAYS_REG;
use crate::rtc8564::YEARS_REG;

const STOP_THE_CLOCK: u8 = 0x20;
const START_THE_CLOCK: u8 = 0x00;

pub struct SC2004 {
    interface: HD44780<
        FourBitBus<
            Pin<Gpio28, FunctionSio<SioOutput>, PullDown>,
            Pin<Gpio27, FunctionSio<SioOutput>, PullDown>,
            Pin<Gpio26, FunctionSio<SioOutput>, PullDown>,
            Pin<Gpio22, FunctionSio<SioOutput>, PullDown>,
            Pin<Gpio21, FunctionSio<SioOutput>, PullDown>,
            Pin<Gpio20, FunctionSio<SioOutput>, PullDown>,
        >,
    >,
    address: u8,
    position: u8,
    top_initialized: bool,
    elements_initialized: bool,
    date_time_initialized: bool,
    set_date_time_initialized: bool,
    set_position: i32,
    set_date_time_up_down: bool,
    y10: u8,
    y1: u8,
    mo10: u8,
    mo1: u8,
    d10: u8,
    d1: u8,
    h10: u8,
    h1: u8,
    mi10: u8,
    mi1: u8,
    s10: u8,
    s1: u8,
    buf2: [u8; 20],
    buf3: [u8; 20],
    buf4: [u8; 20],
}

impl SC2004 {
    pub fn new(
        interface: HD44780<
            FourBitBus<
                Pin<Gpio28, FunctionSio<SioOutput>, PullDown>,
                Pin<Gpio27, FunctionSio<SioOutput>, PullDown>,
                Pin<Gpio26, FunctionSio<SioOutput>, PullDown>,
                Pin<Gpio22, FunctionSio<SioOutput>, PullDown>,
                Pin<Gpio21, FunctionSio<SioOutput>, PullDown>,
                Pin<Gpio20, FunctionSio<SioOutput>, PullDown>,
            >,
        >,
    ) -> Self {
        Self {
            interface,
            address: 0,
            position: 0,
            top_initialized: false,
            elements_initialized: false,
            date_time_initialized: false,
            set_date_time_initialized: false,
            set_position: 0,
            set_date_time_up_down: false,
            y10: 0,
            y1: 0,
            mo10: 0,
            mo1: 1,
            d10: 0,
            d1: 1,
            h10: 0,
            h1: 0,
            mi10: 0,
            mi1: 0,
            s10: 0,
            s1: 0,
            buf2: [0; 20],
            buf3: [0; 20],
            buf4: [0; 20],
        }
    }
    pub fn init<D: DelayUs<u16> + DelayMs<u8>>(&mut self, delay: &mut D) {
        let _ = self.interface.reset(delay);
        let _ = self.interface.clear(delay);
        let _ = self.interface.set_display_mode(
            DisplayMode {
                display: Display::On,
                cursor_visibility: Cursor::Invisible,
                cursor_blink: CursorBlink::Off,
            },
            delay,
        );
    }
    pub fn clear_screen<D: DelayUs<u16> + DelayMs<u8>>(&mut self, delay: &mut D) {
        let _ = self.interface.clear(delay);
    }

    pub fn set_elements<D: DelayUs<u16> + DelayMs<u8>>(
        &mut self,
        delay: &mut D,
        tup: (f64, f64, f64),
        rtc: &mut RTC8564<
            I2C<
                I2C0,
                (
                    Pin<Gpio16, FunctionI2c, PullUp>,
                    Pin<Gpio17, FunctionI2c, PullUp>,
                ),
            >,
        >,
        state: &mut ScreenState,
    ) {
        if !self.elements_initialized {
            self.elements_initialized = true;

            let s_temp = "Temp:     .   C     "; // buf2[]の長さに合わせること、さもないと panic する
            let s_humi = "Humi:     .  %      "; // buf3[]　同上
            let s_pres = "Pres:     .  hPa    "; // buf4[]　同上

            let ref_temp = s_temp.as_bytes();
            let ref_humi = s_humi.as_bytes();
            let ref_pres = s_pres.as_bytes();

            for i in 0..ref_temp.len() {
                self.buf2[i] = ref_temp[i];
            }
            self.buf2[13] = POCHI_CODE; // ° を表示するコード

            for i in 0..ref_humi.len() {
                self.buf3[i] = ref_humi[i];
            }
            for i in 0..ref_pres.len() {
                self.buf4[i] = ref_pres[i];
            }
        }

        self.display_date_time(delay, rtc);

        let tens_digit: u8 = ((tup.0 as i32 / 10) % 10) as u8 | b'0';
        let ones_digit: u8 = ((tup.0 as i32) % 10) as u8 | b'0';
        let tenths_digit: u8 = (((tup.0 * 10.0) as i32) % 10) as u8 | b'0';

        self.buf2[8] = tens_digit;
        self.buf2[9] = ones_digit;
        self.buf2[11] = tenths_digit;

        let tens_digit: u8 = ((tup.1 as i32 / 10) % 10) as u8 | b'0';
        let ones_digit: u8 = ((tup.1 as i32) % 10) as u8 | b'0';
        let tenths_digit: u8 = (((tup.1 * 10.0) as i32) % 10) as u8 | b'0';

        self.buf3[8] = tens_digit;
        self.buf3[9] = ones_digit;
        self.buf3[11] = tenths_digit;

        let mut thousands_digit: u8 = ((tup.2 as i32 / 1000) % 10) as u8 | b'0';
        let hundreds_digit: u8 = ((tup.2 as i32 / 100) % 10) as u8 | b'0';
        let tens_digit: u8 = ((tup.2 as i32 / 10) % 10) as u8 | b'0';
        let ones_digit: u8 = ((tup.2 as i32) % 10) as u8 | b'0';
        let tenths_digit: u8 = (((tup.2 * 10.0) as i32) % 10) as u8 | b'0';

        if thousands_digit == b'0' {
            thousands_digit = b' ';
        }

        self.buf4[6] = thousands_digit;
        self.buf4[7] = hundreds_digit;
        self.buf4[8] = tens_digit;
        self.buf4[9] = ones_digit;
        self.buf4[11] = tenths_digit;

        let _ = self.interface.set_cursor_pos(DDRAM_ADDRESS_SECOND, delay);
        for i in 0..20 {
            let _ = self.interface.write_char(self.buf2[i] as char, delay);
        }
        let _ = self.interface.set_cursor_pos(DDRAM_ADDRESS_THIRD, delay);
        for i in 0..20 {
            let _ = self.interface.write_char(self.buf3[i] as char, delay);
        }
        let _ = self.interface.set_cursor_pos(DDRAM_ADDRESS_FOURTH, delay);
        for i in 0..20 {
            let _ = self.interface.write_char(self.buf4[i] as char, delay);
        }

        let _ = self
            .interface
            .set_cursor_visibility(Cursor::Invisible, delay);
        unsafe {
            match SWITCH {
                SW::None => (),
                _ => {
                    // None 以外（何かのSW押下で）
                    *state = ScreenState::Top;
                    SWITCH = SW::None;
                    self.elements_initialized = false;
                    self.date_time_initialized = false;
                    let _ = self.interface.set_display_mode(
                        DisplayMode {
                            display: Display::On,
                            cursor_visibility: Cursor::Visible,
                            cursor_blink: CursorBlink::Off,
                        },
                        delay,
                    );
                }
            }
        }
    }

    pub fn set_cursor_visibility<D: DelayUs<u16> + DelayMs<u8>>(
        &mut self,
        visibility: Cursor,
        delay: &mut D,
    ) {
        let _ = self.interface.set_cursor_visibility(visibility, delay);
    }
    pub fn display_date_time<D: DelayUs<u16> + DelayMs<u8>>(
        &mut self,
        delay: &mut D,
        rtc: &mut RTC8564<
            I2C<
                I2C0,
                (
                    Pin<Gpio16, FunctionI2c, PullUp>,
                    Pin<Gpio17, FunctionI2c, PullUp>,
                ),
            >,
        >,
    ) {
        if !self.date_time_initialized {
            self.date_time_initialized = true;
        }
        let t = rtc.read_date_time();

        let s: &mut [u8; 19] = &mut [
            b'2', b'0', b'2', b'4', b'/', b'0', b'1', b'/', b'0', b'1', b' ', b'0', b'1', b':',
            b'0', b'0', b':', b'0', b'0',
        ];

        let _ = self.interface.set_cursor_pos(0x00, delay);

        let mut dig10: u8 = 0;
        let mut dig1: u8 = 0;

        dig10 = (t.year / 10) | b'0';
        dig1 = (t.year % 10) | b'0';

        s[2] = dig10;
        s[3] = dig1;

        dig10 = (t.month / 10) | b'0';
        dig1 = (t.month % 10) | b'0';

        s[5] = dig10;
        s[6] = dig1;

        dig10 = (t.day / 10) | b'0';
        dig1 = (t.day % 10) | b'0';

        s[8] = dig10;
        s[9] = dig1;

        dig10 = (t.hour / 10) | b'0';
        dig1 = (t.hour % 10) | b'0';

        s[11] = dig10;
        s[12] = dig1;

        dig10 = (t.minute / 10) | b'0';
        dig1 = (t.minute % 10) | b'0';

        s[14] = dig10;
        s[15] = dig1;

        dig10 = (t.second / 10) | b'0';
        dig1 = (t.second % 10) | b'0';

        s[17] = dig10;
        s[18] = dig1;

        let _ = self.interface.write_bytes(s, delay);
    }

    pub fn set_date_time<D: DelayUs<u16> + DelayMs<u8>>(
        &mut self,
        delay: &mut D,
        rtc: &mut RTC8564<
            I2C<
                I2C0,
                (
                    Pin<Gpio16, FunctionI2c, PullUp>,
                    Pin<Gpio17, FunctionI2c, PullUp>,
                ),
            >,
        >,
        state: &mut ScreenState,
    ) {
        if !self.set_date_time_initialized {
            self.set_date_time_initialized = true;
            let _ = self.interface.set_cursor_pos(DDRAM_ADDRESS_FIRST, delay);
            let _ = self.interface.write_str(" Set Date Time      ", delay);
            let _ = self.interface.set_cursor_pos(DDRAM_ADDRESS_SECOND, delay);
            let _ = self.interface.write_str("                    ", delay);
            let _ = self.interface.set_cursor_pos(DDRAM_ADDRESS_FOURTH, delay);
            let _ = self.interface.write_str("                    ", delay);
            let _ = self.interface.set_cursor_visibility(Cursor::Visible, delay);

            let t = rtc.read_date_time();
            let _ = self.interface.set_cursor_pos(DDRAM_ADDRESS_THIRD, delay);
            let _ = self.interface.write_char(b'2' as char, delay);
            let _ = self.interface.write_char(b'0' as char, delay);
            let dig10 = (t.year / 10) | b'0';
            let dig1 = (t.year % 10) | b'0';
            self.y10 = dig10;
            self.y1 = dig1;
            let _ = self.interface.write_char(self.y10 as char, delay);
            let _ = self.interface.write_char(self.y1 as char, delay);
            let _ = self.interface.write_char(b'/' as char, delay);

            let dig10 = (t.month / 10) | b'0';
            let dig1 = (t.month % 10) | b'0';
            self.mo10 = dig10;
            self.mo1 = dig1;
            let _ = self.interface.write_char(self.mo10 as char, delay);
            let _ = self.interface.write_char(self.mo1 as char, delay);
            let _ = self.interface.write_char(b'/' as char, delay);

            let dig10 = (t.day / 10) | b'0';
            let dig1 = (t.day % 10) | b'0';
            self.d10 = dig10;
            self.d1 = dig1;
            let _ = self.interface.write_char(self.d10 as char, delay);
            let _ = self.interface.write_char(self.d1 as char, delay);
            let _ = self.interface.write_char(b' ' as char, delay);

            let dig10 = (t.hour / 10) | b'0';
            let dig1 = (t.hour % 10) | b'0';
            self.h10 = dig10;
            self.h1 = dig1;
            let _ = self.interface.write_char(self.h10 as char, delay);
            let _ = self.interface.write_char(self.h1 as char, delay);
            let _ = self.interface.write_char(b':' as char, delay);

            let dig10 = (t.minute / 10) | b'0';
            let dig1 = (t.minute % 10) | b'0';
            self.mi10 = dig10;
            self.mi1 = dig1;
            let _ = self.interface.write_char(self.mi10 as char, delay);
            let _ = self.interface.write_char(self.mi1 as char, delay);
            let _ = self.interface.write_char(b':' as char, delay);

            let dig10 = (t.second / 10) | b'0';
            let dig1 = (t.second % 10) | b'0';
            self.s10 = dig10;
            self.s1 = dig1;
            let _ = self.interface.write_char(self.s10 as char, delay);
            let _ = self.interface.write_char(self.s1 as char, delay);
        }

        if self.set_date_time_up_down {
            self.set_date_time_up_down = false;
            let _ = self
                .interface
                .set_cursor_pos(DDRAM_ADDRESS_THIRD + 2, delay);
            let _ = self.interface.write_char(self.y10 as char, delay);
            let _ = self.interface.write_char(self.y1 as char, delay);
            let _ = self.interface.write_char(b'/' as char, delay);
            let _ = self.interface.write_char(self.mo10 as char, delay);
            let _ = self.interface.write_char(self.mo1 as char, delay);
            let _ = self.interface.write_char(b'/' as char, delay);
            let _ = self.interface.write_char(self.d10 as char, delay);
            let _ = self.interface.write_char(self.d1 as char, delay);
            let _ = self.interface.write_char(b' ' as char, delay);
            let _ = self.interface.write_char(self.h10 as char, delay);
            let _ = self.interface.write_char(self.h1 as char, delay);
            let _ = self.interface.write_char(b':' as char, delay);
            let _ = self.interface.write_char(self.mi10 as char, delay);
            let _ = self.interface.write_char(self.mi1 as char, delay);
            let _ = self.interface.write_char(b':' as char, delay);
            let _ = self.interface.write_char(self.s10 as char, delay);
            let _ = self.interface.write_char(self.s1 as char, delay);
        }

        let move_cursor_pos = [0, 1, 3, 4, 6, 7, 9, 10, 12, 13, 15, 16];

        let _ = self.interface.set_cursor_pos(
            DDRAM_ADDRESS_SET_DATE_TIME_TOP + move_cursor_pos[self.set_position as usize],
            delay,
        );

        unsafe {
            match SWITCH {
                SW::None => (),
                SW::Left => {
                    if self.set_position == 0 {
                        // 最初に Left 押下でキャンセルしてTopに戻る
                        *state = ScreenState::Top;
                        self.set_date_time_initialized = false;
                    } else {
                        self.set_position -= 1;
                        if self.set_position < 0 {
                            self.set_position = 11;
                        }
                    }
                    SWITCH = SW::None;
                }
                SW::Center => {
                    let mut temp: u8 = 0;
                    self.set_date_time_initialized = false;
                    SWITCH = SW::None;
                    rtc.write_register(CONTROL1_REG, STOP_THE_CLOCK); // 計時を停止する

                    temp = (self.s10 & 0xf) << 4 | (self.s1 & 0xf);
                    rtc.write_register(SECONDS_REG, temp);
                    temp = (self.mi10 & 0xf) << 4 | (self.mi1 & 0xf);
                    rtc.write_register(MINUTES_REG, temp);
                    temp = (self.h10 & 0xf) << 4 | (self.h1 & 0xf);
                    rtc.write_register(HOURS_REG, temp);
                    temp = (self.d10 & 0xf) << 4 | (self.d1 & 0xf);
                    rtc.write_register(DAYS_REG, temp);
                    rtc.write_register(WEEKDAYS_REG, 0);
                    temp = (self.mo10 & 0xf) << 4 | (self.mo1 & 0xf);
                    rtc.write_register(MONTHS_CENTURY_REG, temp);
                    temp = (self.y10 & 0xf) << 4 | (self.y1 & 0xf);
                    rtc.write_register(YEARS_REG, temp);

                    rtc.write_register(CONTROL1_REG, START_THE_CLOCK); // 計時を始める
                    *state = ScreenState::Top;
                    SWITCH = SW::None;
                    self.set_date_time_initialized = false;
                }
                SW::Right => {
                    self.set_position += 1;
                    if self.set_position > 11 {
                        self.set_position = 0;
                    }
                    SWITCH = SW::None;
                }
                SW::Up => {
                    self.set_date_time_up_down = true;
                    match self.set_position {
                        0 => {
                            self.y10 &= 0xf;
                            if self.y10 == 9 {
                                self.y10 = 0;
                            } else {
                                self.y10 += 1;
                            }
                            self.y10 |= b'0';
                        }
                        1 => {
                            self.y1 &= 0xf;
                            if self.y1 == 9 {
                                self.y1 = 0;
                            } else {
                                self.y1 += 1;
                            }
                            self.y1 |= b'0';
                        }
                        2 => {
                            self.mo10 &= 0xf;
                            if self.mo10 == 9 {
                                self.mo10 = 0;
                            } else {
                                self.mo10 += 1;
                            }
                            self.mo10 |= b'0';
                        }
                        3 => {
                            self.mo1 &= 0xf;
                            if self.mo1 == 9 {
                                self.mo1 = 0;
                            } else {
                                self.mo1 += 1;
                            }
                            self.mo1 |= b'0';
                        }
                        4 => {
                            self.d10 &= 0xf;
                            if self.d10 == 9 {
                                self.d10 = 0;
                            } else {
                                self.d10 += 1;
                            }
                            self.d10 |= b'0';
                        }
                        5 => {
                            self.d1 &= 0xf;
                            if self.d1 == 9 {
                                self.d1 = 0;
                            } else {
                                self.d1 += 1;
                            }
                            self.d1 |= b'0';
                        }
                        6 => {
                            self.h10 &= 0xf;
                            if self.h10 == 9 {
                                self.h10 = 0;
                            } else {
                                self.h10 += 1;
                            }
                            self.h10 |= b'0';
                        }
                        7 => {
                            self.h1 &= 0xf;
                            if self.h1 == 9 {
                                self.h1 = 0;
                            } else {
                                self.h1 += 1;
                            }
                            self.h1 |= b'0';
                        }
                        8 => {
                            self.mi10 &= 0xf;
                            if self.mi10 == 9 {
                                self.mi10 = 0;
                            } else {
                                self.mi10 += 1;
                            }
                            self.mi10 |= b'0';
                        }
                        9 => {
                            self.mi1 &= 0xf;
                            if self.mi1 == 9 {
                                self.mi1 = 0;
                            } else {
                                self.mi1 += 1;
                            }
                            self.mi1 |= b'0';
                        }
                        10 => {
                            self.s10 &= 0xf;
                            if self.s10 == 9 {
                                self.s10 = 0;
                            } else {
                                self.s10 += 1;
                            }
                            self.s10 |= b'0';
                        }
                        11 => {
                            self.s1 &= 0xf;
                            if self.s1 == 9 {
                                self.s1 = 0;
                            } else {
                                self.s1 += 1;
                            }
                            self.s1 |= b'0';
                        }
                        i32::MIN..=-1_i32 | 12_i32..=i32::MAX => (),
                    }
                    SWITCH = SW::None;
                }
                SW::Down => {
                    self.set_date_time_up_down = true;
                    match self.set_position {
                        0 => {
                            self.y10 &= 0xf;
                            if self.y10 == 0 {
                                self.y10 = 9;
                            } else {
                                self.y10 = (self.y10 as i8 - 1) as u8;
                            }
                            self.y10 |= b'0';
                        }
                        1 => {
                            self.y1 &= 0xf;
                            if self.y1 == 0 {
                                self.y1 = 9;
                            } else {
                                self.y1 = (self.y1 as i8 - 1) as u8;
                            }
                            self.y1 |= b'0';
                        }
                        2 => {
                            self.mo10 &= 0xf;
                            if self.mo10 == 0 {
                                self.mo10 = 9;
                            } else {
                                self.mo10 = (self.mo10 as i8 - 1) as u8;
                            }
                            self.mo10 |= b'0';
                        }
                        3 => {
                            self.mo1 &= 0xf;
                            if self.mo1 == 0 {
                                self.mo1 = 9;
                            } else {
                                self.mo1 = (self.mo1 as i8 - 1) as u8;
                            }
                            self.mo1 |= b'0';
                        }
                        4 => {
                            self.d10 &= 0xf;
                            if self.d10 == 0 {
                                self.d10 = 9;
                            } else {
                                self.d10 = (self.d10 as i8 - 1) as u8;
                            }
                            self.d10 |= b'0';
                        }
                        5 => {
                            self.d1 &= 0xf;
                            if self.d1 == 0 {
                                self.d1 = 9;
                            } else {
                                self.d1 = (self.d1 as i8 - 1) as u8;
                            }
                            self.d1 |= b'0';
                        }
                        6 => {
                            self.h10 &= 0xf;
                            if self.h10 == 0 {
                                self.h10 = 9;
                            } else {
                                self.h10 = (self.h10 as i8 - 1) as u8;
                            }
                            self.h10 |= b'0';
                        }
                        7 => {
                            self.h1 &= 0xf;
                            if self.h1 == 0 {
                                self.h1 = 9;
                            } else {
                                self.h1 = (self.h1 as i8 - 1) as u8;
                            }
                            self.h1 |= b'0';
                        }
                        8 => {
                            self.mi10 &= 0xf;
                            if self.mi10 == 0 {
                                self.mi10 = 9;
                            } else {
                                self.mi10 = (self.mi10 as i8 - 1) as u8;
                            }
                            self.mi10 |= b'0';
                        }
                        9 => {
                            self.mi1 &= 0xf;
                            if self.mi1 == 0 {
                                self.mi1 = 9;
                            } else {
                                self.mi1 = (self.mi1 as i8 - 1) as u8;
                            }
                            self.mi1 |= b'0';
                        }
                        10 => {
                            self.s10 &= 0xf;
                            if self.s10 == 0 {
                                self.s10 = 9;
                            } else {
                                self.s10 = (self.s10 as i8 - 1) as u8;
                            }
                            self.s10 |= b'0';
                        }
                        11 => {
                            self.s1 &= 0xf;
                            if self.s1 == 0 {
                                self.s1 = 9;
                            } else {
                                self.s1 = (self.s1 as i8 - 1) as u8;
                            }
                            self.s1 |= b'0';
                        }
                        i32::MIN..=-1_i32 | 12_i32..=i32::MAX => (),
                    }
                    SWITCH = SW::None;
                }
            }
        }
    }

    pub fn set_top<D: DelayUs<u16> + DelayMs<u8>>(
        &mut self,
        delay: &mut D,
        state: &mut ScreenState,
    ) {
        if !self.top_initialized {
            self.top_initialized = true;
            self.address = DDRAM_ADDRESS_SECOND;
            let _ = self.interface.set_cursor_pos(DDRAM_ADDRESS_FIRST, delay);
            let _ = self.interface.write_str(" Select Item        ", delay);
            let _ = self.interface.set_cursor_pos(DDRAM_ADDRESS_SECOND, delay);
            let _ = self.interface.write_str("1.Display elements  ", delay);
            let _ = self.interface.set_cursor_pos(DDRAM_ADDRESS_THIRD, delay);
            let _ = self.interface.write_str("2.Set Date Time     ", delay);
            let _ = self.interface.set_cursor_pos(DDRAM_ADDRESS_FOURTH, delay);
            let _ = self.interface.write_str("                    ", delay);
            let _ = self.interface.set_cursor_pos(self.address, delay);

            let _ = self.interface.set_cursor_visibility(Cursor::Visible, delay);
        }
        unsafe {
            match SWITCH {
                SW::None => {
                    if self.position == 0 {
                        self.address = DDRAM_ADDRESS_SECOND;
                    } else if self.position == 1 {
                        self.address = DDRAM_ADDRESS_THIRD;
                    }
                    let _ = self.interface.set_cursor_pos(self.address, delay);
                }
                SW::Center => {
                    if self.position == 0 {
                        *state = ScreenState::Elements;
                    } else {
                        *state = ScreenState::SetDateTime;
                    }
                    SWITCH = SW::None;
                    self.top_initialized = false;
                }
                SW::Left => {
                    self.set_position -= 1;
                    if self.set_position < 0 {
                        self.set_position = 11;
                    }
                }
                SW::Right => {
                    self.set_position += 1;
                    if self.set_position > 11 {
                        self.set_position = 0;
                    }
                }
                SW::Down => {
                    self.position ^= 1;
                    if self.position == 0 {
                        self.address = DDRAM_ADDRESS_SECOND;
                    } else {
                        self.address = DDRAM_ADDRESS_THIRD;
                    }
                    let _ = self.interface.set_cursor_pos(self.address, delay);
                    SWITCH = SW::None;
                }
                SW::Up => {
                    self.position ^= 1;
                    if self.position == 0 {
                        self.address = DDRAM_ADDRESS_SECOND;
                    } else {
                        self.address = DDRAM_ADDRESS_THIRD;
                    }
                    let _ = self.interface.set_cursor_pos(self.address, delay);
                    SWITCH = SW::None;
                }
            }
        }
    }

    pub fn set_display<D: DelayUs<u16> + DelayMs<u8>>(&mut self, delay: &mut D, display: Display) {
        let _ = self.interface.set_display(display, delay);
    }
}
