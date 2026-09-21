use crate::AdcValue;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::peripherals::{ADC1, ADC2, GPIO4, GPIO14};

#[embassy_executor::task]
pub async fn get_adc(
    adc1: ADC1<'static>,
    adc2: ADC2<'static>,
    pin1: GPIO4<'static>,
    pin2: GPIO14<'static>,
    sender: Sender<'static, CriticalSectionRawMutex, AdcValue, 4>,
) {
    let mut config1 = AdcConfig::new();
    let mut config2 = AdcConfig::new();
    let mut adc_pin1 = config1.enable_pin(pin1, Attenuation::_11dB);
    let mut adc_pin2 = config2.enable_pin(pin2, Attenuation::_11dB);
    let mut adc1 = Adc::new(adc1, config1).into_async();
    let mut adc2 = Adc::new(adc2, config2).into_async();
    let mut adc_value = AdcValue::new();
    loop {
        adc_value.voltage = adc1.read_oneshot(&mut adc_pin1).await;
        adc_value.temp = adc2.read_oneshot(&mut adc_pin2).await;
        sender.send(adc_value).await;
        Timer::after(Duration::from_millis(1_000)).await;
    }
}
