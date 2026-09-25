//! `#[surface]`, `#[query]` and `#[command]` declare their id as a const, and the dispatcher
//! stamps a rendered surface with the declared id — no `ids.rs`, no `.with_id(…)`.

use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::Text;
use portaki_sdk::wasm::registry::declarations;

#[surface(host, id = "main")]
pub fn render_main(_ctx: HostContext) -> Result<Surface> {
    Ok(Surface::new(Text::new().text("i18n:host.title")))
}

#[surface(host, id = "stats-detail")]
pub fn render_stats(_ctx: HostContext) -> Surface {
    Surface::new(Text::new()).with_id(SurfaceId::new("kept"))
}

#[query(name = "listSources")]
pub fn list_sources(_ctx: Context) -> Result<Vec<String>> {
    Ok(Vec::new())
}

#[command(name = "applyFeeds")]
pub fn apply_feeds(_ctx: Context) -> Result<()> {
    Ok(())
}

#[test]
fn the_ids_are_consts_next_to_their_handler() {
    assert_eq!(MAIN, SurfaceId::new("main"));
    assert_eq!(STATS_DETAIL.as_str(), "stats-detail");
    assert_eq!(LIST_SOURCES, OperationName::new("listSources"));
    assert_eq!(APPLY_FEEDS.as_str(), "applyFeeds");
}

#[test]
fn the_dispatcher_stamps_the_declared_id_unless_one_is_set() {
    let render = |name: &str| {
        let declaration = declarations().find(|d| d.name == name).expect("declared");
        (declaration.dispatch)(Context::default(), serde_json::Value::Null).unwrap()
    };
    assert_eq!(render("main")["id"], "main");
    assert_eq!(render("stats-detail")["id"], "kept");
}
