//! Host dashboard surfaces.

use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Card, Field, FieldHint, Form, Page, Stack, TextInput};

use crate::config::ModuleConfig;

/// The settings the host fills in, inside the module sheet.
///
/// No page title and no Save button: the sheet draws both, and posts the fields below as
/// `updateConfig`, which the platform handles against `ModuleConfig`. A surface that drew its own
/// would show two. A config that does not load is an error, not an empty form a Save would
/// write over what the host had.
#[portaki_sdk::surface(
    host,
    id = "main",
    placement = HostPlacement::PropertyModuleSheet,
    label_key = "nav.main",
    icon = IconName::Grid
)]
pub fn render_host_main(ctx: HostContext) -> Result<Surface> {
    let config = ModuleConfig::load(&ctx)?;

    let children: Vec<Component> = vec![
        Card::new()
            .title("i18n:host.section.greeting")
            .subtitle("i18n:host.section.greeting.help")
            .children(vec![
                Field::new()
                    .name("greeting")
                    .label("i18n:host.greeting.label")
                    .child(
                        TextInput::new()
                            .name("greeting")
                            .value(config.greeting)
                            .placeholder("i18n:host.greeting.placeholder"),
                    )
                    .into(),
                FieldHint::new().text("i18n:host.greeting.hint").into(),
            ])
            .into(),
    ];

    // The dispatcher stamps the declared id (`main`, the const `MAIN`) on the surface.
    Ok(Surface::new(
        Page::new().child(Form::new().child(Stack::new().gap(16.0).children(children))),
    ))
}
