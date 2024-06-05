#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::mem::MaybeUninit;

use esp_hal::embassy::executor::Executor;
use esp_hal::prelude::*;
use static_cell::make_static;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

/// Initialize the heap
///
/// # Safety
///
/// Must be called exactly once, before any allocations
pub unsafe fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    // SAFETY:
    // - `init_heap` is required to be called exactly once, before any allocations
    // - `HEAP_SIZE` is > 0
    unsafe { ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE) };
}

#[entry]
fn entry() -> ! {
    // SAFETY: entry() will only run once
    unsafe { init_heap() };

    esp_println::logger::init_logger(log::LevelFilter::Info);
    log::info!("Logger initialized");

    let executor = make_static!(Executor::new());
    executor.run(vertx::main)
}
