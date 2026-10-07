# Inventory of the guest booklet primitives

Working document for the visual design of the SDUI primitives **on the guest side** — the booklet
seen by the person staying in the property. The **host** screens (dashboard) are being worked on
elsewhere; where a primitive serves both, it is called out.

Frozen as of 29 September 2026, against the published versions (`origin/main`) of `portaki-sdk`,
`portaki-guest` and `portaki-modules`.

---

## 1. What you need to know before reading

**What a primitive is.** A module (the weather, the access guide, the house rules…) draws nothing
itself. It describes what it wants to display as a tree of named blocks — "a card, containing a
stack, containing a text and a button". The booklet is the one that knows how to draw each of
those blocks. Those blocks are the **primitives**. There are **116** of them in the catalogue.

The direct consequence for design: drawing a primitive fixes the appearance of **every** module
that uses it, present and future. There is no per-module touch-up afterwards.

**Three settings common to every primitive.** Each of the 116 accepts, on top of its own fields:

| Setting | What it says | Values |
|---|---|---|
| `tone` | the role of the colour | 9 values — see §4 |
| `emphasis` | how much weight the text carries | `subtle` · `default` · `strong` |
| `surface` | the level of the background | `default` · `elevated` · `sunken` |
| `animation` | the entrance / the exit | `fadeUp` · `fadeIn` · `scaleIn` · `slideRight` · `none` |
| `visibility` | a display condition | (an expression, not an appearance) |

Those five exist everywhere, but they do not **do** something everywhere: see §5.

**Two words of the repo's vocabulary, translated once and for all:**

