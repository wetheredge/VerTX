#![no_std]
#![allow(internal_features)]
#![feature(core_intrinsics)]

use core::ptr;
use core::sync::atomic::{self, AtomicBool};

#[allow(unused)]
mod current {
    include!("../../vertx-config/out/current.rs");
}

#[allow(unused)]
mod old {
    include!("../../vertx-config/out/old.rs");
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::intrinsics::abort()
}

const LEN: usize = if current::BYTE_LENGTH > old::BYTE_LENGTH {
    current::BYTE_LENGTH
} else {
    old::BYTE_LENGTH
};

fn data() -> &'static mut [u8; LEN] {
    static TAKEN: AtomicBool = AtomicBool::new(false);
    // Ensure this can only run once
    assert!(!TAKEN.swap(true, atomic::Ordering::AcqRel));

    #[no_mangle]
    static mut DATA: [u8; LEN] = [0; LEN];

    // SAFETY: This is safe to dereference because DATA is local to this function
    // and the !TAKEN check ensures this function only runs once. It is available to
    // JavaScript, but is still sound when used with a regular WebAssembly memory
    // since JS and wasm are single-threaded. Shared WebAssembly memories could be
    // used to introduce soundness bugs, but that is not intended use of this module
    // and (afaik) there is no way to disallow them.
    unsafe { &mut *ptr::addr_of_mut!(DATA) }
}

#[cfg(feature = "up")]
#[no_mangle]
extern "C" fn run() -> usize {
    let data = data();

    let old = old::RawConfig::deserialize(data).map_err(|_| ()).unwrap();
    let current = current::RawConfig {
        name: old.name,
        ..Default::default()
    };

    current.serialize(data).map_err(|_| ()).unwrap()
}

#[cfg(feature = "down")]
#[no_mangle]
extern "C" fn run() -> usize {
    let data = data();

    let current = current::RawConfig::deserialize(data)
        .map_err(|_| ())
        .unwrap();
    let old = old::RawConfig { name: current.name };

    old.serialize(data).map_err(|_| ()).unwrap()
}
