//! Closed vocabularies of the module manifest — typed, so a misspelt value does not compile.
//!
//! Each enum carries the exact wire string the platform reads. The manifest macros take them by
//! path (`placement = HostPlacement::PropertyStatsCard`): the compiler checks the variant exists,
//! and `portaki build` writes [`Vocabulary::wire`] for it. There is no other list to keep in sync.
//!
//! ```
//! use portaki_sdk::vocab::{HostPlacement, Vocabulary};
//!
//! assert_eq!(HostPlacement::PropertyStatsCard.wire(), "property-stats-card");
//! assert_eq!(
//!     HostPlacement::from_variant("PropertyStatsCard"),
//!     Some(HostPlacement::PropertyStatsCard)
//! );
//! ```

use serde::{Deserialize, Serialize};

/// A closed vocabulary: a Rust variant per wire value.
pub trait Vocabulary: Sized + Copy + 'static {
    /// Every value, in declaration order.
    const ALL: &'static [Self];
    /// The string the platform reads.
    fn wire(self) -> &'static str;
    /// The Rust name of the variant — what a manifest macro receives.
    fn variant(self) -> &'static str;
    /// The value whose Rust variant is `name`.
    fn from_variant(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|value| value.variant() == name)
    }
    /// The value whose wire string is `wire` — for data stored as text, read back at render time.
    fn from_wire(wire: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|value| value.wire() == wire)
    }
}

/// Declares an enum whose variants each map to one wire string.
macro_rules! vocabulary {
    ($(#[$doc:meta])* $name:ident { $($(#[$vdoc:meta])* $variant:ident = $wire:literal,)+ }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $($(#[$vdoc])* #[doc = concat!("On the wire: `", $wire, "`.")] #[serde(rename = $wire)] $variant,)+
        }

        impl Vocabulary for $name {
            const ALL: &'static [Self] = &[$(Self::$variant,)+];
            fn wire(self) -> &'static str {
                match self { $(Self::$variant => $wire,)+ }
            }
            fn variant(self) -> &'static str {
                match self { $(Self::$variant => stringify!($variant),)+ }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.wire())
            }
        }
    };
}

vocabulary! {
    /// Where the host dashboard shows a surface (`#[surface(host, placement = …)]`).
    HostPlacement {
        /// A tab of the property workspace — the module's editor.
        PropertyWorkspaceTab = "property-workspace-tab",
        /// The module's sheet, opened from the property's module list.
        PropertyModuleSheet = "property-module-sheet",
        /// A card of the property's statistics page.
        PropertyStatsCard = "property-stats-card",
        /// The detail page behind a statistics card.
        PropertyStatsDetail = "property-stats-detail",
        /// A task on the workspace timeline.
        WorkspaceTimelineTask = "workspace-timeline-task",
        /// A block of the stay detail page.
        StayDetail = "stay-detail",
        /// An action of the stay detail page.
        StayAction = "stay-action",
    }
}

vocabulary! {
    /// What a guest route is for in the booklet (`#[surface(guest, role = …)]`).
    GuestRole {
        /// A card of the booklet home.
        Card = "card",
        /// A formality to complete before arrival.
        ArrivalFormality = "arrival-formality",
        /// Shown after checkout.
        PostStay = "post-stay",
        /// Shown in the "upcoming" section before arrival.
        Upcoming = "upcoming",
    }
}

vocabulary! {
    /// How far a module is along (`portaki_module!(maturity = …)`).
    Maturity {
        /// Works end to end; listed first.
        Stable = "stable",
        /// Usable, still moving.
        Beta = "beta",
    }
}

vocabulary! {
    /// Who a module works for (`portaki_module!(audience = …)`).
    ModuleAudience {
        /// Shown in the guest booklet.
        Guest = "guest",
        /// Host-only — never shown to guests, never placed on the booklet.
        Host = "host",
    }
}

vocabulary! {
    /// Who publishes a module (`portaki_module!(module_type = …)`), also its `author.type`.
    ModuleType {
        /// Published by Portaki.
        Official = "official",
        /// Published by someone else.
        Community = "community",
    }
}

vocabulary! {
    /// When a declared email goes out (`#[email(trigger = …)]`).
    EmailTrigger {
        /// When the command it sits on runs — the default for a command.
        ModuleCommand = "moduleCommand",
        /// At an offset from check-in.
        RelativeToCheckIn = "relativeToCheckIn",
        /// At an offset from check-out.
        RelativeToCheckOut = "relativeToCheckOut",
        /// During the scheduled feed sync's `applyQuery`.
        OnApplyFeeds = "onApplyFeeds",
    }
}

