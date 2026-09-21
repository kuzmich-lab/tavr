// #![no_std]

// use core::fmt::Debug;
// use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
// use embedded_hal_async::digital::Wait;
// use esp_hal::Async;
// use esp_hal::gpio::OutputPin;
// use lora_phy::mod_params::{Bandwidth, CodingRate, PacketStatus, RadioError, SpreadingFactor};
// use lora_phy::mod_traits::InterfaceVariant;
// use lora_phy::{DelayNs, LoRa};

// // ── Re-exports ──────────────────────────────────────────────────

// pub use lora_phy::iv::{GenericSx126xInterfaceVariant, GenericSx127xInterfaceVariant};
// pub use lora_phy::sx126x::{
//     self, Config as Sx126xConfig, Sx126x, Sx126xVariant, Sx1261, Sx1262, TcxoCtrlVoltage,
// };

// // ── Конфигурация пинов ──────────────────────────────────────────

// /// Пины, нужные для подключения SX1262 (Heltec V3, LILYGO T3S3 и т. п.).
// pub struct Sx126xPins<RESET, DIO1, BUSY> {
//     pub reset: RESET,
//     pub dio1: DIO1,
//     pub busy: BUSY,
// }

// /// Опциональные пины RF-переключателя (TX/RX antenna switch).
// pub struct RfSwitchPins<RX, TX> {
//     pub rx: Option<RX>,
//     pub tx: Option<TX>,
// }

// /// Параметры LoRa-канала (P2P).
// #[derive(Clone, Copy, Debug)]
// pub struct LoRaChannel {
//     pub frequency_hz: u32,
//     pub spreading_factor: SpreadingFactor,
//     pub bandwidth: Bandwidth,
//     pub coding_rate: CodingRate,
// }

// impl Default for LoRaChannel {
//     fn default() -> Self {
//         Self {
//             frequency_hz: 868_000_000,
//             spreading_factor: SpreadingFactor::_7,
//             bandwidth: Bandwidth::_125KHz,
//             coding_rate: CodingRate::_4_5,
//         }
//     }
// }

// // ── Обёртка для SX1262 ───────────────────────────────────────────

// /// Готовая обёртка LoRa-радио на базе SX1262 для ESP32-S3.
// ///
// /// Параметр `SPI` — реализация `SpiDevice<u8>` (например
// /// `embedded_hal_bus::spi::ExclusiveDevice<...>`).
// pub struct LoraSx1262<SPI, IV> {
//     pub lora: LoRa<Sx126x<SPI, IV, Sx1262>>,
//     pub channel: LoRaChannel,
// }

// impl<SPI, IV> LoraSx1262<SPI, IV>
// where
//     SPI: embassy_embedded_hal::shared_bus::asynch::spi,
//     IV: InterfaceVariant,
// {
//     /// Создаёт инициализированное радио.
//     ///
//     /// - `spi` — `SpiDevice`, полученный через `ExclusiveDevice::new(spi_bus, cs, delay)`.
//     /// - `iv` — `GenericSx126xInterfaceVariant` с пинами reset / DIO1 / BUSY.
//     /// - `delay` — реализация `DelayNs` (например `embassy_time::Delay`).
//     /// - `channel` — параметры LoRa-канала.
//     /// - `is_public_network` — `true` для публичной сети LoRaWAN.
//     pub async fn new<D: DelayNs>(
//         spi: SPI,
//         iv: IV,
//         delay: &mut D,
//         channel: LoRaChannel,
//         is_public_network: bool,
//     ) -> Result<Self, RadioError> {
//         let config = Sx126xConfig {
//             chip: Sx1262,
//             tcxo_ctrl: Some(TcxoCtrlVoltage::Ctrl1V6),
//             use_dcdc: false,
//             rx_boost: false,
//         };
//         let radio_kind = Sx126x::new(spi, iv, config);
//         let lora = LoRa::new(radio_kind, is_public_network, delay).await?;

