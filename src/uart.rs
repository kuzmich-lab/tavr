use crate::NmeaPosition;
//use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};
use esp_hal::Async;
use esp_hal::uart::Uart;
use nmea::Nmea;

const UART_BUFFER_SIZE: usize = 256;

#[embassy_executor::task]
pub async fn uart_reader(
    mut uart: Uart<'static, Async>,
    sender: Sender<'static, CriticalSectionRawMutex, NmeaPosition, 4>,
) {
    let mut read_buf = [0u8; UART_BUFFER_SIZE];
    let mut buf = [0u8; 2048];
    let mut temp_buf = [0u8; 2048];
    let mut buf_count = 0;
    let nmea = Nmea::default();
    let mut nmea_position = NmeaPosition::new();
    sender.send(nmea_position).await;

    loop {
        let r = uart.read_async(&mut read_buf).await;
        match r {
            Ok(_) => {
                for i in read_buf.iter() {
                    buf[buf_count] = *i;
                    buf_count += 1;
                }
            }
            Err(_) => {} //info!("read ERR: {}", err),
        }
        if buf_count > 1024 {
            let mut iter = buf.split_inclusive(|x| x == &10).peekable();
            while iter.peek().is_some() {
                let item = iter.next().unwrap();
                if iter.peek().is_some() {
                    buf_count -= item.len();
                    //i.strip_prefix(&0x24);
                    match nmea::parse_bytes(&item) {
                        Ok(_) => {
                            nmea_position.date = nmea.fix_date.unwrap_or_default();
                            nmea_position.time = nmea.fix_time.unwrap_or_default();
                            nmea_position.latitude = nmea.latitude.unwrap_or_default();
                            nmea_position.longitude = nmea.longitude.unwrap_or_default();
                            nmea_position.altitude = nmea.altitude.unwrap_or_default();
                            nmea_position.speed_over_ground =
                                nmea.speed_over_ground.unwrap_or_default();
                            nmea_position.num_of_fix_satellites =
                                nmea.num_of_fix_satellites.unwrap_or_default();

                            //info!("nmea_parse: {}", nmea_position);
                            sender.send(nmea_position).await;
                        }
                        Err(_) => {} // info!("NMEA Parse Error: {:?}", e),
                    }
                } else {
                    buf_count = 0;
                    let mut i = item.iter().peekable();
                    while i.peek() != Some(&&0_u8) {
                        temp_buf[buf_count] = *i.next().unwrap();
                        buf_count += 1;
                    }
                    break;
                };
            }
        }
        buf = temp_buf;

        Timer::after(Duration::from_millis(100)).await;
    }
}
