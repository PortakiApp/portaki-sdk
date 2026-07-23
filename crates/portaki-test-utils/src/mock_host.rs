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

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use portaki_sdk::context::Context;
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
        });
        (self.context, host)
    }

    /// Installs the mock host on the current thread and runs `f` with the built [`Context`].
    ///
    /// Nested `run` calls replace the thread-local backend for the duration of the
    /// inner closure.
    pub fn run<R, F: FnOnce(Context) -> R>(self, f: F) -> R {
        let (ctx, host) = self.build();
        with_host(host, ctx.clone(), || f(ctx))
    }
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
        _args_json: &str,
    ) -> Result<String> {
        Ok(self
            .connector_responses
            .get(&(connector_id.to_string(), operation.to_string()))
            .cloned()
            .unwrap_or_else(|| "{}".to_string()))
    }

    fn emit_event(&self, _event_type: &str, _payload_json: &str) -> Result<()> {
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
