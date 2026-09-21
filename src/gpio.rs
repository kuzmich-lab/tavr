use defmt::info;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Input, Output};

#[embassy_executor::task]
pub async fn blink_led(mut pin_led: Output<'static>) {
    loop {
        pin_led.toggle();
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[embassy_executor::task]
pub async fn press_button(mut pin_button: Input<'static>) {
    loop {
        pin_button.wait_for_low().await;
        info!("Button Pressed!");
        Timer::after(Duration::from_millis(100)).await;
    }
}

#[embassy_executor::task]
pub async fn pps_flash(mut pps_1: Input<'static>) {
    loop {
        pps_1.wait_for_high().await;
        info!("---------------------------------1 pps---------------------------------");
        Timer::after(Duration::from_millis(900)).await;
    }
}

#[embassy_executor::task]
pub async fn get_temp() {
    loop {
        Timer::after(Duration::from_millis(100)).await;
    }
}
