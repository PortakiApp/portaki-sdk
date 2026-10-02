//! Platform transactional email template catalog.
//!
//! Mirrors `app.portaki.domain.model.email.EmailTemplates` on the Java gateway.
//! Guest-stay modules filter [`EmailContextArgs::template_key`] against this enum
//! instead of raw strings.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::email::EmailTemplateKey;
//!
//! assert_eq!(EmailTemplateKey::ArrivalDay.as_str(), "arrival-day");
//! assert!(EmailTemplateKey::Arrival.is_guest_stay());
//! assert!(!EmailTemplateKey::Welcome.is_guest_stay());
//! ```
//!
//! [`EmailVar`] is the catalogue of variables a module may give a guest email, with the
//! templates that render each; a module declares its own with
//! [`#[email_vars]`](crate::email_vars).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Closed catalog of Portaki Thymeleaf email template keys (wire: JSON string).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmailTemplateKey {
    /// Account welcome.
    #[serde(rename = "welcome")]
    Welcome,
    /// Email verification.
    #[serde(rename = "verify-email")]
    VerifyEmail,
    /// One-time password.
    #[serde(rename = "otp")]
    Otp,
    /// Password reset.
    #[serde(rename = "reset-password")]
    ResetPassword,
    /// Workspace member invitation.
    #[serde(rename = "member-invitation")]
    MemberInvitation,
    /// Account deletion confirmation.
    #[serde(rename = "account-deletion")]
    AccountDeletion,
    /// SIREN verified.
    #[serde(rename = "siren-verified")]
    SirenVerified,
    /// SIREN verification failed.
    #[serde(rename = "siren-verification-failed")]
    SirenVerificationFailed,
    /// Billing subscription thank-you.
    #[serde(rename = "billing-subscription-thank-you")]
    BillingSubscriptionThankYou,
    /// Plan upgraded.
    #[serde(rename = "plan-upgraded")]
    PlanUpgraded,
    /// Payment failed.
    #[serde(rename = "payment-failed")]
    PaymentFailed,
    /// Password changed.
    #[serde(rename = "password-changed")]
    PasswordChanged,
    /// New device login.
    #[serde(rename = "new-device")]
    NewDevice,
    /// Invite accepted.
    #[serde(rename = "invite-accepted")]
    InviteAccepted,
    /// Role changed.
    #[serde(rename = "role-changed")]
    RoleChanged,
    /// Member removed.
    #[serde(rename = "member-removed")]
    MemberRemoved,
    /// Receipt.
    #[serde(rename = "receipt")]
    Receipt,
    /// Trial ending.
    #[serde(rename = "trial-ending")]
    TrialEnding,
    /// Subscription canceled.
    #[serde(rename = "subscription-canceled")]
    SubscriptionCanceled,
    /// Property suspended.
    #[serde(rename = "property-suspended")]
    PropertySuspended,
    /// Company address change.
    #[serde(rename = "company-address")]
    CompanyAddress,
    /// Company change.
    #[serde(rename = "company-change")]
    CompanyChange,
    /// Company closed.
    #[serde(rename = "company-closed")]
    CompanyClosed,
    /// Stay registry action.
    #[serde(rename = "registry")]
    Registry,
    /// Police form generated.
    #[serde(rename = "police-generated")]
    PoliceGenerated,
    /// Police form submitted.
    #[serde(rename = "police-submitted")]
    PoliceSubmitted,
    /// Attestation expiring.
    #[serde(rename = "attestation-expiring")]
    AttestationExpiring,
    /// Compliance alert.
    #[serde(rename = "compliance-alert")]
    ComplianceAlert,
    /// Product update.
    #[serde(rename = "product-update")]
    ProductUpdate,
    /// Weekly digest.
    #[serde(rename = "weekly-digest")]
    WeeklyDigest,
    /// Data export ready.
    #[serde(rename = "data-export")]
    DataExport,
    /// Deletion canceled.
    #[serde(rename = "deletion-canceled")]
    DeletionCanceled,
    /// Quota warning.
    #[serde(rename = "quota-warning")]
    QuotaWarning,
    /// Module down.
    #[serde(rename = "module-down")]
    ModuleDown,
    /// Magic link.
    #[serde(rename = "magic-link")]
    MagicLink,
    /// Email change confirmation.
    #[serde(rename = "email-change")]
    EmailChange,
    /// Two-factor auth enabled.
    #[serde(rename = "twofa")]
    Twofa,
    /// Card expiring.
    #[serde(rename = "card-expiring")]
    CardExpiring,
    /// Dunning reminder.
    #[serde(rename = "dunning")]
    Dunning,
    /// Annual renewal.
    #[serde(rename = "annual-renewal")]
    AnnualRenewal,
    /// New review.
    #[serde(rename = "new-review")]
    NewReview,
    /// Guest message.
    #[serde(rename = "guest-message")]
    GuestMessage,
    /// Monthly report.
    #[serde(rename = "monthly-report")]
    MonthlyReport,
    /// Stay link ready (guest).
    #[serde(rename = "stay-link")]
    StayLink,
    /// Arrival J-1 (guest).
    #[serde(rename = "arrival")]
    Arrival,
    /// Stay modified (guest).
    #[serde(rename = "stay-modified")]
    StayModified,
    /// Stay canceled (guest).
    #[serde(rename = "stay-canceled")]
    StayCanceled,
    /// Host message (guest).
    #[serde(rename = "host-message")]
    HostMessage,
    /// New access code (guest).
    #[serde(rename = "new-code")]
    NewCode,
    /// Post-arrival check-in (guest).
    #[serde(rename = "post-arrival")]
    PostArrival,
    /// Deposit receipt (guest).
    #[serde(rename = "deposit-receipt")]
    DepositReceipt,
    /// Arrival day (guest).
    #[serde(rename = "arrival-day")]
    ArrivalDay,
    /// Police form request (guest).
    #[serde(rename = "police-form")]
    PoliceForm,
    /// Lost & found (guest).
    #[serde(rename = "lost-found")]
    LostFound,
    /// Extras invoice (guest).
    #[serde(rename = "extras-invoice")]
    ExtrasInvoice,
}

