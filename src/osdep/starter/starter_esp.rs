use crate::LOG_LEVEL;
use crate::osdep::network::net::*;
use crate::osdep::startup::*;
use crate::osdep::statics::{NetworkStatics, SystemStatics, TLS};
use crate::osdep::time::delay_ns_async;
use crate::osdep::typedefs::{GlobalStatics, InitFunc, Mutex, SpawnerStatics, Statics};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::cell::RefCell;
use core::net::Ipv4Addr;
use core::ptr::addr_of_mut;
use core::str::FromStr;
use core::sync::atomic::{AtomicBool, Ordering};
use edge_nal_embassy::TcpBuffers;
use embassy_executor::{Spawner, task};
use embassy_net::{Ipv4Cidr, Runner, Stack, StackResources, StaticConfigV4};
use embassy_sync::blocking_mutex::CriticalSectionMutex;
use esp_hal::interrupt::software::{SoftwareInterrupt, SoftwareInterruptControl};
use esp_hal::psram::PsramSize;
use esp_hal::psram::SpiTimingConfigCoreClock::SpiTimingConfigCoreClock240m;
use esp_hal::system::Cpu;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};
use esp_mbedtls::{Certificates, Tls};
use esp_println::println;
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{AuthMethod, CountryInfo};
use esp_radio::wifi::{
    ModeConfig, ScanConfig, WifiController, WifiDevice, WifiEvent, WifiStationState,
};
use log::Record;

esp_bootloader_esp_idf::esp_app_desc!();
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}
#[unsafe(link_section = ".dram2_uninit")]
static mut APP_CORE_STACK: esp_hal::system::Stack<{ 16384 - 6640 }> = esp_hal::system::Stack::new();

pub fn boot_second_thread<'a>(
    cpu_control: esp_hal::peripherals::CPU_CTRL,
    int1: SoftwareInterrupt<'static, 1>,
    sys: SpawnerStatics,
) {
    esp_rtos::start_second_core(
        cpu_control,
        int1,
        unsafe { &mut *addr_of_mut!(APP_CORE_STACK) },
        || {
            second_core_fn(sys);
        },
    );
}

struct FilteredEspLogger;
impl log::Log for FilteredEspLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if let Some(path) = record.module_path() {
            if !path.starts_with("xapi") && !path.contains("psram") {
                return;
            }
            if path.contains("esp_radio::wifi::os_adapter") {
                if false {
                    if let Some(args) = Some(record.args()) {
                        let path = format!("{}", args);
                        if path.contains("wifi_int_disable")
                            || path.contains("wifi_int_restore")
                            || path.contains("coex")
                        {
                            return;
                        }
                    }
                } else {
                    return;
                }
            }
        }
        let core_num = Cpu::current();
        println!(
            "cpu={core_num:?} {}: {} - {}",
            record.module_path().or(Some("root")).unwrap(),
            record.level(),
            record.args()
        );
    }

    fn flush(&self) {}
}
fn init_logger(level: log::LevelFilter) {
    unsafe {
        log::set_logger_racy(&FilteredEspLogger).unwrap();
        log::set_max_level_racy(level);
    }
}

pub fn startup(
    _wifi_name: &'static str,
    _password: &'static str,
) -> (SpawnerStatics, GlobalStatics) {
    init_logger(LOG_LEVEL);
    let config = esp_hal::Config::default()
        .with_cpu_clock(CpuClock::max())
        .with_psram(esp_hal::psram::PsramConfig {
            size: PsramSize::Size(2097152),
            core_clock: Some(SpiTimingConfigCoreClock240m),
            ..Default::default()
        });
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 96 * 1024);
    esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: 64000);
    let psram = crate::osdep::mem::PsramPeriph(peripherals.PSRAM);
    let psram = crate::osdep::mem::init_psram_heap(psram);

    let interrupt_control = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    let timg0 = TimerGroup::new(peripherals.TIMG0);

    esp_rtos::start(timg0.timer0, interrupt_control.software_interrupt0);

    let wifi_config =
        esp_radio::wifi::Config::default().with_country_code(CountryInfo::from(*b"CA"));
    let (controller, interfaces) = esp_radio::wifi::new(peripherals.WIFI, wifi_config).unwrap();
    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let systimer = SystemTimer::new(peripherals.SYSTIMER);

    let config = embassy_net::Config::dhcpv4(Default::default());

    // Init network stack
    let (stack, runner) = embassy_net::new(
        interfaces.station,
        config,
        mk_static!(
            StackResources<{ TOTAL_CONNECTIONS }>,
            StackResources::<TOTAL_CONNECTIONS>::new()
        ),
        seed,
    );

    let tcp_buffers = mk_static!(TcpBuffs, TcpBuffers::new());
    let tls = TLS.get_or_init(|| {
        Tls::new(peripherals.SHA)
            .unwrap()
            .with_hardware_rsa(peripherals.RSA)
    });
    let certs = Certificates::new();
    let alt_buffs = mk_static!(TcpBuffs, TcpBuffers::new());
    let sys = Arc::new(SystemStatics {
        core0_spawner: CriticalSectionMutex::new(RefCell::new(None)),
        core1_spawner: CriticalSectionMutex::new(RefCell::new(None)),
    });
    boot_second_thread(
        peripherals.CPU_CTRL,
        interrupt_control.software_interrupt1,
        sys.clone(),
    );
    println!("about to return statics");
    (
        sys,
        Arc::new(Statics {
            booted: AtomicBool::new(false),
            net: Some(stack),
            net_controller: Some(Arc::new(Mutex::new(controller))),
            net_runner: Some(Arc::new(Mutex::new(runner))),
            core0_net: NetworkStatics {
                stack: TcpStack::new(stack.clone(), tcp_buffers),
                dns: DnsStack::new(stack.clone()),
                tls: tls.reference(),
                certs,
            },
            core1_net: NetworkStatics {
                stack: TcpStack::new(stack.clone(), alt_buffs),
                dns: DnsStack::new(stack.clone()),
                tls: tls.reference(),
                certs,
            },
        }),
    )
}

