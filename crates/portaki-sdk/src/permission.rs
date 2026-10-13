//! Manifest permissions — what `portaki.module.json` may list under `permissions`.
//!
//! The manifest declares, the gateway enforces. Most permissions guard a host
//! operation (`kv` → `kv.*`, `email` → `email.send`…). One guards data instead:
//! [`STAY_GUEST_CONTACT_READ`] decides whether [`crate::StayContext::guest_email`]
//! and [`crate::StayContext::guest_phone`] are filled.
//!
//! The same list as the `permission` definition of `schema/module.v1.json`; a
//! test keeps the two together.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::permission;
//!
//! assert!(permission::is_known(permission::STAY_GUEST_CONTACT_READ));
//! assert!(permission::is_known("connectors:nuki"));
//! assert!(!permission::is_known("stay:read"));
//! ```

/// `kv.get` / `kv.set` / `kv.delete` / `kv.list` on the module's own namespace.
pub const KV: &str = "kv";

/// `repo.find` / `repo.create` / `repo.delete` on the module's own schema.
pub const REPO: &str = "repo";

/// `email.send`, which reaches real guests.
pub const EMAIL: &str = "email";

/// `events.emit`, which publishes to the platform outbox.
pub const EVENTS: &str = "events";

/// `capabilities.has`, `module.status`, `module.listByCapability`.
pub const PLATFORM: &str = "platform";

/// Prefix of a connector permission — `connectors:nuki`, never `connectors` alone.
pub const CONNECTORS_PREFIX: &str = "connectors:";

/// Guest email and phone on [`crate::StayContext`].
///
/// Everything else about the stay — dates, channel, party size, announced arrival,
/// guest language — reaches every module without asking. Contact details are
/// personal data a booklet module rarely needs, and a way to reach the guest
/// outside the platform: they go only to a module that says so in its manifest,
/// where the host reviewing it can see it. Sending an email to the guest does not
/// need it — `host::email::send` with `EmailAudience::Guest` lets the platform
/// resolve the address.
pub const STAY_GUEST_CONTACT_READ: &str = "stay:guest_contact:read";

/// Files a guest attaches from the booklet (an `ImageUpload` field), see [`crate::files`].
///
/// Guards no host operation: the platform's guest upload endpoint refuses a module that does
/// not declare it, so a host reviewing the manifest sees which modules collect guest photos.
pub const GUEST_FILES: &str = "guest:files";

/// Every permission without a parameter.
pub const FIXED: &[&str] = &[
    KV,
    REPO,
    EMAIL,
    EVENTS,
    PLATFORM,
    STAY_GUEST_CONTACT_READ,
    GUEST_FILES,
];

/// Whether `permission` is one the manifest schema accepts.
pub fn is_known(permission: &str) -> bool {
    if FIXED.contains(&permission) {
        return true;
    }
    permission
        .strip_prefix(CONNECTORS_PREFIX)
        .is_some_and(is_connector_id)
}

/// Prefix of the release-notes key of a revealed secret: `reveal:<field>:<place>`.
///
/// Not a manifest permission: `config.fields[].reveal` declares it. A reveal added since the
/// previous stable is justified like an added permission, under this key — `<field>` is the
/// field's key, or `<list>[].<sub>` for a secret sub-key of a structured list's rows.
pub const REVEAL_PREFIX: &str = "reveal:";

/// Where a secret may be shown — the schema's `reveal` enum, closed.
pub const REVEAL_PLACES: &[&str] = &["guest_pre_arrival", "guest_stay", "arrival_email"];

/// Whether `key` is a `reveal:<field>:<place>` key, read as the registry reads it
/// (`SecretReveal.fromToken`): a non-empty field, then a known place after the last `:`.
///
/// ```
/// use portaki_sdk::permission;
///
/// assert!(permission::is_reveal("reveal:bin_room_code:guest_pre_arrival"));
/// assert!(permission::is_reveal("reveal:networks[].password:guest_stay"));
/// assert!(!permission::is_reveal("reveal:code:host"));
/// ```
pub fn is_reveal(key: &str) -> bool {
    key.strip_prefix(REVEAL_PREFIX)
        .and_then(|rest| rest.rsplit_once(':'))
        .is_some_and(|(field, place)| !field.is_empty() && REVEAL_PLACES.contains(&place))
}

/// `[a-z][a-z0-9-]*` — the schema's connector id pattern.
fn is_connector_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_permissions_need_an_id() {
        assert!(is_known("connectors:open-weather"));
        assert!(!is_known("connectors:"));
        assert!(!is_known("connectors:Nuki"));
        assert!(!is_known("connectors:9lives"));
    }

    /// `stay:read` is an API token scope, not a manifest permission: the stay
    /// context needs no declaration, only the guest's contact details do.
    #[test]
    fn stay_read_is_not_a_manifest_permission() {
        assert!(!is_known("stay:read"));
        assert!(is_known("stay:guest_contact:read"));
    }

    #[test]
    fn reveal_keys_name_a_field_and_a_known_place() {
        for key in [
            "reveal:bin_room_code:guest_pre_arrival",
            "reveal:bin_room_code:guest_stay",
            "reveal:networks[].password:guest_stay",
            "reveal:keyboxCode:arrival_email",
        ] {
            assert!(is_reveal(key), "{key}");
            assert!(!is_known(key), "{key} is not a manifest permission");
        }
        for key in [
            "reveal:code:host",
            "reveal::guest_stay",
            "reveal:guest_stay",
            "reveal:",
            "code:guest_stay",
        ] {
            assert!(!is_reveal(key), "{key}");
        }
    }

    #[test]
    fn the_reveal_places_are_the_schema_ones() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/module.v1.json")).expect("schema");
        assert_eq!(
            schema["$defs"]["configField"]["properties"]["reveal"]["items"]["enum"],
            serde_json::json!(REVEAL_PLACES)
        );
    }

    /// The schema's pattern and this list are the same contract, written twice.
    #[test]
    fn the_schema_pattern_lists_the_same_permissions() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/module.v1.json")).expect("schema");
        let pattern = schema["$defs"]["permission"]["pattern"]
            .as_str()
            .expect("permission pattern");
        for permission in FIXED {
            assert!(
                pattern.contains(&format!("|{permission}|"))
                    || pattern.contains(&format!("({permission}|")),
                "{permission} missing from the schema pattern {pattern}"
            );
        }
        assert!(pattern.contains("connectors:[a-z][a-z0-9-]*"));
    }
}
