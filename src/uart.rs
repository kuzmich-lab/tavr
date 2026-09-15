use defmt::info;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use esp_hal::Async;
use esp_hal::uart::UartRx;
use nmea::Nmea;
use static_cell::StaticCell;

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

#[embassy_executor::task]
pub async fn uart_reader(mut rx: UartRx<'static, Async>) {
    struct NmeaPosition {
        latitude: Option<f64>,
        longitude: Option<f64>,
        altitude: Option<f32>,
        speed_over_ground: Option<f32>,
        num_of_fix_satellites: Option<u32>,
    }
    static SIGNAL_POSITION: StaticCell<Signal<NoopRawMutex, NmeaPosition>> = StaticCell::new();
    let signal_position = &*SIGNAL_POSITION.init(Signal::new());
    let mut read_buf = [0u8; UART_BUFFER_SIZE];
    let mut buf = [0u8; 4096];
    let mut buf_count = 0;
    let nmea = Nmea::default();
    loop {
        let r = embedded_io_async::Read::read(&mut rx, &mut read_buf).await;
        match r {
            Ok(len) => {
                info!("read: {}", len);
                for i in read_buf.iter() {
                    if i != &0x00 {
                        buf[buf_count] = *i;
                        buf_count += 1;
                    }
                }
            }
            Err(err) => info!("read ERR: {}", err),
        }

        if buf_count > 1024 {
            let iter = buf.split_inclusive(|x| x == &10);
            for (n, i) in iter.enumerate() {
                match nmea::parse_bytes(i) {
                    Ok(_) => {
                        let position = NmeaPosition {
                            //fix_time: nmea.fix_time,
                            latitude: nmea.latitude,
                            longitude: nmea.longitude,
                            altitude: nmea.altitude,
                            speed_over_ground: nmea.speed_over_ground,
                            num_of_fix_satellites: nmea.num_of_fix_satellites,
                        };
                        info!("latitude: {}", &nmea.latitude.unwrap_or(0.0));
                        info!("longitude: {}", &nmea.longitude.unwrap_or(0.0));
                        info!("altitude: {}", &nmea.altitude.unwrap_or(0.0));
                        info!("ground_speed: {}", &nmea.speed_over_ground.unwrap_or(0.0));
                        info!("num_sat: {}", &nmea.num_of_fix_satellites.unwrap_or(0));
                        signal_position.signal(position);
                    }
                    Err(e) => info!("NMEA Parse Error: {:?}", e),
                }
            }
            buf_count = 0;
        }
        Timer::after(Duration::from_millis(100)).await;
        // match r {
        //     Ok(_) => {
        //         let mut read_iter = read_buf.split_inclusive(|byte| byte == &0);
        //         let read_res = read_iter.next().unwrap();
        //         info!(
        //             "Read: {:?}, data: {:?}",
        //             read_res.len(),
        //             get_ascii_str(&read_res)
        //         );
        //         info!("buf_count: {}", buf_count);
        //         for i in read_res.iter() {
        //             buf[buf_count] = *i;
        //             buf_count += 1;
        //         }
        //         info!("buf_count: {}", buf_count);
        //         let mut iter = buf.split_inclusive(|byte| byte == &10).peekable();

        //         let mut left = iter.next().unwrap();
        //         if iter.peek().is_some() {
        //             loop {
        //                 info!("left: {}", get_ascii_str(&left));
        //                 buf_count -= left.len();
        //                 match nmea::parse_bytes(left) {
        //                     Ok(_) => {
        //                         info!("latitude: {}", &nmea.latitude.unwrap_or(0.0));
        //                         info!("longitude: {}", &nmea.longitude.unwrap_or(0.0));
        //                         info!("altitude: {}", &nmea.altitude.unwrap_or(0.0));
        //                         info!("ground_speed: {}", &nmea.speed_over_ground.unwrap_or(0.0));
        //                         info!("num_sat: {}", &nmea.num_of_fix_satellites.unwrap_or(0));
        //                     }
        //                     Err(e) => info!("NMEA Parse Error: {:?}", e),
        //                     //signal.signal(len);
        //                 }
        //                 left = iter.next().unwrap();
        //                 if iter.peek().is_none() {
        //                     info!("Break");
        //                     break;
        //                 };
        //             }
        //         }
        //         buf_count -= left.len();
        //         info!("buf_count: {}", buf_count);
        //         let mut buf = [0u8; UART_BUFFER_SIZE];
        //         let mut count = 0;
        //         for into in iter {
        //             for i in into.iter() {
        //                 buf[count] = *i;
        //                 count += 1;
        //             }
        //         }

        //         info!(
        //             "left: {:?}, right: {:?}",
        //             get_ascii_str(&left),
        //             get_ascii_str(&buf)
        //         );
        //     }
        //     Err(e) => {
        //         info!("UART RX Error: {:?}", e);
        //     }
        // }
        //Timer::after(Duration::from_millis(100)).await;
    }
}

pub fn get_ascii_str<'a>(buffer: &'a [u8]) -> Result<&'a str, ()> {
    for byte in buffer.into_iter() {
        if byte >= &128 {
            return Err(());
        }
    }
    Ok(unsafe { core::str::from_utf8_unchecked(buffer) })
}
