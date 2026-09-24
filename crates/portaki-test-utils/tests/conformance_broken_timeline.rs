//! A `taskToggle` that ticks a photo-required item without a photo fails `contracts`.

mod common;

use common::{assert_reports, failing, passing};
use portaki_sdk::contracts::i18n::I18nText;
use portaki_sdk::contracts::timeline::{
    self, TaskToggleArgs, TimelineTaskItem, TimelineTasks, TimelineTasksArgs,
};
use portaki_sdk::prelude::*;

#[portaki_sdk::query(name = "timelineTasks")]
pub fn timeline_tasks(_ctx: Context, args: TimelineTasksArgs) -> Result<TimelineTasks> {
    let stay = &args.stays[0];
    let task = timeline::task(
        format!("cleaning:{}", stay.id),
        stay.check_out,
        args.property_id,
        I18nText::new("Ménage", "Cleaning"),
        I18nText::new("Après le départ", "After check-out"),
    )
    .items(vec![TimelineTaskItem::new(
        "meters",
        I18nText::new("Compteurs", "Meters"),
    )
    .photo_required()]);
    Ok(TimelineTasks { tasks: vec![task] })
}

/// Ticks whatever it is asked to.
#[portaki_sdk::command(name = "taskToggle")]
pub fn task_toggle(_ctx: Context, _args: TaskToggleArgs) -> Result<()> {
    Ok(())
}

#[test]
fn ticking_a_photo_required_item_without_a_photo_must_be_refused() {
    let findings = failing("contracts", passing().check_contracts());

    assert_reports(
        &findings,
        &[
            "command `taskToggle`",
            "item `meters`",
            "was accepted",
            "photo_required",
        ],
    );
}
