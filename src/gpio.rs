use crate::{MAX_PAYLOAD_LEN, Message, MsgType};
use crate::{MESSAGE_PBC, NaiveTime, POSITION_MUTEX};
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
    let mut message = Message {
        source: [0x00; 6],
        destination: [0x00; 6],
        msg_type: MsgType::Ping,
        message: [0; MAX_PAYLOAD_LEN as usize],
        time_stamp: NaiveTime::MIN,
    };
    let pub0 = MESSAGE_PBC.publisher().unwrap();
    loop {
        Timer::after(Duration::from_millis(1000)).await;
        pin_button.wait_for_falling_edge().await;
        info!("Button Pressed!");
        let mut msg_msg = [0; MAX_PAYLOAD_LEN as usize];
        let nmea_position = POSITION_MUTEX.lock().await;
        msg_msg[0..8].copy_from_slice(&nmea_position.latitude.to_le_bytes());
        msg_msg[8..16].copy_from_slice(&nmea_position.longitude.to_le_bytes());
        message = Message {
            msg_type: MsgType::Ping,
            message: msg_msg,
            ..message
        };
        let _ = pub0.publish_immediate(message);
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
