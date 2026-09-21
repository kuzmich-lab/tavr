use crate::{AdcValue, NmeaPosition};
use core::str::from_utf8_unchecked;
//use defmt::info;
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
use lexical_core::{FormattedSize, write};
use oled_async::Builder;
use oled_async::prelude::GraphicsMode;

#[embassy_executor::task]
pub async fn viewer(
    i2c0: I2c<'static, Async>,
    receiver_nmea: Receiver<'static, CriticalSectionRawMutex, NmeaPosition, 4>,
    receiver_adc: Receiver<'static, CriticalSectionRawMutex, AdcValue, 4>,
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

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X12)
        .text_color(BinaryColor::On)
        .background_color(BinaryColor::Off)
        .build();

    let mut int_buf = [0u8; i64::FORMATTED_SIZE_DECIMAL];
    let mut float_buf = [0u8; f64::FORMATTED_SIZE_DECIMAL];
    let mut u16_buf = [0u8; i16::FORMATTED_SIZE_DECIMAL];

    Text::with_baseline("lat:", Point { x: 68, y: 0 }, text_style, Baseline::Top)
        .draw(&mut disp)
        .unwrap();
    Text::with_baseline("lon:", Point { x: 68, y: 10 }, text_style, Baseline::Top)
        .draw(&mut disp)
        .unwrap();
    Text::with_baseline("sat:", Point { x: 68, y: 20 }, text_style, Baseline::Top)
        .draw(&mut disp)
        .unwrap();
    Text::with_baseline("volt:", Point { x: 0, y: 0 }, text_style, Baseline::Top)
        .draw(&mut disp)
        .unwrap();
    Text::with_baseline("temp:", Point { x: 0, y: 10 }, text_style, Baseline::Top)
        .draw(&mut disp)
        .unwrap();

    loop {
        let nmea_position_res = receiver_nmea.try_receive();
        match nmea_position_res {
            Ok(nmea_position) => {
                Text::with_baseline(
                    unsafe { from_utf8_unchecked(write(nmea_position.latitude, &mut float_buf)) },
                    Point { x: 94, y: 0 },
                    text_style,
                    Baseline::Top,
                )
                .draw(&mut disp)
                .unwrap();
                Text::with_baseline(
                    unsafe { from_utf8_unchecked(write(nmea_position.longitude, &mut float_buf)) },
                    Point { x: 94, y: 10 },
                    text_style,
                    Baseline::Top,
                )
                .draw(&mut disp)
                .unwrap();
                Text::with_baseline(
                    unsafe {
                        from_utf8_unchecked(write(
                            nmea_position.num_of_fix_satellites,
                            &mut int_buf,
                        ))
                    },
                    Point { x: 94, y: 20 },
                    text_style,
                    Baseline::Top,
                )
                .draw(&mut disp)
                .unwrap();
            }
            Err(_) => {}
        };

        let adc_value_res = receiver_adc.try_receive();
        match adc_value_res {
            Ok(adc_value) => {
                Text::with_baseline(
                    unsafe { from_utf8_unchecked(write(adc_value.voltage, &mut u16_buf)) },
                    Point { x: 32, y: 0 },
                    text_style,
                    Baseline::Top,
                )
                .draw(&mut disp)
                .unwrap();
                Text::with_baseline(
                    unsafe { from_utf8_unchecked(write(adc_value.temp, &mut u16_buf)) },
                    Point { x: 32, y: 10 },
                    text_style,
                    Baseline::Top,
                )
                .draw(&mut disp)
                .unwrap();
            }
            Err(_) => {}
        };

        // info!("nmea_position: {}", nmea_position);
        // info!("adc_value: {}", adc_value);

        disp.flush().await.unwrap();
        Timer::after(Duration::from_millis(100)).await;
    }
}
