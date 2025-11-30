#[cfg_attr(not(all(target_arch = "xtensa")), path = "time_hosted.rs")]
#[cfg_attr(all(target_arch = "xtensa", target_os = "none"), path = "time_esp.rs")]
#[cfg_attr(
    all(target_arch = "xtensa", target_os = "espidf"),
    path = "time_idf.rs"
)]
pub mod time;
pub use time::*;
