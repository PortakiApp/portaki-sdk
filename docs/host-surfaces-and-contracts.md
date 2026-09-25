# Host surfaces and typed contracts

Where a module shows up in the host dashboard, and what each placement commits it to. Every
promise below is checked by `portaki_test_utils::conformance!()` (test `contracts`).

Related: [module-layout.md](./module-layout.md), [typed-ids.md](./typed-ids.md).

---

## Host surfaces (`hostSurfaces[].type`)

A closed list since 6.13: any other value fails the `manifest` check.

| Type | What the dashboard does | What the module serves |
|------|-------------------------|------------------------|
| `property-workspace-tab` | Full-screen tab under `/listings/[id]/<pathSegment>` | host `#[surface]` |
| `property-module-sheet` | Cards in the module configure sheet | host `#[surface]` |
| `property-stats-card` | Tile in the property statistics; `label` and `icon` name it | query `statsSummary` with `key = pathSegment` |
| `property-stats-detail` | Page opened from the tile of the same `pathSegment` | host `#[surface]` whose id is `pathSegment`, reading `input.periodDays` |
| `workspace-timeline-task` | Dated tasks between the stays of « À venir » | query `timelineTasks`, commands `taskToggle` / `taskComplete` |
| `stay-detail` | Embed on the stay page (`input.stayId`) | host `#[surface]` |
| `stay-action` | Stay page button opening a modal | host `#[surface]` |

A module may declare several `property-stats-card` / `property-stats-detail` pairs (`checklist`:
`checklist` and `cleaning`).

## Displayed text

Every string the dashboard shows from these contracts is an `I18nText`
(`portaki_sdk::contracts::i18n`): a flat `{ "fr": "…", "en": "…", "de": "…" }`, `fr` and `en`
required.

## Contracts

One JSON Schema each in `contracts/`, shipped in the SDK contracts bundle; the Rust types in
`portaki_sdk::contracts` serialize to them (tested).

### `statsSummary` — `contracts/stats-summary.v1.json`

Args `StatsSummaryArgs { propertyId, period: 30 | 90 | 365, key }`. Answer `StatsSummary`:

```json
{ "value": "5", "label": { "fr": "signalements", "en": "reports" },
  "attention": { "level": "action", "text": { "fr": "2 en cours", "en": "2 open" } },
  "trend": { "delta": "+2", "direction": "up", "good": false } }
```

`value` is at most 12 characters; `attention` and `trend` are `null` when there is nothing to
show. The platform caches each answer 15 minutes and drops it on any event the module emits for
the property. It must answer within 300 ms on the conformance fixture.

```rust
use portaki_sdk::contracts::{i18n::I18nText, stats};

#[query(name = "statsSummary")]
fn stats_summary(_ctx: Context, args: stats::StatsSummaryArgs) -> Result<stats::StatsSummary> {
    let period = args.period(); // 30, 90 or 365 — anything else reads 30
    Ok(stats::summary("5", I18nText::new("signalements", "reports"))
        .attention(stats::AttentionLevel::Action, I18nText::new("2 en cours", "2 open")))
}
```

The detail surface reads the same window with `ctx.stats_period()` (`input.periodDays`);
`period.since(now)` starts it, `period.window(&ctx.lang())` writes it (« sur 90 jours »).

### `timelineTasks`, `taskToggle`, `taskComplete` — `contracts/timeline-tasks.v1.json`

Args `TimelineTasksArgs { propertyId, from, to, stays: [{ id, checkIn, checkOut, guestName,
status }] }` — stays sorted, one past each end of the window. Answer `TimelineTasks { tasks }`,
each `TimelineTask { id, at, dueAt?, stayId?, propertyId, title, context, assignee?: { name,
role }, items: [{ id, label, done, photoRequired, photo? }] }`. Dates are RFC 3339 instants;
`items` is never empty; `id` is stable across computations (`cleaning:<stayId>`) since only the
ticked state is stored.

- `taskToggle` — `TaskToggleArgs { propertyId, taskId, itemId, done, photo? }`.
- `taskComplete` — `TaskCompleteArgs { propertyId, taskId }`, ticks every item.
- Both refuse with the code `photo_required` (`timeline::PHOTO_REQUIRED`) when an item requires a
  photo and none is given: `TimelineTaskItem::check_toggle(done, photo)` does it.
- After either, emit `checklist.task-updated` (`timeline::TASK_UPDATED`) with `TaskUpdated {
  propertyId, stayId?, taskId, done, total, assigneeName? }`.

```rust
use portaki_sdk::contracts::{i18n::I18nText, timeline};

let task = timeline::task(format!("cleaning:{}", stay.id), stay.check_out, args.property_id,
        I18nText::new("Ménage", "Cleaning"), I18nText::new("Avant l'arrivée de Liam", "Before Liam arrives"))
    .stay(stay.id)
    .assignee("Julie Martin", I18nText::new("Ménage", "Cleaning"))
    .items(vec![timeline::TimelineTaskItem::new("floors", I18nText::new("Sols", "Floors")).photo_required()]);
```

### `publishReadiness` — `contracts/publish-readiness.v1.json`

Optional, and reserved to **conditional** rules. An empty config field needs none: declare it
`required` or `recommended` with `#[portaki_sdk::config]` and the platform adds its item (id
`config.<key>`) itself. The query stays for what the schema cannot say — "a code once the smart
lock is off" — and its items are merged with the platform's. Args `{ propertyId }`, in a host context reading the draft KV. Answer
`PublishReadiness { items: [PublishCheck { id, level: "required" | "recommended" | "optional",
ok, label, hint }] }`. A `required` item not `ok` blocks the property publication; the others never
do. A module without the query adds nothing.

## SDUI primitives for these screens

| Primitive | Fields | Notes |
|-----------|--------|-------|
| `EditableList` | `name`, `items: [EditableListItem { id?, label, labelEn?, photo?, checked? }]`, `bilingual?`, `photoToggle?`, `checkbox?`, `addLabel`, `placeholder?` | Reorderable rows, ✕ removes; submits the rows as JSON under `name` |
| `FeedItem` | `title`, `tag?`, `meta?`, `date?`, `dotTone: Tone`, `status: FeedStatus { label, tone }`, `action?` | One row of a stats detail feed; pill and chevron never wrap |

`Select` in a rendered tree must have options, and a `value` among them (or empty): the
`surfaces` check refuses the others.
