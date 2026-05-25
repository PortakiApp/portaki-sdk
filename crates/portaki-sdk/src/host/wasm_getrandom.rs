//! Wasm32 randomness for `uuid` and other deps without browser `wasm-bindgen` imports.
//!
//! Extism loads modules as `wasm32-unknown-unknown`; the `uuid` `js` feature (or
//! getrandom's `wasm_js` backend) emits `__wbindgen_*` imports the runtime does not provide.

#[cfg(target_arch = "wasm32")]
use core::sync::atomic::{AtomicU64, Ordering};

pub use getrandom;

#[cfg(target_arch = "wasm32")]
static STATE: AtomicU64 = AtomicU64::new(0x853c49e6748fea9b_u64);

/// Fills `buf` with pseudo-random bytes for module-local IDs (not cryptographic).
#[cfg(target_arch = "wasm32")]
pub fn fill(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    let mut offset = 0usize;
    while offset + 8 <= buf.len() {
        buf[offset..offset + 8].copy_from_slice(&next_u64().to_le_bytes());
        offset += 8;
    }
    if offset < buf.len() {
        let tail = next_u64().to_le_bytes();
        let remaining = buf.len() - offset;
        buf[offset..].copy_from_slice(&tail[..remaining]);
    }
    Ok(())
}

/// No-op on non-wasm targets (modules are built for wasm32 in production).
#[cfg(not(target_arch = "wasm32"))]
pub fn fill(_buf: &mut [u8]) -> Result<(), getrandom::Error> {
    Err(getrandom::Error::UNSUPPORTED)
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
