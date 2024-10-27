#![no_std]
#![no_main]

use embedded_hal_0_2::blocking::delay::{DelayMs, DelayUs}; // embedded-hal ver0.2.x

//use fugit::MicrosDurationU32;
use fugit::RateExtU32;
use rp2040_hal::fugit::MicrosDurationU32;

use hal::pac;
use hal::uart::{DataBits, StopBits, UartConfig};
use rp2040_hal::spi::Enabled;
use rp2040_hal::Clock;
use rp2040_lib::sc2004::SC2004;
use rp_pico::entry;

use embedded_hal::digital::InputPin;
use embedded_hal::digital::StatefulOutputPin;

//use panic_halt as _;
use rp2040_hal as hal;

use rp2040_lib::my_macro::UART_TRANSMITTER;
use rp2040_lib::print;
use rp2040_lib::println;

use rp2040_lib::bme280::spi::BME280;
use rp2040_lib::rtc8564::RTC8564;
use rp2040_lib::rtc8564::RTC_DEVICE_ADDRESS;

use rp2040_hal::gpio::bank0::Gpio15;
use rp2040_hal::gpio::bank0::Gpio19;
use rp2040_hal::gpio::bank0::Gpio2;
use rp2040_hal::gpio::bank0::Gpio3;
use rp2040_hal::gpio::bank0::Gpio9;

use rp2040_hal::gpio::bank0::Gpio4;
use rp2040_hal::gpio::bank0::Gpio5;
use rp2040_hal::gpio::bank0::Gpio6;
use rp2040_hal::gpio::bank0::Gpio7;
use rp2040_hal::gpio::FunctionSio;
use rp2040_hal::gpio::Pin;
use rp2040_hal::gpio::PullDown;
use rp2040_hal::gpio::PullUp;
use rp2040_hal::gpio::{FunctionSpi, SioInput, SioOutput};

use rp2040_hal::gpio::bank0::Gpio10;
use rp2040_hal::gpio::bank0::Gpio11;
use rp2040_hal::gpio::bank0::Gpio12;
use rp2040_hal::gpio::bank0::Gpio13;

use crate::pac::SPI0;
use crate::pac::SPI1;
use rp2040_hal::Spi;

use hd44780_driver::HD44780;

use rp2040_hal::gpio::bank0::Gpio16;
use rp2040_hal::gpio::bank0::Gpio17;
use rp2040_hal::gpio::FunctionI2c;

use crate::pac::I2C0;
use rp2040_hal::I2C;

use core::cell::RefCell;
use critical_section::Mutex;

use rp2040_hal::timer::Alarm;

// use core::fmt::Display;

use crate::pac::interrupt;

type LedAndAlarm = (
    hal::gpio::Pin<hal::gpio::bank0::Gpio25, hal::gpio::FunctionSioOutput, hal::gpio::PullDown>,
    hal::timer::Alarm0,
);

static mut LED_AND_ALARM: Mutex<RefCell<Option<LedAndAlarm>>> = Mutex::new(RefCell::new(None));

const FAST_BLINK_INTERVAL_US: MicrosDurationU32 = MicrosDurationU32::millis(20);

type Rtc = RTC8564<
    I2C<
        I2C0,
        (
            Pin<Gpio16, FunctionI2c, PullUp>,
            Pin<Gpio17, FunctionI2c, PullUp>,
        ),
    >,
>;

type Volume_Manager = VolumeManager<
    SdCard<
        Spi<
            Enabled,
            SPI1,
            (
                Pin<Gpio11, FunctionSpi, PullDown>,
                Pin<Gpio12, FunctionSpi, PullDown>,
                Pin<Gpio10, FunctionSpi, PullDown>,
            ),
        >,
        Pin<Gpio13, FunctionSio<SioOutput>, PullDown>,
        rp2040_hal::timer::Timer,
    >,
    DummyTimesource,
>;

static mut SS_NOW: u8 = 0; // 現在の状態
static mut SS_ONE_BEFORE: u8 = 0; // ひとつ前の状態
static mut SS_TWO_BEFORE: u8 = 0; // ふたつ前の状態

