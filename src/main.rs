#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use chrono::{NaiveDate, NaiveTime};
use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::pubsub::PubSubChannel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::efuse::base_mac_address;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::I2c;
use esp_hal::spi::Mode;
use esp_hal::spi::master::Spi;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart::{RxConfig, Uart};
use esp_println as _;

use crate::MsgType::Ping;
use crate::packet::Packet;
//use packet::Packet;
//use packet::msg_type;
mod adc;
mod gpio;
//mod lora_device;
mod lora_receive;
mod lora_send;
mod low_prio;
mod oled;
mod packet;
mod uart;

#[derive(Clone, Copy)]
pub struct NmeaPosition {
    date: NaiveDate,
    time: NaiveTime,
    latitude: f64,
    longitude: f64,
    altitude: f32,
    speed_over_ground: f32,
    fix_satellites: u32,
}
#[derive(Clone, Copy)]
pub struct AdcValue {
    voltage: u16,
    temp: u16,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum MsgType {
    None = 0x00,
    NavData = 0x01,
    Message = 0x02,
    Ack = 0x03,
    Ping = 0x04,
    Pong = 0x05,
}
// #[derive(Clone, Copy, Debug, PartialEq)]
// enum MsgStatus {
//     Create,
//     Sent,
//     Delivered,
//     Received,
//     Acknowledged,
//}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Message {
    source: [u8; 6],
    destination: [u8; 6],
    msg_type: MsgType,
    message_len: u8,
    message: [u8; MAX_PAYLOAD_LEN as usize],
    time_stamp: NaiveTime,
}

pub static ADC_SIGNAL: Signal<CriticalSectionRawMutex, AdcValue> = Signal::new();
pub static MESSAGE_IN_CHANNEL: Channel<CriticalSectionRawMutex, Message, 4> = Channel::new();
pub static MESSAGE_OUT_CHANNEL: Channel<CriticalSectionRawMutex, Message, 4> = Channel::new();
pub static POSITION_MUTEX: Mutex<CriticalSectionRawMutex, NmeaPosition> =
    Mutex::new(NmeaPosition {
        date: NaiveDate::MIN,
        time: NaiveTime::MIN,
        latitude: 0.0,
        longitude: 0.0,
        altitude: 0.0,
        speed_over_ground: 0.0,
        fix_satellites: 0,
    });
pub static MESSAGE_PBC: PubSubChannel<CriticalSectionRawMutex, Message, 2, 2, 8> =
    PubSubChannel::new();
pub static LORA_FREQUENCY_IN_HZ: u32 = 870_000_000;
pub static MAX_PAYLOAD_LEN: usize = 200;

esp_bootloader_esp_idf::esp_app_desc!();
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let default_message = Message {
        source: [0x00; 6],
        destination: [0x00; 6],
        msg_type: MsgType::None,
        message_len: 0,
        message: [0; MAX_PAYLOAD_LEN as usize],
        time_stamp: NaiveTime::MIN,
    };
    let mut messages = [default_message; 10];

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
    let uart0 = Uart::new(peripherals.UART0, config)
        .unwrap()
        .with_tx(gnss_txd)
        .with_rx(gnss_rxd)
        .into_async();

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

    spawner.spawn(uart::uart_reader(uart0).unwrap());
    spawner.spawn(oled::viewer(i2c0).unwrap());
    spawner.spawn(lora_send::send_packet(spi, reset, busy, dio1, cs).unwrap());
    //spawner.spawn(lora_receive::receive_packet(spi, reset, busy, dio1, cs).unwrap());
    spawner.spawn(gpio::blink_led(led).unwrap());
    spawner.spawn(gpio::press_button(button).unwrap());
    spawner.spawn(gpio::pps_flash(pps_1).unwrap());
    spawner.spawn(
        adc::get_adc(
            peripherals.ADC1,
            peripherals.ADC2,
            battery_voltage,
            temp_samp,
        )
        .unwrap(),
    );
    spawner.spawn(low_prio::low_prio_async().unwrap());
    let mac = base_mac_address();
    info!("Base MAC: {}", mac);
    let pub0 = MESSAGE_PBC.publisher().unwrap();
    loop {
        messages[0] = Message {
            msg_type: Ping,
            message: [255; 200],
            ..default_message
        };
        info!("source: {}", messages[0].source);
        info!("destination: {}", messages[0].destination);
        info!("message_len: {}", messages[0].message_len);
        info!("message: {}", messages[0].message);
        info!("time_stamp: {}", messages[0].time_stamp);
        let pack: Packet = Packet::new(messages[0]);
        //info!("pack: {}", pack);
        let mut send_buf = [0u8; 30];
        let _ = pack.encode(&mut send_buf);
        info!("send_buf: {}", send_buf);
        pub0.publish_immediate(messages[0]);
        messages[1] = packet::decode(&mut send_buf).unwrap();
        info!("source: {}", messages[1].source);
        info!("destination: {}", messages[1].destination);
        info!("message_len: {}", messages[1].message_len);
        info!("message: {}", messages[1].message);
        info!("time_stamp: {}", messages[1].time_stamp);
        assert_eq!(messages[0], messages[1]);

        Timer::after(Duration::from_millis(10_000)).await;
    }
}