vocabulary! {
    /// A condition the platform skips a declared email on (`#[email(skip_when = …)]`).
    SkipWhen {
        /// The stay has no guest email address.
        GuestEmailMissing = "guest.email.missing",
        /// The stay was cancelled.
        StayCancelled = "stay.cancelled",
    }
}

vocabulary! {
    /// A host fragment a guest route embeds (`#[surface(guest, embeds = …)]`).
    HostFragmentId {
        /// The police registration form — [`crate::contracts::host_fragments::POLICE_FORM`].
        PoliceForm = "regulatory.police-form",
    }
}

vocabulary! {
    /// The dashboard design that edits a module (`#[surface(host, design_id = …)]`).
    DesignId {
        /// access-guide.
        AccessEditorV1 = "access-editor-v1",
        /// appliances.
        AppliancesEditorV1 = "appliances-editor-v1",
        /// checklist.
        ChecklistEditorV1 = "checklist-editor-v1",
        /// consumables.
        ConsumablesEditorV1 = "consumables-editor-v1",
        /// emergency-contacts.
        EmergencyEditorV1 = "emergency-editor-v1",
        /// ev-parking.
        EvparkingEditorV1 = "evparking-editor-v1",
        /// facility-hours.
        FacilityEditorV1 = "facility-editor-v1",
        /// guest-reviews.
        ReviewsEditorV1 = "reviews-editor-v1",
        /// local-guide.
        GuideEditorV1 = "guide-editor-v1",
        /// nuki.
        NukiEditorV1 = "nuki-editor-v1",
        /// pre-arrival-form.
        PrearrivalEditorV1 = "prearrival-editor-v1",
        /// rules.
        RulesEditorV1 = "rules-editor-v1",
        /// sections.
        SectionsEditorV1 = "sections-editor-v1",
        /// waste-recycling.
        WasteEditorV1 = "waste-editor-v1",
    }
}

vocabulary! {
    /// An icon token the shells render (`.icon(IconName::Key)`, `#[surface(icon = …)]`).
    ///
    /// The union of what the host dashboard and the guest booklet draw. `contracts/sdui_types.json`
    /// lists it for both, so each shell can check it renders every token.
    IconName {
        Ban = "ban",
        Bell = "bell",
        Building = "building",
        Calendar = "calendar",
        Car = "car",
        Check = "check",
        CheckCircle = "check-circle",
        ChevronRight = "chevron-right",
        CircleX = "circle-x",
        Clipboard = "clipboard",
        ClipboardList = "clipboard-list",
        Clock = "clock",
        ClockCircle = "clock-circle",
        Cloud = "cloud",
        CloudFog = "cloud-fog",
        CloudLightning = "cloud-lightning",
        CloudOff = "cloud-off",
        CloudRain = "cloud-rain",
        CloudSnow = "cloud-snow",
        CloudSun = "cloud-sun",
        DangerTriangle = "danger-triangle",
        Dots = "dots",
        Droplets = "droplets",
        FileText = "file-text",
        Fingerprint = "fingerprint",
        Gauge = "gauge",
        Gift = "gift",
        Grid = "grid",
        Guests = "guests",
        Handshake = "handshake",
        HeartHandshake = "heart-handshake",
        Home = "home",
        Info = "info",
        InfoCircle = "info-circle",
        Key = "key",
        Link = "link",
        List = "list",
        ListChecks = "list-checks",
        Lock = "lock",
        Logout = "logout",
        Mail = "mail",
        MapPin = "map-pin",
        Message = "message",
        MessageCircle = "message-circle",
        Minus = "minus",
        MoreHorizontal = "more-horizontal",
        No = "no",
        Noise = "noise",
        Ok = "ok",
        Package = "package",
        PackageSearch = "package-search",
        Parking = "parking",
        Paw = "paw",
        PawPrint = "paw-print",
        Pets = "pets",
        Phone = "phone",
        Plug = "plug",
        Plus = "plus",
        Quiet = "quiet",
        Recycle = "recycle",
        Refresh = "refresh",
        Scale = "scale",
        Search = "search",
        SearchX = "search-x",
        Send = "send",
        Sliders = "sliders",
        Smile = "smile",
        Sparkles = "sparkles",
        Star = "star",
        Sun = "sun",
        Thermometer = "thermometer",
        Ticket = "ticket",
        Train = "train",
        TriangleAlert = "triangle-alert",
        User = "user",
        Users = "users",
        Volume2 = "volume-2",
        VolumeX = "volume-x",
        Wifi = "wifi",
        Wind = "wind",
        X = "x",
        Zap = "zap",
    }
}