type CenterSw = Pin<Gpio2, FunctionSio<SioInput>, PullDown>;
type DownSw = Pin<Gpio3, FunctionSio<SioInput>, PullDown>;
type LeftSw = Pin<Gpio19, FunctionSio<SioInput>, PullDown>;
type RightSw = Pin<Gpio9, FunctionSio<SioInput>, PullDown>;
type UpSw = Pin<Gpio15, FunctionSio<SioInput>, PullDown>;

static mut CENTER_SW: Option<CenterSw> = None;
static mut DOWN_SW: Option<DownSw> = None;
static mut LEFT_SW: Option<LeftSw> = None;
static mut RIGHT_SW: Option<RightSw> = None;
static mut UP_SW: Option<UpSw> = None;

use rp2040_lib::ScreenState;
use rp2040_lib::SW;
use rp2040_lib::SWITCH;

use embedded_sdmmc::{Directory, SdCard, TimeSource, Timestamp, Volume, VolumeIdx, VolumeManager};

use embedded_sdmmc::filesystem::Mode;

use embedded_hal::delay::DelayNs;

// use defmt::*;
// use defmt::Debug2Format;
// use defmt_rtt as _;

pub struct Vol_items {
    vol_man: Volume_Manager,
    vol: Volume,
    dir: Directory,
}

#[derive(Default)]
pub struct DummyTimesource();

impl TimeSource for DummyTimesource {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let sio = hal::Sio::new(pac.SIO);

    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let center = pins.gpio2.into_pull_down_input();
    let down = pins.gpio3.into_pull_down_input();
    let left = pins.gpio19.into_pull_down_input();
    let right = pins.gpio9.into_pull_down_input();
    let up = pins.gpio15.into_pull_down_input();

    let uart_pins = (pins.gpio0.reconfigure(), pins.gpio1.reconfigure());
    let uart = hal::uart::UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(9600.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    let (_, uart_tx) = uart.split();

    critical_section::with(|_| unsafe {
        UART_TRANSMITTER = Some(uart_tx);
        CENTER_SW = Some(center);
        DOWN_SW = Some(down);
        LEFT_SW = Some(left);
        RIGHT_SW = Some(right);
        UP_SW = Some(up);
    });

    let led_pin = pins.led.into_push_pull_output();

    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    critical_section::with(|cs| {
        let mut alarm = timer.alarm_0().unwrap();
        // Schedule an alarm in 1 second
        let _ = alarm.schedule(FAST_BLINK_INTERVAL_US);
        // Enable generating an interrupt on alarm
        alarm.enable_interrupt();
        // Move alarm into ALARM, so that it can be accessed from interrupts
        unsafe {
            LED_AND_ALARM.borrow(cs).replace(Some((led_pin, alarm)));
        }
    });
    // Unmask the timer0 IRQ so that it will generate an interrupt
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
    }

    let cs = pins.gpio5.into_push_pull_output();

    // LCD Display

    let rs = pins.gpio28.reconfigure();
    let en = pins.gpio27.reconfigure();
    let d4 = pins.gpio26.reconfigure();
    let d5 = pins.gpio22.reconfigure();
    let d6 = pins.gpio21.reconfigure();
    let d7 = pins.gpio20.reconfigure();

    let hd44780 = HD44780::new_4bit(rs, en, d4, d5, d6, d7, &mut delay).unwrap();
    let mut lcd = SC2004::new(hd44780);

    let sda_pin = pins.gpio16.reconfigure();
    let scl_pin = pins.gpio17.reconfigure();

