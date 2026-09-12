//! In-memory [`portaki_sdk::host::HostBackend`] for unit tests.
//!
//! [`MockContextBuilder`] constructs a [`portaki_sdk::context::Context`] and
//! optional stub data. [`MockContextBuilder::run`] installs [`MockHostFunctions`]
//! on the current thread so module code can call `host::kv`, `host::i18n`,
//! `host::connectors::call`, etc. without a gateway.
//!
//! # Entry points
//!
//! | Constructor | Surface | Default capabilities |
//! |-------------|---------|----------------------|
//! | [`MockContextBuilder::guest`] | `home.cards` | `core.storage` + default guest identity |
//! | [`MockContextBuilder::host`] | `main` | `core.storage`, `core.images` |
//!
//! [`MockContext`] is a type alias for [`MockContextBuilder`].
//!
//! # Connector stubbing
//!
//! Keys are `(connector_id, operation)` pairs matching
//! [`portaki_sdk::host::connectors::call`] arguments. Unregistered calls return
//! `"{}"`.
//!
//! ```
//! use portaki_test_utils::MockContext;
//!
//! MockContext::guest()
//!     .with_connector_response("open-weather", "forecast", r#"{"list":[]}"#)
//!     .run(|_ctx| { /* module under test */ });
//! ```
//!
//! A third party that says no is the other half of the contract, and most of what a
//! module's own code does — retry, cache, fall back, warn. [`MockContextBuilder::with_connector_error`]
//! makes the call fail, and [`MockHostFunctions::connector_calls`] records what was sent.
//!
//! ```
//! use portaki_test_utils::MockContext;
//!
//! let (ctx, host) = MockContext::host()
//!     .with_connector_error("nuki", "remote_unlock", "connector_egress_failed")
//!     .build();
//! portaki_sdk::host::with_host(host.clone(), ctx.clone(), || {
//!     let failed: portaki_sdk::Result<serde_json::Value> =
//!         portaki_sdk::host::connectors::call("nuki", "remote_unlock", &());
//!     assert!(failed.is_err());
//! });
//! assert_eq!(host.connector_calls().len(), 1);
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use portaki_sdk::context::{CapabilityGrant, Context};
use portaki_sdk::error::Result;
use portaki_sdk::host::{with_host, HostBackend};

use crate::fixtures::Property;

/// Fluent builder for a test [`Context`] and [`MockHostFunctions`] backend.
///
/// Clone before [`Self::run`] when the same configuration must drive multiple
/// isolated invocations (each `run` installs a fresh host scope).
#[derive(Debug, Clone, Default)]
pub struct MockContextBuilder {
    context: Context,
    translations: HashMap<String, String>,
    kv: HashMap<String, Vec<u8>>,
    connector_responses: HashMap<(String, String), String>,
    connector_errors: HashMap<(String, String), String>,
}

impl MockContextBuilder {
    /// Guest-surface defaults: `home.cards`, `core.storage`, sample guest identity.
    pub fn guest() -> Self {
        let mut context =
            Context::with_capabilities(&[portaki_sdk::capability::CapabilityId::Storage]);
        context.surface = Some("home.cards".to_string());
        context.guest = Some(crate::fixtures::GuestIdentityFixture::default().into());
        Self {
            context,
            ..Default::default()
        }
    }

    /// Host-dashboard defaults: `main`, `core.storage` + `core.images`.
    pub fn host() -> Self {
        let mut context = Context::with_capabilities(&[
            portaki_sdk::capability::CapabilityId::Storage,
            portaki_sdk::capability::CapabilityId::Images,
        ]);
        context.surface = Some("main".to_string());
        Self {
            context,
            ..Default::default()
        }
    }

    /// Applies `property` to the built [`Context`] (`property_id`, `property`, locale, timezone).
    pub fn with_property(mut self, property: Property) -> Self {
        property.apply(&mut self.context);
        self
    }

    /// Replaces the context with one built from `capability_ids` via [`Context::with_capabilities`].
    ///
    /// Preserves surface and guest set by [`Self::guest`] / [`Self::host`] only when
    /// those fields were not cleared by the new context.
    pub fn with_capabilities(
        mut self,
        capability_ids: &[portaki_sdk::capability::CapabilityId],
    ) -> Self {
        self.context = Context::with_capabilities(capability_ids);
        self
    }

    /// Appends raw capability ids (e.g. roadmap / newly added wire ids) without
    /// requiring a matching [`portaki_sdk::capability::CapabilityId`] variant.
    pub fn with_extra_capability_ids(mut self, ids: &[&str]) -> Self {
        for id in ids {
            self.context.capabilities.push(CapabilityGrant {
                id: (*id).to_string(),
            });
        }
        self
    }

