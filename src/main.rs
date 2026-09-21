#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::array::from_ref;

use chrono::{NaiveDate, NaiveTime};
use defmt::Format;
use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::I2c;
use esp_hal::spi::Mode;
use esp_hal::spi::master::Spi;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart::{AtCmdConfig, RxConfig, Uart};
use esp_println as _;
use packet::Packet;
use packet::msg_type;
mod adc;
mod gpio;
mod lora_receive;
mod lora_send;
mod low_prio;
mod oled;
mod packet;
mod uart;

#[derive(Clone, Copy, Format, Default)]
pub struct NmeaPosition {
    date: NaiveDate,
    time: NaiveTime,
    latitude: f64,
    longitude: f64,
    altitude: f32,
    speed_over_ground: f32,
    fix_satellites: u32,
}
impl NmeaPosition {
    fn new() -> Self {
        Default::default()
    }
}

#[derive(Clone, Copy, Format, Default)]
pub struct AdcValue {
    voltage: u16,
    temp: u16,
}
impl AdcValue {
    fn new() -> Self {
        Default::default()
    }
}

esp_bootloader_esp_idf::esp_app_desc!();
pub static POSITION_CHANNEL: Channel<CriticalSectionRawMutex, NmeaPosition, 4> = Channel::new();
pub static ADC_CHANNEL: Channel<CriticalSectionRawMutex, AdcValue, 4> = Channel::new();
pub static LORA_FREQUENCY_IN_HZ: u32 = 870_000_000;

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let _boot = peripherals.GPIO0;
    let lora_dio1 = peripherals.GPIO1;
    let _exp_p5_15 = peripherals.GPIO2;
    let nreset = peripherals.GPIO3;
    let battery_voltage = peripherals.GPIO4;
    let gnss_rxd = peripherals.GPIO5;
    let gnss_txd = peripherals.GPIO6;
    let gnss_1pps = peripherals.GPIO7;
    let i2c_sda = peripherals.GPIO8;
    let i2c_sdl = peripherals.GPIO9;
    let _sd_cs = peripherals.GPIO10;
    let spi_mosi = peripherals.GPIO11;
    let spi_miso = peripherals.GPIO12;
    let spi_sck = peripherals.GPIO13;
    let temp_samp = peripherals.GPIO14;
    let lora_cs = peripherals.GPIO15;
    let _gnss_wake_up = peripherals.GPIO16;
    let user_button = peripherals.GPIO17;
    let user_led = peripherals.GPIO18;
    let _esp_usb_n = peripherals.GPIO19;
    let _esp_usb_p = peripherals.GPIO20;
    let lora_lna_ctl = peripherals.GPIO21;
    let _ = peripherals.GPIO26;
    let _ = peripherals.GPIO27;
    let _ = peripherals.GPIO28;
    let _ = peripherals.GPIO29;
    let _ = peripherals.GPIO30;
    let _ = peripherals.GPIO31;
    let _ = peripherals.GPIO32;
    let _ = peripherals.GPIO33;
    let _ = peripherals.GPIO34;
    let _ = peripherals.GPIO35;
    let _ = peripherals.GPIO36;
    let _ = peripherals.GPIO37;
    let lora_busy = peripherals.GPIO38;
    let _ext_p5_9 = peripherals.GPIO39;
    let lora_ldo_en = peripherals.GPIO40;
    let fan_ctrl = peripherals.GPIO41;
    let _ext_p5_10 = peripherals.GPIO42;
    let _ext_txd = peripherals.GPIO43;
    let _ext_rxd = peripherals.GPIO44;
    let _ext_p5_8 = peripherals.GPIO45;
    let _ext_p6_4 = peripherals.GPIO46;
    let _ext_p5_6 = peripherals.GPIO47;
    let _ext_p5_7 = peripherals.GPIO48;

    let reset = Output::new(nreset, Level::Low, OutputConfig::default());
    let cs = Output::new(lora_cs, Level::Low, OutputConfig::default());
    let busy = Input::new(lora_busy, InputConfig::default());
    let dio1 = Input::new(lora_dio1, InputConfig::default());
    let _ldo = Output::new(lora_ldo_en, Level::High, OutputConfig::default()); // Enable LDO Permanent
    let _lna = Output::new(lora_lna_ctl, Level::High, OutputConfig::default()); // Enable Rx LNA
    let spi = Spi::new(
        peripherals.SPI2,
        esp_hal::spi::master::Config::default()
            .with_frequency(Rate::from_khz(100))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(spi_sck)
    .with_mosi(spi_mosi)
    .with_miso(spi_miso)
    .into_async();

    let config = esp_hal::uart::Config::default()
        .with_baudrate(9600)
        .with_rx(RxConfig::default().with_fifo_full_threshold(64));
    let mut uart0 = Uart::new(peripherals.UART0, config)
        .unwrap()
        .with_tx(gnss_txd)
        .with_rx(gnss_rxd)
        .into_async();
    uart0.set_at_cmd(AtCmdConfig::default().with_cmd_char(0x0A));

    let i2c0 = I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(i2c_sda)
    .with_scl(i2c_sdl)
    .into_async();

    let led = Output::new(user_led, Level::High, OutputConfig::default());
    let _ = Output::new(fan_ctrl, Level::Low, OutputConfig::default()); //fan control
    let button = Input::new(user_button, InputConfig::default().with_pull(Pull::Up));
    let pps_1 = Input::new(gnss_1pps, InputConfig::default());

    spawner.spawn(uart::uart_reader(uart0, POSITION_CHANNEL.sender()).unwrap());
    spawner.spawn(oled::viewer(i2c0, POSITION_CHANNEL.receiver(), ADC_CHANNEL.receiver()).unwrap());
    //spawner.spawn(lora_send::send_packet(spi, reset, busy, dio1, cs).unwrap());
    spawner.spawn(lora_receive::receive_packet(spi, reset, busy, dio1, cs).unwrap());
    spawner.spawn(gpio::blink_led(led).unwrap());
    spawner.spawn(gpio::press_button(button).unwrap());
    spawner.spawn(gpio::pps_flash(pps_1).unwrap());
    spawner.spawn(
        adc::get_adc(
            peripherals.ADC1,
            peripherals.ADC2,
            battery_voltage,
            temp_samp,
            ADC_CHANNEL.sender(),
        )
        .unwrap(),
    );
    spawner.spawn(low_prio::low_prio_async().unwrap());
    //let mac = esp_hal::efuse::base_mac_address().as_bytes();
    loop {
        // Отправка

        // info!("MAC: {}", mac);
        // let mac16: [u8; 2] = mac[..2];
        // let pkt = Packet::new(mac16, msg_type::MESSAGE)
        //     .with_payload("Привет участникам соревнований!".as_bytes())
        //     .unwrap();
        // let (buf, len) = pkt.encode_to_array().unwrap();
        // //radio.send(&buf[..len]).await;

        //info!("buf{}: {}", len, buf);
        Timer::after(Duration::from_millis(10_000)).await;
    }
}
