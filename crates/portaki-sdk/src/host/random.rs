//! `host::random` — unpredictable bytes from the host's cryptographic generator.
//!
//! The Wasm sandbox has no entropy of its own: [`bytes`] asks the host (`random.bytes`, an
//! ambient op — no permission). Use it for anything a guest must not guess: an access code, a
//! token. `uuid::Uuid::new_v4()` draws from the same source through
//! [`wasm_getrandom`](crate::host::wasm_getrandom).
//!
//! # No silent fallback
//!
//! On a runtime without `random.bytes`, [`bytes`] and [`u32_below`] **fail** — they never degrade
//! to a pseudo-random sequence, which would make a secret predictable without a signal. Only
//! UUIDs degrade (an id must exist; it need not be secret).
//!
//! ```no_run
//! use portaki_sdk::host::random;
//!
//! # fn main() -> portaki_sdk::error::Result<()> {
//! let keypad = format!("{:06}", random::u32_below(1_000_000)?);
//! assert_eq!(keypad.len(), 6);
//! # Ok(())
//! # }
//! ```

use std::cell::RefCell;
use std::sync::Arc;

use crate::error::{PortakiError, Result};
use crate::host::runtime::{backend, HostBackend};

/// What the host serves per call. Drawn whole and spent locally: a UUID costs 16 bytes, not a
/// host call — the invocation budget counts calls.
pub const HOST_CHUNK_BYTES: usize = 256;

thread_local! {
    static POOL: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// `n` random bytes from the host. Fails when the host has no `random.bytes`.
pub fn bytes(n: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0; n];
    fill(&mut buf)?;
    Ok(buf)
}

/// A uniform value in `0..n` (rejection sampling, no modulo bias). `n` must be at least 1.
pub fn u32_below(n: u32) -> Result<u32> {
    if n == 0 {
        return Err(PortakiError::Host("random_u32_below_zero".into()));
    }
    // The largest multiple of `n` that fits in 2^32: draws at or above it would bias low values.
    let zone = (1u64 << 32) - (1u64 << 32) % u64::from(n);
    loop {
        let mut word = [0; 4];
        fill(&mut word)?;
        let x = u32::from_le_bytes(word);
        if u64::from(x) < zone {
            return Ok(x % n);
        }
    }
}

/// Fills `buf` from the host. Fails when the host has no `random.bytes`.
pub fn fill(buf: &mut [u8]) -> Result<()> {
    let mut filled = 0;
    while filled < buf.len() {
        let taken = POOL.with(|pool| {
            let mut pool = pool.borrow_mut();
            let take = (buf.len() - filled).min(pool.len());
            let start = pool.len() - take;
            buf[filled..filled + take].copy_from_slice(&pool[start..]);
            pool.truncate(start);
            take
        });
        filled += taken;
        if filled < buf.len() {
            // Outside the borrow: the host call must not find the pool held.
            let chunk = host()?.random_bytes(HOST_CHUNK_BYTES)?;
            if chunk.is_empty() {
                return Err(PortakiError::Host("random_bytes_empty".into()));
            }
            POOL.with(|pool| *pool.borrow_mut() = chunk);
        }
    }
    Ok(())
}

/// The installed backend — or, in Wasm before `with_host` (the invocation id), the gateway.
fn host() -> Result<Arc<dyn HostBackend>> {
    #[cfg(target_arch = "wasm32")]
    {
        Ok(backend().unwrap_or_else(|_| Arc::new(crate::wasm::extism_host::ExtismHostBackend)))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        backend()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use crate::host::runtime::with_host;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Counts its calls; each chunk differs from the last.
    #[derive(Default)]
    struct Entropy(AtomicU64);

    impl HostBackend for Entropy {
        fn context(&self) -> Result<Context> {
            Ok(Context::default())
        }
        fn kv_get(&self, _: &str) -> Result<Option<Vec<u8>>> {
            Ok(None)
        }
        fn kv_set(&self, _: &str, _: &[u8], _: Option<u32>) -> Result<()> {
            Ok(())
        }
        fn kv_delete(&self, _: &str) -> Result<()> {
            Ok(())
        }
        fn kv_list(&self, _: &str) -> Result<Vec<String>> {
            Ok(vec![])
        }
        fn i18n_translate(&self, key: &str, _: &str) -> Result<String> {
            Ok(key.into())
        }
        fn log(&self, _: &str, _: &str, _: &str) -> Result<()> {
            Ok(())
        }
        fn connector_call(&self, _: &str, _: &str, _: &str) -> Result<String> {
            Err(PortakiError::HostNotConfigured)
        }
        fn emit_event(&self, _: &str, _: &str) -> Result<()> {
            Ok(())
        }
        fn random_bytes(&self, len: usize) -> Result<Vec<u8>> {
            let call = self.0.fetch_add(1, Ordering::Relaxed);
            Ok((0..len)
                .map(|i| (call.wrapping_mul(131) as usize ^ i.wrapping_mul(7)) as u8)
                .collect())
        }
    }

    /// Each test on its own thread: the pool is per thread, and must start empty.
    fn fresh<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
        std::thread::spawn(f).join().unwrap()
    }

    #[test]
    fn two_fills_differ_and_share_one_host_call() {
        fresh(|| {
            let host = Arc::new(Entropy::default());
            with_host(host.clone(), Context::default(), || {
                let (a, b) = (bytes(16).unwrap(), bytes(16).unwrap());
                assert_ne!(a, b);
                assert_eq!(host.0.load(Ordering::Relaxed), 1);
                // A draw larger than the pool spans several host calls.
                assert_eq!(bytes(600).unwrap().len(), 600);
            });
        });
    }

    #[test]
    fn without_the_host_op_a_secret_fails_instead_of_degrading() {
        fresh(|| {
            assert!(bytes(4).is_err());
            assert!(u32_below(1_000_000).is_err());
        });
    }

    #[test]
    fn below_stays_in_range() {
        fresh(|| {
            with_host(Arc::new(Entropy::default()), Context::default(), || {
                for _ in 0..500 {
                    assert!(u32_below(1_000_000).unwrap() < 1_000_000);
                }
                assert_eq!(u32_below(1).unwrap(), 0);
                assert!(u32_below(0).is_err());
            });
        });
    }
}
