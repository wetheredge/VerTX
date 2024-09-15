#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

use core::mem::MaybeUninit;

#[cfg(feature = "chip-rp")]
use embassy_executor::main;
use embassy_executor::Spawner;
#[cfg(feature = "chip-esp")]
use esp_hal::prelude::main;

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

#[main]
async fn main(spawner: Spawner) {
    // SAFETY: main() will only run once
    unsafe { init_heap() };

    vertx::main(spawner).await;
}
