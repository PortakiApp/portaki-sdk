# Guest-callable operations

Every query and command is closed to guests unless the module opens it. The guest gateway
refuses a guest call to a closed operation with `operation_not_guest_callable`; the host
dashboard is not affected.

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

Open it only when a guest screen calls it — a form the guest submits, a list the booklet shows.
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
