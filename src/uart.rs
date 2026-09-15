use defmt::{info, warn};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use esp_hal::Async;
use esp_hal::uart::{UartRx, UartTx};

const READ_BUF_SIZE: usize = 64;

#[embassy_executor::task]
pub async fn uart_writer(
    mut tx: UartTx<'static, Async>,
    signal: &'static Signal<NoopRawMutex, usize>,
) {
    use core::fmt::Write;
    embedded_io_async::Write::write(
        &mut tx,
        b"Hello async serial. Enter something ended with EOT (CTRL-D).\r\n",
    )
    .await
    .unwrap();
    warn!("Hello async serial. Enter something ended with EOT (CTRL-D).\r\n");
    embedded_io_async::Write::flush(&mut tx).await.unwrap();
    loop {
        let bytes_read = signal.wait().await;
        signal.reset();
        write!(&mut tx, "\r\n-- received {} bytes --\r\n", bytes_read).unwrap();
        embedded_io_async::Write::flush(&mut tx).await.unwrap();
    }
}

#[embassy_executor::task]
pub async fn uart_reader(
    mut rx: UartRx<'static, Async>,
    signal: &'static Signal<NoopRawMutex, usize>,
) {
    const MAX_BUFFER_SIZE: usize = 10 * READ_BUF_SIZE + 16;

    let mut rbuf: [u8; MAX_BUFFER_SIZE] = [0u8; MAX_BUFFER_SIZE];
    let mut offset = 0;
    loop {
        let r = embedded_io_async::Read::read(&mut rx, &mut rbuf[offset..]).await;
        match r {
            Ok(len) => {
                offset += len;
                info!("Read: {:?}, data: {:?}", len, &rbuf[..offset]);
                offset = 0;
                signal.signal(len);
            }
            Err(e) => info!("RX Error: {:?}", e),
        }
    }
}
