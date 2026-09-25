//! Empty Portaki module template.

portaki_sdk::portaki_module!(
    id = "{{MODULE_NAME}}",
    display_name_key = "module.displayName",
    description_key = "module.description",
    author = "{{AUTHOR_NAME}}",
    author_url = "https://github.com/TODO",
    module_type = ModuleType::Community,
    icon = IconName::Grid,
    maturity = Maturity::Beta,
);

// Add `guest/` and `host/` surface modules when the module gains UI. Each `#[surface(…, id =
// "home.card")]` also declares the const `HOME_CARD`, and `#[query]` / `#[command]` theirs.
