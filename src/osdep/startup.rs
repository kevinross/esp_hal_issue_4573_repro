use crate::osdep::net::Executor;
use crate::osdep::starter::{boot, startup};
use crate::osdep::statics::{ALT_EXECUTOR, EXECUTOR};
use crate::osdep::typedefs::{GlobalStatics, InitFunc, SpawnerStatics};
use core::sync::atomic::Ordering;
use embassy_executor::task;

#[task]
pub async fn startup_wrapper(init: InitFunc, statics: GlobalStatics, sys: SpawnerStatics) {
    log::info!("startup_wrapper startup");
    loop {
        if statics.booted.load(Ordering::SeqCst) {
            break;
        }
        crate::osdep::time::delay_ns_async(core::time::Duration::from_millis(100)).await;
    }
    log::info!("startup_wrapper booted");
    init(statics, sys);
}

pub fn second_core_fn(sys: SpawnerStatics) {
    let executor = ALT_EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        log::info!("second core started");
        sys.core1_spawner.lock(|x| x.replace(Some(spawner)));
    });
}
pub fn startup_fn(wifi_name: &'static str, password: &'static str, init: InitFunc) {
    let (sys, statics) = startup(wifi_name, password);
    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        let _ = spawner.spawn(boot(spawner, sys, statics, wifi_name, password, init));
    });
}
