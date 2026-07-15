//! Host function wrappers — the module's only supported API to platform services.
//!
//! Each submodule maps to a **host import family** implemented by the Java gateway
//! (production) or [`runtime::HostBackend`] mocks (tests, `portaki dev`). Calls are
//! synchronous from the module's perspective and return [`crate::error::Result`].
//!
//! ## Contract
//!
//! | Area | Module may | Module must not |
//! |------|------------|-----------------|
//! | [`kv`] | Store small opaque blobs scoped to property + module | Store secrets (keys are linted) |
//! | [`repo`] | CRUD module-owned entities via typed builders | Issue SQL or access other modules' tables |
//! | [`connectors`] | Invoke declared connector operations | Open raw HTTP clients |
//! | [`credentials`] | Obtain opaque handles for connector egress | Read cleartext tokens in Wasm |
//! | [`capabilities`] | Probe grants (hint); prefer `Context::has_capability` | Assume optional capabilities are present |
//! | [`module`] | Read install/config readiness from orchestrator | Mutate enablement or config |
//!
//! ## Thread-local runtime
//!
//! [`runtime::with_host`] installs a [`runtime::HostBackend`] and [`crate::Context`]
//! for the current thread. Production Wasm sets this inside the Extism dispatch shim
//! before handler code runs.
//!
//! # Examples
//!
//! ```no_run
//! use portaki_sdk::prelude::*;
//! use std::sync::Arc;
//!
//! // In tests — swap the gateway for a mock backend:
//! # struct Mock;
//! # impl portaki_sdk::host::runtime::HostBackend for Mock {
//! #     fn context(&self) -> portaki_sdk::Result<portaki_sdk::Context> { Ok(Context::default()) }
//! #     fn has_capability(&self, _: &str) -> portaki_sdk::Result<bool> { Ok(true) }
//! #     fn kv_get(&self, _: &str) -> portaki_sdk::Result<Option<Vec<u8>>> { Ok(None) }
//! #     fn kv_set(&self, _: &str, _: &[u8], _: Option<u32>) -> portaki_sdk::Result<()> { Ok(()) }
//! #     fn kv_delete(&self, _: &str) -> portaki_sdk::Result<()> { Ok(()) }
//! #     fn kv_list(&self, _: &str) -> portaki_sdk::Result<Vec<String>> { Ok(vec![]) }
//! #     fn i18n_translate(&self, key: &str, _: &str) -> portaki_sdk::Result<String> { Ok(key.into()) }
//! #     fn log(&self, _: &str, _: &str, _: &str) -> portaki_sdk::Result<()> { Ok(()) }
//! #     fn connector_call(&self, _: &str, _: &str, _: &str) -> portaki_sdk::Result<String> { Ok("{}".into()) }
//! #     fn emit_event(&self, _: &str, _: &str) -> portaki_sdk::Result<()> { Ok(()) }
//! # }
//! portaki_sdk::host::runtime::with_host(Arc::new(Mock), Context::default(), || {
//!     host::kv::set("cache.version", b"v1", None).unwrap();
//! });
//! ```

pub mod capabilities;
pub mod connectors;
pub mod credentials;
pub mod events;
pub mod geo;
pub mod i18n;
pub mod images;
pub mod kv;
pub mod log;
pub mod module;
pub mod notifications;
pub mod repo;
pub mod runtime;
pub mod time;
pub mod wasm_getrandom;

pub use runtime::{with_host, HostBackend};