    let i2c = hal::I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.peripheral_clock,
    );

    let mut rtc8564 = RTC8564::<
        I2C<
            I2C0,
            (
                Pin<Gpio16, FunctionI2c, PullUp>,
                Pin<Gpio17, FunctionI2c, PullUp>,
            ),
        >,
    >::new(i2c, RTC_DEVICE_ADDRESS);

    let _ = rtc8564.init();

    let spi0_mosi = pins.gpio7.reconfigure();
    let spi0_miso = pins.gpio4.reconfigure();
    let spi0_sclk = pins.gpio6.reconfigure();

    let spi0 = hal::spi::Spi::<_, _, _, 8>::new(pac.SPI0, (spi0_mosi, spi0_miso, spi0_sclk));

    let spi0 = spi0.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        10.MHz(),
        embedded_hal::spi::MODE_0,
    );

    let mut bme280 = BME280::<
        Spi<
            Enabled,
            SPI0,
            (
                Pin<Gpio7, FunctionSpi, PullDown>,
                Pin<Gpio4, FunctionSpi, PullDown>,
                Pin<Gpio6, FunctionSpi, PullDown>,
            ),
        >,
        Pin<Gpio5, FunctionSio<SioOutput>, PullDown>,
    >::new(spi0, cs);

    let spi1_mosi = pins.gpio11.into_function::<hal::gpio::FunctionSpi>();
    let spi1_miso = pins.gpio12.into_function::<hal::gpio::FunctionSpi>();
    let spi1_sclk = pins.gpio10.into_function::<hal::gpio::FunctionSpi>();

    let spi1 = hal::spi::Spi::<_, _, _, 8>::new(pac.SPI1, (spi1_mosi, spi1_miso, spi1_sclk));

    let spi1 = spi1.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        1.MHz(),
        embedded_hal::spi::MODE_0,
    );

    let cs_sd = pins.gpio13.into_push_pull_output();

    let cd_sd = pins.gpio14.into_pull_down_input(); // Card Detect pin

    // DeviceのIDコード(0x60)を正しく読めれば成功としている
    if bme280.init() {
        println!("BME280 initialization successful.");
        println!("BME280 ID = 0x60.\r\n");
    } else {
        println!("BME280 initialization failed.\r\n");
    }

    delay.delay_ms(2000);

    lcd.init(&mut delay);

    // let a = 3;
    // let b = 5;
    // assert!(a == b);

    let mut screen_state = ScreenState::Top;

    let sdcard = SdCard::new(spi1, cs_sd, timer);

    let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource::default());

    println!("Init SD card controller and retrieve card size...");
    match volume_mgr.device().num_bytes() {
        Ok(size) => println!("card size is {} bytes", size),
        _ => loop {},
    }

    // Now that the card is initialized, clock can go faster
    volume_mgr
        .device()
        .spi(|spi| spi.set_baudrate(clocks.peripheral_clock.freq(), 16.MHz()));

    println!("Getting Volume 0...");
    let mut volume = match volume_mgr.get_volume(VolumeIdx(0)) {
        Ok(v) => v,
        _ => loop {},
    };

    // After we have the volume (partition) of the drive we got to open the
    // root directory:
    let dir = match volume_mgr.open_root_dir(&volume) {
        Ok(dir) => dir,
        _ => loop {},
    };

    println!("Root directory opened!");

    // This shows how to iterate through the directory and how
    // to get the file names (and print them in hope they are UTF-8 compatible):
    volume_mgr
        .iterate_dir(&volume, &dir, |ent| {
            println!(
                "/{}.{}",
                core::str::from_utf8(ent.name.base_name()).unwrap(),
                core::str::from_utf8(ent.name.extension()).unwrap()
            );
        })
        .unwrap();

    //    volume_mgr.free();

    let mut vi = Vol_items {
        vol_man: volume_mgr,
        vol: volume,
        dir: dir,
    };

    loop {
        bme280.read_data();

        let (temp, humi, pres) = bme280.get_elements();

        // println!("T = {:.2} ℃", temp);
        // println!("H = {:.2} %", humi);
        // println!("P = {:.2} hPa\r\n", pres);

        lcd_display(
            &mut delay,
            &mut lcd,
            &mut rtc8564,
            &mut screen_state,
            (temp, humi, pres),
        );
        if rtc_updated(&mut rtc8564) {
            save_elements(&mut vi, &mut rtc8564, (temp, humi, pres));
        }
    }
}