impl EmailTemplateKey {
    /// Guest-stay templates that modules commonly contribute context to.
    pub const GUEST_STAY: &'static [EmailTemplateKey] = &[
        Self::StayLink,
        Self::Arrival,
        Self::StayModified,
        Self::StayCanceled,
        Self::HostMessage,
        Self::NewCode,
        Self::PostArrival,
        Self::DepositReceipt,
        Self::ArrivalDay,
        Self::PoliceForm,
        Self::LostFound,
        Self::ExtrasInvoice,
    ];

    /// Exhaustive catalog.
    pub const ALL: &'static [EmailTemplateKey] = &[
        Self::Welcome,
        Self::VerifyEmail,
        Self::Otp,
        Self::ResetPassword,
        Self::MemberInvitation,
        Self::AccountDeletion,
        Self::SirenVerified,
        Self::SirenVerificationFailed,
        Self::BillingSubscriptionThankYou,
        Self::PlanUpgraded,
        Self::PaymentFailed,
        Self::PasswordChanged,
        Self::NewDevice,
        Self::InviteAccepted,
        Self::RoleChanged,
        Self::MemberRemoved,
        Self::Receipt,
        Self::TrialEnding,
        Self::SubscriptionCanceled,
        Self::PropertySuspended,
        Self::CompanyAddress,
        Self::CompanyChange,
        Self::CompanyClosed,
        Self::Registry,
        Self::PoliceGenerated,
        Self::PoliceSubmitted,
        Self::AttestationExpiring,
        Self::ComplianceAlert,
        Self::ProductUpdate,
        Self::WeeklyDigest,
        Self::DataExport,
        Self::DeletionCanceled,
        Self::QuotaWarning,
        Self::ModuleDown,
        Self::MagicLink,
        Self::EmailChange,
        Self::Twofa,
        Self::CardExpiring,
        Self::Dunning,
        Self::AnnualRenewal,
        Self::NewReview,
        Self::GuestMessage,
        Self::MonthlyReport,
        Self::StayLink,
        Self::Arrival,
        Self::StayModified,
        Self::StayCanceled,
        Self::HostMessage,
        Self::NewCode,
        Self::PostArrival,
        Self::DepositReceipt,
        Self::ArrivalDay,
        Self::PoliceForm,
        Self::LostFound,
        Self::ExtrasInvoice,
    ];

    /// Stable wire id matching Java `EmailTemplates.id()`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Welcome => "welcome",
            Self::VerifyEmail => "verify-email",
            Self::Otp => "otp",
            Self::ResetPassword => "reset-password",
            Self::MemberInvitation => "member-invitation",
            Self::AccountDeletion => "account-deletion",
            Self::SirenVerified => "siren-verified",
            Self::SirenVerificationFailed => "siren-verification-failed",
            Self::BillingSubscriptionThankYou => "billing-subscription-thank-you",
            Self::PlanUpgraded => "plan-upgraded",
            Self::PaymentFailed => "payment-failed",
            Self::PasswordChanged => "password-changed",
            Self::NewDevice => "new-device",
            Self::InviteAccepted => "invite-accepted",
            Self::RoleChanged => "role-changed",
            Self::MemberRemoved => "member-removed",
            Self::Receipt => "receipt",
            Self::TrialEnding => "trial-ending",
            Self::SubscriptionCanceled => "subscription-canceled",
            Self::PropertySuspended => "property-suspended",
            Self::CompanyAddress => "company-address",
            Self::CompanyChange => "company-change",
            Self::CompanyClosed => "company-closed",
            Self::Registry => "registry",
            Self::PoliceGenerated => "police-generated",
            Self::PoliceSubmitted => "police-submitted",
            Self::AttestationExpiring => "attestation-expiring",
            Self::ComplianceAlert => "compliance-alert",
            Self::ProductUpdate => "product-update",
            Self::WeeklyDigest => "weekly-digest",
            Self::DataExport => "data-export",
            Self::DeletionCanceled => "deletion-canceled",
            Self::QuotaWarning => "quota-warning",
            Self::ModuleDown => "module-down",
            Self::MagicLink => "magic-link",
            Self::EmailChange => "email-change",
            Self::Twofa => "twofa",
            Self::CardExpiring => "card-expiring",
            Self::Dunning => "dunning",
            Self::AnnualRenewal => "annual-renewal",
            Self::NewReview => "new-review",
            Self::GuestMessage => "guest-message",
            Self::MonthlyReport => "monthly-report",
            Self::StayLink => "stay-link",
            Self::Arrival => "arrival",
            Self::StayModified => "stay-modified",
            Self::StayCanceled => "stay-canceled",
            Self::HostMessage => "host-message",
            Self::NewCode => "new-code",
            Self::PostArrival => "post-arrival",
            Self::DepositReceipt => "deposit-receipt",
            Self::ArrivalDay => "arrival-day",
            Self::PoliceForm => "police-form",
            Self::LostFound => "lost-found",
            Self::ExtrasInvoice => "extras-invoice",
        }
    }

    /// Returns `true` when this key is in [`Self::GUEST_STAY`].
    pub fn is_guest_stay(self) -> bool {
        Self::GUEST_STAY.contains(&self)
    }

    /// Returns `true` when `id` is a registered template key.
    pub fn is_known(id: &str) -> bool {
        Self::from_str(id).is_ok()
    }
}

