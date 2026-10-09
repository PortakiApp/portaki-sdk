//! Wasm32 randomness for `uuid` and other deps without browser `wasm-bindgen` imports.
//!
//! Extism loads modules as `wasm32-unknown-unknown`; the `uuid` `js` feature (or
//! getrandom's `wasm_js` backend) emits `__wbindgen_*` imports the runtime does not provide.
//! Bytes come from the host ([`crate::host::random`]); a runtime without `random.bytes` falls
//! back to a fixed-seed xorshift — ids that may repeat across instances, never secrets.

#[cfg(target_arch = "wasm32")]
use core::sync::atomic::{AtomicU64, Ordering};

pub use getrandom;

#[cfg(target_arch = "wasm32")]
static STATE: AtomicU64 = AtomicU64::new(0x853c49e6748fea9b_u64);

/// Fills `buf` from the host's generator; on an old runtime, with pseudo-random bytes (Wasm) or
/// `UNSUPPORTED` (native, where getrandom has the OS and never calls this).
pub fn fill(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    if crate::host::random::fill(buf).is_ok() {
        return Ok(());
    }
    fallback(buf)
}

#[cfg(not(target_arch = "wasm32"))]
fn fallback(_buf: &mut [u8]) -> Result<(), getrandom::Error> {
    Err(getrandom::Error::UNSUPPORTED)
}

// ponytail: fixed seed — only for runtimes predating `random.bytes`; delete once none is left.
#[cfg(target_arch = "wasm32")]
fn fallback(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    for chunk in buf.chunks_mut(8) {
        chunk.copy_from_slice(&next_u64().to_le_bytes()[..chunk.len()]);
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn next_u64() -> u64 {
    let mut x = STATE.load(Ordering::Relaxed);
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    let result = x.wrapping_mul(0x2545_f491_4f6c_dd1d);
    STATE.store(result, Ordering::Relaxed);
    result
}
