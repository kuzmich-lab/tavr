use crate::{NmeaPosition, POSITION_CHANNEL};
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};
use esp_hal::Async;
use esp_hal::uart::Uart;
use nmea::Nmea;

const UART_BUFFER_SIZE: usize = 256;

// #[embassy_executor::task]
// pub async fn uart_writer(
//     mut tx: UartTx<'static, Async>,
//     signal: &'static Signal<NoopRawMutex, usize>,
// ) {
//     use core::fmt::Write;
//     embedded_io_async::Write::write(
//         &mut tx,
//         b"Hello async serial. Enter something ended with EOT (CTRL-D).\r\n",
//     )
//     .await
//     .unwrap();
//     warn!("Hello async serial. Enter something ended with EOT (CTRL-D).\r\n");
//     embedded_io_async::Write::flush(&mut tx).await.unwrap();
//     loop {
//         let bytes_read = signal.wait().await;
//         signal.reset();
//         write!(&mut tx, "\r\n-- received {} bytes --\r\n", bytes_read).unwrap();
//         embedded_io_async::Write::flush(&mut tx).await.unwrap();
//     }
// }
//

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
    loop {
        let r = uart.read_async(&mut read_buf).await;
        match r {
            Ok(_) => {
                for i in read_buf.iter() {
                    buf[buf_count] = *i;
                    buf_count += 1;
                }
            }
            Err(err) => info!("read ERR: {}", err),
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
                            let nmea_position = NmeaPosition {
                                date: nmea.fix_date,
                                time: nmea.fix_time,
                                latitude: nmea.latitude,
                                longitude: nmea.longitude,
                                altitude: nmea.altitude,
                                speed_over_ground: nmea.speed_over_ground,
                                num_of_fix_satellites: nmea.num_of_fix_satellites,
                            };
                            info!("position: {}", nmea_position);
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

// pub fn get_ascii_str<'a>(buffer: &'a [u8]) -> Result<&'a str, ()> {
//     for byte in buffer.into_iter() {
//         if byte >= &128 {
//             return Err(());
//         }
//     }
//     Ok(unsafe { core::str::from_utf8_unchecked(buffer) })
// }