impl AsRef<str> for EmailTemplateKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for EmailTemplateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EmailTemplateKey {
    type Err = ParseEmailTemplateKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "welcome" => Ok(Self::Welcome),
            "verify-email" => Ok(Self::VerifyEmail),
            "otp" => Ok(Self::Otp),
            "reset-password" => Ok(Self::ResetPassword),
            "member-invitation" => Ok(Self::MemberInvitation),
            "account-deletion" => Ok(Self::AccountDeletion),
            "siren-verified" => Ok(Self::SirenVerified),
            "siren-verification-failed" => Ok(Self::SirenVerificationFailed),
            "billing-subscription-thank-you" => Ok(Self::BillingSubscriptionThankYou),
            "plan-upgraded" => Ok(Self::PlanUpgraded),
            "payment-failed" => Ok(Self::PaymentFailed),
            "password-changed" => Ok(Self::PasswordChanged),
            "new-device" => Ok(Self::NewDevice),
            "invite-accepted" => Ok(Self::InviteAccepted),
            "role-changed" => Ok(Self::RoleChanged),
            "member-removed" => Ok(Self::MemberRemoved),
            "receipt" => Ok(Self::Receipt),
            "trial-ending" => Ok(Self::TrialEnding),
            "subscription-canceled" => Ok(Self::SubscriptionCanceled),
            "property-suspended" => Ok(Self::PropertySuspended),
            "company-address" => Ok(Self::CompanyAddress),
            "company-change" => Ok(Self::CompanyChange),
            "company-closed" => Ok(Self::CompanyClosed),
            "registry" => Ok(Self::Registry),
            "police-generated" => Ok(Self::PoliceGenerated),
            "police-submitted" => Ok(Self::PoliceSubmitted),
            "attestation-expiring" => Ok(Self::AttestationExpiring),
            "compliance-alert" => Ok(Self::ComplianceAlert),
            "product-update" => Ok(Self::ProductUpdate),
            "weekly-digest" => Ok(Self::WeeklyDigest),
            "data-export" => Ok(Self::DataExport),
            "deletion-canceled" => Ok(Self::DeletionCanceled),
            "quota-warning" => Ok(Self::QuotaWarning),
            "module-down" => Ok(Self::ModuleDown),
            "magic-link" => Ok(Self::MagicLink),
            "email-change" => Ok(Self::EmailChange),
            "twofa" => Ok(Self::Twofa),
            "card-expiring" => Ok(Self::CardExpiring),
            "dunning" => Ok(Self::Dunning),
            "annual-renewal" => Ok(Self::AnnualRenewal),
            "new-review" => Ok(Self::NewReview),
            "guest-message" => Ok(Self::GuestMessage),
            "monthly-report" => Ok(Self::MonthlyReport),
            "stay-link" => Ok(Self::StayLink),
            "arrival" => Ok(Self::Arrival),
            "stay-modified" => Ok(Self::StayModified),
            "stay-canceled" => Ok(Self::StayCanceled),
            "host-message" => Ok(Self::HostMessage),
            "new-code" => Ok(Self::NewCode),
            "post-arrival" => Ok(Self::PostArrival),
            "deposit-receipt" => Ok(Self::DepositReceipt),
            "arrival-day" => Ok(Self::ArrivalDay),
            "police-form" => Ok(Self::PoliceForm),
            "lost-found" => Ok(Self::LostFound),
            "extras-invoice" => Ok(Self::ExtrasInvoice),
            other => Err(ParseEmailTemplateKeyError {
                key: other.to_string(),
            }),
        }
    }
}

impl TryFrom<&str> for EmailTemplateKey {
    type Error = ParseEmailTemplateKeyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

/// Error returned when parsing an unknown email template key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEmailTemplateKeyError {
    /// The unrecognized template key.
    pub key: String,
}

impl fmt::Display for ParseEmailTemplateKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown email template key: {}", self.key)
    }
}

impl std::error::Error for ParseEmailTemplateKeyError {}

/// Arguments the platform passes to a module's `emailContext` — every field it sends.
///
/// Prefer this type over redefining `template_key` / `locale` in each module.
#[portaki_sdk_macros::wire]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EmailContextArgs {
    /// Template being composed; modules filter on guest-stay keys.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_key: Option<EmailTemplateKey>,
    /// Stay identifier when composing a guest-stay email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stay_id: Option<String>,
    /// Optional locale override (BCP-47). Falls back to [`crate::Context::locale`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// Check-in clock time, formatted in the property's timezone (`16:00`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkin_time_formatted: Option<String>,
    /// The property's address, when it has one — a place name for copy (`Antibes`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_hint: Option<String>,
}

impl EmailContextArgs {
    /// `true` when `template_key` is unset or listed in `allowed`.
    ///
    /// Most guest-stay modules treat a missing key as “contribute” (gateway
    /// probe / legacy callers).
    pub fn allows_template(&self, allowed: &[EmailTemplateKey]) -> bool {
        match self.template_key {
            None => true,
            Some(key) => allowed.contains(&key),
        }
    }

    /// Resolved locale: trimmed `locale` override, else `fallback` (typically
    /// `ctx.locale`).
    pub fn locale_or<'a>(&'a self, fallback: &'a str) -> &'a str {
        self.locale
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback)
    }
}

