use crate::{NmeaPosition, POSITION_CHANNEL};
use defmt::Format;
use display_interface_i2c::I2CInterface;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, iso_8859_5::FONT_6X12},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use esp_hal::Async;
use esp_hal::i2c::master::I2c;
use oled_async::Builder;
use oled_async::prelude::GraphicsMode;

#[embassy_executor::task]
pub async fn viewer(
    i2c0: I2c<'static, Async>,
    receiver: Receiver<'static, CriticalSectionRawMutex, NmeaPosition, 4>,
) {
    let di = I2CInterface::new(
        i2c0, // I2C
        0x3C, // I2C Address
        0x40, // Databyte
    );
    let raw_disp = Builder::new(oled_async::displays::sh1106::Sh1106_128_64 {}).connect(di);
    let mut disp: GraphicsMode<_, _> = raw_disp.into();

    disp.init().await.unwrap();
    disp.clear();
    disp.flush().await.unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X12)
        .text_color(BinaryColor::On)
        .build();

    loop {
        Text::with_baseline("Привет", Point::zero(), text_style, Baseline::Top)
            .draw(&mut disp)
            .unwrap();
        disp.flush().await.unwrap();
        let nmea_position = receiver.receive().await;
        Timer::after(Duration::from_millis(1_000)).await;
    }
}
