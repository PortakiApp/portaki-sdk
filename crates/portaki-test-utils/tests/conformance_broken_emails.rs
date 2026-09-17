//! Email commands and `emailContext` must compose around a stay; a manifest email must be served.

mod common;

use common::{assert_reports, broken, failing, passing};
use portaki_sdk::email::{EmailContextArgs, EmailTemplateKey};
use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::Text;

#[portaki_sdk::surface(guest, id = "explore.detail")]
pub fn render_explore_detail(_ctx: GuestContext) -> Surface {
    Surface::new(Text::new().text("i18n:guest.empty.title"))
}

#[portaki_sdk::command(name = "sendReminder")]
pub fn send_reminder(ctx: Context) -> Result<()> {
    let checkin = ctx
        .stay
        .and_then(|stay| stay.checkin_at)
        .expect("a check-in");
    let now = host::time::now()?;
    // Past check-out the stay is over, and this code did not see it coming.
    assert!(now < checkin, "reminder sent after check-in");
    Ok(())
}

#[portaki_sdk::query(name = "emailContext")]
pub fn email_context(_ctx: Context, args: EmailContextArgs) -> Result<serde_json::Value> {
    if args.template_key == Some(EmailTemplateKey::Otp) {
        panic!("no snippet for otp");
    }
    Ok(serde_json::json!({}))
}

#[test]
fn an_email_command_that_panics_around_a_stay_is_reported() {
    let findings = failing("emails", passing().check_emails());

    assert_reports(
        &findings,
        &[
            "email `reminder`",
            "command `sendReminder`",
            "after check-out",
            "reminder sent after check-in",
        ],
    );
    assert!(
        !findings
            .problems()
            .iter()
            .any(|p| p.contains("sendReminder") && p.contains("before check-in")),
        "{findings}"
    );
}

#[test]
fn an_email_context_that_panics_for_one_template_names_it() {
    let findings = failing("emails", passing().check_emails());

    assert_reports(
        &findings,
        &["query `emailContext`", "`otp`", "no snippet for otp"],
    );
    assert!(
        !findings.problems().iter().any(|p| p.contains("`welcome`")),
        "{findings}"
    );
}

#[test]
fn a_manifest_email_without_its_command_is_reported() {
    let findings = failing("emails", broken().check_emails());

    assert_reports(&findings, &["email `ghost`", "`sendGhost`", "no #[command"]);
}