/// Before [`EmailVar`]: fields a module was told to return into guest-stay emails.
///
/// Four of these keys were never read by the platform (`houseRulesSummary`, `checklistSummary`,
/// `wasteTip`, `applianceTip`). The keys it reads are [`EmailVar`]; declare them with
/// [`#[email_vars]`](crate::email_vars).
#[deprecated(
    since = "8.3.0",
    note = "the platform reads none of houseRulesSummary, checklistSummary, wasteTip, applianceTip — use EmailVar / #[email_vars]"
)]
#[portaki_sdk_macros::wire]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EmailContextContribution {
    /// Same key as [`EmailVar::WeatherSummary`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weather_summary: Option<String>,
    /// Same key as [`EmailVar::ArrivalCallout`] — a string, not an object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrival_callout: Option<serde_json::Value>,
    /// Never read: the platform reads [`EmailVar::HouseRulesTeaser`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_rules_summary: Option<String>,
    /// Never read: the platform reads [`EmailVar::CheckoutTips`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checklist_summary: Option<String>,
    /// Never read: no template renders it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waste_tip: Option<String>,
    /// Never read: no template renders it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appliance_tip: Option<String>,
}

/// Declares [`EmailVar`] and its catalogue from one table: wire key, templates that render it.
macro_rules! email_var_catalog {
    ($($(#[$doc:meta])* $variant:ident = $wire:literal in [$($template:ident),+],)+) => {
        /// A variable a module may give a Portaki guest email — the whole catalogue.
        ///
        /// Mirrors the module-provided keys of `EmailTemplates` on the platform: each variable is
        /// rendered by the templates listed on it, and only there. Declare the ones a module
        /// provides with [`#[email_vars]`](crate::email_vars); declaring one for a template that
        /// does not render it does not compile.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum EmailVar {
            $($(#[$doc])* #[doc = concat!("\n\nKey `", $wire, "`, rendered by ", $("`", stringify!($template), "` ",)+ ".")] #[serde(rename = $wire)] $variant,)+
        }

        impl EmailVar {
            /// The whole catalogue, in declaration order.
            pub const ALL: &'static [EmailVar] = &[$(Self::$variant,)+];

            /// The key of the template variable (`wifiName`).
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $wire,)+ }
            }

            /// The templates that render this variable.
            pub const fn templates(self) -> &'static [EmailTemplateKey] {
                match self { $(Self::$variant => &[$(EmailTemplateKey::$template,)+],)+ }
            }
        }

        impl crate::vocab::Vocabulary for EmailVar {
            const ALL: &'static [Self] = EmailVar::ALL;
            fn wire(self) -> &'static str {
                self.as_str()
            }
            fn variant(self) -> &'static str {
                match self { $(Self::$variant => stringify!($variant),)+ }
            }
        }
    };
}

email_var_catalog! {
    /// How to get in: self check-in callout (access-guide).
    ArrivalCallout = "arrivalCallout" in [Arrival, NewCode],
    /// The door or key-box code, once the reveal policy allows it (access-guide).
    EntryAccessCode = "entryAccessCode" in [Arrival, NewCode, ArrivalDay],
    /// What the code opens — "Key box code" (access-guide).
    AccessCodeLabel = "accessCodeLabel" in [Arrival, NewCode, ArrivalDay],
    /// Today's weather in one line (weather).
    WeatherSummary = "weatherSummary" in [ArrivalDay],
    /// A few house rules, one per line (rules).
    HouseRulesTeaser = "houseRulesTeaser" in [StayLink, Arrival, PostArrival],
    /// The phone to call during the stay (emergency-contacts).
    HostPhone = "hostPhone" in [Arrival, ArrivalDay, PostArrival, LostFound],
    /// One local recommendation (local-guide, events).
    LocalTip = "localTip" in [ArrivalDay, PostArrival],
    /// Before-leaving reminders, one per line (checklist).
    CheckoutTips = "checkoutTips" in [PostArrival, LostFound],
    /// Where to charge the car (ev-parking).
    EvParkingSpot = "evParkingSpot" in [Arrival, ArrivalDay],
    /// What the guest declared lost (lost-found).
    LostItemDescription = "lostItemDescription" in [LostFound],
    /// The Wi-Fi network name (wifi-guest); wins over the property's own when given.
    WifiName = "wifiName" in [StayLink, Arrival, ArrivalDay],
}

impl EmailVar {
    /// `true` when `template` renders this variable — usable in `const` context, which is how
    /// `#[email_vars]` refuses a declaration at compile time.
    pub const fn renders_in(self, template: EmailTemplateKey) -> bool {
        let templates = self.templates();
        let mut i = 0;
        while i < templates.len() {
            if templates[i] as usize == template as usize {
                return true;
            }
            i += 1;
        }
        false
    }
}

impl fmt::Display for EmailVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The values a module gives one email — what an `#[email_vars]` function returns.
///
/// A blank value is the same as none: the template hides the section.
///
/// ```
/// use portaki_sdk::email::{EmailVar, EmailVars};
///
/// let vars = EmailVars::new().with(EmailVar::WifiName, "Belledonne_Guest");
/// assert_eq!(vars.get(EmailVar::WifiName), Some("Belledonne_Guest"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmailVars(Vec<(EmailVar, String)>);

impl EmailVars {
    /// No value.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `var`, replacing an earlier value.
    pub fn insert(&mut self, var: EmailVar, value: impl Into<String>) {
        let value = value.into();
        match self.0.iter_mut().find(|(set, _)| *set == var) {
            Some(entry) => entry.1 = value,
            None => self.0.push((var, value)),
        }
    }

    /// [`insert`](Self::insert), chained.
    pub fn with(mut self, var: EmailVar, value: impl Into<String>) -> Self {
        self.insert(var, value);
        self
    }

    /// The value set for `var`.
    pub fn get(&self, var: EmailVar) -> Option<&str> {
        self.0
            .iter()
            .find(|(set, _)| *set == var)
            .map(|(_, value)| value.as_str())
    }
}

