#[cfg_attr(not(all(target_arch = "xtensa")), path = "storage_hosted.rs")]
#[cfg_attr(
    all(target_arch = "xtensa", target_os = "none"),
    path = "storage_esp.rs"
)]
#[cfg_attr(
    all(target_arch = "xtensa", target_os = "espidf"),
    path = "storage_idf.rs"
)]
pub mod kv_store;
