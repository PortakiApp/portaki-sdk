//! Caps the platform applies to modules — a single table.
//!
//! Every value here is **also** enforced by the platform (runtime and orchestrator), and the
//! platform's copy is the one that counts: the SDK grants nothing on top of it. It mirrors the
//! values in order to fail early. An overrun caught on the SDK side surfaces as a typed error
//! as soon as `cargo test` runs (through `portaki-test-utils`), whereas in production an email
//! the orchestrator rejects is simply dropped, with nothing reported back to the module.
//!
//! Some limits count beyond a single invocation or a single module (sliding windows, all
//! modules taken together): the SDK cannot check those. They are listed here all the same,
//! marked "platform only", so that a module author finds them in the same place.
//!
//! Changing a value here without the platform changing its own changes nothing in
//! production — only what the tests let through.

// ── Email content (`host::email::send`) ──────────────────────────────────────────────────
//
// Counted in characters (`char`), not in bytes: a subject with accents must not be rejected
// before an ASCII subject of the same length. Each locale is checked separately.

/// Maximum subject length, in characters, for each locale.
pub const EMAIL_SUBJECT_MAX_CHARS: usize = 200;

/// Maximum eyebrow length (above the title), in characters, for each locale.
pub const EMAIL_EYEBROW_MAX_CHARS: usize = 120;

/// Maximum title length, in characters, for each locale.
pub const EMAIL_TITLE_MAX_CHARS: usize = 200;

/// Maximum body length, in characters, for each locale.
pub const EMAIL_BODY_MAX_CHARS: usize = 5000;

/// Maximum CTA label length, in characters, for each locale.
pub const EMAIL_CTA_LABEL_MAX_CHARS: usize = 80;

// ── Email blocks (`ModuleEmailSdui::blocks`) ─────────────────────────────────────────────
//
// `emailBlocks` in `contracts/module-limits.json` on the platform side. A block that falls
// outside the contract makes it reject the whole email.

/// Maximum number of blocks per email.
pub const EMAIL_BLOCKS_MAX: usize = 10;

/// Maximum number of items per block (at least one; `stats` takes 2 or 4).
pub const EMAIL_BLOCK_ITEMS_MAX: usize = 12;

/// Maximum length of each text in a block, in characters, for each locale.
pub const EMAIL_BLOCK_TEXT_MAX_CHARS: usize = 200;

/// Maximum length of a block emoji, in UTF-16 units (flags, ZWJ sequences).
///
/// On top of that the platform requires a single grapheme in the "symbol" category; the SDK,
/// which has no Unicode segmentation, only rejects ASCII, letters, digits and spaces.
pub const EMAIL_BLOCK_EMOJI_MAX_UTF16: usize = 16;

// ── Guest stay email zone (`#[email_blocks]`) ────────────────────────────────────────────
//
// `guestEmailBlocks` in `contracts/module-limits.json` on the platform side. Unlike the blocks
// of an email a module writes, a block outside the contract is dropped and the email still
// goes: the email belongs to Portaki, and no module keeps it from arriving.

/// Maximum number of blocks one module gives one email.
pub const GUEST_EMAIL_BLOCKS_MAX: usize = 4;

/// Maximum number of rows in a `pairs` or `list` block; the platform keeps the first ones.
pub const GUEST_EMAIL_BLOCK_ROWS_MAX: usize = 3;

/// Maximum number of items in a `checklist` block; the platform keeps the first ones.
pub const GUEST_EMAIL_BLOCK_ITEMS_MAX: usize = 4;

/// Maximum length of a block eyebrow, in characters, for each locale.
pub const GUEST_EMAIL_BLOCK_LABEL_MAX_CHARS: usize = 24;

/// Maximum length of a block title, in characters, for each locale.
pub const GUEST_EMAIL_BLOCK_TITLE_MAX_CHARS: usize = 40;

/// Maximum length of a block text, in characters, for each locale.
pub const GUEST_EMAIL_BLOCK_TEXT_MAX_CHARS: usize = 140;

/// Maximum length of a block link label, in characters, for each locale.
pub const GUEST_EMAIL_BLOCK_LINK_LABEL_MAX_CHARS: usize = 32;

// ── Per invocation ───────────────────────────────────────────────────────────────────────

/// Maximum number of `email.send` calls per invocation.
///
/// Beyond that, the platform answers `email_limit_exceeded`
/// ([`crate::host::email::EmailError::LimitExceeded`]). The SDK does not do the counting
/// itself: the counter lives on the host side, and the `portaki-test-utils` mock reproduces it.
pub const EMAIL_SENDS_PER_INVOCATION: usize = 5;

/// Maximum number of events emitted to the gateway per invocation.
///
/// Beyond that, the platform answers `event_limit_exceeded`
/// ([`crate::error::PortakiError::EventLimitExceeded`]).
pub const EVENTS_PER_INVOCATION: usize = 20;

/// Maximum number of `connector.call` calls per invocation.
pub const CONNECTOR_CALLS_PER_INVOCATION: usize = 5;

/// Maximum size of a connector response, in bytes (1 MiB).
pub const CONNECTOR_RESPONSE_MAX_BYTES: usize = 1024 * 1024;

// ── Guest emails ─────────────────────────────────────────────────────────────────────────

/// Days after check-out during which an email to the guest audience is still accepted.
///
/// Past `checkout_at + 7 days`, the platform refuses (`email_stay_ended`): the guest left long
/// ago, and a late reminder looks like spam. The SDK checks the rule when the stay being
/// targeted is the invocation's own — the only one whose check-out it knows.
pub const GUEST_EMAIL_DAYS_AFTER_CHECKOUT: i64 = 7;

/// Module emails per guest stay over a sliding 24 h window, all modules taken together.
///
/// Platform only: the count covers the other modules and past invocations.
pub const GUEST_STAY_EMAILS_PER_24H: usize = 3;

/// Module emails per guest stay in total, all modules taken together.
///
/// Platform only: the count covers the other modules and past invocations.
pub const GUEST_STAY_EMAILS_TOTAL: usize = 10;

// ── Host emails ──────────────────────────────────────────────────────────────────────────

/// Emails to the host audience per module and per workspace over a sliding 24 h window.
///
/// Platform only: the count covers past invocations.
pub const HOST_EMAILS_PER_MODULE_PER_24H: usize = 20;

// ── Guest files (`ImageUpload`, `guest:files` permission) ────────────────────────────────
//
// Enforced by the platform's traveller upload endpoint: the SDK never sees the bytes, only the
// reference ([`crate::files::FileRef`]) that a form hands it.

/// Maximum size of a guest file, in bytes (5 MiB).
pub const GUEST_FILE_MAX_BYTES: usize = 5 * 1024 * 1024;

/// Accepted types, checked on the bytes and not on the declared header. The platform re-encodes
/// the image, which strips the metadata (EXIF, GPS position).
pub const GUEST_FILE_CONTENT_TYPES: &[&str] = &["image/jpeg", "image/png"];

/// Files a stay may send, all modules taken together.
///
/// Platform only: the count covers the other modules and past invocations.
pub const GUEST_FILES_PER_STAY: usize = 20;