/// What a module declared with `#[email_vars]`: per template, the variables it provides.
pub type DeclaredEmailVars = &'static [(EmailTemplateKey, &'static [EmailVar])];

/// The `#[email_vars]` of a linked module crate — native targets only, for the conformance battery.
#[doc(hidden)]
pub struct EmailVarsDeclaration {
    /// The declaration, as written.
    pub declared: DeclaredEmailVars,
    /// The shim behind the generated `emailContext` query.
    pub dispatch: crate::wasm::registry::WasmHandlerFn,
}

inventory::collect!(EmailVarsDeclaration);

/// The `#[email_vars]` declarations linked into this binary (empty on `wasm32`).
pub fn declarations() -> impl Iterator<Item = &'static EmailVarsDeclaration> {
    inventory::iter::<EmailVarsDeclaration>.into_iter()
}

/// The body of the `emailContext` query `#[email_vars]` generates.
///
/// Nothing is asked of the module for a template it did not declare (nor without a template).
/// Of what it answers, only the variables declared for this template are sent, blank ones left
/// out; a variable declared for no template at all is an error — a missing declaration. The
/// answer is the flat object the platform merges: `{ "wifiName": "…" }`.
pub fn serve(
    declared: DeclaredEmailVars,
    ctx: crate::Context,
    args: EmailContextArgs,
    provide: impl FnOnce(crate::Context, EmailContextArgs) -> crate::Result<EmailVars>,
) -> crate::Result<serde_json::Value> {
    let mut out = serde_json::Map::new();
    let Some(template) = args.template_key else {
        return Ok(out.into());
    };
    let Some((_, allowed)) = declared.iter().find(|(key, _)| *key == template) else {
        return Ok(out.into());
    };
    for (var, value) in provide(ctx, args)?.0 {
        if !declared.iter().any(|(_, vars)| vars.contains(&var)) {
            return Err(crate::PortakiError::Host(format!(
                "email_var_undeclared: `{var}` is returned but not declared in #[email_vars]"
            )));
        }
        let value = value.trim();
        if !allowed.contains(&var) {
            continue;
        }
        if !value.is_empty() {
            out.insert(var.as_str().to_string(), value.into());
        }
    }
    Ok(out.into())
}

// ── Blocs de la zone modules des e-mails de séjour ───────────────────────────────────────

/// A kind of block a module may add to a Portaki stay email — the whole catalogue.
///
/// Mirrors `guestEmailBlocks.perEmail` in the platform's `contracts/module-limits.json`
/// (`GuestEmailBlockSlots` in Java): each kind is rendered by the emails listed on it, and only
/// there. Declare the ones a module provides with [`#[email_blocks]`](crate::email_blocks);
/// declaring one for an email that does not render it does not compile.
///
/// Not to be confused with the blocks of [`crate::host::email`], which are the body of an email a
/// module *writes*. These are a short addition to an email Portaki owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockType {
    /// A tile: eyebrow, title, two lines of text.
    #[serde(rename = "info")]
    Info,
    /// A tile of label/value rows — a code, a spot, a height.
    #[serde(rename = "pairs")]
    Pairs,
    /// A tile of rows read as a list — departures, opening hours.
    #[serde(rename = "list")]
    List,
    /// A tile of things to tick off before leaving.
    #[serde(rename = "checklist")]
    Checklist,
    /// Full width, tinted, at the top of the zone: something that changes the stay.
    #[serde(rename = "alert")]
    Alert,
}

impl BlockType {
    /// The whole catalogue, in declaration order.
    pub const ALL: &'static [BlockType] = &[
        Self::Info,
        Self::Pairs,
        Self::List,
        Self::Checklist,
        Self::Alert,
    ];

    /// The wire string the platform reads.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Pairs => "pairs",
            Self::List => "list",
            Self::Checklist => "checklist",
            Self::Alert => "alert",
        }
    }

    /// The emails that render this kind of block.
    pub const fn templates(self) -> &'static [EmailTemplateKey] {
        match self {
            Self::Info => &[
                EmailTemplateKey::StayLink,
                EmailTemplateKey::Arrival,
                EmailTemplateKey::ArrivalDay,
                EmailTemplateKey::StayModified,
                EmailTemplateKey::PostArrival,
            ],
            Self::Pairs => &[EmailTemplateKey::Arrival, EmailTemplateKey::ArrivalDay],
            Self::List => &[
                EmailTemplateKey::StayLink,
                EmailTemplateKey::Arrival,
                EmailTemplateKey::ArrivalDay,
                EmailTemplateKey::PostArrival,
            ],
            Self::Checklist => &[EmailTemplateKey::StayLink, EmailTemplateKey::PostArrival],
            Self::Alert => &[
                EmailTemplateKey::Arrival,
                EmailTemplateKey::ArrivalDay,
                EmailTemplateKey::StayModified,
            ],
        }
    }

    /// `true` when `template` renders this kind — usable in `const` context, which is how
    /// `#[email_blocks]` refuses a declaration at compile time.
    pub const fn renders_in(self, template: EmailTemplateKey) -> bool {
        let templates = self.templates();
        let mut i = 0;
        while i < templates.len() {
            if templates[i] as usize == template as usize {
                return true;
            }
            i += 1;
        }
        false
    }
}

impl crate::vocab::Vocabulary for BlockType {
    const ALL: &'static [Self] = BlockType::ALL;
    fn wire(self) -> &'static str {
        self.as_str()
    }
    fn variant(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Pairs => "Pairs",
            Self::List => "List",
            Self::Checklist => "Checklist",
            Self::Alert => "Alert",
        }
    }
}

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How an [`BlockType::Alert`] reads: something to act on, or something to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockTone {
    /// Amber: the guest has to do something differently.
    #[serde(rename = "warning")]
    Warning,
    /// Blue: good to know, nothing to do.
    #[serde(rename = "info")]
    Info,
}