fn rtc_updated(rtc: &mut Rtc) -> bool {
    let mut b = false;
    let minute = rtc.get_minutes();
    let time = rtc.read_date_time();
    if time.minute != minute {
        b = true;
    }
    b
}

fn save_elements(vol_item: &mut Vol_items, rtc: &mut Rtc, tup: (f64, f64, f64)) {
    let mut buf: [u8; 10] = [0; 10];

    let mut y10: u8 = 0;
    let mut y1: u8 = 0;
    let mut mo10: u8 = 0;
    let mut mo1: u8 = 0;
    let mut d10: u8 = 0;
    let mut d1: u8 = 0;
    let mut h10: u8 = 0;
    let mut h1: u8 = 0;
    let mut mi10: u8 = 0;
    let mut mi1: u8 = 0;

    let time = rtc.read_date_time();

    y10 = time.year / 10 | b'0';
    y1 = time.year % 10 | b'0';
    mo10 = time.month / 10 | b'0';
    mo1 = time.month % 10 | b'0';
    d10 = time.day / 10 | b'0';
    d1 = time.day % 10 | b'0';
    h10 = time.hour / 10 | b'0';
    h1 = time.hour % 10 | b'0';
    mi10 = time.minute / 10 | b'0';
    mi1 = time.minute % 10 | b'0';

    // buf format: yymmdd.txt
    buf[0] = y10;
    buf[1] = y1;
    buf[2] = mo10;
    buf[3] = mo1;
    buf[4] = d10;
    buf[5] = d1;
    buf[6] = b'.';
    buf[7] = b't';
    buf[8] = b'x';
    buf[9] = b't';

    let temp_tens_digit: u8 = ((tup.0 as i32 / 10) % 10) as u8 | b'0';
    let temp_ones_digit: u8 = ((tup.0 as i32) % 10) as u8 | b'0';
    let temp_tenths_digit: u8 = (((tup.0 * 10.0) as i32) % 10) as u8 | b'0';

    let humi_tens_digit: u8 = ((tup.1 as i32 / 10) % 10) as u8 | b'0';
    let humi_ones_digit: u8 = ((tup.1 as i32) % 10) as u8 | b'0';
    let humi_tenths_digit: u8 = (((tup.1 * 10.0) as i32) % 10) as u8 | b'0';

    let mut pres_thousands_digit: u8 = ((tup.2 as i32 / 1000) % 10) as u8 | b'0';
    let pres_hundreds_digit: u8 = ((tup.2 as i32 / 100) % 10) as u8 | b'0';
    let pres_tens_digit: u8 = ((tup.2 as i32 / 10) % 10) as u8 | b'0';
    let pres_ones_digit: u8 = ((tup.2 as i32) % 10) as u8 | b'0';
    let pres_tenths_digit: u8 = (((tup.2 * 10.0) as i32) % 10) as u8 | b'0';

    if pres_thousands_digit == b'0' {
        pres_thousands_digit = b' ';
    }

    if let Ok(mut file) = vol_item.vol_man.open_file_in_dir(
        &mut vol_item.vol,
        &vol_item.dir,
        core::str::from_utf8(&buf).unwrap(),
        Mode::ReadWriteCreateOrAppend,
    ) {
        vol_item
            .vol_man
            .write(
                &mut vol_item.vol,
                &mut file,
                &[
                    b'2',
                    b'0',
                    y10,
                    y1,
                    b'/',
                    mo10,
                    mo1,
                    b'/',
                    d10,
                    d1,
                    b' ',
                    h10,
                    h1,
                    b':',
                    mi10,
                    mi1,
                    b' ',
                    b'T',
                    b':',
                    b' ',
                    temp_tens_digit,
                    temp_ones_digit,
                    b'.',
                    temp_tenths_digit,
                    b',',
                    b' ',
                    b'H',
                    b':',
                    b' ',
                    humi_tens_digit,
                    humi_ones_digit,
                    b'.',
                    humi_tenths_digit,
                    b',',
                    b' ',
                    b'P',
                    b':',
                    b' ',
                    pres_thousands_digit,
                    pres_hundreds_digit,
                    pres_tens_digit,
                    pres_ones_digit,
                    b'.',
                    pres_tenths_digit,
                    0x0d,
                    0x0a,
                ],
            )
            .unwrap();
        vol_item.vol_man.close_file(&vol_item.vol, file).unwrap();
    }
}

