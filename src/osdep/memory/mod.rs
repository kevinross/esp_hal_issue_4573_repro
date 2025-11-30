#[cfg_attr(not(all(target_arch = "xtensa")), path = "mem_hosted.rs")]
#[cfg_attr(all(target_arch = "xtensa", target_os = "none"), path = "mem_esp.rs")]
#[cfg_attr(all(target_arch = "xtensa", target_os = "espidf"), path = "mem_idf.rs")]
pub mod mem;