/// One row of a [`BlockType::Pairs`] or [`BlockType::List`] block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlockRow {
    /// Left side: what it is.
    pub label: crate::contracts::i18n::I18nText,
    /// Right side: the value, in mono.
    pub value: crate::contracts::i18n::I18nText,
}

/// A block a module adds to a stay email.
///
/// The platform cuts every text to the design's lengths, drops a block that says nothing in the
/// guest's language, and builds the link itself from the stay page and `anchor` — which is why
/// there is no URL to set here.
///
/// ```
/// use portaki_sdk::email::EmailBlock;
///
/// let block = EmailBlock::info("Tri des déchets", "Collecte le mardi et le vendredi.")
///     .title("Sortez les bacs la veille")
///     .link("Guide du tri", "waste-recycling");
/// assert_eq!(block.block_type().as_str(), "info");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EmailBlock {
    #[serde(rename = "type")]
    block_type: BlockType,
    label: crate::contracts::i18n::I18nText,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<crate::contracts::i18n::I18nText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<crate::contracts::i18n::I18nText>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rows: Vec<BlockRow>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    items: Vec<crate::contracts::i18n::I18nText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tone: Option<BlockTone>,
    #[serde(rename = "linkLabel", skip_serializing_if = "Option::is_none")]
    link_label: Option<crate::contracts::i18n::I18nText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    anchor: Option<String>,
}

impl EmailBlock {
    fn of(block_type: BlockType, label: impl Into<crate::contracts::i18n::I18nText>) -> Self {
        Self {
            block_type,
            label: label.into(),
            title: None,
            text: None,
            rows: Vec::new(),
            items: Vec::new(),
            tone: None,
            link_label: None,
            anchor: None,
        }
    }

    /// A tile of text.
    pub fn info(
        label: impl Into<crate::contracts::i18n::I18nText>,
        text: impl Into<crate::contracts::i18n::I18nText>,
    ) -> Self {
        let mut block = Self::of(BlockType::Info, label);
        block.text = Some(text.into());
        block
    }

    /// Something that changes the stay, at the top of the zone.
    pub fn alert(
        label: impl Into<crate::contracts::i18n::I18nText>,
        text: impl Into<crate::contracts::i18n::I18nText>,
        tone: BlockTone,
    ) -> Self {
        let mut block = Self::of(BlockType::Alert, label);
        block.text = Some(text.into());
        block.tone = Some(tone);
        block
    }

    /// A tile of label/value rows; add them with [`Self::row`].
    pub fn pairs(label: impl Into<crate::contracts::i18n::I18nText>) -> Self {
        Self::of(BlockType::Pairs, label)
    }

    /// A tile of rows read as a list; add them with [`Self::row`].
    pub fn list(label: impl Into<crate::contracts::i18n::I18nText>) -> Self {
        Self::of(BlockType::List, label)
    }

    /// A tile of things to tick off; add them with [`Self::item`].
    pub fn checklist(label: impl Into<crate::contracts::i18n::I18nText>) -> Self {
        Self::of(BlockType::Checklist, label)
    }

    /// The headline, under the eyebrow. An alert has none.
    pub fn title(mut self, title: impl Into<crate::contracts::i18n::I18nText>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// One row. The platform keeps the first three.
    pub fn row(
        mut self,
        label: impl Into<crate::contracts::i18n::I18nText>,
        value: impl Into<crate::contracts::i18n::I18nText>,
    ) -> Self {
        self.rows.push(BlockRow {
            label: label.into(),
            value: value.into(),
        });
        self
    }

    /// One thing to tick off. The platform keeps the first four.
    pub fn item(mut self, item: impl Into<crate::contracts::i18n::I18nText>) -> Self {
        self.items.push(item.into());
        self
    }

    /// The one link of the block, to `anchor` on the stay page. The URL is the platform's: it is
    /// built from the stay page, and a module cannot point a guest anywhere else.
    pub fn link(
        mut self,
        label: impl Into<crate::contracts::i18n::I18nText>,
        anchor: impl Into<String>,
    ) -> Self {
        self.link_label = Some(label.into());
        self.anchor = Some(anchor.into());
        self
    }

    /// What kind of block this is.
    pub fn block_type(&self) -> BlockType {
        self.block_type
    }
}

/// The blocks a module gives one email — what an `#[email_blocks]` function returns.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EmailBlocks(Vec<EmailBlock>);

impl EmailBlocks {
    /// No block.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a block.
    pub fn push(&mut self, block: EmailBlock) {
        self.0.push(block);
    }

    /// [`push`](Self::push), chained.
    pub fn with(mut self, block: EmailBlock) -> Self {
        self.push(block);
        self
    }

    /// The blocks, in order.
    pub fn blocks(&self) -> &[EmailBlock] {
        &self.0
    }
}

impl FromIterator<EmailBlock> for EmailBlocks {
    fn from_iter<I: IntoIterator<Item = EmailBlock>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// What a module declared with `#[email_blocks]`: per email, the kinds of block it provides.
pub type DeclaredEmailBlocks = &'static [(EmailTemplateKey, &'static [BlockType])];

/// The `#[email_blocks]` of a linked module crate — native targets only, for the conformance battery.
#[doc(hidden)]
pub struct EmailBlocksDeclaration {
    /// The declaration, as written.
    pub declared: DeclaredEmailBlocks,
    /// The shim behind the generated `emailContext` query.
    pub dispatch: crate::wasm::registry::WasmHandlerFn,
}

