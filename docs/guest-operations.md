# Guest-only operations

An operation has exactly one audience. Without a flag it is **host-only**: the guest gateway
refuses it with `operation_not_guest_callable`. With `guest` it is **guest-only**: the host
gateway refuses it with `operation_not_host_callable`. There is no "both" — an operation that
serves the two sides is two operations, each with the checks its caller needs.

```rust
#[portaki_sdk::query(name = "listForStay", guest)]
pub fn list_for_stay(ctx: Context) -> Result<Vec<Submission>> { /* ... */ }

#[portaki_sdk::command(name = "submit", guest)]
pub fn submit(ctx: Context, args: SubmitArgs) -> Result<()> { /* ... */ }
```

`guest` takes no value. `portaki build` stamps it on every operation of the manifest,
`"guest": true` or `"guest": false`, never absent: a manifest without it was built by an older
SDK.

## When to open an operation

Open it only when a guest screen calls it, and no host screen does — a form the guest submits, a list the booklet shows.
Configuration, moderation, status changes, seeding and task toggles are host gestures: leave
them closed. `portaki lint` warns about a guest command named like one (`updateConfig`,
`resolve`, `updateStatus`, `seedDefaults`, `replaceItems`, `task*`).

## What an open operation owes

Assume the caller is hostile. A guest query returns the caller's stay and nothing else: filter
by `ctx.stay` (`stay_id`), never by an id taken from the arguments, and return nothing when
`ctx.stay` is `None`. A guest command writes under that same stay.

```rust
let Some(stay) = &ctx.stay else { return Ok(Vec::new()) };
// every read and write keyed by stay.stay_id
```

## Where a stay's data lives

When a stay is deleted, the platform deletes what modules kept about it — but only where it can
find it:

- **Tables**: rows of any table in your schema that has a `stay_id` column (next to
  `property_id`) are deleted where `stay_id` is that stay. Give every per-guest table a
  `stay_id` column.
- **KV**: keys under `stay:<stay_id>:` are deleted, for every module of the property. Build
  them with `host::kv::stay_key(stay.stay_id, "review")`, never by hand.

Anything stored elsewhere (a property-wide KV blob, a table without `stay_id`) outlives the
stay. Do not put guest data there.