#[task]
pub(crate) async fn boot(
    spawner: Spawner,
    sys: SpawnerStatics,
    statics_ref: GlobalStatics,
    wifi_name: &'static str,
    password: &'static str,
    init: InitFunc,
) {
    let net = statics_ref.net.clone();
    let controller = &statics_ref.net_controller.clone();
    let runner = &statics_ref.net_runner.clone();
    sys.core0_spawner.lock(|core0_spawner| {
        let _ = core0_spawner.replace(Some(spawner.clone()));
    });
    loop {
        if sys.core1_spawner.lock(|x| x.borrow().is_none()) {
            delay_ns_async(core::time::Duration::from_secs(1)).await;
        } else {
            break;
        }
    }
    if let Some(net) = net {
        if let Some(controller) = controller {
            if let Some(driver) = runner {
                let _ = spawner.spawn(connection(
                    controller.clone(),
                    wifi_name.to_string(),
                    password.to_string(),
                ));
                let _ = spawner.spawn(net_task(driver.clone()));
                let _ = spawner.spawn(boot_net(net, statics_ref.clone()));
                while !statics_ref.booted.load(Ordering::SeqCst) {
                    delay_ns_async(core::time::Duration::from_millis(100)).await;
                }
                log::info!("setting up clients");
                println!("doing wrapper init");
                spawner
                    .spawn(startup_wrapper(init, statics_ref.clone(), sys.clone()))
                    .unwrap();
            }
        }
    }
}

#[task]
pub(crate) async fn connection(
    controller: Arc<Mutex<WifiController<'static>>>,
    wifi_name: String,
    password: String,
) {
    println!("start connection task");
    let mut controller = controller.write().await;

    println!("Device capabilities: {:?}", controller.capabilities());
    loop {
        match esp_radio::wifi::station_state() {
            WifiStationState::Connected => {
                // wait until we're no longer connected
                controller
                    .wait_for_event(WifiEvent::StationDisconnected)
                    .await;
                delay_ns_async(core::time::Duration::from_millis(5000)).await;
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = ModeConfig::Station(
                StationConfig::default()
                    .with_auth_method(AuthMethod::WpaWpa2Personal)
                    .with_ssid(wifi_name.clone())
                    .with_password(password.clone()),
            );
            controller.set_config(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");

            println!("Scan");
            let result = controller
                .scan_with_config_async(ScanConfig::default().with_max(10))
                .await
                .unwrap();
            for entry in result {
                log::info!("{:?}", entry);
            }
        }
        println!("About to connect...");
        match controller.connect_async().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                delay_ns_async(core::time::Duration::from_millis(5000)).await;
                println!("Restarting wifi...");
                controller.stop_async().await.unwrap();
                delay_ns_async(core::time::Duration::from_millis(5000)).await;
            }
        }
    }
}

#[task]
pub(crate) async fn boot_net(net: Stack<'static>, statics: GlobalStatics) {
    while !net.is_link_up() {
        delay_ns_async(core::time::Duration::from_millis(100)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = net.config_v4() {
            println!("network configuration: {:?}", config.address);
            break;
        }
        delay_ns_async(core::time::Duration::from_millis(100)).await;
    }
    println!("network configured");
    statics.booted.store(true, Ordering::SeqCst);
}
#[task]
pub(crate) async fn net_task(runner: Arc<Mutex<Runner<'static, WifiDevice<'static>>>>) {
    println!("starting network task");
    let mut runner = runner.write().await;
    runner.run().await
}