inventory::collect!(EmailBlocksDeclaration);

/// The `#[email_blocks]` declarations linked into this binary (empty on `wasm32`).
pub fn block_declarations() -> impl Iterator<Item = &'static EmailBlocksDeclaration> {
    inventory::iter::<EmailBlocksDeclaration>.into_iter()
}

/// The body of the `emailContext` query `#[email_blocks]` generates.
///
/// Nothing is asked of the module for an email it did not declare (nor without one). Of what it
/// answers, only the kinds declared for *this* email are sent; a kind declared for no email at all
/// is an error — a missing declaration. The answer is `{ "blocks": [ … ] }`, which is the only key
/// the platform reads.
pub fn serve_blocks(
    declared: DeclaredEmailBlocks,
    ctx: crate::Context,
    args: EmailContextArgs,
    provide: impl FnOnce(crate::Context, EmailContextArgs) -> crate::Result<EmailBlocks>,
) -> crate::Result<serde_json::Value> {
    let mut out = serde_json::Map::new();
    let Some(template) = args.template_key else {
        return Ok(out.into());
    };
    let Some((_, allowed)) = declared.iter().find(|(key, _)| *key == template) else {
        return Ok(out.into());
    };
    let mut kept: Vec<EmailBlock> = Vec::new();
    for block in provide(ctx, args)?.0 {
        let kind = block.block_type;
        if !declared.iter().any(|(_, kinds)| kinds.contains(&kind)) {
            return Err(crate::PortakiError::Host(format!(
                "email_block_undeclared: a `{kind}` block is returned but not declared in #[email_blocks]"
            )));
        }
        if !allowed.contains(&kind) {
            continue;
        }
        kept.push(block);
    }
    if kept.is_empty() {
        return Ok(out.into());
    }
    if kept.len() > crate::limits::GUEST_EMAIL_BLOCKS_MAX {
        return Err(crate::PortakiError::Host(format!(
            "email_block_too_many: {} blocks for one email, the platform takes {}",
            kept.len(),
            crate::limits::GUEST_EMAIL_BLOCKS_MAX
        )));
    }
    out.insert(
        "blocks".to_string(),
        serde_json::to_value(&kept).map_err(|e| crate::PortakiError::Host(e.to_string()))?,
    );
    Ok(out.into())
}

#[cfg(test)]
mod block_tests {
    use super::*;
    use crate::contracts::i18n::I18nText;

    /// La table des types et celle de `guestEmailBlocks.perEmail` côté plateforme sont un miroir :
    /// recopiée ici, elle ne vaut que si elle est écrite une fois et lue dans les deux sens.
    #[test]
    fn every_email_with_a_zone_renders_at_least_one_kind() {
        let with_a_zone = [
            EmailTemplateKey::StayLink,
            EmailTemplateKey::Arrival,
            EmailTemplateKey::ArrivalDay,
            EmailTemplateKey::StayModified,
            EmailTemplateKey::PostArrival,
        ];
        for template in with_a_zone {
            assert!(
                BlockType::ALL.iter().any(|kind| kind.renders_in(template)),
                "{template:?} has a zone but renders no kind"
            );
            assert!(template.is_guest_stay(), "{template:?}");
        }
        // Et aucun autre e-mail n'en rend.
        for kind in BlockType::ALL {
            for template in kind.templates() {
                assert!(with_a_zone.contains(template), "{kind} in {template:?}");
            }
        }
    }

    #[test]
    fn the_catalogue_round_trips_through_the_wire() {
        use crate::vocab::Vocabulary;
        for kind in BlockType::ALL {
            assert_eq!(BlockType::from_wire(kind.as_str()), Some(*kind));
            assert_eq!(BlockType::from_variant(kind.variant()), Some(*kind));
            assert_eq!(serde_json::to_value(kind).unwrap(), kind.as_str());
        }
    }

    #[test]
    fn a_block_serialises_to_what_the_platform_parses() {
        let block = EmailBlock::pairs("Parking")
            .title("Votre place")
            .row("Place", "n° 14")
            .link("Voir le plan", "parking");

        assert_eq!(
            serde_json::to_value(&block).unwrap(),
            serde_json::json!({
                "type": "pairs",
                "label": { "fr": "Parking", "en": "Parking" },
                "title": { "fr": "Votre place", "en": "Votre place" },
                "rows": [{
                    "label": { "fr": "Place", "en": "Place" },
                    "value": { "fr": "n° 14", "en": "n° 14" },
                }],
                "linkLabel": { "fr": "Voir le plan", "en": "Voir le plan" },
                "anchor": "parking",
            })
        );
    }

    /// Le module ne donne jamais d'URL : il nomme une section de la page de séjour, la plateforme
    /// construit le lien.
    #[test]
    fn a_block_carries_no_url() {
        let json = serde_json::to_string(&EmailBlock::info("L", "t").link("V", "parking")).unwrap();
        assert!(!json.contains("linkUrl"), "{json}");
        assert!(!json.contains("http"), "{json}");
    }

    #[test]
    fn an_alert_carries_its_tone_and_no_title() {
        let alert = EmailBlock::alert("Travaux", "La rue est fermée.", BlockTone::Warning);
        let json = serde_json::to_value(&alert).unwrap();
        assert_eq!(json["tone"], "warning");
        assert!(json.get("title").is_none(), "{json}");
    }

    const DECLARED: DeclaredEmailBlocks = &[
        (EmailTemplateKey::Arrival, &[BlockType::Pairs]),
        (EmailTemplateKey::PostArrival, &[BlockType::Info]),
    ];

