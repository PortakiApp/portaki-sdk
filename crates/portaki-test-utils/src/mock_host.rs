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
//!
//! # Email and event limits
//!
//! The mock enforces what the platform enforces per invocation, so a module test fails where
//! production would silently drop mail: `email.send` re-checks
//! [`SendEmailArgs::validate`], refuses past
//! [`portaki_sdk::limits::EMAIL_SENDS_PER_INVOCATION`], and refuses a guest email once the
//! mock stay's checkout is more than [`portaki_sdk::limits::GUEST_EMAIL_DAYS_AFTER_CHECKOUT`]
//! days behind the mock clock; `events.emit` refuses past
//! [`portaki_sdk::limits::EVENTS_PER_INVOCATION`]. One built backend is one invocation.
//! The rolling per-stay / per-workspace caps span invocations and stay platform-only.
//!
//! ```
//! use portaki_sdk::host::email::{self, EmailAudience, EmailError, LocalizedEmailText, ModuleEmailSdui, SendEmailArgs};
//! use portaki_sdk::PortakiError;
//! use portaki_test_utils::{Booking, MockContext};
//!
//! let checkout = Booking::default().check_out; // 2026-06-08T10:00:00Z
//! MockContext::guest()
//!     .with_stay(Booking::default())
//!     .with_now(checkout + chrono::Duration::days(8))
//!     .run_with(|_ctx, host| {
//!         let args = SendEmailArgs {
//!             email_id: "review-reminder".into(),
//!             audience: EmailAudience::Guest,
//!             content: ModuleEmailSdui {
//!                 subject: LocalizedEmailText::both("Merci !"),
//!                 body: LocalizedEmailText::both("…"),
//!                 ..Default::default()
//!             },
//!             stay_id: None,
//!             property_id: None,
//!             action_url: None,
//!         };
//!         assert!(matches!(email::send(&args), Err(PortakiError::Email(EmailError::StayEnded))));
//!         assert!(host.sent_emails().is_empty());
//!     });
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use portaki_sdk::context::{CapabilityGrant, Context, PropertyContext, StayContext};
use portaki_sdk::error::{PortakiError, Result};
use portaki_sdk::host::email::{EmailError, SendEmailArgs};
use portaki_sdk::host::module::ModuleStatus;
use portaki_sdk::host::{with_host, HostBackend};
use portaki_sdk::limits;
use portaki_sdk::sdui::common::GeoPoint;

use serde::Serialize;

use crate::fixtures::Property;

/// Fluent builder for a test [`Context`] and [`MockHostFunctions`] backend.
///
/// Clone before [`Self::run`] when the same configuration must drive multiple
/// isolated invocations (each `run` installs a fresh host scope).
#[derive(Debug, Clone, Default)]
pub struct MockContextBuilder {
    pub(crate) context: Context,
    translations: HashMap<String, String>,
    kv: HashMap<String, Vec<u8>>,
    connector_responses: HashMap<(String, String), String>,
    connector_errors: HashMap<(String, String), String>,
    now: Option<DateTime<Utc>>,
    module_status: Option<ModuleStatus>,
    module_status_error: Option<String>,
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

    /// Moves the property — `None` for one that is not geocoded yet, which a module showing
    /// weather or nearby places must render as its empty state. Keeps the deprecated
    /// `lat` / `lng` in step. Call after [`Self::with_property`] / [`Self::with_capabilities`].
    pub fn with_coordinates(mut self, coordinates: Option<GeoPoint>) -> Self {
        let property = &self.context.property;
        self.context.property = PropertyContext::new(
            property.name.clone(),
            property.locale.clone(),
            property.timezone.clone(),
            coordinates,
            property.address.clone(),
        );
        self
    }

    /// Sets the invocation stay (`Context::stay`) — e.g. `.with_stay(Booking::default())`.
    ///
    /// Its checkout drives the after-stay email rule. Call after [`Self::with_capabilities`],
    /// which rebuilds the context.
    pub fn with_stay(mut self, stay: impl Into<StayContext>) -> Self {
        self.context.stay = Some(stay.into());
        self
    }

    /// Fills the guest's email and phone on the invocation stay, as the gateway does for a
    /// module that declares [`portaki_sdk::permission::STAY_GUEST_CONTACT_READ`].
    ///
    /// The mock does not read the manifest: a test that leaves this out sees what an
    /// undeclared module sees in production — both `None`. Without a stay set yet, a
    /// [`crate::Booking::default`] one is created. Call after [`Self::with_stay`], which
    /// replaces the stay.
    pub fn with_guest_contact(mut self, email: Option<&str>, phone: Option<&str>) -> Self {
        let stay = self
            .context
            .stay
            .get_or_insert_with(|| crate::fixtures::Booking::default().into());
        stay.guest_email = email.map(str::to_string);
        stay.guest_phone = phone.map(str::to_string);
        self
    }

