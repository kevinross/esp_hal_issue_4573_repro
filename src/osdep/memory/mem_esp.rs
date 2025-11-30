#[allow(unused)]
use esp_backtrace as _;
use esp_backtrace::Backtrace;
use esp_println::println;

mod memory_internal {

    use core::alloc::{AllocError, Allocator, GlobalAlloc, Layout};
    use core::ptr::NonNull;
    use core::sync::atomic;
    use core::sync::atomic::AtomicBool;

    struct GlobalTracingAlloc(AtomicBool);
    #[derive(Debug)]
    pub struct PSRAMTracingAlloc(AtomicBool);
    // #[global_allocator]
    static ALLOCATOR: GlobalTracingAlloc = GlobalTracingAlloc(AtomicBool::new(false));
    impl GlobalTracingAlloc {
        fn start_tracing(&self) {
            self.0.store(true, atomic::Ordering::SeqCst);
        }
        fn stop_tracing(&self) {
            self.0.store(false, atomic::Ordering::SeqCst);
        }
    }
    impl PSRAMTracingAlloc {
        fn start_tracing(&self) {
            self.0.store(true, atomic::Ordering::SeqCst);
        }
        fn stop_tracing(&self) {
            self.0.store(false, atomic::Ordering::SeqCst);
        }
    }
    unsafe impl GlobalAlloc for GlobalTracingAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            unsafe { GlobalAlloc::alloc(&esp_alloc::HEAP, layout) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe {
                GlobalAlloc::dealloc(&esp_alloc::HEAP, ptr, layout);
            }
        }
    }
    unsafe impl Allocator for GlobalTracingAlloc {
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            Allocator::allocate(&esp_alloc::HEAP, layout)
        }
        unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
            unsafe {
                Allocator::deallocate(&esp_alloc::HEAP, ptr, layout);
            }
        }
    }
    impl Clone for PSRAMTracingAlloc {
        fn clone(&self) -> Self {
            Self(AtomicBool::new(self.0.load(atomic::Ordering::SeqCst)))
        }
    }
    impl Default for PSRAMTracingAlloc {
        fn default() -> Self {
            PSRAM_ALLOCATOR.clone()
        }
    }
    impl Default for &'static EspHeap {
        fn default() -> Self {
            &PSRAM_ALLOCATOR
        }
    }
    unsafe impl Allocator for PSRAMTracingAlloc {
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            cfg_if::cfg_if! {
                if #[cfg(feature = "unified_memory")] {
                    esp_alloc::HEAP.allocate(layout)
                } else {
                    Allocator::allocate(&PSRAM_ALLOCATOR_INNER, layout)
                }
            }
        }

        unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
            unsafe {
                cfg_if::cfg_if! {
                    if #[cfg(feature = "unified_memory")] {
                        esp_alloc::HEAP.deallocate(ptr, layout)
                    } else {
                        Allocator::deallocate(&PSRAM_ALLOCATOR_INNER, ptr, layout)
                    }
                }
            }
        }
    }
    pub type EspHeapInner = esp_alloc::EspHeap;
    pub type EspHeap = PSRAMTracingAlloc;
    pub static PSRAM_ALLOCATOR_INNER: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();
    pub static PSRAM_ALLOCATOR: EspHeap = PSRAMTracingAlloc(AtomicBool::new(false));
    pub struct PsramPeriph(pub esp_hal::peripherals::PSRAM<'static>);

    pub fn init_psram_heap(psram: PsramPeriph) -> PsramPeriph {
        let (start, size) = esp_hal::psram::psram_raw_parts(&psram.0);
        unsafe {
            cfg_if::cfg_if! {
                if #[cfg(feature = "unified_memory")] {
                    esp_alloc::HEAP.add_region(esp_alloc::HeapRegion::new(
                        start,
                        size,
                        esp_alloc::MemoryCapability::External.into(),
                    ));
                } else {
                    PSRAM_ALLOCATOR_INNER.add_region(esp_alloc::HeapRegion::new(
                        start,
                        size,
                        esp_alloc::MemoryCapability::External.into(),
                    ));
                }
            }
        }
        psram
    }
    pub fn dump_mem_stats(comment: &str) {
        let stats: esp_alloc::HeapStats = esp_alloc::HEAP.stats();
        log::info!("{comment}: \n{}", stats);
        let stats = PSRAM_ALLOCATOR_INNER.stats();
        log::info!("{comment} PSRAM: \n{}", stats);
    }

    pub fn start_tracing() {
        ALLOCATOR.start_tracing();
    }
    pub fn stop_tracing() {
        ALLOCATOR.stop_tracing();
    }
    pub fn start_other_tracing() {
        PSRAM_ALLOCATOR.start_tracing();
    }
    pub fn stop_other_tracing() {
        PSRAM_ALLOCATOR.stop_tracing();
    }
}
pub use memory_internal::*;
pub fn dump_backtrace(comment: &str) {
    log::info!("Backtrace: {}\n", comment);
    let backtrace = Backtrace::capture();
    for frame in backtrace.frames() {
        println!("0x{:x}", frame.program_counter());
    }
}