    /// Registers a static i18n string returned by `host::i18n::translate`.
    ///
    /// Missing keys fall through to the key string itself.
    pub fn with_translation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.translations.insert(key.into(), value.into());
        self
    }

    /// Pre-seeds a KV entry visible to `host::kv::get` before the test closure runs.
    pub fn with_kv(mut self, key: impl Into<String>, value: Vec<u8>) -> Self {
        self.kv.insert(key.into(), value);
        self
    }

    /// Registers canned JSON for `host::connectors::call(connector_id, operation, _)`.
    ///
    /// `json` must be a valid JSON object string; it is returned verbatim without
    /// inspecting `args_json`.
    pub fn with_connector_response(
        mut self,
        connector_id: impl Into<String>,
        operation: impl Into<String>,
        json: impl Into<String>,
    ) -> Self {
        self.connector_responses
            .insert((connector_id.into(), operation.into()), json.into());
        self
    }

    /// Makes `host::connectors::call(connector_id, operation, _)` fail.
    ///
    /// `reason` is the gateway's own wording — `connector_credential_missing`,
    /// `connector_egress_failed`, an upstream status — and comes back as
    /// [`portaki_sdk::PortakiError::Connector`]. An error registered for a pair wins over a
    /// response registered for the same one.
    pub fn with_connector_error(
        mut self,
        connector_id: impl Into<String>,
        operation: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        self.connector_errors
            .insert((connector_id.into(), operation.into()), reason.into());
        self
    }

    /// Returns a clone of the configured [`Context`] without installing a host backend.
    pub fn context(&self) -> Context {
        self.context.clone()
    }

    /// Builds `(Context, Arc<MockHostFunctions>)` without entering `with_host`.
    ///
    /// Use when tests need direct access to the backend Arc or manual
    /// [`portaki_sdk::host::with_host`] scoping.
    pub fn build(self) -> (Context, Arc<MockHostFunctions>) {
        let host = Arc::new(MockHostFunctions {
            context: self.context.clone(),
            translations: self.translations,
            kv: Mutex::new(self.kv),
            connector_responses: self.connector_responses,
            connector_errors: self.connector_errors,
            connector_calls: Mutex::new(Vec::new()),
        });
        (self.context, host)
    }

    /// Installs the mock host on the current thread and runs `f` with the built [`Context`].
    ///
    /// Nested `run` calls replace the thread-local backend for the duration of the
    /// inner closure.
    pub fn run<R, F: FnOnce(Context) -> R>(self, f: F) -> R {
        self.run_with(|ctx, _host| f(ctx))
    }

    /// Same as [`Self::run`], with the backend handed to the closure.
    ///
    /// What a module *sent* is as much of its behaviour as what it did with the answer, and
    /// reading it back took a manual `build` + `with_host` dance before.
    pub fn run_with<R, F: FnOnce(Context, &MockHostFunctions) -> R>(self, f: F) -> R {
        let (ctx, host) = self.build();
        let backend = Arc::clone(&host);
        with_host(host, ctx.clone(), || f(ctx, backend.as_ref()))
    }
}

/// One `host::connectors::call`, as the module made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorCall {
    /// The connector the module named — `trmnl`, `open-weather`…
    pub connector_id: String,
    /// The operation on it, as `#[connector_op]` declared it.
    pub operation: String,
    /// The serialized args, path parameters and body alike.
    pub args_json: String,
}

/// Alias for [`MockContextBuilder`] — preferred name in module test code.
pub type MockContext = MockContextBuilder;

/// Thread-safe in-memory implementation of [`HostBackend`].
///
/// Created by [`MockContextBuilder::build`]. Holds stub maps for translations,
/// KV, and connector responses; echoes repo create payloads; returns benign
/// defaults for logging, events, and module status.
pub struct MockHostFunctions {
    context: Context,
    translations: HashMap<String, String>,
    kv: Mutex<HashMap<String, Vec<u8>>>,
    connector_responses: HashMap<(String, String), String>,
    connector_errors: HashMap<(String, String), String>,
    connector_calls: Mutex<Vec<ConnectorCall>>,
}

impl MockHostFunctions {
    /// Every connector call the module made, in order — failed ones included.
    pub fn connector_calls(&self) -> Vec<ConnectorCall> {
        self.connector_calls
            .lock()
            .expect("connector calls lock")
            .clone()
    }
}

impl HostBackend for MockHostFunctions {
    fn context(&self) -> Result<Context> {
        Ok(self.context.clone())
    }

    fn has_capability(&self, id: &str) -> Result<bool> {
        Ok(self.context.capabilities.iter().any(|grant| grant.id == id))
    }