//         Ok(Self { lora, channel })
//     }

//     /// Отправляет payload по P2P-каналу.
//     pub async fn send<D: DelayNs>(
//         &mut self,
//         delay: &mut D,
//         payload: &[u8],
//         preamble_length: u16,
//         crc_on: bool,
//         iq_inverted: bool,
//         tx_power_dbm: i32,
//     ) -> Result<(), RadioError> {
//         let mdltn_params = self.lora.create_modulation_params(
//             self.channel.spreading_factor,
//             self.channel.bandwidth,
//             self.channel.coding_rate,
//             self.channel.frequency_hz,
//         )?;

//         let tx_pkt_params = self.lora.create_tx_packet_params(
//             preamble_length,
//             false, // explicit header
//             crc_on,
//             iq_inverted,
//             &mdltn_params,
//         )?;

//         self.lora
//             .tx(&mdltn_params, &tx_pkt_params, payload, tx_power_dbm, delay)
//             .await
//     }

//     /// Подготавливает и выполняет приём одного пакета.
//     ///
//     /// Возвращает `(длина, статус_пакета)`.
//     pub async fn receive(
//         &mut self,
//         buffer: &mut [u8],
//         preamble_length: u16,
//         crc_on: bool,
//         iq_inverted: bool,
//     ) -> Result<(u8, PacketStatus), RadioError> {
//         let mdltn_params = self.lora.create_modulation_params(
//             self.channel.spreading_factor,
//             self.channel.bandwidth,
//             self.channel.coding_rate,
//             self.channel.frequency_hz,
//         )?;

//         let rx_pkt_params = self.lora.create_rx_packet_params(
//             preamble_length,
//             false, // explicit header
//             buffer.len() as u8,
//             crc_on,
//             iq_inverted,
//             &mdltn_params,
//         )?;

//         self.lora
//             .prepare_for_rx(
//                 &mdltn_params,
//                 &rx_pkt_params,
//                 None,        // кад params
//                 true,        // rx_continuous
//                 false,       // rx_boosted
//                 0,           // tx_preamble_length
//                 0x00ff_ffff, // rx_timeout
//             )
//             .await?;

//         self.lora.rx(&rx_pkt_params, buffer).await
//     }

//     /// Переводит чип в standby.
//     pub async fn standby(&mut self) -> Result<(), RadioError> {
//         self.lora.set_standby().await
//     }

//     /// Переводит чип в sleep.
//     pub async fn sleep<D: DelayNs>(
//         &mut self,
//         warm_start: bool,
//         delay: &mut D,
//     ) -> Result<(), RadioError> {
//         self.lora.set_sleep(warm_start, delay).await
//     }
// }

// // ── Обёртка для SX1276 ───────────────────────────────────────────

// /// Готовая обёртка LoRa-радио на базе SX1276 для ESP32-S3.
// pub struct LoraSx1276<SPI, IV> {
//     pub lora: LoRa<Sx127x<SPI, IV>>,
//     pub channel: LoRaChannel,
// }

// impl<SPI, IV> LoraSx1276<SPI, IV>
// where
//     SPI: embedded_hal::spi::SpiDevice<u8>,
//     IV: InterfaceVariant,
// {
//     pub async fn new<D: DelayNs>(
//         spi: SPI,
//         iv: IV,
//         delay: &mut D,
//         channel: LoRaChannel,
//         is_public_network: bool,
//     ) -> Result<Self, RadioError> {
//         let radio_kind = Sx127x::new(spi, iv);
//         let lora = LoRa::new(radio_kind, is_public_network, delay).await?;

//         Ok(Self { lora, channel })
//     }

//     pub async fn send<D: DelayNs>(
//         &mut self,
//         delay: &mut D,
//         payload: &[u8],
//         preamble_length: u16,
//         crc_on: bool,
//         iq_inverted: bool,
//         tx_power_dbm: i32,
//     ) -> Result<(), RadioError> {
//         let mdltn_params = self.lora.create_modulation_params(
//             self.channel.spreading_factor,
//             self.channel.bandwidth,
//             self.channel.coding_rate,
//             self.channel.frequency_hz,
//         )?;

