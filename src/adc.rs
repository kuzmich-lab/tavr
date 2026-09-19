use defmt::info;
use embassy_time::{Duration, Timer};
use esp_hal::Async;
use esp_hal::analog::adc::{Adc, AdcPin};

// #[embassy_executor::task]
// pub async fn get_bat_voltage(
//     adc: Adc<'static, Async>,
//     adc_pin: AdcPin<GPIO4<'static>, ADC1<'static>>,
// ) {
//     loop {
//         info!("ADC (volt): {}",adc.read_oneshot(&mut adc_pin).await());
//         Timer::after(Duration::from_millis(1_000)).await;
//     }
// }

// #[embassy_executor::task]
// pub async fn get_temp(adc: Adc<'static, Async>, adc_pin: AdcPin<GPIO4<'static>, ADC1<'static>>) {
//     loop {
//         info!("ADC (temp): {}", adc.read_oneshot(&mut adc_pin).await());
//         Timer::after(Duration::from_millis(1_000)).await;
//     }
// }
