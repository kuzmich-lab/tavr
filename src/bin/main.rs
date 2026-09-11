#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use esp_backtrace as _;
use esp_hal::analog::adc;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::main;
use esp_hal::time::{Duration, Instant};
use esp_hal::uart::{Config, RxConfig, TxConfig, Uart};
use esp_println as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32s3 -o esp32s3-wroom-1-octal-psram -o unstable-hal -o esp-backtrace -o defmt -o zed -o esp

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // let boot = peripherals.GPIO0;
    // let lora_dio1 = peripherals.GPIO1;
    // let exp_p5_15 = peripherals.GPIO2;
    // let lora_nreset = peripherals.GPIO3;
    // let battery_voltage = peripherals.GPIO4;
    // let gnss_txd = peripherals.GPIO5;
    // let gnss_rxd = peripherals.GPIO6;
    // let gnss_1pps = peripherals.GPIO7;
    // let i2c_sda = peripherals.GPIO8;
    // let i2c_sdl = peripherals.GPIO9;
    // let sd_cs = peripherals.GPIO10;
    // let spi_mosi = peripherals.GPIO11;
    // let spi_miso = peripherals.GPIO12;
    // let spi_sck = peripherals.GPIO13;
    let temp_samp = peripherals.GPIO14;
    // let lora_cs = peripherals.GPIO15;
    // let gnss_wake_up = peripherals.GPIO16;
    let user_button = peripherals.GPIO17;
    let user_led = peripherals.GPIO18;
    //let esp_usb_n = peripherals.GPIO19;
    //let esp_usb_p = peripherals.GPIO20;
    // let lora_ctl = peripherals.GPIO21;
    // let lora_busy = peripherals.GPIO38;
    // let ext_p5_9 = peripherals.GPIO39;
    // let lora_ldo_en = peripherals.GPIO40;
    // let fan_ctrl = peripherals.GPIO41;
    // let ext_p5_10 = peripherals.GPIO42;
    let ext_txd = peripherals.GPIO43;
    let ext_rxd = peripherals.GPIO44;
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

    let mut led = Output::new(user_led, Level::High, OutputConfig::default());
    let temp = Adc::new();
    let temp = Input::new(temp_samp, InputConfig::default().with_pull(Pull::None));
    let button = Input::new(user_button, InputConfig::default().with_pull(Pull::Up));

    let mut uart1 = Uart::new(
        peripherals.UART1,
        Config::default()
            .with_baudrate(115200)
            .with_rx(RxConfig::default())
            .with_tx(TxConfig::default()),
    )
    .unwrap();

    //   .with_rx(gnss_rxd).with_tx(gnss_txd)

    loop {
        //uart1.write(b"Hello uart1!\r\n").unwrap();
        let temp = temp.peripheral_input().level();
        info!("Temp: {}", temp);
        let button = button.peripheral_input().level();
        info!("Button: {}", button);
        led.toggle();
        let delay_start = Instant::now();
        while delay_start.elapsed() < Duration::from_millis(500) {}
    }
}
