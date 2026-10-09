//! `amenities.list` — the equipment a module adds to the property's amenities.
//!
//! A module declares it provides amenities with
//! `#[portaki_sdk::capability(provided, id = "amenities.provide")]` and exports the query
//! [`AMENITIES_LIST`] (`portaki lint` refuses the first without the second). The platform calls it,
//! with no args, when the module's config is published and when the module is activated; the
//! module answers from its published config. Ids unknown to the amenities catalogue are ignored,
//! `detail` is cut to 40 characters, and nothing else is read: the schema
//! (`contracts/amenities-list.v1.json`) admits no other field, so no usage data can leave.
//!
//! ```
//! use portaki_sdk::contracts::amenities::{self, AmenitiesList};
//!
//! let answer: AmenitiesList = amenities::list([
//!     amenities::amenity("wifi").detail("fibre 1 Gb/s"),
//!     amenities::amenity("ev-charger"),
//! ]);
//! let wire = serde_json::to_value(&answer).unwrap();
//! assert_eq!(wire["amenities"][0]["detail"], "fibre 1 Gb/s");
//! assert!(wire["amenities"][1]["detail"].is_null());
//! ```

use serde::{Deserialize, Serialize};

use crate::capability::CapabilityId;
use crate::ids::OperationName;

/// Capability id providers declare (`amenities.provide`).
pub const CAPABILITY: CapabilityId = CapabilityId::AmenitiesProvide;

/// Query name (`amenities.list`). No args: the module reads its published config.
pub const AMENITIES_LIST: OperationName = OperationName::new("amenities.list");

/// Answer of [`AMENITIES_LIST`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AmenitiesList {
    /// What the module provides, in the order it wants them shown.
    pub amenities: Vec<ProvidedAmenity>,
}

/// One amenity the module provides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProvidedAmenity {
    /// Id in the platform's amenities catalogue (`wifi`, `ev-charger`…).
    pub id: String,
    /// A precision shown next to it (`fibre 1 Gb/s`) — at most 40 characters are kept.
    #[serde(default)]
    pub detail: Option<String>,
}

/// An answer listing `amenities`.
pub fn list(amenities: impl IntoIterator<Item = ProvidedAmenity>) -> AmenitiesList {
    AmenitiesList {
        amenities: amenities.into_iter().collect(),
    }
}

/// The amenity of catalogue id `id`, without detail.
pub fn amenity(id: impl Into<String>) -> ProvidedAmenity {
    ProvidedAmenity {
        id: id.into(),
        detail: None,
    }
}

impl ProvidedAmenity {
    /// Adds a precision shown next to the amenity.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amenities_wire_names() {
        assert_eq!(CAPABILITY.as_str(), "amenities.provide");
        assert_eq!(AMENITIES_LIST.as_str(), "amenities.list");
        let back: AmenitiesList =
            serde_json::from_value(serde_json::json!({ "amenities": [{ "id": "wifi" }] })).unwrap();
        assert_eq!(back, list([amenity("wifi")]));
    }
}