fn lcd_display<D: DelayUs<u16> + DelayMs<u8>>(
    delay: &mut D,
    lcd: &mut SC2004,
    rtc: &mut Rtc,
    screen_state: &mut ScreenState,
    tup: (f64, f64, f64),
) {
    match screen_state {
        ScreenState::Top => lcd.set_top(delay, screen_state),
        ScreenState::Elements => lcd.set_elements(delay, tup, rtc, screen_state),
        ScreenState::SetDateTime => lcd.set_date_time(delay, rtc, screen_state),
    }
}

#[interrupt]
fn TIMER_IRQ_0() {
    critical_section::with(|cs| {
        // Temporarily take our LED_AND_ALARM
        let ledalarm = unsafe { LED_AND_ALARM.borrow(cs).take() };
        if let Some((mut led, mut alarm)) = ledalarm {
            // Clear the alarm interrupt or this interrupt service routine will keep firing
            alarm.clear_interrupt();
            // Schedule a new alarm after SLOW_BLINK_INTERVAL_US have passed (1 second)
            let _ = alarm.schedule(FAST_BLINK_INTERVAL_US);

            unsafe {
                static mut COUNT: u8 = 0;
                COUNT += 1;
                if 9 < COUNT {
                    COUNT = 0;
                    // Blink the LED so we know we hit this interrupt
                    led.toggle().unwrap();
                }
            }

            // Return LED_AND_ALARM into our static variable
            unsafe {
                LED_AND_ALARM
                    .borrow(cs)
                    .replace_with(|_| Some((led, alarm)));

                SS_TWO_BEFORE = SS_ONE_BEFORE;
                SS_ONE_BEFORE = SS_NOW;

                SS_NOW = 0;
                if let Some(ref mut reader) = CENTER_SW.as_mut() {
                    if reader.is_low().unwrap() {
                        SS_NOW |= 0x10;
                    }
                }
                if let Some(ref mut reader) = DOWN_SW.as_mut() {
                    if reader.is_low().unwrap() {
                        SS_NOW |= 0x08;
                    }
                }
                if let Some(ref mut reader) = LEFT_SW.as_mut() {
                    if reader.is_low().unwrap() {
                        SS_NOW |= 0x04;
                    }
                }
                if let Some(ref mut reader) = RIGHT_SW.as_mut() {
                    if reader.is_low().unwrap() {
                        SS_NOW |= 0x02;
                    }
                }
                if let Some(ref mut reader) = UP_SW.as_mut() {
                    if reader.is_low().unwrap() {
                        SS_NOW |= 0x01;
                    }
                }
                if (SS_TWO_BEFORE & 0x10 == 0x10)
                    && (SS_ONE_BEFORE & 0x10 == 0x10)
                    && (SS_NOW & 0x10 == 0)
                {
                    SWITCH = SW::Center;
                } else if (SS_TWO_BEFORE & 0x08 == 0x08)
                    && (SS_ONE_BEFORE & 0x08 == 0x08)
                    && (SS_NOW & 0x08 == 0)
                {
                    SWITCH = SW::Down;
                } else if (SS_TWO_BEFORE & 0x04 == 0x04)
                    && (SS_ONE_BEFORE & 0x04 == 0x04)
                    && (SS_NOW & 0x04 == 0)
                {
                    SWITCH = SW::Left;
                } else if (SS_TWO_BEFORE & 0x02 == 0x02)
                    && (SS_ONE_BEFORE & 0x02 == 0x02)
                    && (SS_NOW & 0x02 == 0)
                {
                    SWITCH = SW::Right;
                } else if (SS_TWO_BEFORE & 0x01 == 0x01)
                    && (SS_ONE_BEFORE & 0x01 == 0x01)
                    && (SS_NOW & 0x01 == 0)
                {
                    SWITCH = SW::Up;
                }
            }
        }
    });
}