//         let tx_pkt_params = self.lora.create_tx_packet_params(
//             preamble_length,
//             false,
//             crc_on,
//             iq_inverted,
//             &mdltn_params,
//         )?;

//         self.lora
//             .tx(&mdltn_params, &tx_pkt_params, payload, tx_power_dbm, delay)
//             .await
//     }

//     pub async fn receive(
//         &mut self,
//         buffer: &mut [u8],
//         preamble_length: u16,
//         crc_on: bool,
//         iq_inverted: bool,
//     ) -> Result<(u8, PacketStatus), RadioError> {
//         let mdltn_params = self.lora.create_modulation_params(
//             self.channel.spreading_factor,
//             self.channel.bandwidth,
//             self.channel.coding_rate,
//             self.channel.frequency_hz,
//         )?;

//         let rx_pkt_params = self.lora.create_rx_packet_params(
//             preamble_length,
//             false,
//             buffer.len() as u8,
//             crc_on,
//             iq_inverted,
//             &mdltn_params,
//         )?;

//         self.lora
//             .prepare_for_rx(
//                 &mdltn_params,
//                 &rx_pkt_params,
//                 None,
//                 true,
//                 false,
//                 0,
//                 0x00ff_ffff,
//             )
//             .await?;

//         self.lora.rx(&rx_pkt_params, buffer).await
//     }

//     pub async fn standby(&mut self) -> Result<(), RadioError> {
//         self.lora.set_standby().await
//     }

//     pub async fn sleep<D: DelayNs>(
//         &mut self,
//         warm_start: bool,
//         delay: &mut D,
//     ) -> Result<(), RadioError> {
//         self.lora.set_sleep(warm_start, delay).await
//     }
// }

// // ── Хелпер для создания InterfaceVariant ─────────────────────────

// /// Создаёт `GenericSx126xInterfaceVariant` для ESP32-S3 + SX1262.
// ///
// /// `RESET`, `DIO1`, `BUSY` — пины `esp-hal`, реализующие
// /// `OutputPin` (reset) и `Wait` (dio1, busy) соответственно.
// /// `rf_switch_rx` / `rf_switch_tx` — опциональные пины антенного переключателя.
// pub fn sx1262_interface<RESET, DIO1, BUSY>(
//     reset: RESET,
//     dio1: DIO1,
//     busy: BUSY,
//     rf_switch_rx: Option<RESET>,
//     rf_switch_tx: Option<RESET>,
// ) -> Result<GenericSx126xInterfaceVariant<RESET, DIO1>, RadioError>
// where
//     RESET: OutputPin,
//     DIO1: Wait,
//     BUSY: Wait,
// {
//     // `GenericSx126xInterfaceVariant` хранит dio1 и busy под одним типом WAIT.
//     // Если типы пинов DIO1 и BUSY совпадают — можно передать напрямую.
//     // Если нет — пользователь должен привести их к одному типу (degrade() в esp-hal).
//     GenericSx126xInterfaceVariant::new(reset, dio1, busy, rf_switch_rx, rf_switch_tx)
// }

// /// Создаёт `GenericSx127xInterfaceVariant` для ESP32-S3 + SX1276.
// pub fn sx1276_interface<RESET, IRQ>(
//     reset: RESET,
//     irq: IRQ,
//     rf_switch_rx: Option<RESET>,
//     rf_switch_tx: Option<RESET>,
// ) -> Result<GenericSx127xInterfaceVariant<RESET, IRQ>, RadioError>
// where
//     RESET: OutputPin,
//     IRQ: Wait,
// {
//     GenericSx127xInterfaceVariant::new(reset, irq, rf_switch_rx, rf_switch_tx)
// }