    /// Sets the install's configuration (`Context::module_config`), as the platform hands it
    /// over — what the `load` of `#[portaki_sdk::config]` reads. Once set, even to an empty
    /// object, the KV key `config` is no longer read. Without it, the platform has not taken
    /// the config over and `load` reads the KV key.
    ///
    /// An `I18nText` serializes as the platform stores a `localized` value
    /// (`{ "fr": "…", "en": "…" }`); pass a `serde_json::json!` value to give a legacy plain string.
    ///
    /// Call after [`Self::with_capabilities`], which rebuilds the context.
    ///
    /// # Panics
    ///
    /// When `config` does not serialize to JSON.
    pub fn with_config<T: Serialize>(mut self, config: &T) -> Self {
        self.context.module_config =
            Some(serde_json::to_value(config).expect("config serializes to JSON"));
        self
    }

    /// What `host::module::status` answers — e.g. `incomplete: true` with the required keys
    /// still empty. Without it, a ready install: active, complete, no config required.
    pub fn with_module_status(mut self, status: ModuleStatus) -> Self {
        self.module_status = Some(status);
        self
    }

    /// Makes `host::module::status` fail with `reason` — a platform that cannot answer. A guest
    /// surface then renders the SDK's error state (see `portaki_sdk::guest_shell`).
    pub fn with_module_status_error(mut self, reason: impl Into<String>) -> Self {
        self.module_status_error = Some(reason.into());
        self
    }

