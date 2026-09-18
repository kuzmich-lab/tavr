// defmt = "0.3.10"
// embassy-embedded-hal = { version = "0.3.0", features = ["defmt"] }
// embassy-executor = { version = "0.10.0", features = ["defmt"] }
// embassy-sync = "0.6.2"
// embassy-time = { version = "0.5.0", features = ["defmt", "generic-queue-64"] }
// esp-hal = { version = "1.2.0", features = ["esp32s3", "defmt", "unstable"] }
// esp-rtos = { version = "0.4.0", features = ["defmt", "embassy", "esp32s3"] }

// #esp-hal-embassy = { version = "0.8.0", features = ["esp32s3", "defmt"] }
// esp-println = { version = "0.13.1", features = ["esp32s3", "defmt-espflash"] }
// lora-phy = "3.0.1"
// static_cell = "2.1.0"

// #esp-rtos = { version = "^0.3.0", features = ["defmt", "embassy", "esp32s3"] }
// esp-bootloader-esp-idf = { version = "0.5.0", features = ["defmt", "esp32s3"] }
// esp-backtrace = { version = "0.15.0", features = [
//   "defmt",
//   "esp32s3",
//   "panic-handler",
// ] }
// critical-section = "1.2.0"

use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::spi;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Delay, Duration, Timer};
use esp_backtrace as _;
use esp_hal::Async;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::Input;
use esp_hal::gpio::InputConfig;
use esp_hal::gpio::Level;
use esp_hal::gpio::Output;
use esp_hal::gpio::OutputConfig;
use esp_hal::spi::Mode;
use esp_hal::spi::master::Config;
use esp_hal::spi::master::Spi;
use esp_hal::spi::master::SpiDmaBus;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;
use lora_phy::LoRa;
use lora_phy::iv::GenericSx126xInterfaceVariant;
use lora_phy::mod_params::Bandwidth;
use lora_phy::mod_params::CodingRate;
use lora_phy::mod_params::SpreadingFactor;
use lora_phy::sx126x;
use lora_phy::sx126x::Sx126x;
use lora_phy::sx126x::Sx1262;
use lora_phy::sx126x::TcxoCtrlVoltage;

esp_bootloader_esp_idf::esp_app_desc!();
const LORA_FREQUENCY_IN_HZ: u32 = 868_900_000; // WARNING: Set this appropriately for the region

// static SPI_BUS: StaticCell<
//     Mutex<CriticalSectionRawMutex, esp_hal::spi::master::Spi<'static, Async>>,
// > = StaticCell::new();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[embassy_executor::task]
pub async fn send_packet(spi_device) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    //let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    //esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

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

    // let lora_dio1 = peripherals.GPIO1;
    // let lora_nreset = peripherals.GPIO3;
    // let gnss_rxd = peripherals.GPIO5;
    // let gnss_txd = peripherals.GPIO6;
    // let i2c_sda = peripherals.GPIO8;
    // let i2c_sdl = peripherals.GPIO9;
    // let spi_mosi = peripherals.GPIO11;
    // let spi_miso = peripherals.GPIO12;
    // let spi_sck = peripherals.GPIO13;
    // let lora_cs = peripherals.GPIO15;
    // let lora_ctl = peripherals.GPIO21;
    // let lora_busy = peripherals.GPIO38;
    // let lora_ldo_en = peripherals.GPIO40;

    // let nss = Output::new(peripherals.GPIO15, Level::High, OutputConfig::default());
    // let sclk = peripherals.GPIO13;
    // let mosi = peripherals.GPIO11;
    // let miso = peripherals.GPIO12;

    let reset = Output::new(peripherals.GPIO3, Level::Low, OutputConfig::default());
    let busy = Input::new(peripherals.GPIO38, InputConfig::default());
    let dio1 = Input::new(peripherals.GPIO1, InputConfig::default());

    info!("Embassy initialized!");

    // let spi = Spi::new(
    //     peripherals.SPI2,
    //     Config::default()
    //         .with_frequency(Rate::from_khz(100))
    //         .with_mode(Mode::_0),
    // )
    // .unwrap()
    // .with_sck(sclk)
    // .with_mosi(mosi)
    // .with_miso(miso)
    // .into_async();

    // Initialize the static SPI bus
    //let mut spi_bus = SpiDmaBus.init(Mutex::new(spi));
    //let spi_device = spi::SpiDevice::new(spi_bus, nss);
    //let spi_device = embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(spi_bus, nss);

    // Create the SX126x configuration
    let sx126x_config = sx126x::Config {
        chip: Sx1262,
        tcxo_ctrl: Some(TcxoCtrlVoltage::Ctrl1V7),
        use_dcdc: false,
        rx_boost: true,
    };

    // Create the radio instance
    let iv = GenericSx126xInterfaceVariant::new(reset, dio1, busy, None, None).unwrap();
    let mut lora = LoRa::new(Sx126x::new(spi_device, iv, sx126x_config), false, Delay)
        .await
        .unwrap();

    let modulation_params = {
        match lora.create_modulation_params(
            SpreadingFactor::_12,
            Bandwidth::_125KHz,
            CodingRate::_4_5,
            LORA_FREQUENCY_IN_HZ,
        ) {
            Ok(mp) => mp,
            Err(err) => {
                info!("Radio error = {}", err);
                panic!()
            }
        }
    };

    let mut tx_packet_params = {
        match lora.create_tx_packet_params(4, false, true, false, &modulation_params) {
            Ok(pp) => pp,
            Err(err) => {
                info!("Radio error = {}", err);
                panic!()
            }
        }
    };

    let buffer = [1_u8; 64];

    match lora
        .prepare_for_tx(&modulation_params, &mut tx_packet_params, 5, &buffer)
        .await
    {
        Ok(()) => {}
        Err(err) => {
            info!("Radio error = {}", err);
            panic!()
        }
    };

    match lora.tx().await {
        Ok(()) => {
            info!("TX DONE");
        }
        Err(err) => {
            info!("Radio error = {}", err);
            panic!()
        }
    };

    match lora.sleep(false).await {
        Ok(()) => info!("Sleep successful"),
        Err(err) => info!("Sleep unsuccessful = {}", err),
    }
    loop {
        //info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }
}
