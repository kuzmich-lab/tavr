#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use chrono::{NaiveDate, NaiveTime};
use defmt::Format;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::I2c;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::spi::Mode;
use esp_hal::spi::master::Spi;
use esp_hal::spi::master::SpiDmaBus;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart::{RxConfig, Uart};
use esp_hal::{Async, dma_buffers};
use esp_println as _;
use static_cell::StaticCell;

mod gpio;
mod lora_receive;
mod lora_send;
mod low_prio;
mod uart;
mod viewer;

esp_bootloader_esp_idf::esp_app_desc!();

#[derive(Clone, Copy, Format, Default)]
pub struct NmeaPosition {
    date: NaiveDate,
    time: NaiveTime,
    latitude: f64,
    longitude: f64,
    altitude: f32,
    speed_over_ground: f32,
    num_of_fix_satellites: u32,
}
impl NmeaPosition {
    fn new() -> Self {
        Default::default()
    }
}

pub static POSITION_CHANNEL: Channel<CriticalSectionRawMutex, NmeaPosition, 4> = Channel::new();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // let boot = peripherals.GPIO0;
    // let lora_dio1 = peripherals.GPIO1;
    // let exp_p5_15 = peripherals.GPIO2;
    // let lora_nreset = peripherals.GPIO3;
    // let battery_voltage = peripherals.GPIO4;
    let gnss_rxd = peripherals.GPIO5;
    let gnss_txd = peripherals.GPIO6;
    // let gnss_1pps = peripherals.GPIO7;
    let i2c_sda = peripherals.GPIO8;
    let i2c_sdl = peripherals.GPIO9;
    //let sd_cs = peripherals.GPIO10;
    let spi_mosi = peripherals.GPIO11;
    let spi_miso = peripherals.GPIO12;
    let spi_sck = peripherals.GPIO13;
    // let temp_samp = peripherals.GPIO14;
    let lora_cs = peripherals.GPIO15;
    // let gnss_wake_up = peripherals.GPIO16;
    let user_button = peripherals.GPIO17;
    let user_led = peripherals.GPIO18;
    //let esp_usb_n = peripherals.GPIO19;
    //let esp_usb_p = peripherals.GPIO20;
    // let lora_ctl = peripherals.GPIO21;
    // let lora_busy = peripherals.GPIO38;
    // let ext_p5_9 = peripherals.GPIO39;
    // let lora_ldo_en = peripherals.GPIO40;
    let fan_ctrl = peripherals.GPIO41;
    // let ext_p5_10 = peripherals.GPIO42;
    //let ext_txd = peripherals.GPIO43;
    //let ext_rxd = peripherals.GPIO44;
    // let ext_p5_8 = peripherals.GPIO45;
    // let ext_p6_4 = peripherals.GPIO46;
    // let ext_p5_6 = peripherals.GPIO47;
    // let ext_p5_7 = peripherals.GPIO48;

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

    let dma_channel = peripherals.DMA_CH0;
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(32000);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

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
    .with_cs(lora_cs)
    .with_dma(dma_channel)
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    static SPI_BUS: StaticCell<Mutex<CriticalSectionRawMutex, SpiDmaBus<'static, Async>>> =
        StaticCell::new();

    //let spi_bus = Mutex::new(spi);
    let spi_bus = SPI_BUS.init(spi);

    let spi_device =
        embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(spi_bus, lora_cs);

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    let config = esp_hal::uart::Config::default()
        .with_baudrate(9600)
        .with_rx(RxConfig::default().with_fifo_full_threshold(64));

    let uart0 = Uart::new(peripherals.UART0, config)
        .unwrap()
        .with_tx(gnss_txd)
        .with_rx(gnss_rxd)
        .into_async();
    //uart0.set_at_cmd(AtCmdConfig::default().with_cmd_char(0x04));
    //let (rx, _) = uart0.split();

    let led = Output::new(user_led, Level::High, OutputConfig::default());
    let _ = Output::new(fan_ctrl, Level::Low, OutputConfig::default()); //fan control
    let button = Input::new(user_button, InputConfig::default().with_pull(Pull::Up));

    let i2c0 = I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(i2c_sda)
    .with_scl(i2c_sdl)
    .into_async();

    spawner.spawn(uart::uart_reader(uart0, POSITION_CHANNEL.sender()).unwrap());
    spawner.spawn(low_prio::low_prio_async().unwrap());
    spawner.spawn(gpio::blink_led(led).unwrap());
    spawner.spawn(viewer::viewer(i2c0, POSITION_CHANNEL.receiver()).unwrap());
    spawner.spawn(lora_send::send_packet(spi_device).unwrap());
    spawner.spawn(gpio::press_button(button).unwrap());

    loop {
        Timer::after(Duration::from_millis(5_000)).await;
    }
}
