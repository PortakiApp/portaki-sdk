//! Empty Portaki module template.

use portaki_sdk::prelude::*;

mod ids;

portaki_sdk::portaki_module!(
    id = "{{MODULE_NAME}}",
    display_name_key = "module.displayName",
    description_key = "module.description",
    author = "Portaki",
);

// Add `guest/` and `host/` surface modules when the module gains UI.
// Boundary ids live in `ids.rs` (SDK 2.1.0+ typed catalogs).
