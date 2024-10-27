use embedded_hal::i2c::I2c;

pub const RTC_DEVICE_ADDRESS: u8 = 0x51;

pub const CONTROL1_REG: u8 = 0x00;
pub const CONTROL2_REG: u8 = 0x01;
pub const SECONDS_REG: u8 = 0x02;
pub const MINUTES_REG: u8 = 0x03;
pub const HOURS_REG: u8 = 0x04;
pub const DAYS_REG: u8 = 0x05;
pub const WEEKDAYS_REG: u8 = 0x06;
pub const MONTHS_CENTURY_REG: u8 = 0x07;
pub const YEARS_REG: u8 = 0x08;
const MINUTE_ALARM_REG: u8 = 0x09;
const HOUR_ALARM_REG: u8 = 0x0a;
const DAY_ALARM_REG: u8 = 0x0b;
const WEEKDAY_ALARM_REG: u8 = 0x0c;
const CLKOUT_FREQUENCY_REG: u8 = 0x0d;
const TIMER_CONTROL_REG: u8 = 0x0e;
const TIMER_DOWN_COUNTER_REG: u8 = 0x0f;

const VLOW_STATUS: u8 = 0x80; // 1 で電圧低下あり
const VLOW_DETECTED: u8 = VLOW_STATUS;

const RTC_STOP: u8 = 0x1;
const RTC_RUN: u8 = 0x0;
const CONTROL1_WRITE_DATA_RTC_STOP: u8 = RTC_STOP << 5;
const CONTROL1_WRITE_DATA_RTC_RUN: u8 = RTC_RUN << 5;

const TI_TP: u8 = 0x1; // 定周期割り込みを繰り返し発生させる
const TIE: u8 = 0x1; // 定周期割り込み発生時に INT=L にする
const CONTROL2_WRITE_DATA: u8 = TI_TP << 4 | TIE;

const SECONDS_WRITE_DATA: u8 = 0;
const MINUTES_WRITE_DATA: u8 = 0x42;
const HOURS_WRITE_DATA: u8 = 0x18;
const DAYS_WRITE_DATA: u8 = 0x25; // 25日
const WEEKDAYS_WRITE_DATA: u8 = 0;
const MONTHS_CENTURY_WRITE_DATA: u8 = 0x06; // 6月
const YEARS_WRITE_DATA: u8 = 0x24; // 2024年

const MINUTE_ALARM_WRITE_DATA: u8 = 0;
const HOUR_ALARM_WRITE_DATA: u8 = 0;
const DAY_ALARM_WRITE_DATA: u8 = 0;
const WEEKDAY_ALARM_WRITE_DATA: u8 = 0;

const FE: u8 = 0x0;
const FD1_FD0: u8 = 0x0;
const CLKOUT_FREQUENCY_WRITE_DATA: u8 = FE << 7 | FD1_FD0;

const TE_ENABLED: u8 = 1;
const TE_DISABLED: u8 = 0;
const TD1_TD0: u8 = 0x2;
const TIMER_CONTROL_WRITE_DATA_TE_ENABLED: u8 = TE_ENABLED << 7 | TD1_TD0;
const TIMER_CONTROL_WRITE_DATA_TE_DISABLED: u8 = TE_DISABLED << 7 | TD1_TD0;

const TIMER_DOWN_COUNTER_WRITE_DATA: u8 = 1; // 1sec周期の割り込み用

pub struct Time {
    pub year: u8,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

pub struct RTC8564<IF: I2c> {
    interface: IF,
    address: u8,
    updated: bool,
    minutes: u8,
}

impl<IF: I2c> RTC8564<IF> {
    pub fn new(interface: IF, address: u8) -> Self {
        Self {
            interface,
            address,
            updated: false,
            minutes: 0,
        }
    }