    /// Freezes the mock clock (`host::time::now`) at `now`.
    ///
    /// Without it the mock answers the real current time, which makes the after-stay email
    /// rule depend on the day the test runs.
    pub fn with_now(mut self, now: DateTime<Utc>) -> Self {
        self.now = Some(now);
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
            translated_keys: Mutex::new(Vec::new()),
            now: self.now,
            email_send_calls: Mutex::new(0),
            sent_emails: Mutex::new(Vec::new()),
            event_emits: Mutex::new(0),
            logs: Mutex::new(Vec::new()),
            module_status_error: self.module_status_error,
            module_status: self.module_status.unwrap_or(ModuleStatus {
                active: true,
                workspace_enabled: true,
                incomplete: false,
                requires_config: false,
                missing_required_keys: Vec::new(),
            }),
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

/// One `host::log` line, as the module wrote it.
#[derive(Debug, Clone, PartialEq)]
pub struct LogLine {
    /// `debug`, `info`, `warn` or `error`.
    pub level: String,
    /// The event name — `wifi_guest_home_card_render_failed`.
    pub message: String,
    /// The structured fields, as a JSON object.
    pub fields: serde_json::Value,
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
    translated_keys: Mutex<Vec<String>>,
    now: Option<DateTime<Utc>>,
    email_send_calls: Mutex<usize>,
    sent_emails: Mutex<Vec<SendEmailArgs>>,
    event_emits: Mutex<usize>,
    logs: Mutex<Vec<LogLine>>,
    module_status: ModuleStatus,
    module_status_error: Option<String>,
}

impl MockHostFunctions {
    /// Every connector call the module made, in order — failed ones included.
    pub fn connector_calls(&self) -> Vec<ConnectorCall> {
        self.connector_calls
            .lock()
            .expect("connector calls lock")
            .clone()
    }

    /// Every key the module asked `host::i18n` to translate, in order — repeats included.
    ///
    /// The mock answers with the key itself when no translation is seeded, so a missing entry in
    /// the module's bundles goes unnoticed in a rendered tree; this is where it shows.
    pub fn translated_keys(&self) -> Vec<String> {
        self.translated_keys
            .lock()
            .expect("translated keys lock")
            .clone()
    }

    /// Every `host::log` line the module wrote, in order.
    pub fn logs(&self) -> Vec<LogLine> {
        self.logs.lock().expect("logs lock").clone()
    }

    /// Emails the mock accepted, in order — refused ones are not included.
    pub fn sent_emails(&self) -> Vec<SendEmailArgs> {
        self.sent_emails.lock().expect("sent emails lock").clone()
    }

    fn now(&self) -> DateTime<Utc> {
        self.now.unwrap_or_else(Utc::now)
    }
}

impl HostBackend for MockHostFunctions {
    fn context(&self) -> Result<Context> {
        Ok(self.context.clone())
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
        self.translated_keys
            .lock()
            .expect("translated keys lock")
            .push(key.to_string());
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

    fn log(&self, level: &str, message: &str, fields_json: &str) -> Result<()> {
        self.logs.lock().expect("logs lock").push(LogLine {
            level: level.to_string(),
            message: message.to_string(),
            fields: serde_json::from_str(fields_json).unwrap_or_default(),
        });
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
        let mut emits = self.event_emits.lock().expect("event emits lock");
        *emits += 1;
        if *emits > limits::EVENTS_PER_INVOCATION {
            return Err(PortakiError::EventLimitExceeded);
        }
        Ok(())
    }

    fn email_send(&self, payload_json: &str) -> Result<()> {
        // Comme la plateforme : chaque appel de l'op compte, même refusé ensuite.
        {
            let mut calls = self.email_send_calls.lock().expect("email calls lock");
            *calls += 1;
            if *calls > limits::EMAIL_SENDS_PER_INVOCATION {
                return Err(EmailError::LimitExceeded.into());
            }
        }
        // `email::send` a déjà validé ; le mock revérifie parce que la plateforme ne fait pas
        // confiance au SDK non plus, et qu'un backend peut être appelé sans passer par lui.
        let args: SendEmailArgs = serde_json::from_str(payload_json)?;
        args.validate()?;
        if args.is_after_stay(&self.context, self.now()) {
            return Err(EmailError::StayEnded.into());
        }
        self.sent_emails
            .lock()
            .expect("sent emails lock")
            .push(args);
        Ok(())
    }

    fn time_now_iso(&self) -> Result<String> {
        Ok(self.now().to_rfc3339())
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
        match &self.module_status_error {
            Some(reason) => Err(PortakiError::Host(reason.clone())),
            None => Ok(self.module_status.clone()),
        }
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

    mod email_limits {
        use chrono::Duration;
        use portaki_sdk::host::email::{
            self, EmailAudience, EmailError, EmailField, LocalizedEmailText, ModuleEmailSdui,
            SendEmailArgs,
        };
        use portaki_sdk::limits;
        use portaki_sdk::PortakiError;
        use portaki_sdk::{contracts, host::HostBackend};

        use crate::{Booking, MockContext};

        fn guest_email() -> SendEmailArgs {
            SendEmailArgs {
                email_id: "checkout-tips".into(),
                audience: EmailAudience::Guest,
                content: ModuleEmailSdui {
                    subject: LocalizedEmailText::both("Avant de partir"),
                    body: LocalizedEmailText::both("Merci de laisser les clés."),
                    ..Default::default()
                },
                stay_id: None,
                property_id: None,
                action_url: None,
            }
        }

        fn during_stay() -> MockContext {
            let booking = Booking::default();
            let now = booking.check_in + Duration::days(1);
            MockContext::guest().with_stay(booking).with_now(now)
        }

        #[test]
        fn the_sixth_send_of_an_invocation_is_refused() {
            during_stay().run_with(|_ctx, host| {
                for _ in 0..limits::EMAIL_SENDS_PER_INVOCATION {
                    email::send(&guest_email()).expect("within the cap");
                }
                let sixth = email::send(&guest_email());
                assert!(matches!(
                    sixth,
                    Err(PortakiError::Email(EmailError::LimitExceeded))
                ));
                assert_eq!(host.sent_emails().len(), limits::EMAIL_SENDS_PER_INVOCATION);
            });
        }

        #[test]
        fn each_build_is_a_fresh_invocation() {
            let config = during_stay();
            for _ in 0..2 {
                config.clone().run(|_ctx| {
                    for _ in 0..limits::EMAIL_SENDS_PER_INVOCATION {
                        email::send(&guest_email()).expect("fresh counter");
                    }
                });
            }
        }

        #[test]
        fn guest_email_seven_days_after_checkout_is_still_accepted() {
            let booking = Booking::default();
            let now = booking.check_out + Duration::days(limits::GUEST_EMAIL_DAYS_AFTER_CHECKOUT);
            MockContext::guest()
                .with_stay(booking)
                .with_now(now)
                .run_with(|_ctx, host| {
                    email::send(&guest_email()).expect("on the boundary");
                    assert_eq!(host.sent_emails().len(), 1);
                });
        }

        #[test]
        fn guest_email_after_the_grace_period_is_refused() {
            let booking = Booking::default();
            let now = booking.check_out
                + Duration::days(limits::GUEST_EMAIL_DAYS_AFTER_CHECKOUT)
                + Duration::seconds(1);
            MockContext::guest()
                .with_stay(booking)
                .with_now(now)
                .run_with(|_ctx, host| {
                    let err = email::send(&guest_email()).unwrap_err();
                    assert!(matches!(err, PortakiError::Email(EmailError::StayEnded)));
                    assert!(err.to_string().starts_with("email_stay_ended"));

                    // Host audience is not bound to the stay window.
                    let mut to_host = guest_email();
                    to_host.audience = EmailAudience::Host;
                    email::send(&to_host).expect("host email");
                    assert_eq!(host.sent_emails().len(), 1);
                });
        }

        /// The backend applies the rules itself, not only through `email::send`.
        #[test]
        fn the_backend_checks_payloads_that_bypass_the_sdk() {
            let booking = Booking::default();
            let late = booking.check_out + Duration::days(30);
            let (_ctx, host) = MockContext::guest()
                .with_stay(booking)
                .with_now(late)
                .build();

            let mut too_long = guest_email();
            too_long.audience = EmailAudience::Host;
            too_long.content.body =
                LocalizedEmailText::both("x".repeat(limits::EMAIL_BODY_MAX_CHARS + 1));
            let raw = serde_json::to_string(&too_long).unwrap();
            assert!(matches!(
                host.email_send(&raw),
                Err(PortakiError::Email(EmailError::FieldTooLong {
                    field: EmailField::Body,
                    ..
                }))
            ));

            let raw = serde_json::to_string(&guest_email()).unwrap();
            assert!(matches!(
                host.email_send(&raw),
                Err(PortakiError::Email(EmailError::StayEnded))
            ));
            assert!(host.sent_emails().is_empty());
        }

        #[test]
        fn invalid_content_never_reaches_the_host() {
            during_stay().run_with(|_ctx, host| {
                let mut bad = guest_email();
                bad.action_url = Some("http://app.portaki.app/stays".into());
                assert!(matches!(
                    email::send(&bad),
                    Err(PortakiError::Email(EmailError::ActionUrlNotHttps))
                ));
                // Refused by the SDK, so it did not count toward the per-invocation cap.
                for _ in 0..limits::EMAIL_SENDS_PER_INVOCATION {
                    email::send(&guest_email()).expect("cap untouched");
                }
                assert_eq!(host.sent_emails().len(), limits::EMAIL_SENDS_PER_INVOCATION);
            });
        }

        #[test]
        fn the_twenty_first_event_of_an_invocation_is_refused() {
            MockContext::host().run(|_ctx| {
                for _ in 0..limits::EVENTS_PER_INVOCATION {
                    portaki_sdk::host::events::emit(contracts::platform::EMAIL_SEND, &())
                        .expect("within the cap");
                }
                assert!(matches!(
                    portaki_sdk::host::events::emit(contracts::platform::EMAIL_SEND, &()),
                    Err(PortakiError::EventLimitExceeded)
                ));
            });
        }
    }

    /// Sans déclaration, le runtime ne transmet pas le contact : le mock part du même vide.
    #[test]
    fn guest_contact_is_absent_unless_the_test_grants_it() {
        let ctx = MockContext::guest()
            .with_stay(crate::Booking::default())
            .context();
        let stay = ctx.stay.expect("stay");
        assert_eq!(stay.guest_email, None);
        assert_eq!(stay.guest_phone, None);
    }

    #[test]
    fn with_guest_contact_fills_the_stay_it_finds() {
        let booking = crate::Booking::default();
        let stay_id = booking.id;
        let ctx = MockContext::guest()
            .with_stay(booking)
            .with_guest_contact(Some("marie@example.com"), None)
            .context();
        let stay = ctx.stay.expect("stay");
        assert_eq!(stay.stay_id, stay_id);
        assert_eq!(stay.guest_email.as_deref(), Some("marie@example.com"));
        assert_eq!(stay.guest_phone, None);
    }

    #[test]
    fn with_guest_contact_creates_a_stay_when_none_is_set() {
        MockContext::guest()
            .with_guest_contact(None, Some("+33600000000"))
            .run(|ctx| {
                let stay = ctx.stay.expect("a default booking stay");
                assert!(stay.checkout_at.is_some());
                assert_eq!(stay.guest_phone.as_deref(), Some("+33600000000"));
            });
    }

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
