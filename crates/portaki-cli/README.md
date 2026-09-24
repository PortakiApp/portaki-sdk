<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-cli</h1>

<p align="center">
  <strong>Command-line toolchain for Portaki Wasm modules</strong><br>
  Binary name <code>portaki</code> — init, build, lint, test, and OCI publish.
</p>

<p align="center">
  <a href="https://crates.io/crates/portaki-cli"><img src="https://img.shields.io/crates/v/portaki-cli.svg" alt="crates.io"></a>
  <a href="https://docs.rs/portaki-cli"><img src="https://img.shields.io/docsrs/portaki-cli" alt="docs.rs"></a>
  <a href="https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75+-dea584?logo=rust&logoColor=white" alt="Rust 1.75+"></a>
  <a href="https://extism.org/"><img src="https://img.shields.io/badge/Extism-Wasm-7C3AED" alt="Extism"></a>
  <a href="https://portaki.app"><img src="https://img.shields.io/badge/site-portaki.app-f59e0b" alt="portaki.app"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#commands">Commands</a> ·
  <a href="#typical-workflow">Workflow</a> ·
  <a href="#related-crates">Crates</a> ·
  <a href="#license">License</a>
</p>

---

Authors write modules against [`portaki-sdk`](https://crates.io/crates/portaki-sdk). At build time this binary compiles to `wasm32`, merges proc-macro emissions from `OUT_DIR/portaki-emissions/`, and packages OCI layers for registries such as GHCR.

## Install

```bash
cargo install portaki-cli
# or tip of main:
cargo install --git https://github.com/PortakiApp/portaki-sdk --locked portaki-cli
```

Requires the Wasm target:

```bash
rustup target add wasm32-unknown-unknown
```

## Commands

| Command | Contract |
|---------|----------|
| `portaki init` | Scaffold a module from a template, `listing.json` included — asks for its name, description, tagline, category and author in a terminal |
| `portaki build` | Compile Wasm + merge emissions → `manifest.json`, tamponne la version SDK liée |
| `portaki check` | Everything CI runs: fmt, clippy, tests, the wasm build, the manifest |
| `portaki connectors` | Show each declared egress, its permission and its credential |
| `portaki dev --forget` | Remove this module from the sandbox — a tried-once module leaves a row otherwise |
| `portaki lint` | Validate capabilities, connectors, i18n keys |
| `portaki test` | Forward to `cargo test` in the module crate |
| `portaki publish` | Push the OCI artifact, then announce it to the registry |
| `portaki link` | Open the dashboard page that links this module — and its monorepo siblings — to a repository |
| `portaki catalog` | Dump the SDUI primitive catalog |
| `portaki inspect` | Inspect a published OCI artifact |
| `portaki docs` / `dev` | Docs helper / local mock gateway (evolves with the SDK) |

## Scaffolding a module

`portaki init <id>` writes a buildable module crate and its `listing.json` — the public listing,
published with each release. There is no catalogue manifest to write: see below. In a terminal it asks five
questions; Enter skips one and keeps the default:

```
    display name [<id>]:
    description, in one sentence:
    catalogue tagline, 90 characters at most:
    category [stay]:        (1. arrival  2. stay  3. around  4. formalities)
    author name [<git config user.name, or TODO>]:
```

A tagline past 90 characters is asked again. `--display-name`, `--description`, `--tagline` and
`--category` answer ahead of time and are not asked; `--yes` — or no terminal, as in a CI — asks
nothing and keeps the template for the rest. What is not answered keeps an instruction starting
with « À compléter » / "To be completed", which the `listing` conformance check refuses to publish:
fill `listing.json` in, or delete it to write the listing in the dashboard.

## The manifest is written from the code

`portaki build` writes the catalogue manifest the platform reads; nobody writes it by hand.

| Manifest field | Declared by |
|---|---|
| `id`, `version` | the crate's `name` and `version` in `Cargo.toml` |
| `name`, `description` | `module.displayName` / `module.description` in `i18n/*.json` (keys set by `portaki_module!`) |
| `author`, `icon`, `type`, `maturity`, `sortOrder` | `portaki_module!(author, author_url, icon, module_type, maturity, sort_order)` |
| `hostSurfaces`, `guestSurfaces` | `#[surface(host, id, placement, design_id, label_key, icon, path)]` / `#[surface(guest, id, path, label_key, role, embeds)]` |
| `emails` | `#[email(id, audience, …)]` above the `#[command]` that sends it |
| `permissions` | the features enabled on `portaki-sdk` (`kv`, `repo`, `email`, `events`, `platform`, `guest-files`, `stay-guest-contact`) and each `#[connector]` id |
| `requiresModuleSdk` | the `portaki-sdk` version cargo resolved |

A module that still keeps a `portaki.module.json` is read as before: what it says wins, and the
code fills what it leaves out. Delete a field there and the code takes over.

## Monorepos

A repository whose modules live under `modules/*/` — crates on `portaki-sdk` (the layout of
`portaki-modules`) — is a monorepo. Inside `modules/<id>/`, every command acts on that module as
before. From the repository root, `portaki dev`, `portaki build` and `portaki publish` take
`--module <id>`; `build` and `publish` also take `--all`. With neither, a terminal asks which one,
and anything else — a CI — gets an error listing the ids.

`portaki sdk upgrade` moves the whole monorepo when the SDK is inherited from the workspace
(`portaki-sdk = { workspace = true }`): the root `Cargo.toml`, `Cargo.lock` and
`requiresModuleSdk` in every `portaki.module.json` still kept, then builds and tests the workspace, and
assembles and lints each module in turn. Run it from the repository root, or from any module —
the render comparison, which needs one module's sandbox, only runs in the latter case.

`portaki publish --all` publishes each module on its own — one OIDC token, one digest — prints a
result per module, keeps going after a refusal, and exits non-zero if any module failed. Modules
refused with `module_not_linked` are gathered into one link:

```text
✖ 403 module_not_linked — « access-guide » n'est lié à aucun dépôt
  Modules non liés dans ce dépôt : access-guide, nuki
  → Liez-les en une fois : https://developer.portaki.app/access-guide/repository?also=nuki
```

The CLI never creates the link itself: it needs a GitHub installation chosen in the dashboard.
`portaki link` opens that same page for the current module and the other modules of the
repository (`--no-browser` prints it). The page's address comes from the registry — the
`linkUrl` of the refusal, or `GET /registry/v1/modules/{id}/link-page` for `portaki link` — so
the CLI never guesses where the developer console lives.

## Output

Every command writes the same way: a line under the title saying what it actually does, one step
per line, a spinner while it runs, the elapsed time once it is done, and a `next` block naming
what to run afterwards and what each one gives you. The tools the CLI drives (`cargo build`,
`cargo test`) stay quiet unless they fail — then their whole output surfaces, because that is
what you were looking for.

The explanatory lines earn their place: `dev` does not start a local gateway, `publish` does not
just push, and `init` leaves a tree whose halves (`ids.rs` and `i18n/`) only make sense together.
Saying so costs a line each.

| Flag | Effect |
|------|--------|
| `--plain` | Bare output for scripts and CI — no logo, no headings, no glyphs, no advice. Implies `--no-color` |
| `--no-color` | Same layout, without colour or spinners |
| `-v`, `--verbose` | Stream the raw output of the tools the CLI drives |

`--no-color` and `--plain` answer different questions. The first keeps the layout and only drops
what a terminal paints. The second drops the layer written for a person who is discovering the
command — the logo, the heading, the `next` block, the glyphs, the margins — and keeps what
another program would come to read: the steps, the fields, the results, the errors. Failures
there are prefixed `error:` rather than marked with a cross, so a log stays greppable.

Colour and animation turn themselves off when the output is not a terminal, and `NO_COLOR` is
honoured. `portaki catalog` and `portaki inspect` write nothing but their JSON to stdout, so
they stay pipeable into `jq`.

`portaki` with no arguments, and `portaki --help`, open on the logo and close on the licence and
the copyright — they travel with the binary, which often circulates without its repository.
`portaki --version` adds where the source lives and the Apache-2.0 "AS IS" disclaimer, while
`-V` stays a single parseable line for scripts.

After a command has done its work, `portaki` says so when a newer version is published:

```
  ▲ portaki 2.4.0 → 2.5.0 is available
    cargo install portaki-cli --locked --force
    PORTAKI_NO_UPDATE_CHECK=1 silences this
```

The answer is cached for a day, and the one command that refreshes it gives up after a second
and a half — the notice must never be the reason a build feels slow. It is printed after the
command, not before, so it delays nothing and does not push the line you came to read out of
sight. It stays quiet under `--plain`, when the output is not a terminal, and when
`PORTAKI_NO_UPDATE_CHECK` is set.

Only one `portaki dev` runs at a time — with or without `--watch`. A second one refuses before it compiles anything
and says who holds the session:

```
✖ a portaki dev session is already running on weather (pid 41999) — stop it first
```

A one-shot `portaki dev` overwrites the sandbox exactly as a looping session does — once
instead of endlessly, which makes it no less surprising for whoever's module just vanished. It
takes the place too, and hands it back as soon as it is done.

Two sessions push different modules into the same sandbox in turn, each undoing what the
other just did — and one is rarely started on purpose: it is forgotten in a tab, and another is
started elsewhere. The lock lives next to the credentials, so it covers you on this machine,
not a repository.

A session ended with ctrl-c never releases it — nothing runs then. The next launch takes it over
as soon as the recorded process is gone, so an interruption never wedges the command.

The same exclusion holds **account-wide**, from the dev platform: two laptops on one account
share one sandbox, and only the server sees both. It is a lease, not a lock — nothing can ask a
remote process whether it is still alive, so the session pushes the deadline back while it runs
and the account frees itself when it stops.

The lease is held by the service that owns the sandbox, so it is the deploy itself that gets
refused, not merely the request to hold the place. A client that skips the lease — or that could
not reach the server to take it — still cannot overwrite a session that holds it.

What a network failure does, since it will happen:

| | |
|---|---|
| Unreachable when the session starts | Warns and carries on. Refusing to work because a lock service is down costs more than the nuisance it prevents — the local lock still covers this machine, and the server refuses the deploy anyway if someone else holds the account |
| A renewal fails | Retries in silence. The lease outlives several missed renewals; only a long outage is reported |
| The lease was taken over | Stops. Carrying on would be exactly the mutual clobbering this exists to prevent |

`portaki dev --dispatch`, with no operation name, lists what the module exposes — queries and
commands, each with the Rust function behind it — read from the manifest, without building or
deploying. And when an argument is refused, the refusal is rendered like everything else: the
CLI's own commands follow when the question was *which command*, `clap`'s suggestion is kept,
and the pointer goes to the help page of the command you were actually in.

`portaki logout` ends the session **on the platform**, not only here: it hands the stored
refresh token back for revocation. Clearing the file alone left that token valid until it
expired, so anyone holding a copy stayed signed in. The local credentials go first and
unconditionally — a logout that leaves them behind because the network hiccuped is the worse
half of both, since you believe you are out and you are out nowhere.

`portaki login` opens the browser on the verification URL — pre-filled with the code when the
platform returns one, so there is nothing left to paste. The code is printed either way; use
`--no-browser` over SSH or on a headless box.

## From a CI workflow

`portaki ci` answers, from the CLI, what a workflow used to ask in `bash`, `jq` and `curl`.
Every subcommand prints for a human and writes `GITHUB_OUTPUT` when it exists, so the same
invocation serves both.

| Command | Answers |
|---------|---------|
| `portaki ci modules [--changed-since <ref>] [--only a,b]` | Which modules this run should build — one repo per module, or `modules/*` in a monorepo |
| `portaki ci sdk-version` | The Portaki SDK this checkout resolves to, and the CLI version to install with it |
| `portaki ci check [--offline]` | Warns about an outdated SDK, a deprecated capability, or a manifest the shell has moved past |
| `portaki ci info` | This module's id and version — one per line under `--plain` |

`ci modules` reads the layout from the crates, not from a flag: a crate on `portaki-sdk` at the
root means one module, crates under `modules/*/` mean several. A change to the shared workspace
(`Cargo.toml`, `Cargo.lock`, `.cargo/`, `rust-toolchain`) rebuilds everything; a change to a
workflow file rebuilds nothing.

`ci sdk-version` reads `Cargo.lock`, not `Cargo.toml`: a module may declare the SDK by semver, by
git branch or by path, and only the lock says what will actually compile. The key it prints is
the version alone, because the CLI installs from crates.io — `cargo install portaki-cli@<key>` —
so the cache turns over when the SDK does, not on every commit to its branch.

Deprecations come from the registry, which serves them per SDK version alongside the other
contracts. A module is warned only about what it actually declares — capabilities required,
optional or provided, and built-in connectors. A registry that cannot be reached, or an SDK
version it has never seen, is reported and never fails the run: neither is a defect of the
module.

```bash
portaki --plain ci modules --changed-since "$BASE"   # ["access-guide","weather"]
portaki --plain ci sdk-version                       # 2.2.0
```

## Typical workflow

```bash
cd modules/weather
portaki check
PORTAKI_PUBLISH_VERSION=0.3.5 portaki publish --registry ghcr.io/portakiapp
```

Image name: `ghcr.io/portakiapp/portaki-modules-<module-id>:<semver>`.

`publish` runs the module's tests first — `cargo test` in the module crate, on the host — and
refuses to build or push anything when they fail, or when the conformance battery
(`tests/conformance.rs` with `portaki_test_utils::conformance!();`) is not among them. `--dry-run`
and `--skip-build` run them too; there is no flag to skip them. Only `--announce-only`, which
compiles and pushes nothing, does not.

`publish` refuses a version the registry already holds, **before** pushing anything. Publications
are immutable, so a second push could only leave the OCI tag pointing at something the catalogue
does not reference — two sources of truth disagreeing, with nothing to say so. That covers a
re-run; two jobs starting together both look before either announces, so serialise them with a
`concurrency:` group per module in the workflow.

`publish` announces the version to the registry after the push (needs `portaki login`).
`--no-announce` skips it — the artifact then belongs to no catalogue. `--announce-only` announces
a version already on GHCR without pushing anything, which is how an existing catalogue is adopted.

### What is new

`publish` writes `changelog` into the published manifest: two or three lines a host with an older
version sees before updating. `--notes "<line>"` (repeatable, at most 5 lines of 160 characters,
`--notes-lang` sets their language, `en` by default) wins; otherwise the lines come from this
version's section of the module's `CHANGELOG.md` (`## [x.y.z]` or `## x.y.z`, then bullets — the
release-please format, scope and commit link dropped), the first five kept. Neither: the field is
left as the manifest declares it, absent usually.

### Public listing

A module can version its public catalogue listing in `listing.json`, at the crate root
(`category`, `tagline`, `description`, `guestSurface`, `hostSurface`,
`configItems`, `capabilities`, `publishedLangs`). Its schema is
[`schema/listing.v1.json`](https://raw.githubusercontent.com/PortakiApp/portaki-sdk/main/schema/listing.v1.json)
— point `"$schema"` at it and an editor completes and checks the file, each field describing what
to write. The `listing` check of the conformance battery validates the file against it, so
`publish`, which runs the battery, refuses an off-schema listing — or one still holding the
`portaki init` instructions — before pushing. `publish` reads it before anything else — invalid JSON stops the
run before a push — and sends it as is once the version is in the registry: after the
announcement, and also when the version was already there, so a fixed listing does not wait for
the next release. An already-published version is not a failure: nothing is pushed, a warning says so,
the listing goes out and the command succeeds — re-running the publish workflow is enough to push
a fixed listing. `--dry-run` sends nothing; `--no-announce` skips it. In CI it takes a fresh
OIDC exchange, the announcement having used its single-use credential.

The file **overwrites the listing edited in the dashboard** — the repository is the source of
truth. A refused listing does not undo the publication, but fails the run with the registry's
reasons. No `listing.json`, nothing changes.

### Publishing from CI

No publication secret to store. In a GitHub Actions job with `id-token: write`, the CLI asks
GitHub for the job's OIDC token and exchanges it at the registry for a single-use publication
credential:

```yaml
permissions:
  contents: read
  packages: write
  id-token: write     # sans quoi il n'y a pas de jeton à échanger

jobs:
  publish:
    environment: release   # exigé par la liaison pour le canal stable
    steps:
      - run: portaki publish --registry ghcr.io/portakiapp
```

Link the module to its repository from the dashboard first: the registry authorises on the
repository *id* recorded there, and checks the workflow file, the triggering event and the
runner. The token says where it comes from; the link says what it may publish.

`PORTAKI_DEV_TOKEN` still wins when it is set — an explicit choice beats a mechanism that turns
itself on.

Official modules: [`portaki-modules`](https://github.com/PortakiApp/portaki-modules).

## Related crates

| Crate | Role |
|-------|------|
| [`portaki-sdk`](https://crates.io/crates/portaki-sdk) | Host APIs + SDUI |
| [`portaki-sdk-macros`](https://crates.io/crates/portaki-sdk-macros) | Manifest emissions |
| [`portaki-connectors`](https://crates.io/crates/portaki-connectors) | Typed connector ops |
| [`portaki-test-utils`](https://crates.io/crates/portaki-test-utils) | Mock host for tests |

## License

[Apache-2.0](https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE) · Copyright 2026 Syntax Labs
