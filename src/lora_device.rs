#![no_std]

use core::fmt::Debug;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Delay;
use embedded_hal_async::delay;
use embedded_hal_async::digital::Wait;
use esp_hal::Async;
use esp_hal::gpio::OutputPin;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::spi::master::Spi;
use lora_phy::RxMode;
use lora_phy::mod_params::{Bandwidth, CodingRate, PacketStatus, RadioError, SpreadingFactor};
use lora_phy::mod_traits::InterfaceVariant;
use lora_phy::{DelayNs, LoRa};

// ── Re-exports ──────────────────────────────────────────────────

pub use lora_phy::iv::GenericSx126xInterfaceVariant;
pub use lora_phy::sx126x::{
    self, Config as Sx126xConfig, Sx126x, Sx126xVariant, Sx1262, TcxoCtrlVoltage,
};

// ── Конфигурация пинов ──────────────────────────────────────────

/// Пины, нужные для подключения SX1262 (Heltec V3, LILYGO T3S3 и т. п.).
pub struct Sx126xPins<RESET, DIO1, BUSY> {
    pub reset: RESET,
    pub dio1: DIO1,
    pub busy: BUSY,
}

/// Опциональные пины RF-переключателя (TX/RX antenna switch).
pub struct RfSwitchPins<RX, TX> {
    pub rx: Option<RX>,
    pub tx: Option<TX>,
}

/// Параметры LoRa-канала (P2P).
#[derive(Clone, Copy, Debug)]
pub struct LoRaChannel {
    pub frequency_hz: u32,
    pub spreading_factor: SpreadingFactor,
    pub bandwidth: Bandwidth,
    pub coding_rate: CodingRate,
}

impl Default for LoRaChannel {
    fn default() -> Self {
        Self {
            frequency_hz: 868_000_000,
            spreading_factor: SpreadingFactor::_7,
            bandwidth: Bandwidth::_125KHz,
            coding_rate: CodingRate::_4_5,
        }
    }
}

// ── Обёртка для SX1262 ───────────────────────────────────────────

/// Готовая обёртка LoRa-радио на базе SX1262 для ESP32-S3.
///
/// Параметр `SPI` — реализация `SpiDevice<u8>` (например
/// `embedded_hal_bus::spi::ExclusiveDevice<...>`).
pub struct LoraSx1262 {
    pub lora: LoRa,
    pub channel: LoRaChannel,
}

impl LoraSx1262 {
    pub async fn new<D: DelayNs>(
        spi: Spi<'static, Async>,
        iv: IV,
        channel: LoRaChannel,
        is_public_network: bool,
        delay: Delay,
        cs: Output<'static>,
    ) -> Result<Self, RadioError> {
        let sx126x_config = Sx126xConfig {
            chip: Sx1262,
            tcxo_ctrl: Some(TcxoCtrlVoltage::Ctrl1V6),
            use_dcdc: false,
            rx_boost: false,
        };
        let spi_bus: Mutex<NoopRawMutex, Spi<'static, Async>> = Mutex::new(spi);
        let spi_device = SpiDevice::new(&spi_bus, cs);
        let radio_kind = Sx126x::new(spi_device, iv, sx126x_config);
        let lora = LoRa::new(radio_kind, is_public_network, delay).await?;
        let mut lora = LoRa::new(Sx126x::new(spi_device, iv, sx126x_config), false, Delay)
            .await
            .unwrap();

        Ok(Self { lora, channel })
    }

    /// Отправляет payload по P2P-каналу.
    pub async fn send<D: DelayNs>(
        &mut self,
        delay: &mut D,
        payload: &[u8],
        preamble_length: u16,
        crc_on: bool,
        iq_inverted: bool,
        tx_power_dbm: i32,
    ) -> Result<(), RadioError> {
        let mdltn_params = self.lora.create_modulation_params(
            self.channel.spreading_factor,
            self.channel.bandwidth,
            self.channel.coding_rate,
            self.channel.frequency_hz,
        )?;

        let tx_pkt_params = self.lora.create_tx_packet_params(
            preamble_length,
            false, // explicit header
            crc_on,
            iq_inverted,
            &mdltn_params,
        )?;

        self.lora.tx().await
    }

    /// Подготавливает и выполняет приём одного пакета.
    ///
    /// Возвращает `(длина, статус_пакета)`.
    pub async fn receive(
        &mut self,
        buffer: &mut [u8],
        preamble_length: u16,
        crc_on: bool,
        iq_inverted: bool,
    ) -> Result<(u8, PacketStatus), RadioError> {
        let mdltn_params = self.lora.create_modulation_params(
            self.channel.spreading_factor,
            self.channel.bandwidth,
            self.channel.coding_rate,
            self.channel.frequency_hz,
        )?;

        let rx_pkt_params = self.lora.create_rx_packet_params(
            preamble_length,
            false, // explicit header
            buffer.len() as u8,
            crc_on,
            iq_inverted,
            &mdltn_params,
        )?;

        self.lora
            .prepare_for_rx(RxMode::Continuous, &mdltn_params, &rx_pkt_params)
            .await?;

        self.lora.rx(&rx_pkt_params, buffer).await
    }

    /// Переводит чип в standby.
    pub async fn standby(&mut self) -> Result<(), RadioError> {
        self.lora.set_standby().await
    }

    /// Переводит чип в sleep.
    pub async fn sleep<D: DelayNs>(
        &mut self,
        warm_start: bool,
        delay: &mut D,
    ) -> Result<(), RadioError> {
        self.lora.set_sleep(warm_start, delay).await
    }
}
