use crate::LORA_FREQUENCY_IN_HZ;
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Delay, Duration, Timer};
use esp_backtrace as _;
use esp_hal::Async;
use esp_hal::gpio::Input;
use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;
use esp_println as _;
use lora_phy::LoRa;
use lora_phy::RxMode;
use lora_phy::iv::GenericSx126xInterfaceVariant;
use lora_phy::mod_params::Bandwidth;
use lora_phy::mod_params::CodingRate;
use lora_phy::mod_params::SpreadingFactor;
use lora_phy::sx126x;
use lora_phy::sx126x::Sx126x;
use lora_phy::sx126x::Sx1262;
use lora_phy::sx126x::TcxoCtrlVoltage;

#[embassy_executor::task]
pub async fn receive_packet(
    spi: Spi<'static, Async>,
    reset: Output<'static>,
    busy: Input<'static>,
    dio1: Input<'static>,
    cs: Output<'static>,
) -> ! {
    let spi_bus: Mutex<NoopRawMutex, Spi<'static, Async>> = Mutex::new(spi);
    let spi_device = SpiDevice::new(&spi_bus, cs);

    let sx126x_config = sx126x::Config {
        chip: Sx1262,
        tcxo_ctrl: Some(TcxoCtrlVoltage::Ctrl1V7),
        use_dcdc: false,
        rx_boost: true,
    };

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

    let mut receiving_buffer = [0u8; 64];
    let expected_msg_len = 64u8;
    let rx_packet_params = {
        match lora.create_rx_packet_params(
            4,
            false,
            receiving_buffer.len() as u8,
            true,
            false,
            &modulation_params,
        ) {
            Ok(pp) => pp,
            Err(err) => {
                info!("Radio error = {}", err);
                panic!();
            }
        }
    };

    match lora
        .prepare_for_rx(RxMode::Continuous, &modulation_params, &rx_packet_params)
        .await
    {
        Ok(()) => {}
        Err(err) => {
            info!("Radio error = {}", err);
            panic!();
        }
    };

    loop {
        receiving_buffer = [0u8; 64];
        match lora.rx(&rx_packet_params, &mut receiving_buffer).await {
            Ok((received_len, _rx_pkt_status)) => {
                if (received_len == expected_msg_len as u8) && (receiving_buffer == [1_u8; 64]) {
                    info!(
                        "rx successful: {}",
                        core::str::from_utf8(&receiving_buffer[..received_len as usize]).unwrap()
                    );
                    info!("receiving_buffer: {}", receiving_buffer);
                } else {
                    info!("rx unknown packet");
                }
            }
            Err(err) => info!("rx unsuccessful = {}", err),
        }
        Timer::after(Duration::from_secs(1)).await;
    }
}

// // Приём
// let data = radio.recv().await?;
// match Packet::decode(&data) {
//     Ok(pkt) => { /* обрабатываем */ }
//     Err(DecodeError::CrcMismatch) => { /* пакет повреждён */ }
//     Err(e) => { /* другая ошибка */ }
// }
