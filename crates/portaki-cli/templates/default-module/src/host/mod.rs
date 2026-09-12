//! Host dashboard surfaces.

use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Stack, Text};

use crate::ids::HOST_MAIN;

#[portaki_sdk::surface(host, id = "main")]
pub fn render_host_main(ctx: HostContext) -> Surface {
    let _ = ctx;
    Surface::new(Stack::new().child(Text::new())).with_id(HOST_MAIN)
}