- a **surface** is a screen or a block that a module supplies to the booklet. There are five kinds
  on the guest side: `home.card` (the module's card on the home screen), `explore.detail` /
  `explore.sheet` / `explore.forecast` (the detail page), `upcoming.card` (the card before
  arrival), `post-stay.card` (after departure) and `guest.form` (a form);
- a **module** is an installable feature. There are 21 of them today.

---

## 2. The numbers, and how I got them

| | |
|---|---|
| Primitives in the catalogue | **116** |
| That the booklet knows how to draw | **92** — the other 24 would show an "Unknown primitive" box |
| Actually used by at least one module on the guest side | **35** |
| Used on the host side | 36 |
| Used on both sides | 18 |
| **Used by nobody, anywhere** | **63** |
| Existing guest surfaces | 45, spread over 19 of the 21 modules |

The two modules with no guest surface at all are `ical-sync` and `nuki`: they work in the
background and only ever talk to the host.

### How I counted

Two independent passes, then the union of the two.

1. **The modules' source.** For each of the 21 modules, I searched its whole source for calls to
   each primitive, keeping the files under the "guest" directory separate from those under the
   "host" one. Code shared between the two (for instance the table that maps a weather condition
   to an icon, over in the weather module) is counted on the side the module actually serves.
2. **The shipped previews.** 17 modules publish a preview file holding the **already built** tree
   of 28 of their guest surfaces. I walked it, recording every block and every variant value. This
   is the more reliable pass: it is exactly what goes out to the booklet.

The second pass caught what the first missed: values chosen at runtime rather than written out in
full (the bin colours in `waste-recycling`, the `train` icon in `train`). The first caught what
the second missed: the 17 surfaces with no published preview.

### What my count can miss

- **I did not run the modules.** A module could build a block through a code path that neither the
  textual pass nor the previews show. It is unlikely — the two passes agree — but it is not ruled
  out.
- **The 17 surfaces with no preview** are covered by the textual pass only, which is less precise
  about variant values.
- **Counting is per module, not per occurrence.** "Used by 9 modules" does not say whether it
  appears once or fifty times in each of them.
- **The booklet itself builds demonstration trees** (demo mode, with no real stay). They use a few
  primitives that no module uses — `Hero`, `HeaderTitle`, `ChecklistItem`, `ProgressBar`. I have
  **not** counted those as "used": they are showcase data, not product usage. They would
  nevertheless show up in a demo.
- I only looked at the 21 official modules. A future community module could use any of the 116.

---

## 3. The primitives, by family

In the tables:

- **Used** = used today by at least one module, on the guest side, with the number of modules and
  a concrete example (module + surface).
- **host too** flags that it also serves the host dashboard.
- **not drawn** flags that the booklet has no rendering for it at all: if a module sent it, the
  guest would see a dashed "Unknown primitive" box.

> **Quick read for design.** The rows marked **used** deserve a finished drawing. The **never
> used** rows can wait, or go away (§5). The **not drawn** rows do not concern the booklet for
> now.

### 3.1 Containers and layout — *stack, align, space, without displaying anything of their own*

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `Stack` | stacks its blocks, vertically or horizontally, with an adjustable gap | `direction`: `vertical` · `horizontal` | **yes — 18 modules** (e.g. `access-guide` / `explore.detail`) |
| `Grid` | lays its blocks out in even columns | column count, gap, minimum width | **yes — 1** (`weather` / `explore.forecast`, the forecast table) |
| `Group` | groups without changing the layout | — | no |
| `Split` | two blocks side by side, with a ratio | ratio | no |
| `Indent` | shifts a block to the right | 6 levels, 12 px each | no |
| `Spacer` | an empty space of a given height | height | no |
| `Divider` | a separating rule | — | **yes — 2** (`weather` / `explore.forecast`) |
| `SafeArea` | keeps clear of the screen edges (notch, bottom bar) | edges to keep clear | no |
| `Anchor` | drops an anchor point in the page | — | no |
| `PullToRefresh` | is supposed to let you pull to refresh | — | no — **and the booklet does nothing with it**: it simply displays its content |

### 3.2 Page blocks — *what frames a piece of content and gives it a title*

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `Card` | **the centrepiece of the booklet.** A rounded box, headed by an icon, a title, a subtitle, and a "Voir ›" if it leads somewhere | icon, action; **three different renderings**, see below | **yes — 19 modules**, all of them (e.g. `access-guide` / `upcoming.card`) · host too |
| `Section` | a title + subtitle followed by their content | — | no · *(not to be confused with the `sections` module)* |
| `Surface` | a plain block with its own padding | — | no |
| `Hero` | an opening banner, title over an illustration | 5 named illustrations (mediterranean, forest, cove, city, countryside) | no — except in the booklet's demo |
| `Accordion` | panels you unfold one at a time | — | no (host: 1) |
| `Tabs` | tabs | — | no — **and the rendering is incomplete**: the tabs are not clickable, only the first one's content shows |
| `Page` | the wrapper of a full screen | — | no — **not drawn** on the guest side; it is the wrapper of the host screens (20 modules) |

> **`Card` has three renderings, and this is the single most important point in the document.**
> Depending on where it sits:
> 1. **the full card** — a box, a soft shadow, an icon/title/subtitle header, content underneath;
> 2. **the compact row** — inside a tab list, the card folds down to a single line (square icon,
>    label, teaser, chevron) that opens the content; its children are **not** displayed there;
> 3. **the check-in formalities banner** — a dedicated rendering when the card holds the stay
>    declaration block.
>
> So the same object, sent by the same module, takes three appearances. Any mockup of `Card` has
> to cover all three, or say which one it fixes.

### 3.3 Windows and layers — *what opens on top*

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `Modal` | a window on top of the page | — | no — **not drawn** |
| `BottomSheet` | a panel that rises from the bottom | snap points | no — **not drawn** |
| `ConfirmDialog` | a confirmation request | confirm / cancel actions | no — **not drawn** |
| `PreviewPane` | a preview panel | — | no — **not drawn** (developer space tooling) |

> **Careful, vocabulary trap.** The booklet's panels do exist — the guest opens them constantly —
> but they do **not** go through these primitives. A module asks for one to open through an
> *action* (`openOverlay`), specifying `modal`, `bottomSheet` or `fullscreen`; the booklet owns
> its own panel and puts the requested surface inside it. The four primitives above are, in
> practice, unused. The real panel, on the other hand, does deserve a mockup: it has a header
> (icon in a pellet, title, close cross), and it is the booklet's only panel rendering.
> **Today, `fullscreen` is the only presentation in use** (5 modules).

### 3.4 Text — *the words*

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `Text` | **the basic text block** | `variant`: `body` · `caption` · `title` · `display` | **yes — 19 modules**, all of them (e.g. `access-guide` / `explore.detail`) |
| `RichText` | text formatted by the host (bold, lists, links) | — | **yes — 2** (`appliances` / `explore.item`) |
| `Markdown` | text in Markdown format | — | **yes — 1** (`sections` / `home.card`) |
| `Eyebrow` | a short overline, in capitals | — | **yes — 1** (`appliances` / `explore.item`) · host too |
| `Quote` | a quotation, vertical bar and italics | attribution | no |
| `Highlight` | a highlight | — | no |
| `Code` | monospaced text on a grey background | language | no |

> **`Text` carries four sizes, but `display` is not a size.** What it renders today: `body` =
> running text; `caption` = 12.5 px, grey; `title` = 19 px in the display typeface; `display` =
> **a 54 px square pellet** meant to hold an emoji or a symbol — unless the text looks like a
> temperature (`23 °`), in which case it becomes a large 30 px number. Two unrelated renderings
> under one name, told apart by the **shape of the text**. See §5.

### 3.5 Lists and rows — *successive lines separated by a hairline*

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `ListItem` | **the standard row**: visual on the left, title, subtitle, chevron on the right | leading icon or emoji, action, chevron | **yes — 12 modules** (e.g. `access-guide` / `explore.detail`) · host too |
| `ColorDotItem` | a row preceded by a coloured pellet | `swatch`: 10 named shades | **yes — 1** (`waste-recycling` / `explore.detail`, the sorting bins) |
| `TimedEntry` | a timed row: time on the left, label, note on the right | — | **yes — 1** (`train` / `explore.detail`) |
| `List` | the wrapper of a run of rows, with the hairlines | — | no (host: 4) |
| `ChecklistItem` | a row you tick | ticked / unticked | no (host: 2) — used in the booklet's demo |
| `SectionListItem` | a subgroup heading inside a list | — | no — **and its rendering is broken**: it shows the same title twice (see §5) |
| `BulletList` | a bulleted list | — | no |
| `IndexedInput` | a numbered row with a field | — | no — **not drawn** |

> **`ListItem` carries three automatic behaviours** design needs to know about: if the leading
> visual looks like an icon name, it becomes an icon in a square pellet; otherwise it is an emoji
> in a slightly larger pellet. And if the **title is a bare number** with a subtitle, it becomes a
> round numbered pellet and the subtitle takes the title's place — this is how step sequences end
> up being drawn.

### 3.6 Actions — *the things you press*

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `Button` | the button, full width, 42 px, fully rounded | `variant`: `filled` · `outline` · `ghost` | **yes — 9 modules** (e.g. `access-guide` / `explore.detail`) · host too |
| `Link` | an underlined link in the primary colour | — | **yes — 6** (`appliances` / `explore.item`) |
| `Pressable` | makes any block clickable, with no appearance of its own | — | **yes — 4** (`emergency-contacts` / `explore.detail`) |
| `EmergencyButton` | a full-width button in emergency red | — | no |
| `IconButton` | a 40 px square button carrying an icon | icon | no — **and its rendering writes the icon's name out in full** instead of drawing it |
| `ActionRow` | a row of small buttons | list of actions | no |
| `BackButton` | a "← Retour" | — | no — **and it cannot do anything**: the contract gives it no action |

### 3.7 Input — *the guest's forms*

Five modules have the guest fill something in: `consumables`, `guest-reviews`, `issue-report`,
`lost-found`, `pre-arrival-form`.

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `Form` | the form, and what it submits for validation | submit action | **yes — 5** (`consumables` / `guest.form`) · host too |
| `Field` | the label above a field, with the asterisk if required | required or not, `visibleWhen` | **yes — 5** · host too (`visibleWhen`: **guest only**) |
| `TextInput` | a single input line, 40 px, 10 px corner | — | **yes — 3** (`issue-report` / `guest.form`) · host too |
| `TextArea` | a multi-line input area | number of lines | **yes — 5** · host too |
| `Select` | a dropdown list | options | **yes — 1** (`guest-reviews` / `post-stay.card`) · host too |
| `ChoiceList` | a choice among several | `layout`: `compact` · `cards` | **yes — 3** (`consumables` / `guest.form`) · host too |
| `TimePicker` | a time | — | **yes — 1** (`pre-arrival-form` / `guest.form`) |
| `ImageUpload` | uploading a photo, with preview and removal | — | **yes — 1** (`issue-report` / `guest.form`) |
| `RadioGroup` | radio buttons | options | no |
| `Checkbox` | a checkbox | — | no |
| `Toggle` | a checkbox with a label | — | no (host: 1) |
| `NumberInput` | a numeric field | min, max | no |
| `SearchInput` | a search field | — | no |
| `Slider` | a slider | min, max | no |
| `DatePicker` | a date | — | no |
| `TimeSlotPicker` | a choice of time slots | slots | no — **and the slots cannot be selected** |
| `TagInput` | tags you add | — | no — **and its placeholder is hardcoded English** |
| `Chips` | read-only tags | — | no |
| `FormStepper` | progress through a multi-step form | count, current step | no |
| `FieldHint` | a hint under a field | — | no — **not drawn** (host: 5) |
| `SecretInput` | a masked field | — | no — **not drawn** (host: 4) |
| `SelectableCard` | a card you pick | icon, action | no — **not drawn** (host: 1) |
| `ToggleRow` | a row with a switch | icon, action | no — **not drawn** (host: 5) |
| `EditableList` | a list of items you edit | bilingual, photo, checkbox | no — **not drawn** (host: 1) |
| `StepList` | steps you add and remove | add action | no — **not drawn** (host: 3) |
| `RichTextEditor` | a rich text editor | — | no — **not drawn** (host: 4) |
| `AddressMapPicker` | an address picked on a map | — | no — **not drawn** (host: 2) |
| `MapEditor` | markers you drop on a map | — | no — **not drawn** |
| `CredentialField` | an access key for a service | — | no — **not drawn** |
| `InlineNotice` | a message inside a form | — | no — **not drawn** (host: 1) |

> **The "input" family is badly lopsided.** Eight primitives are used on the guest side; the other
> fourteen are either host material or nothing at all. And **the eight that are used are, for the
> most part, native browser controls** dressed in a border and a rounded corner. There is real
> design work to be done here, but it only covers those eight.

### 3.8 Filters

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `FilterChip` | a filter pellet, filled when active | selected, action | **yes — 1** (`train` / `explore.detail`) |
| `FilterBar` | the row of filters | list of filters | **yes — 1** (`train` / `explore.detail`) |

> `FilterBar` has two renderings: if it holds `FilterChip`s, it lines them up and they are
> clickable; if it only carries a list of labels, it shows them as grey **inert** pellets. Only
> the first path is used.

### 3.9 Media and markers — *the images and the map*

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `Map` | a Mapbox map with markers, clustered or not | `interactionMode`: `pan-zoom` · `none`; markers: `property` · `poi` | **yes — 3** (`access-guide`, `events`, `local-guide`) |
| `Icon` | a lone pictogram, adjustable size: an icon token (`name`) or an emoji (`emoji`) | `name`: ~83 tokens — see §4 | **yes — 2** (`weather` / `explore.forecast`, `appliances` / `explore.item`) |
| `Image` | a full-width image with a rounded corner and a loading blur | `size`: `full` · `thumb` (**ignored by the booklet**) | **yes — 1** (`local-guide`) · host too |
| `QRCode` | supposed to show a QR code | size | **yes — 1** (`guest-reviews`) — **but what it renders today is a fake hardcoded pattern**, unreadable by a phone (see §5) |
| `Avatar` | a round photo, or initials | — | no |

> In **preview** (developer space, catalogue showcase), the map is not displayed: a grey rectangle
> merely says "N repères — non affichée en aperçu". That is expected, and it explains why there is
> no map on the preview screenshots.

### 3.10 Tags and values — *the small elements that qualify*

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `KeyValue` | a "label … value" line, value on the right | monospaced display | **yes — 5** (`access-guide` / `explore.detail` — codes, opening hours, passwords) |
| `Pill` | a rounded pellet: coloured dot + label | the role's shade | **yes — 2** (`local-guide` / `explore.detail`) · host too |
| `Badge` | a rounded pellet without the dot | the role's shade | **yes — 1** (`access-guide` / `explore.detail`) |
| `Temperature` | a temperature | `variant`: `inline` · `hero` · `compact`; `unit`: `C` · `F` | **yes — 1** (`weather` / `explore.forecast`) |
| `Tag` | a grey word, no background | — | no — **and it ignores `tone` entirely** |
| `Dot` | a coloured dot, optionally blinking | `swatch` or `tone` | no |
| `StatusBadge` | a "label · status" pellet | — | no — **and the status is displayed untranslated** |
| `Stat` | a big figure with its label and its trend | icon, `deltaTone` | no — **not drawn** (host: 6) |
| `CountdownTimer` | supposed to count down to a date | — | no — **and it shows the raw date, with no countdown** |
| `WeatherIcon` | supposed to draw a weather condition | condition | no — **and it always shows the same "☀"**, whatever the weather. The code itself calls it legacy; `Icon` replaces it |
| `TimeColumn` | a time above a label | — | no |
| `DateColumn` | a date above a label | — | no |

### 3.11 Data at scale

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `Chart` | a chart | `kind`: `bars` · `horizontal_bars` · `donut` · `heatmap` | no — **not drawn** (host: 5) |
| `DataTable` | a data table | — | no — **not drawn** |
| `Timeline` | a timeline of events | — | no — **not drawn** |
| `FeedItem` | a log entry, with a status and a coloured dot | `dotTone`, action | no — **not drawn** (host: 5) |

> Those four are clearly dashboard material. Nothing to draw for the booklet.

### 3.12 Status feedback — *saying there is nothing, that it is loading, that it is done*

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `EmptyState` | "there is nothing here": title, description, icon, centred | icon | **yes — 14 modules** · host too |
| `InfoBanner` | the information callout: icon, title, message, softly tinted background | `tone` (partly) | **yes — 9 modules** (e.g. `access-guide` / `explore.detail`) · host too |
| `Notice` | a plain grey line | — | no |
| `ErrorState` | a red box: title, message, and normally a "retry" button | retry action (**ignored**) | no |
| `SuccessState` | a green box: title, message | — | no |
| `CompletionState` | a plain green line in bold | — | no |
| `LoadingState` | a spinning circle + a message | — | no |
| `Spinner` | a spinning circle, on its own | — | no |
| `Skeleton` | a pulsing grey rectangle, while the content loads | `variant`, `lines` (**both ignored**); height, width | no |
| `ProgressBar` | a progress bar | value, maximum | no — used in the booklet's demo |
| `Stepper` | a row of segments, the passed ones coloured | count, current | no |
| `DotIndicator` | pagination dots | count, current | no |
| `Toast` | a transient message | — | no — **and the booklet displays nothing at all** for this primitive |

> **Two traps around `EmptyState`.** First, **it is very often invisible**: when the emptiness
> comes from missing configuration ("the host has not filled anything in"), the booklet shows
> nothing rather than a message. Only emptiness caused by an error is shown. Second, when a `Card`
> contains nothing but an `EmptyState` of that kind, **the whole card disappears**. That is the
> right product decision; you just need to know that an `EmptyState` mockup will rarely be seen.

### 3.13 Application chrome

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `TopBar` | a title bar at the top | — | no |
| `HeaderTitle` | a large title with a subtitle | — | no — used in the booklet's demo |
| `BottomTabBar` | a tab bar at the bottom | tabs, active tab | no |
| `Chevron` | a `›` chevron | `direction` (**ignored**) | no |

> The booklet's actual chrome — header, navigation, tabs — is drawn by the booklet itself, not by
> these primitives. No module sends any of them, and it is unlikely any ever will: it is not a
> module's place to decide the navigation bar. **A good candidate for removal** (§5).

### 3.14 What belongs to the platform

| Primitive | What it is for | Variants | Used on the guest side |
|---|---|---|---|
| `HostFragment` | a block the **booklet** draws, which a module merely asks for | the block's identifier | **yes — 1** (`pre-arrival-form` / `home.card`) |
| `CapabilityNotice` | "this feature is not available" | — | no — **not drawn** |
| `QuotaIndicator` | "you have used N out of M" | — | no — **not drawn** |

> `HostFragment` has **exactly one** realisation today: the stay declaration line (check-in
> formalities). Any other identifier displays nothing. This is the mechanism by which a sensitive
> subject — the guest's identity — stays drawn and served by the platform, never by the module.

---

## 4. The variants: which ones work, which ones sleep

### 4.1 `Tone` — 9 values, 5 used on the guest side

| Value | Guest | Host | What it gives in the booklet |
|---|---|---|---|
| `neutral` | — | 3 | surface background, default ink |
| `primary` | 1 (`pre-arrival-form`) | 3 | background in the primary colour, contrasting ink |
| `secondary` | **never** | **never** | a blend of the primary and the ink |
| `accent` | **never** | **never** | the accent yellow |
| `info` | 1 (`weather`) | 1 | — |
| `success` | 2 (`pre-arrival-form`, `weather`) | 5 | — |
| `warning` | 1 (`weather`) | 10 | — |
| `danger` | 1 (`weather`) | 3 | — |
| `emergency` | **never** | **never** | solid red background |

**The tone set needs revisiting, and here is precisely why.** Three values out of nine are used
nowhere (`secondary`, `accent`, `emergency`). But the more serious problem is elsewhere: **tones
do not behave the same depending on whether they colour a background or an ink.**

| Tone | As a **background** | As **ink / dot** |
|---|---|---|
| `info` | elevated surface + a hairline in the primary colour | the *info* colour |
| `success` | the **primary** colour at 12 % | the *success* colour (green) |
| `warning` | the **accent** colour at 35 % | the *warning* colour (orange) |
| `danger` | emergency red at 15 % | emergency red |
| `emergency` | emergency red, solid | emergency red |

In other words: a "success" block **as a background** takes the brand's shade, not green — while
the same word "success" **as text** is green. And `danger` and `emergency` give exactly the same
ink: they differ only in the intensity of the background. A designer handed nine tone names will
believe there are nine colours; there are fewer, and they are not stable from one use to the next.

### 4.2 `TextVariant` — 4 values, 4 used

| Value | Guest | Rendering |
|---|---|---|
| `caption` | **18 modules** | 12.5 px, grey at 55 % |
| `body` | **12 modules** | running text |
| `title` | 5 modules | 19 px, display typeface |
| `display` | 1 module (`appliances`) | 54 px emoji pellet, **or** a large number if the text looks like a temperature |

A healthy set — the only one that is both fully used and more or less coherent. Only `display` is
a problem (§5).

### 4.3 `ButtonVariant` — 3 values, 1 used on the guest side

| Value | Guest | Host |
|---|---|---|
| `filled` | **never explicitly** — but it is the booklet's default, so any button without a variant is filled | — |
| `outline` | 4 modules | 4 |
| `ghost` | **never** | 2 |

The only button a module deliberately picks on the guest side is the **outline** one. The filled
button is never asked for: it arrives by default.

### 4.4 `Swatch` — 10 named shades, 4 used on the guest side

| Used on the guest side | `yellow`, `green`, `brown`, `grey` — all by `waste-recycling`, for the sorting bin colours |
|---|---|
| Used on the host side only | `blue`, `black`, `red`, `orange` |
| **Never used** | `white`, `purple` |

`Swatch` has a clear role, and a different one from `Tone`: these are **real-world** colours (a
yellow bin is yellow), not interface roles. The distinction is a good one. Two shades sleep.

### 4.5 `IconName` — 83 tokens, 36 used on the guest side

**Used on the guest side (36)** — `calendar`, `car`, `check-circle`, `circle-x`, `clipboard`,
`clipboard-list`, `clock`, `clock-circle`, `cloud`, `cloud-fog`, `cloud-lightning`, `cloud-rain`,
`cloud-snow`, `cloud-sun`, `danger-triangle`, `droplets`, `gauge`, `home`, `key`, `list-checks`,
`logout`, `map-pin`, `message-circle`, `package`, `package-search`, `phone`, `plug`, `recycle`,
`scale`, `search`, `search-x`, `sparkles`, `star`, `sun`, `thermometer`, `train`, `volume-2`,
`wifi`, `wind`, `zap`.

**Used on the host side only (16)** — `bell`, `building`, `gift`, `grid`, `image`, `info-circle`,
`link`, `lock`, `mail`, `message`, `more-horizontal`, `plus`, `refresh`, `smile`, `ticket`,
`users`.

**Never used, anywhere (27)** — `ban`, `check`, `chevron-right`, `cloud-off`, `dots`, `file-text`,
`fingerprint`, `guests`, `handshake`, `heart-handshake`, `info`, `list`, `minus`, `no`, `noise`,
`ok`, `parking`, `paw`, `paw-print`, `pets`, `quiet`, `send`, `sliders`, `triangle-alert`, `user`,
`volume-x`, `x`.

**And above all: 17 glyphs are already shared by several tokens.** So the booklet draws the same
thing under different names:

| One single drawing | …for these tokens |
|---|---|
| clock | `clock-circle`, `clock`, `quiet` |
| ticked list | `clipboard-list`, `list`, `list-checks` |
| prohibition | `minus`, `ban`, `volume-x` |
| warning triangle | `triangle-alert`, `danger-triangle` |
| circled tick | `check-circle`, `ok` |
| cross | `x`, `no` |
| circled information | `info-circle`, `info` |
| car | `car`, `parking` |
| parcel | `package`, `package-search` |
| heart | `paw`, `pets` |
| people | `users`, `guests` |
| handshake | `handshake`, `heart-handshake` |
| speech bubble | `message`, `message-circle` |
| loudspeaker | `noise`, `volume-2` |
| ellipsis | `more-horizontal`, `dots` |
| clipboard | `clipboard`, `file-text` |
| **gift** | `gift`, **`paw-print`** |

The last row is surely not intended: **`paw-print` draws a gift**. Elsewhere, `sliders` draws a
cog and `gauge` draws an activity chart — two fallbacks the code owns up to, but which do not
match the name.

All told: 83 names, 36 used, and the booklet only knows how to draw about 66 distinct ones.

### 4.6 The sets that are entirely unused

| Set | Values | Finding |
|---|---|---|
| `AnimationKind` | `fadeUp` · `fadeIn` · `scaleIn` · `slideRight` · `none` | **no module ever asks for an animation.** The booklet nevertheless animates every `Stack` with `fadeUp` by default, and `Stack` is the only thing animated: `fadeIn`, `scaleIn` and `slideRight` are drawn nowhere, and `slideRight` is not even supported |
| `DeltaTone` | `good` · `bad` · `neutral` | never used (depends on `Stat`, a host primitive) |
| `Emphasis` | `subtle` · `default` · `strong` | `subtle` (2 modules) and `strong` (1); **`default` never asked for** — it is the implicit default |
| `SurfaceLevel` | `default` · `elevated` · `sunken` | `elevated` only (3 modules); `default` and `sunken` **never** |
| `TempVariant` | `inline` · `hero` · `compact` | `hero` only; `inline` and `compact` **never** |
| `TemperatureUnit` | `C` · `F` | `C` only; **no module serves Fahrenheit** |
| `StackDirection` | `vertical` · `horizontal` | `horizontal` asked for by 2 modules; `vertical` never — it is the default |
| `ChoiceListLayout` | `compact` · `cards` | `compact` asked for by 3 modules; `cards` never on the guest side — **and the booklet ignores the setting anyway** (§5) |
| `MapInteractionMode` | `pan-zoom` · `none` | **every map in the booklet is frozen** (`none`, 3 modules). Not one manipulable map |
| `ImageSize` | `full` · `thumb` | never on the guest side, and **ignored by the booklet** |
| `OverlayPresentation` | `modal` · `bottomSheet` · `fullscreen` | `fullscreen` only (5 modules). The bottom sheet and the modal window exist in the booklet but **nobody asks for them** |
| `ChartKind` | 4 values | 2 used on the host side; `donut` and `heatmap` **never** |

---

## 5. What I would propose clarifying

**Nothing has been changed.** These are proposals, each argued in one sentence. The order runs
from the most decisive to the most cosmetic.

### Renderings that do not do what their name promises

1. **`QRCode` does not display a QR code.** What it renders today is a 36-cell pattern derived
   from a hardcoded string, identical whatever the content; a phone cannot read anything from it.
   `guest-reviews` uses it on the guest side all the same. To be treated as a primitive **to
   implement**, not one to redesign.
2. **`WeatherIcon` always shows "☀".** The code calls it legacy and points to `Icon`; no module
   uses it. Proposal: drop it from the catalogue rather than draw it.
3. **`Toast` displays nothing at all.** Its rendering always returns empty. Either that is an
   oversight, or it is an admission that transient messages are the booklet's business and not a
   module's — in which case, drop it.
4. **`SectionListItem` shows its title twice**, once small and grey and once in bold, because the
   rendering expects two fields while the contract declares only one. Either the rendering or the
   contract is behind the other.
5. **`BackButton` cannot do anything**: the contract gives it no action, and the rendering looks
   for one. Either give it one, or drop it — going back being the booklet's business anyway.
6. **`PullToRefresh` refreshes nothing**, `Tabs` has no clickable tabs, `TimeSlotPicker` has no
   selectable slot, `CountdownTimer` does not count down. Four primitives whose name describes a
   behaviour the rendering does not have. They would be better named after what they show, or
   finished.

### Names that mislead

7. **`Text` + `display` does two unrelated things**: a 54 px emoji pellet, or a large number —
   told apart by a regular expression that tests whether the text looks like a temperature.
   Proposal: split it in two, say a "symbol" variant and a "big figure" variant, and drop the
   guesswork.
8. **`Section` has nothing to do with the `sections` module.** The primitive is a title followed
   by its content; the module is the host's free-form section editor. Two very different things
   under the same word, in a product where the two cross paths.
9. **`Surface` means three things at once**: a primitive (a block with its padding), a common
   setting (`surface`: background level) and the unit a module delivers ("a guest surface"). Three
   senses for one word; it is the one that trips you up when reading the code.
10. **`Modal` and `BottomSheet` are not how you open a panel.** Panels go through an *action*, not
    through these primitives. Keeping both names in the catalogue keeps alive the idea that you
    can drop a window into a tree. Proposal: remove them, and document the real panel as the only
    mechanism.
11. **`paw-print` draws a gift**, `sliders` a cog, `gauge` an activity chart. Three icon names
    that do not describe the drawing you get.

### Primitives that do almost the same thing

12. **`Badge`, `Pill`, `Tag`, `StatusBadge`**: four textual tags. `Badge` is a tinted pellet;
    `Pill` is the same with a dot in front; `Tag` is a grey word that ignores `tone`;
    `StatusBadge` is a pellet showing "label · status" with the status untranslated. Two would be
    enough: a pellet (with or without a dot) and a discreet word.
13. **`Stepper` and `FormStepper` render identically** — a row of segments — and differ only in
    the names of their fields (`current` versus `currentStep`). A plain duplicate.
14. **`ChoiceList` renders exactly like `RadioGroup`.** Its `layout` (`compact` / `cards`), its
    `emitOnChange`, its action and its options' icons and descriptions are **all ignored** by the
    booklet. Three modules send `layout: compact` and it changes nothing. Either `ChoiceList` gets
    its own drawing, or it is folded into `RadioGroup`.
15. **`Notice`, `InlineNotice`, `InfoBanner`**: three ways of saying something in passing. Only
    `InfoBanner` is used; `InlineNotice` is not even drawn; and `InlineNotice`'s contract carries
    **two** fields for the same thing (`message` **and** `text`).
16. **`SuccessState`, `CompletionState`** say the same thing with two different layouts, and
    neither is used.
17. **`LoadingState` and `Spinner`**: the first is the second plus a message.
18. **Settled.** `Icon` used to carry an icon token only, so an emoji travelled as
    `Text`+`display` — a hero-title variant for something that is not a title. `Icon` now carries
    `emoji` beside `name`: a lone pictogram is an `Icon`, whichever kind, and `display` is for
    titles again. `ListItem.leading` keeps accepting both, with its regular expression, for rows.

### Fields never filled in, or ignored

19. **The booklet plainly and simply ignores**: `Image.size`, `Chevron.direction`,
    `Skeleton.variant` and `Skeleton.lines`, `ErrorState.retryAction`, `TimeColumn.times`,
    `DateColumn.dates`, `ChoiceList.layout`. Fields a module can fill in with no visible effect
    whatsoever.
20. **`TextArea` and `Select` lose their initial value**, whereas `TextInput` keeps it. A
    pre-filled form is therefore only half pre-filled.
21. **`ColorDotItem` and `Dot` accept both `swatch`** (a named shade, from the theme) **and
    `color`** (a free string, any CSS colour). The second bypasses the theme. Proposal: keep
    `swatch` only.
22. **`IconButton` writes the icon's name out in full** instead of drawing it, whereas `Card`,
    `ListItem` and `Icon` resolve it correctly. A rendering inconsistency, not a contract one.
23. **`TagInput` shows "Add tag" as hardcoded English**, in a booklet that is otherwise fully
    translated.

### Value sets to tighten up

24. **`Tone`: 9 values, 5 used, and behaviour that changes between background and ink.** The
    detail is in §4.1. It is the most structural decision in the document: either every tone is
    aligned on its own colour in both uses, or the set is reduced to what is actually
    distinguishable — to my mind *neutral, brand, info, success, warning, danger*, merging
    `danger` and `emergency`, which already share their ink.
25. **`IconName`: 83 names, 36 used, 17 shared glyphs.** The set can be cut by about a third
    without losing anything on screen, which would lighten the drawing work by as much.
26. **`AnimationKind` is asked for by no module**, and `Stack` is the only thing animated. Either
    animation becomes the booklet's decision and leaves the contract, or it is applied everywhere
    — the current state is the worst of both.
27. **`Swatch`: `white` and `purple` are never used**, and `white` needs a hairline to be seen at
    all. To be confirmed against real usage (sorting bins, categories).

### The chrome that has no business being in a module contract

28. **`TopBar`, `BottomTabBar`, `HeaderTitle`, `Chevron`, `Page`**: the booklet's chrome is drawn
    by the booklet. No module sends any of them, and a module should not be able to decide the
    navigation bar. Proposal: take them out of the catalogue offered to modules.

---

## 6. What I could not establish

These points are **open questions**, not findings.

1. **Are the 63 primitives nobody uses a catalogue for the future or a leftover?** The code does
   not say. The answer changes everything: either they have to be drawn (they are waiting for
   modules to come), or they have to be removed. I cannot settle it by reading.
2. **`SectionListItem`: is it the contract or the rendering that is behind?** The rendering
   visibly expects a group heading *and* a title; the contract declares only one. Which of the two
   expresses the intent, I do not know.
3. **`InlineNotice` carries `message` and `text`.** Which one is right, and why both exist, I
   found no trace of.
4. **The background `Tone` that takes the brand colour rather than the role's** (`success`,
   `warning`): I do not know whether it is a choice — keeping a booklet that stays in the host's
   colour — or a drift. If it is a choice, it deserves writing down, because it makes the names
   `success` and `warning` misleading.
