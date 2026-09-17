//! A command that panics on empty input fails the `operations` check; one that errors does not.

mod common;

use common::{assert_reports, failing, passing};
use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::Text;

#[portaki_sdk::surface(guest, id = "explore.detail")]
pub fn render_explore_detail(_ctx: GuestContext) -> Surface {
    Surface::new(Text::new().text("i18n:guest.empty.title"))
}

#[derive(Debug, Default, Deserialize)]
pub struct ReplaceItemsArgs {
    #[serde(default)]
    pub items: Vec<String>,
}

#[portaki_sdk::command(name = "replaceItems")]
pub fn replace_items(_ctx: Context, args: ReplaceItemsArgs) -> Result<()> {
    let first = &args.items[0];
    host::kv::set("first", first.as_bytes(), None)
}

#[derive(Debug, Deserialize)]
pub struct StrictArgs {
    pub required: String,
}

#[portaki_sdk::command(name = "strict")]
pub fn strict(_ctx: Context, args: StrictArgs) -> Result<()> {
    Err(PortakiError::Host(args.required))
}

#[portaki_sdk::query(name = "guestName")]
pub fn guest_name(ctx: Context) -> Result<String> {
    // Fine in a guest mock, a panic in a host one.
    Ok(ctx.guest.expect("a guest").session_id.to_string())
}

#[test]
fn a_command_that_panics_on_empty_input_is_reported() {
    let findings = failing("operations", passing().check_operations());

    assert_reports(
        &findings,
        &[
            "command `replaceItems` (replace_items)",
            "guest mock",
            "index out of bounds",
        ],
    );
    assert_reports(&findings, &["command `replaceItems`", "host mock"]);
}

#[test]
fn a_query_is_dispatched_in_both_shells() {
    let findings = failing("operations", passing().check_operations());

    assert_reports(&findings, &["query `guestName`", "host mock", "a guest"]);
    assert!(
        !findings
            .problems()
            .iter()
            .any(|p| p.contains("guestName") && p.contains("guest mock")),
        "{findings}"
    );
}

#[test]
fn an_error_on_empty_input_is_an_answer_not_a_finding() {
    let findings = failing("operations", passing().check_operations());

    assert!(
        !findings.problems().iter().any(|p| p.contains("strict")),
        "{findings}"
    );
}
