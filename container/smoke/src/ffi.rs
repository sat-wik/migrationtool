//! The C boundary: the only module in this crate allowed to contain unsafe code.
#![allow(unsafe_code)]

// SAFETY: the exported symbol name `mt_smoke_add` is unique to this crate, so
// the unmangled export cannot collide with another definition at link time.
/// C ABI wrapper for [`crate::add`].
///
/// C: `uint32_t mt_smoke_add(uint32_t a, uint32_t b)` in `smoke.c`.
#[no_mangle]
pub extern "C" fn mt_smoke_add(a: u32, b: u32) -> u32 {
    crate::add(a, b)
}
