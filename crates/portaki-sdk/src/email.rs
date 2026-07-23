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

/// Arguments passed to module `emailContext` handlers by the gateway.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmailContextArgs {
    /// Template being composed; modules filter on guest-stay keys.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_key: Option<EmailTemplateKey>,
    /// Stay identifier when composing a guest-stay email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stay_id: Option<String>,
}

/// Documented contribution fields modules may return into guest-stay emails.
///
/// Modules typically serialize a subset as `serde_json::Value` / a dedicated
/// response struct. Field names match gateway merge keys (camelCase).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmailContextContribution {
    /// Weather one-liner for arrival / arrival-day templates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weather_summary: Option<String>,
    /// Access-guide callout block for arrival emails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrival_callout: Option<serde_json::Value>,
    /// Rules / house notes snippet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_rules_summary: Option<String>,
    /// Checklist reminder snippet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checklist_summary: Option<String>,
    /// Waste / recycling tip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waste_tip: Option<String>,
    /// Appliance quick tip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appliance_tip: Option<String>,
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
}
