use crate::POSITION_MUTEX;
use defmt::info;
use esp_hal::Async;
use esp_hal::uart::Uart;
use nmea::{Error::DisabledSentence, ParseResult, parse_bytes};

const UART_BUFFER_SIZE: usize = 64;

#[embassy_executor::task]
pub async fn uart_reader(mut uart: Uart<'static, Async>) {
    let mut read_buf = [0u8; UART_BUFFER_SIZE];
    let mut buf = [0u8; 1024];
    let mut buf_count = 0;

    loop {
        let r = uart.read_async(&mut read_buf).await;
        match r {
            Ok(_) => {}
            Err(err) => info!("read ERR: {}", err),
        }
        for i in read_buf.iter() {
            if i != &0x00 {
                buf[buf_count] = *i;
                buf_count += 1;
            }
        }
        if buf_count > 768 {
            let mut iter = buf.split_inclusive(|x| x == &0x0A).peekable();
            while iter.peek().is_some() {
                let item = iter.next().unwrap();
                if iter.peek().is_some() {
                    buf_count -= item.len();
                    //info!("parse_item: {}", &item);
                    match parse_bytes(&item) {
                        Ok(ParseResult::GGA(gga)) => {
                            let mut nmea_position = POSITION_MUTEX.lock().await;
                            nmea_position.time = gga.fix_time.unwrap_or_default();
                            nmea_position.latitude = gga.latitude.unwrap_or_default();
                            nmea_position.longitude = gga.longitude.unwrap_or_default();
                            nmea_position.altitude = gga.altitude.unwrap_or_default();
                            nmea_position.fix_satellites = gga.fix_satellites.unwrap_or_default();
                            //info!("gga_parse: {}", &gga);
                        }
                        Ok(ParseResult::GNS(gns)) => {
                            let mut nmea_position = POSITION_MUTEX.lock().await;
                            nmea_position.time = gns.fix_time.unwrap_or_default();
                            nmea_position.latitude = gns.lat.unwrap_or_default();
                            nmea_position.longitude = gns.lon.unwrap_or_default();
                            nmea_position.altitude = gns.alt.unwrap_or_default();
                            info!("gns_parse: {}", &gns);
                        }
                        Ok(ParseResult::RMC(rmc)) => {
                            let mut nmea_position = POSITION_MUTEX.lock().await;
                            nmea_position.date = rmc.fix_date.unwrap_or_default();
                            nmea_position.time = rmc.fix_time.unwrap_or_default();
                            nmea_position.latitude = rmc.lat.unwrap_or_default();
                            nmea_position.longitude = rmc.lon.unwrap_or_default();
                            nmea_position.speed_over_ground =
                                rmc.speed_over_ground.unwrap_or_default();
                            //info!("rmc_parse: {}", &rmc);
                        }
                        Ok(_) => {}
                        Err(DisabledSentence) => {}
                        Err(_) => {} //info!("NMEA Parse Error: {:?}", e),
                    }
                } else {
                    let mut temp_buf = [0u8; 1024];
                    buf_count = 0;
                    let mut i = item.iter().peekable();
                    while i.peek() != Some(&&0x00) {
                        temp_buf[buf_count] = *i.next().unwrap();
                        buf_count += 1;
                    }
                    buf = temp_buf;
                    break;
                };
            }
        }
    }
}