5. **`Card`'s row-mode behaviour** (inside a tab list): I can see how it is triggered, but not
   which product rule decides that a surface ends up in a tab list rather than as full cards.
6. **I have not seen the booklet running.** Everything in this document comes from reading the
   contract, the rendering code and the shipped preview trees. The renderings I describe are the
   ones the code produces, not the ones I observed on screen.
7. **Community modules to come** could use any of the 116 primitives. The list of 35 describes
   today's usage, by the official modules; it does not predict tomorrow's.

---

## 7. Sources

| What | Where |
|---|---|
| The 116 primitives, their fields | `portaki-sdk` → `crates/portaki-sdk/sdui_primitives.json` |
| The variants and composed types | `portaki-sdk` → `contracts/sdui_types.json` |
| The settings common to all of them | `portaki-sdk` → `crates/portaki-sdk/build.rs` |
| The rendering the guest sees | `portaki-guest` → `src/features/sdui-renderer-guest/ui/primitives/` |
| The list of drawn primitives | `portaki-guest` → `src/features/sdui-renderer-guest/ui/primitive-registry.ts` |
| The tone → colour mapping | `portaki-guest` → `src/features/sdui-renderer-guest/model/semantic-styles.ts` |
| The name → icon drawing mapping | `portaki-guest` → `src/shared/lib/resolve-sdui-icon.ts` |
| The theme colours | `portaki-guest` → `src/app/_fsd/styles/globals.css` |
| Real usage | `portaki-modules` → `modules/*/src/guest/` and `modules/*/previews.json` |
