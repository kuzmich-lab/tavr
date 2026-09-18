use defmt::info;
use embassy_time::{Duration, Ticker};

#[embassy_executor::task]
pub async fn low_prio_async() {
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        info!("Low priority ticks");
        ticker.next().await;
    }
}
