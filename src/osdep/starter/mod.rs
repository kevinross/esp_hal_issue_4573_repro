#[cfg_attr(not(all(target_arch = "xtensa")), path = "starter_hosted.rs")]
#[cfg_attr(
    all(target_arch = "xtensa", target_os = "none"),
    path = "starter_esp.rs"
)]
#[cfg_attr(
    all(target_arch = "xtensa", target_os = "espidf"),
    path = "starter_idf.rs"
)]
pub mod start;
pub use start::*;
