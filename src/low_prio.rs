use defmt::info;
use embassy_time::{Duration, Ticker};

#[embassy_executor::task]
pub async fn low_prio_async() {
    info!(
        "Starting low-priority task that will not be able to run while the blocking task is running"
    );
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        info!("Low priority ticks");
        ticker.next().await;
    }
}