    fn kv_get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.kv.lock().expect("kv lock").get(key).cloned())
    }

    fn kv_set(&self, key: &str, value: &[u8], _ttl_seconds: Option<u32>) -> Result<()> {
        self.kv
            .lock()
            .expect("kv lock")
            .insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn kv_delete(&self, key: &str) -> Result<()> {
        self.kv.lock().expect("kv lock").remove(key);
        Ok(())
    }

    fn kv_list(&self, prefix: &str) -> Result<Vec<String>> {
        Ok(self
            .kv
            .lock()
            .expect("kv lock")
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect())
    }

    fn i18n_translate(&self, key: &str, vars_json: &str) -> Result<String> {
        let mut text = self
            .translations
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string());
        if let Ok(vars) = serde_json::from_str::<HashMap<String, String>>(vars_json) {
            for (name, value) in vars {
                text = text.replace(&format!("{{{name}}}"), &value);
            }
        }
        Ok(text)
    }

    fn log(&self, _level: &str, _message: &str, _fields_json: &str) -> Result<()> {
        Ok(())
    }

    fn connector_call(
        &self,
        connector_id: &str,
        operation: &str,
        args_json: &str,
    ) -> Result<String> {
        let key = (connector_id.to_string(), operation.to_string());
        self.connector_calls
            .lock()
            .expect("connector calls lock")
            .push(ConnectorCall {
                connector_id: connector_id.to_string(),
                operation: operation.to_string(),
                args_json: args_json.to_string(),
            });

        if let Some(reason) = self.connector_errors.get(&key) {
            return Err(portaki_sdk::PortakiError::Connector(reason.clone()));
        }
        Ok(self
            .connector_responses
            .get(&key)
            .cloned()
            .unwrap_or_else(|| "{}".to_string()))
    }

    fn emit_event(&self, _event_type: &str, _payload_json: &str) -> Result<()> {
        Ok(())
    }

    fn email_send(&self, _payload_json: &str) -> Result<()> {
        Ok(())
    }

    fn notify_host(&self, _payload_json: &str) -> Result<()> {
        Ok(())
    }

    fn time_now_iso(&self) -> Result<String> {
        Ok(chrono::Utc::now().to_rfc3339())
    }

    fn repo_find(&self, _entity: &str, _query_json: &str) -> Result<String> {
        Ok(r#"{"items":[],"total":0}"#.to_string())
    }

    fn repo_create(&self, _entity: &str, entity_json: &str) -> Result<String> {
        Ok(entity_json.to_string())
    }

    fn repo_delete(&self, _entity: &str, _id: &str) -> Result<bool> {
        Ok(true)
    }

    fn module_status(&self) -> Result<portaki_sdk::host::module::ModuleStatus> {
        Ok(portaki_sdk::host::module::ModuleStatus {
            active: true,
            workspace_enabled: true,
            incomplete: false,
            requires_config: false,
            missing_required_keys: Vec::new(),
        })
    }

    fn module_list_by_capability(
        &self,
        _capability_id: &str,
    ) -> Result<Vec<portaki_sdk::host::module::ModulePeer>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A third party that refuses is most of what a module's own code is about.
    #[test]
    fn a_connector_can_be_made_to_fail() {
        let error = MockContextBuilder::host()
            .with_connector_response("trmnl", "push", r#"{"ok":true}"#)
            .with_connector_error("trmnl", "push", "connector_credential_missing")
            .run(|_ctx| {
                portaki_sdk::host::connectors::call::<serde_json::Value, serde_json::Value>(
                    "trmnl",
                    "push",
                    &serde_json::json!({ "plugin_id": "7f1c" }),
                )
                .unwrap_err()
            });

        // The error wins over a response registered for the same pair.
        assert!(error.to_string().contains("connector_credential_missing"));
    }

    #[test]
    fn an_operation_without_an_error_still_answers() {
        let value = MockContextBuilder::host()
            .with_connector_response("trmnl", "push", r#"{"ok":true}"#)
            .with_connector_error("trmnl", "other", "connector_egress_failed")
            .run(|_ctx| {
                portaki_sdk::host::connectors::call::<serde_json::Value, serde_json::Value>(
                    "trmnl",
                    "push",
                    &serde_json::json!({}),
                )
                .expect("push")
            });

        assert_eq!(value["ok"], serde_json::json!(true));
    }

    /// What was sent is half of what a connector test wants to assert.
    #[test]
    fn every_call_is_recorded_failures_included() {
        let calls = MockContextBuilder::host()
            .with_connector_error("trmnl", "push", "connector_egress_failed")
            .run_with(|_ctx, host| {
                let _ = portaki_sdk::host::connectors::call::<serde_json::Value, serde_json::Value>(
                    "trmnl",
                    "push",
                    &serde_json::json!({ "plugin_id": "7f1c" }),
                );
                host.connector_calls()
            });

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].connector_id, "trmnl");
        assert_eq!(calls[0].operation, "push");
        assert!(calls[0].args_json.contains("7f1c"));
    }

    use portaki_sdk::host::{self, i18n::Vars};

    use super::MockContext;

    #[test]
    fn mock_host_resolves_translations() {
        MockContext::guest()
            .with_translation("greeting", "Bonjour")
            .run(|_ctx| {
                let text = host::i18n::translate("greeting", &Vars::new()).expect("translate");
                assert_eq!(text, "Bonjour");
            });
    }

    #[test]
    fn mock_host_interpolates_translation_vars() {
        MockContext::guest()
            .with_translation("hello", "Hello {name}")
            .run(|_ctx| {
                let mut vars = Vars::new();
                vars.set("name", "Marie");
                let text = host::i18n::translate("hello", &vars).expect("translate");
                assert_eq!(text, "Hello Marie");
            });
    }
}
