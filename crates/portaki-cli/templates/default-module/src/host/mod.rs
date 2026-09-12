//! Host dashboard surfaces.

use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Card, Field, FieldHint, Form, Page, Stack, TextInput};

use crate::config::load_config;
use crate::ids::HOST_MAIN;

/// The settings the host fills in, inside the module sheet.
///
/// No page title and no Save button: the sheet draws both, and posts the fields below to the
/// `updateConfig` command. A surface that drew its own would show two.
#[portaki_sdk::surface(host, id = "main")]
pub fn render_host_main(_ctx: HostContext) -> Surface {
    let config = load_config().unwrap_or_default();

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

    Surface::new(Page::new().child(Form::new().child(Stack::new().gap(16.0).children(children))))
        .with_id(HOST_MAIN)
}
