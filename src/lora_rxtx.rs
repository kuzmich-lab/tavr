use crate::LORA_FREQUENCY_IN_HZ;
use crate::RECEIVE_CHANNEL;
use crate::SEND_CHANNEL;
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Delay;
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
pub async fn send_packet(
    spi: Spi<'static, Async>,
    reset: Output<'static>,
    busy: Input<'static>,
    dio1: Input<'static>,
    cs: Output<'static>,
) -> ! {
    let sx126x_config = sx126x::Config {
        chip: Sx1262,
        tcxo_ctrl: Some(TcxoCtrlVoltage::Ctrl1V7),
        use_dcdc: true,
        rx_boost: true,
    };
    let spi_bus: Mutex<NoopRawMutex, Spi<'static, Async>> = Mutex::new(spi);
    let spi_device = SpiDevice::new(&spi_bus, cs);

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
    let sender = RECEIVE_CHANNEL.sender();
    let receiver = SEND_CHANNEL.receiver();
    let mut receive_buffer = [0x00; 118];
    let mut tx_packet_params = {
        match lora.create_tx_packet_params(4, false, true, false, &modulation_params) {
            Ok(pp) => pp,
            Err(err) => {
                info!("Radio error = {}", err);
                panic!()
            }
        }
    };
    let rx_packet_params = {
        match lora.create_rx_packet_params(
            4,
            false,
            receive_buffer.len() as u8,
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

    loop {
        match receiver.try_receive() {
            Ok(send_buffer) => {
                match lora
                    .prepare_for_tx(&modulation_params, &mut tx_packet_params, 5, &send_buffer)
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
            }
            Err(_) => {}
        }
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

        match lora.rx(&rx_packet_params, &mut receive_buffer).await {
            Ok((received_len, _rx_pkt_status)) => {
                info!("rx successful: {}", received_len);
                info!("receiving_buffer: {}", receive_buffer);
                sender.send(receive_buffer).await;
            }
            Err(err) => info!("rx unsuccessful = {}", err),
        }

        match lora.sleep(false).await {
            Ok(()) => info!("Sleep successful"),
            Err(err) => info!("Sleep unsuccessful = {}", err),
        }
    }
}
