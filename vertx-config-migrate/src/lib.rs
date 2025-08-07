#![no_std]
#![expect(internal_features)]
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

type DataBuf = &'static mut [u8; LEN];

fn data() -> DataBuf {
    static TAKEN: AtomicBool = AtomicBool::new(false);
    // Ensure this can only run once
    assert!(!TAKEN.swap(true, atomic::Ordering::AcqRel));

    #[unsafe(no_mangle)]
    static mut DATA: [u8; LEN] = [0; LEN];

    // SAFETY: This is safe to dereference because DATA is local to this function
    // and the !TAKEN check ensures this function only runs once. It is available to
    // JavaScript, but is still sound when used with a regular WebAssembly memory
    // since JS and wasm are single-threaded. Shared WebAssembly memories could be
    // used to introduce soundness bugs, but that is not intended use of this module
    // and (afaik) there is no way to disallow them.
    unsafe { &mut *ptr::addr_of_mut!(DATA) }
}

fn up(data: DataBuf) -> usize {
    let old = old::RawConfig::deserialize(data).map_err(|_| ()).unwrap();
    let current = current::RawConfig {
        name: old.name,
        leds_brightness: old.leds_brightness,
        network_hostname: old.network_hostname,
        network_ap_password: old.network_password,
        network_sta_ssid: old.network_home_ssid,
        network_sta_password: old.network_home_password,
    };

    current.serialize(data).map_err(|_| ()).unwrap()
}

fn down(data: DataBuf) -> usize {
    let current = current::RawConfig::deserialize(data)
        .map_err(|_| ())
        .unwrap();
    let old = old::RawConfig {
        name: current.name,
        leds_brightness: current.leds_brightness,
        network_hostname: current.network_hostname,
        network_password: current.network_ap_password,
        network_home_ssid: current.network_sta_ssid,
        network_home_password: current.network_sta_password,
    };

    old.serialize(data).map_err(|_| ()).unwrap()
}

#[unsafe(no_mangle)]
extern "C" fn run() -> usize {
    let data = data();
    if cfg!(feature = "up") {
        up(data)
    } else if cfg!(feature = "down") {
        down(data)
    } else {
        unreachable!()
    }
}
