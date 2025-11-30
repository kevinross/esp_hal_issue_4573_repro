use crate::osdep::mem::dump_mem_stats;
use embassy_executor::task;
use embassy_time::{Duration, Timer};
#[cfg(all(not(target_os = "espidf")))]
pub const STACK_SIZE: usize = 16777216 / 4 / 4 / 4 - 65536;
#[cfg(target_os = "espidf")]
pub const STACK_SIZE: usize = 16384;
mod memory;
mod network;
mod starter;
pub mod startup;
pub mod storage;
pub mod time;

use crate::osdep::storage::kv_store::{get_key, put_key};
pub use memory::*;
pub use network::*;

pub mod typedefs {

    use crate::osdep::mem::{EspHeap, PSRAM_ALLOCATOR};
    use crate::osdep::statics::{StaticsValue, SystemStatics};
    use alloc::boxed::Box;
    use alloc::sync::Arc;
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use embassy_sync::rwlock::RwLock as EmbassyMutex;
    use std::prelude::v1::*;

    #[cfg(all(target_arch = "xtensa", target_os = "none"))]
    pub type Str = string_alloc::String<&'static EspHeap>;
    #[cfg(not(any(target_os = "none", feature = "external_strings")))]
    pub type Str = alloc::string::String;
    #[cfg(all(not(target_os = "none"), feature = "external_strings"))]
    pub type Str = string_alloc::String;

    #[cfg(all(target_arch = "xtensa", target_os = "none"))]
    pub fn from_str_in<I: ToString>(from: I) -> Str {
        Str::from_str_in(from.to_string().as_ref(), &PSRAM_ALLOCATOR)
    }
    #[cfg(all(target_arch = "xtensa", target_os = "none"))]
    pub fn new_str() -> Str {
        Str::new_in(&PSRAM_ALLOCATOR)
    }
    #[cfg(not(any(target_os = "none", feature = "external_strings")))]
    pub fn from_str_in<I: ToString>(from: I) -> Str {
        Str::from_iter(from.to_string().chars())
    }
    #[cfg(not(any(target_os = "none", feature = "external_strings")))]
    pub fn new_str() -> Str {
        Str::new()
    }

    #[cfg(all(not(target_os = "none"), feature = "external_strings"))]
    pub fn from_str_in<I: ToString>(from: I) -> Str {
        Str::from(from.to_string())
    }
    #[cfg(all(not(target_os = "none"), feature = "external_strings"))]
    pub fn new_str() -> Str {
        Str::from("")
    }
    pub type Channel<T> = whisk::Channel<T>;
    pub type Mutex<T> = EmbassyMutex<CriticalSectionRawMutex, T>;

    #[cfg(all(target_arch = "xtensa", target_os = "none"))]
    pub type Statics<'a> = StaticsValue<'a, esp_radio::wifi::WifiDevice<'static>>;
    #[cfg(not(all(target_os = "none")))]
    pub type Statics<'a> = StaticsValue<'a>;
    pub type GlobalStatics = Arc<Statics<'static>>;
    pub type SpawnerStatics = Arc<SystemStatics>;
    pub type InitFunc = Box<dyn FnOnce(GlobalStatics, SpawnerStatics)>;
}
pub mod statics {
    use crate::osdep::net::{DnsStack, Executor, TcpStack};
    use crate::osdep::typedefs::Mutex;
    use alloc::sync::Arc;
    use core::cell::RefCell;
    use core::sync::atomic::AtomicBool;
    use embassy_executor::Spawner;
    use embassy_sync::blocking_mutex::CriticalSectionMutex;
    use embassy_sync::once_lock::OnceLock;
    use esp_mbedtls::{Certificates, Tls, TlsReference};
    use static_cell::StaticCell;

    pub struct NetworkStatics<'a> {
        pub stack: TcpStack,
        pub dns: DnsStack,
        pub tls: TlsReference<'a>,
        pub certs: Certificates<'a>,
    }

    pub struct SystemStatics {
        pub core0_spawner: CriticalSectionMutex<RefCell<Option<Spawner>>>,
        pub core1_spawner: CriticalSectionMutex<RefCell<Option<Spawner>>>,
    }

    unsafe impl Send for SystemStatics {}
    unsafe impl Sync for SystemStatics {}

    #[cfg(all(target_arch = "xtensa", target_os = "none"))]
    pub struct StaticsValue<'a, D>
    where
        D: embassy_net::driver::Driver,
    {
        pub booted: AtomicBool,
        pub net: Option<embassy_net::Stack<'a>>,
        pub net_controller: Option<Arc<Mutex<esp_radio::wifi::WifiController<'a>>>>,
        pub net_runner: Option<Arc<Mutex<embassy_net::Runner<'a, D>>>>,
        pub core0_net: NetworkStatics<'a>,
        pub core1_net: NetworkStatics<'a>,
    }
    pub static TLS: OnceLock<Tls> = OnceLock::new();

    pub static CERTS: OnceLock<Certificates> = OnceLock::new();
    pub static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    pub static ALT_EXECUTOR: StaticCell<Executor> = StaticCell::new();
}
pub async fn was_emergency_poweroff() -> bool {
    if let Some(key) = get_key("emerg").await {
        key == "1"
    } else {
        false
    }
}
pub async fn set_emergency_poweroff(status: bool) {
    put_key("emerg", if status { "1" } else { "0" }).await
}

pub fn get_emergency_poweroff() -> bool {
    false
}

#[task]
pub async fn spin_memory() {
    loop {
        dump_mem_stats("memory");
        Timer::after(Duration::from_millis(1000)).await
    }
}