impl Vocabulary for crate::host::email::EmailAudience {
    const ALL: &'static [Self] = &[Self::Guest, Self::Host, Self::PropertyEligibleGuests];
    fn wire(self) -> &'static str {
        match self {
            Self::Guest => "guest",
            Self::Host => "host",
            Self::PropertyEligibleGuests => "propertyEligibleGuests",
        }
    }
    fn variant(self) -> &'static str {
        match self {
            Self::Guest => "Guest",
            Self::Host => "Host",
            Self::PropertyEligibleGuests => "PropertyEligibleGuests",
        }
    }
}

/// The wire value of `variant` in the vocabulary named `vocabulary` — what `portaki build` writes
/// for a typed macro argument. `None` for an unknown vocabulary or variant.
pub fn wire_of(vocabulary: &str, variant: &str) -> Option<&'static str> {
    fn find<V: Vocabulary>(variant: &str) -> Option<&'static str> {
        V::from_variant(variant).map(V::wire)
    }
    match vocabulary {
        "HostPlacement" => find::<HostPlacement>(variant),
        "GuestRole" => find::<GuestRole>(variant),
        "Maturity" => find::<Maturity>(variant),
        "ModuleAudience" => find::<ModuleAudience>(variant),
        "ModuleType" => find::<ModuleType>(variant),
        "EmailTrigger" => find::<EmailTrigger>(variant),
        "SkipWhen" => find::<SkipWhen>(variant),
        "HostFragmentId" => find::<HostFragmentId>(variant),
        "DesignId" => find::<DesignId>(variant),
        "IconName" => find::<IconName>(variant),
        "EmailAudience" => find::<crate::host::email::EmailAudience>(variant),
        "EmailVar" => find::<crate::email::EmailVar>(variant),
        "EmailTemplateKey" => crate::email::EmailTemplateKey::ALL
            .iter()
            .find(|key| format!("{key:?}") == variant)
            .map(|key| key.as_str()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wires<V: Vocabulary>() -> Vec<&'static str> {
        V::ALL.iter().map(|v| v.wire()).collect()
    }

    fn schema_enum(pointer: &str) -> Vec<String> {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/module.v1.json")).unwrap();
        serde_json::from_value(schema.pointer(pointer).cloned().expect(pointer)).unwrap()
    }

    /// The schema's closed lists and the enums say the same thing.
    #[test]
    fn the_enums_match_the_schema() {
        assert_eq!(
            wires::<HostPlacement>(),
            schema_enum("/$defs/hostSurface/properties/type/enum")
        );
        assert_eq!(
            wires::<GuestRole>(),
            schema_enum("/$defs/guestSurface/properties/role/enum")
        );
        assert_eq!(
            wires::<Maturity>(),
            schema_enum("/properties/maturity/enum")
        );
        assert_eq!(
            wires::<ModuleAudience>(),
            schema_enum("/properties/audience/enum")
        );
    }

    /// Serde writes the same string as `wire`, both ways.
    #[test]
    fn serde_and_wire_agree() {
        fn check<
            V: Vocabulary + Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
        >() {
            for value in V::ALL {
                let json = serde_json::to_value(value).unwrap();
                assert_eq!(json, value.wire());
                assert_eq!(&serde_json::from_value::<V>(json).unwrap(), value);
            }
        }
        check::<HostPlacement>();
        check::<GuestRole>();
        check::<Maturity>();
        check::<ModuleAudience>();
        check::<ModuleType>();
        check::<EmailTrigger>();
        check::<SkipWhen>();
        check::<HostFragmentId>();
        check::<DesignId>();
        check::<IconName>();
        check::<crate::host::email::EmailAudience>();
    }

    #[test]
    fn a_macro_argument_resolves_by_vocabulary_and_variant() {
        assert_eq!(wire_of("HostPlacement", "StayAction"), Some("stay-action"));
        assert_eq!(
            wire_of("EmailAudience", "PropertyEligibleGuests"),
            Some("propertyEligibleGuests")
        );
        assert_eq!(
            wire_of("SkipWhen", "GuestEmailMissing"),
            Some("guest.email.missing")
        );
        assert_eq!(wire_of("EmailVar", "WifiName"), Some("wifiName"));
        assert_eq!(
            wire_of("EmailTemplateKey", "ArrivalDay"),
            Some("arrival-day")
        );
        assert_eq!(wire_of("HostPlacement", "Nope"), None);
        assert_eq!(wire_of("Nope", "StayAction"), None);
        assert_eq!(
            HostFragmentId::PoliceForm.wire(),
            crate::contracts::host_fragments::POLICE_FORM.as_str()
        );
    }
}