    fn serve(template: Option<EmailTemplateKey>, blocks: EmailBlocks) -> serde_json::Value {
        serve_blocks(
            DECLARED,
            crate::Context::default(),
            EmailContextArgs {
                template_key: template,
                ..Default::default()
            },
            |_, _| Ok(blocks),
        )
        .unwrap()
    }

    #[test]
    fn nothing_is_asked_for_an_email_the_module_did_not_declare() {
        let pairs = || EmailBlocks::new().with(EmailBlock::pairs("P").row("a", "b"));
        assert_eq!(serve(None, pairs()), serde_json::json!({}));
        assert_eq!(
            serve(Some(EmailTemplateKey::StayLink), pairs()),
            serde_json::json!({})
        );
        assert!(serve(Some(EmailTemplateKey::Arrival), pairs())["blocks"].is_array());
    }

    /// Un type déclaré ailleurs ne part pas dans cet e-mail — le miroir de ce que fait `serve`.
    #[test]
    fn a_kind_declared_for_another_email_is_left_out() {
        let answer = serve(
            Some(EmailTemplateKey::Arrival),
            EmailBlocks::new()
                .with(EmailBlock::info("I", "t"))
                .with(EmailBlock::pairs("P").row("a", "b")),
        );
        let blocks = answer["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "pairs");
    }

    #[test]
    fn a_kind_declared_nowhere_is_a_mistake_in_the_module() {
        let error = serve_blocks(
            DECLARED,
            crate::Context::default(),
            EmailContextArgs {
                template_key: Some(EmailTemplateKey::Arrival),
                ..Default::default()
            },
            |_, _| Ok(EmailBlocks::new().with(EmailBlock::checklist("C").item("x"))),
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("email_block_undeclared"),
            "{error}"
        );
    }

    #[test]
    fn more_blocks_than_the_platform_takes_is_an_error_here_not_a_silent_cut() {
        let mut blocks = EmailBlocks::new();
        for _ in 0..crate::limits::GUEST_EMAIL_BLOCKS_MAX + 1 {
            blocks.push(EmailBlock::pairs("P").row("a", "b"));
        }
        let error = serve_blocks(
            DECLARED,
            crate::Context::default(),
            EmailContextArgs {
                template_key: Some(EmailTemplateKey::Arrival),
                ..Default::default()
            },
            |_, _| Ok(blocks),
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("email_block_too_many"),
            "{error}"
        );
    }

    #[test]
    fn the_schema_lists_the_block_catalogue() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/module.v1.json")).unwrap();
        let declared = &schema["properties"]["emailBlocks"];
        let kinds: Vec<&str> = BlockType::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(
            declared["additionalProperties"]["items"]["enum"],
            serde_json::json!(kinds)
        );
        let mut emails: Vec<&str> = Vec::new();
        for kind in BlockType::ALL {
            for template in kind.templates() {
                if !emails.contains(&template.as_str()) {
                    emails.push(template.as_str());
                }
            }
        }
        let listed = declared["propertyNames"]["enum"].as_array().unwrap();
        assert_eq!(listed.len(), emails.len(), "{listed:?} vs {emails:?}");
        for email in emails {
            assert!(
                listed.iter().any(|l| l == email),
                "{email} absent du schéma"
            );
        }
    }

    #[test]
    fn i18n_text_survives_the_builder() {
        let block = EmailBlock::info(
            I18nText::new("Tri", "Waste"),
            I18nText::new("Mardi", "Tuesday"),
        );
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            json["label"],
            serde_json::json!({ "fr": "Tri", "en": "Waste" })
        );
    }
}

#[cfg(test)]
mod tests {
    use super::EmailTemplateKey;
    use std::str::FromStr;

    #[test]
    fn guest_stay_keys_round_trip() {
        for key in EmailTemplateKey::GUEST_STAY {
            assert_eq!(EmailTemplateKey::from_str(key.as_str()).unwrap(), *key);
            assert!(key.is_guest_stay());
        }
        assert!(!EmailTemplateKey::Welcome.is_guest_stay());
        assert_eq!(
            serde_json::to_value(EmailTemplateKey::ArrivalDay).unwrap(),
            "arrival-day"
        );
    }

    #[test]
    fn the_schema_lists_the_catalogue() {
        use super::EmailVar;
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/module.v1.json")).unwrap();
        let email_vars = &schema["properties"]["emailVars"];
        let vars: Vec<&str> = EmailVar::ALL.iter().map(|v| v.as_str()).collect();
        assert_eq!(
            email_vars["additionalProperties"]["items"]["enum"],
            serde_json::json!(vars)
        );
        let mut templates: Vec<&str> = Vec::new();
        for key in EmailTemplateKey::ALL {
            if EmailVar::ALL.iter().any(|v| v.renders_in(*key)) {
                templates.push(key.as_str());
            }
        }
        let mut listed: Vec<&str> = email_vars["propertyNames"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        listed.sort_unstable();
        templates.sort_unstable();
        assert_eq!(listed, templates);
    }

    #[test]
    fn email_context_args_allows_template_and_locale() {
        use super::EmailContextArgs;

        let empty = EmailContextArgs::default();
        assert!(empty.allows_template(&[EmailTemplateKey::Arrival]));
        assert_eq!(empty.locale_or("fr-FR"), "fr-FR");

        let filtered = EmailContextArgs {
            template_key: Some(EmailTemplateKey::StayLink),
            locale: Some("  en-GB  ".into()),
            ..Default::default()
        };
        assert!(!filtered.allows_template(&[EmailTemplateKey::Arrival]));
        assert!(filtered.allows_template(&[EmailTemplateKey::StayLink]));
        assert_eq!(filtered.locale_or("fr-FR"), "en-GB");
    }
}
