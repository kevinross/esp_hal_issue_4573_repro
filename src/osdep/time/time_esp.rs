use core::time::Duration;
use embassy_time::Timer;

pub fn epoch_ns() -> u64 {
    esp_hal::time::Instant::EPOCH.elapsed().as_micros()
}
pub async fn delay_ns_async(us: Duration) {
    Timer::after(embassy_time::Duration::from_nanos(us.as_nanos() as u64)).await;
}
