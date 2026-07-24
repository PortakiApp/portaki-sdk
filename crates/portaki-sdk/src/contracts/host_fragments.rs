//! Stable host-owned UI fragment ids for SDUI composition.
//!
//! Modules embed [`crate::sdui::primitives::HostFragment`] or open
//! [`crate::sdui::action::Action::OpenHostFragment`] with these ids.
//! Shells (guest / dashboard / mobile) implement each fragment once —
//! never branch on module ids.

use crate::ids::FragmentId;

/// French police registration form (art. R.611-42 CESEDA).
///
/// Guest: Accueil task row + full-page overlay. Host stay: regulatory card.
/// Persistence and emails stay platform regulatory claims.
pub const POLICE_FORM: FragmentId = FragmentId::new("regulatory.police-form");

/// Catalog of well-known host fragments.
pub const ALL: &[FragmentId] = &[POLICE_FORM];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn police_form_wire_name() {
        assert_eq!(POLICE_FORM.as_str(), "regulatory.police-form");
    }
}
