#![no_std]
//! Trivial `no_std` smoke crate for TOOL-04.
//!
//! It has the shape of every crate the tool emits: no heap, no dependencies,
//! `panic = "abort"`, staticlib plus rlib. Building it for
//! `thumbv7em-none-eabihf` proves the pinned toolchain and target work; the
//! C caller in `smoke.c` proves the exported symbol links from C.

mod ffi;

/// Adds two values with explicit wrapping arithmetic.
///
/// Unsigned C arithmetic wraps, so the Rust side says so out loud.
pub fn add(a: u32, b: u32) -> u32 {
    a.wrapping_add(b)
}

/// Halts the core on panic.
///
/// A firmware image can hold only one panic handler, so this sits behind the
/// default-on `panic-handler` feature and is switched off once when several
/// modules are linked into one image.
#[cfg(feature = "panic-handler")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