    pub fn init(&mut self) -> bool {
        let time: Time;
        let reg2 = self.read_register(SECONDS_REG);
        if (reg2 & VLOW_STATUS) == VLOW_DETECTED {
            self.write_register(CONTROL1_REG, CONTROL1_WRITE_DATA_RTC_STOP);
            self.write_register(CONTROL2_REG, CONTROL2_WRITE_DATA);
            self.write_register(SECONDS_REG, SECONDS_WRITE_DATA);
            self.write_register(MINUTES_REG, MINUTES_WRITE_DATA);
            self.write_register(HOURS_REG, HOURS_WRITE_DATA);
            self.write_register(DAYS_REG, DAYS_WRITE_DATA);
            self.write_register(WEEKDAYS_REG, WEEKDAYS_WRITE_DATA);
            self.write_register(MONTHS_CENTURY_REG, MONTHS_CENTURY_WRITE_DATA);
            self.write_register(YEARS_REG, YEARS_WRITE_DATA);
            self.write_register(MINUTE_ALARM_REG, MINUTE_ALARM_WRITE_DATA);
            self.write_register(HOUR_ALARM_REG, HOUR_ALARM_WRITE_DATA);
            self.write_register(DAY_ALARM_REG, DAY_ALARM_WRITE_DATA);
            self.write_register(WEEKDAY_ALARM_REG, WEEKDAY_ALARM_WRITE_DATA);
            self.write_register(CLKOUT_FREQUENCY_REG, CLKOUT_FREQUENCY_WRITE_DATA);
            self.write_register(TIMER_CONTROL_REG, TIMER_CONTROL_WRITE_DATA_TE_DISABLED);
            self.write_register(TIMER_DOWN_COUNTER_REG, TIMER_DOWN_COUNTER_WRITE_DATA);

            self.write_register(CONTROL1_REG, CONTROL1_WRITE_DATA_RTC_RUN);
            self.write_register(TIMER_CONTROL_REG, TIMER_CONTROL_WRITE_DATA_TE_ENABLED);
            time = self.read_date_time();
            self.minutes = time.minute;
            return false;
        }
        time = self.read_date_time();
        self.minutes = time.minute;
        true
    }

    fn read_register(&mut self, register: u8) -> u8 {
        let mut buffer: [u8; 1] = [0; 1];
        let _ = self
            .interface
            .write_read(self.address, &[register], &mut buffer);
        buffer[0]
    }

    pub fn write_register(&mut self, register: u8, value: u8) {
        let _ = self.interface.write(self.address, &[register, value]);
    }

    pub fn read_date_time(&mut self) -> Time {
        let mut seconds: u8;
        let mut seconds2: u8;
        let mut minutes: u8;
        let mut minutes2: u8;
        let mut hours: u8;
        let mut hours2: u8;
        let mut months: u8;
        let mut months2: u8;
        let mut days: u8;
        let mut days2: u8;
        let mut years: u8;
        let mut years2: u8;
        loop {
            seconds = self.read_register(SECONDS_REG);
            minutes = self.read_register(MINUTES_REG);
            hours = self.read_register(HOURS_REG);
            months = self.read_register(MONTHS_CENTURY_REG);
            days = self.read_register(DAYS_REG);
            years = self.read_register(YEARS_REG);

            seconds2 = self.read_register(SECONDS_REG);
            minutes2 = self.read_register(MINUTES_REG);
            hours2 = self.read_register(HOURS_REG);
            months2 = self.read_register(MONTHS_CENTURY_REG);
            days2 = self.read_register(DAYS_REG);
            years2 = self.read_register(YEARS_REG);

            if seconds == seconds2
                && minutes == minutes2
                && hours == hours2
                && months == months2
                && days == days2
                && years == years2
            {
                break;
            }
        }
        let s = ((seconds & 0x7f) >> 4) * 10 + (seconds & 0xf);
        let mi = ((minutes & 0x7f) >> 4) * 10 + (minutes & 0xf);
        let h = ((hours & 0x3f) >> 4) * 10 + (hours & 0xf);
        let mo = ((months & 0x1f) >> 4) * 10 + (months & 0xf);
        let d = ((days & 0x3f) >> 4) * 10 + (days & 0xf);
        let y = ((years & 0xff) >> 4) * 10 + (years & 0xf);
        let mut time = Time {
            second: s,
            minute: mi,
            hour: h,
            month: mo,
            day: d,
            year: y,
        };
        self.minutes = time.minute;
        time
    }

    pub fn get_minutes(&self) -> u8 {
        self.minutes
    }
}
