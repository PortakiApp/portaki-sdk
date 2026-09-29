//! Everything the CLI shows goes through here.
//!
//! A command does not `println!`. It states what it is busy doing — a step, a field, a failure —
//! and this layer decides on the form. That is what keeps the output consistent from one command
//! to the next, and above all what makes it possible to turn off: redirected into a file or read
//! by a CI, the same command writes bare text, with no colour and no animation.
//!
//! Nothing is left to `NO_COLOR` alone: `--no-color` forces it off, `--verbose` replaces the
//! spinners with the raw output of the tools being driven.

use std::fmt::Display;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use console::{style, Emoji, Term};
use indicatif::{ProgressBar, ProgressStyle};

/// The left margin common to every line: the output breathes, and a block of text stands out
/// from what a tool the CLI calls writes with no margin.
const MARGIN: &str = "  ";

/// ASCII fallback included: redirected output, an old Windows terminal, `TERM=dumb`.
static TICK: Emoji<'_, '_> = Emoji("✔", "+");
static CROSS: Emoji<'_, '_> = Emoji("✖", "x");
static BANG: Emoji<'_, '_> = Emoji("▲", "!");
static DOT: Emoji<'_, '_> = Emoji("·", "-");
static ARROW: Emoji<'_, '_> = Emoji("→", "->");

static VERBOSE: AtomicBool = AtomicBool::new(false);

/// The output is bare: no logo, no header, no glyphs, no margin, no advice.
///
/// What is left is what another program would come and read — the steps, the fields, the
/// results. The rest addresses a human discovering the command, and has no business in a CI log
/// or in a `grep`.
static PLAIN: AtomicBool = AtomicBool::new(false);

/// `--json`: stdout carries only the command's JSON document; everything else — steps, fields,
/// advice, and the output of the tools being driven — goes out on stderr.
static JSON: AtomicBool = AtomicBool::new(false);

/// Nothing has been written yet since the header.
///
/// A section puts a blank line in front of itself to stand apart from what precedes it. Right
/// after the header, which already puts one there, that made two — a gap that reads like an
/// oversight.
static FRESH: AtomicBool = AtomicBool::new(false);

/// Fixes the rendering mode for the whole process, before the first line is written.
pub fn init(no_color: bool, verbose: bool, plain: bool) {
    set_plain(plain);
    set_colors(!no_color && !plain);
    VERBOSE.store(verbose, Ordering::Relaxed);
}

/// Switches to bare output, before anything at all is written.
///
/// Kept apart from [`init`] like [`set_colors`]: the help and `--version` are rendered by `clap`
/// during parsing, hence before anything else is known about the arguments.
pub fn set_plain(plain: bool) {
    PLAIN.store(plain, Ordering::Relaxed);
}

/// Switches to `--json`, which implies bare output.
pub fn set_json(json: bool) {
    JSON.store(json, Ordering::Relaxed);
    if json {
        set_plain(true);
        set_colors(false);
    }
}

/// Is stdout reserved for the JSON document?
pub fn json() -> bool {
    JSON.load(Ordering::Relaxed)
}

/// Writes the command's document, on a single line: it is the only thing that goes out on
/// stdout under `--json`. One line per call, which makes NDJSON for a stream (`portaki logs`).
pub fn emit(document: &serde_json::Value) {
    println!("{document}");
}

/// A driven tool whose output would otherwise go straight through: under `--json`, its standard
/// output joins stderr, so that stdout stays a document.
pub fn keep_stdout_clean(cmd: &mut Command) {
    if json() {
        cmd.stdout(std::io::stderr());
    }
}

/// Is the output bare?
pub fn plain() -> bool {
    PLAIN.load(Ordering::Relaxed)
}

/// The indent of the second-plane lines — details and fields.
fn indent() -> &'static str {
    if plain() {
        ""
    } else {
        "    "
    }
}

/// A line with a glyph: the glyph goes away along with the margin when the output is bare.
///
/// The glyph says "done", "skipped", "careful" to a scanning eye. A program, for its part, reads
/// the text — and would otherwise have to learn to cut symbols off the front of every message.
fn glyphed(painted: String, message: impl Display) -> String {
    if plain() {
        format!("{message}")
    } else {
        format!("{MARGIN}{painted} {message}")
    }
}

/// The user wants to see the raw output of the tools being driven.
pub fn verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

/// Is anyone actually watching? If not: no animation, no rewriting of a line.
fn attended() -> bool {
    console::user_attended() && !verbose()
}

/// The logo, in lower case like the brand.
///
/// Written out here rather than generated: a block font is read by eye, not at runtime, and a
/// generator would make the brand's identity depend on one more dependency.
const LOGO: [&str; 7] = [
    "                              ██                          ██",
    "                              ██              ██",
    "██████      ████    ██  ████  ██████  ██████    ██    ████  ██",
    "██    ██  ██    ██  ████        ██        ████  ██  ████    ██",
    "██    ██  ██    ██  ██          ██    ████████  ██████      ██",
    "██████      ████    ██          ████    ██████  ██    ████  ██",
    "██",
];

/// The row the wordmark's dot sits on — the baseline.
const DOT_ROW: usize = 5;

/// The dot carries the green of `portaki dev`, and it is exactly the green of the success ticks.
///
/// A single shade of green in the whole CLI: the wordmark's dot and the tick that says "done"
/// are the same colour, not two greens that look alike without answering each other. In basic
/// ANSI green rather than 256 colours, the logo holds up on a sixteen-colour terminal too.
///
/// The word itself keeps the terminal's foreground colour: a hard-coded white would disappear on
/// a light theme, whereas the brand only asks for "the colour of the text".
const WORDMARK_DOT: &str = "██";

/// The painted logo, ready to be put at the top of a help screen.
///
/// Returned as a `String` rather than printed: `clap` wants it as the header of its help, and the
/// same text serves `--version`.
pub fn banner() -> String {
    let mut out = String::new();
    for (row, line) in LOGO.iter().enumerate() {
        let word = style(line).bold();
        if row == DOT_ROW {
            out.push_str(&format!(
                "{MARGIN}{word}  {}\n",
                style(WORDMARK_DOT).green().bold()
            ));
        } else {
            out.push_str(&format!("{MARGIN}{word}\n"));
        }
    }
    out.push_str(&format!(
        "{MARGIN}{}\n",
        style(format!(
            "the module toolchain  ·  v{}",
            env!("CARGO_PKG_VERSION")
        ))
        .dim()
    ));
    out
}

/// What protects the project and those who use it, in the help footer.
///
/// The licence and the copyright holder are not decoration: they travel with the binary, which
/// often circulates without its repository. Reading them costs two lines.
pub fn legal() -> String {
    format!(
        "{MARGIN}{}\n{MARGIN}{}",
        style(format!(
            "{} · Copyright 2026 Syntax Labs",
            env!("CARGO_PKG_LICENSE")
        ))
        .dim(),
        style(format!(
            "{} · {}",
            env!("CARGO_PKG_HOMEPAGE"),
            env!("CARGO_PKG_REPOSITORY")
        ))
        .dim()
    )
}

/// What `--version` tells, as opposed to the `-V` a script reads.
///
/// A `-V` has to stay one parseable line; it is the long form that carries the licence, the
/// no-warranty clause and where to find the source again.
pub fn long_version() -> String {
    let rows = [
        ("license", env!("CARGO_PKG_LICENSE")),
        ("copyright", "Copyright 2026 Syntax Labs"),
        ("homepage", env!("CARGO_PKG_HOMEPAGE")),
        ("source", env!("CARGO_PKG_REPOSITORY")),
    ];
    let mut out = format!("{}\n\n{}\n", env!("CARGO_PKG_VERSION"), banner());
    for (label, value) in rows {
        out.push_str(&format!(
            "{MARGIN}  {} {value}\n",
            style(format!("{label:<11}")).dim()
        ));
    }
    out.push_str(&format!(
        "\n{MARGIN}{}\n{MARGIN}{}",
        style("This product includes software developed at Syntax Labs.").dim(),
        style("Distributed on an \"AS IS\" basis, without warranties or conditions of any kind.")
            .dim()
    ));
    out
}

/// Turns colour off before anything at all is painted.
///
/// Kept apart from [`init`] because the help and `--version` are rendered by `clap` during
/// parsing, hence before anything else is known about the arguments.
pub fn set_colors(enabled: bool) {
    if !enabled {
        console::set_colors_enabled(false);
        console::set_colors_enabled_stderr(false);
    }
}

/// A blank line — none when the output is bare: they only give air to an eye.
pub fn blank() {
    if !plain() {
        println!();
    }
}

/// A line of output, and the record that one has been written.
///
/// Every write goes through here or through [`eline`]: let a single one slip past, and the flag
/// lies.
fn line(text: String) {
    FRESH.store(false, Ordering::Relaxed);
    if json() {
        eprintln!("{text}");
    } else {
        println!("{text}");
    }
}

/// The same thing on the error output.
fn eline(text: String) {
    FRESH.store(false, Ordering::Relaxed);
    eprintln!("{text}");
}

/// The command's title, and in one line what it does.
///
/// The purpose line is not decoration: `build`, `dev` and `publish` do not do what their names
/// would suggest — `dev` does not bring up a local gateway, `publish` does not stop at pushing.
/// Saying it up front costs one line and saves finding it out some other way.
pub fn header(command: &str, purpose: &str) {
    if plain() {
        return;
    }
    blank();
    println!(
        "{MARGIN}{}  {}",
        style(command).bold(),
        style(format!("v{}", env!("CARGO_PKG_VERSION"))).dim()
    );
    println!("{MARGIN}{}", style(purpose).dim());
    blank();
    FRESH.store(true, Ordering::Relaxed);
}

/// The width beyond which a column of names pushes the explanations too far out.
const COLUMN_CAP: usize = 32;

/// Below it, the column tightens to the point where the two halves touch.
const COLUMN_FLOOR: usize = 16;

/// A discreet subheading that opens a list: "next", "what you got".
pub fn section(title: &str) {
    if plain() {
        // The title stays: for `queries · read-only` against `commands · mutating`, it is the
        // title that carries the information, not the decoration.
        line(title.to_string());
        return;
    }
    // Right after the header, the blank line is already there.
    if !FRESH.swap(false, Ordering::Relaxed) {
        blank();
    }
    line(format!("{MARGIN}{}", style(title).dim()));
}

/// A list under its subheading: each name, then what it is for.
///
/// Without the second column, a list of paths or of commands assumes one already knows how to
/// read them — that is, that one does not need it.
///
/// The column settles on the group, not on a constant: a list of short paths tightens up, a list
/// of long commands breathes. Past [`COLUMN_CAP`], the explanation wraps to the next line rather
/// than heading off towards the right edge of the terminal.
pub fn list(title: &str, rows: &[(&str, &str)]) {
    section(title);

    match column_width(rows) {
        Some(width) => {
            for (name, purpose) in rows {
                let padding = " ".repeat(width - name.chars().count());
                line(format!(
                    "{}{}{padding}  {}",
                    indent(),
                    style(name).cyan(),
                    style(purpose).dim()
                ));
            }
        }
        None => {
            for (name, purpose) in rows {
                line(format!("{}{}", indent(), style(name).cyan()));
                line(format!("{}  {}", indent(), style(purpose).dim()));
            }
        }
    }
}

/// The width of the left column, or `None` if it has to go to two lines.
///
/// Kept apart from the writing so that it can be decided without a terminal: it is the only part
/// of the layout that can be said to be right or wrong.
fn column_width(rows: &[(&str, &str)]) -> Option<usize> {
    let widest = rows.iter().map(|(name, _)| name.chars().count()).max()?;
    (widest <= COLUMN_CAP).then(|| widest.max(COLUMN_FLOOR))
}

/// Something done.
pub fn success(message: impl Display) {
    line(glyphed(style(TICK).green().bold().to_string(), message));
}

/// Something there was no reason to do.
pub fn skipped(message: impl Display) {
    line(glyphed(style(DOT).dim().to_string(), style(message).dim()));
}

/// A failure with no chain of causes — the one `clap` renders, for instance.
pub fn failure(message: impl Display) {
    if plain() {
        // The prefix stands in for the cross: without it, a bare failure can no longer be told
        // apart from a result line in a log.
        eline(format!("error: {message}"));
        return;
    }
    eline(format!("{MARGIN}{} {message}", style(CROSS).red().bold()));
}

/// Something to know that prevents nothing — on stderr: stdout is left to what a script reads.
pub fn warn(message: impl Display) {
    eline(glyphed(
        style(BANG).yellow().bold().for_stderr().to_string(),
        message,
    ));
}

/// A warning that scripts read on stdout — on stderr under `--json`.
///
/// Only use: "already in the registry", which `portaki-release-action` v1 and the
/// `portaki-modules` workflow look for with `grep` on standard output. To be dropped once the
/// action reads the `--json` output.
pub fn warn_on_stdout(message: impl Display) {
    line(glyphed(style(BANG).yellow().bold().to_string(), message));
}

/// A result handed back from elsewhere — an operation's response, a log body.
pub fn result(message: impl Display) {
    line(glyphed(style(ARROW).cyan().to_string(), message));
}

/// A piece of advice on what comes next — kept quiet when the output is bare.
///
/// Ignored, when the output is bare.
///
/// The difference with [`detail`] is one of addressee: a detail informs about what has just
/// happened, a piece of advice addresses someone who is learning the command.
pub fn advice(message: impl Display) {
    if plain() {
        return;
    }
    detail(message);
}

/// A detail under the line above it.
pub fn detail(message: impl Display) {
    line(format!("{}{}", indent(), style(message).dim()));
}

/// A file produced: what it is, then where it is.
pub fn wrote(kind: &str, where_: impl Display) {
    if plain() {
        line(format!("{kind:<10}  {where_}"));
        return;
    }
    line(format!(
        "{MARGIN}{} {} {}",
        style(TICK).green().bold(),
        style(format!("{kind:<10}")),
        style(where_).dim()
    ));
}

/// A label / value pair, aligned with its neighbours.
pub fn field(label: &str, value: impl Display) {
    line(format!(
        "{}{} {value}",
        indent(),
        style(format!("{label:<11}")).dim()
    ));
}

/// What comes next: what the user will type after this, and what it will give them.
pub fn next(steps: &[(&str, &str)]) {
    // This block addresses someone discovering the command. A script has no use for what comes
    // next: it has already written it.
    if plain() {
        return;
    }
    list(&crate::tr!("next", "ensuite"), steps);
}

/// A separating rule, to mark a fresh start in a session that goes on.
pub fn rule(label: &str) {
    let width = Term::stdout().size().1.clamp(20, 100) as usize;
    let filler = width.saturating_sub(label.chars().count() + MARGIN.len() + 4);
    let dash = if Term::stdout().is_term() { '─' } else { '-' };
    let rule = dash.to_string().repeat(filler);
    line(format!(
        "{MARGIN}{} {}",
        style(format!("{dash}{dash} {label}")).dim(),
        style(rule).dim()
    ));
}

/// A size in bytes as one reads it.
pub fn bytes(count: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = count as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{count} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

/// A short code, set apart so that it can be copied out without error.
pub fn code_block(code: &str) {
    let width = code.chars().count() + 4;
    let (tl, tr, bl, br, h, v) = if Term::stdout().is_term() {
        ('┌', '┐', '└', '┘', '─', '│')
    } else {
        ('+', '+', '+', '+', '-', '|')
    };
    let rule = h.to_string().repeat(width);

    line(format!(
        "{MARGIN}{}",
        style(format!("{tl}{rule}{tr}")).dim()
    ));
    line(format!(
        "{MARGIN}{}  {}  {}",
        style(v).dim(),
        style(code).bold().cyan(),
        style(v).dim()
    ));
    line(format!(
        "{MARGIN}{}",
        style(format!("{bl}{rule}{br}")).dim()
    ));
}

/// The failure, as the process's last word: the message, then the chain of causes.
///
/// With no blank line in front: what precedes it has already put one there — the command's
/// header, the result of the step that has just failed, or the block of captured output from the
/// driven tool.
///
/// `anyhow` stacks the context from the closest to the caller down to the deepest. Unfolded
/// rather than printed with `{:#}`, one reads first what failed, then why.
pub fn report(failure: &anyhow::Error) {
    if plain() {
        eline(format!("error: {failure}"));
        for cause in failure.chain().skip(1) {
            eline(format!("caused by: {cause}"));
        }
        return;
    }
    eline(format!("{MARGIN}{} {failure}", style(CROSS).red().bold()));
    for cause in failure.chain().skip(1) {
        eline(format!(
            "{MARGIN}  {} {}",
            style(crate::tr!("caused by", "cause")).dim(),
            style(cause).dim()
        ));
    }
    blank();
}

/// A step under way, which will become a result line.
///
/// The spinner does not outlive the step: `done`, `skip` and `fail` clear it before writing, so
/// that output read back never holds a frozen frame of the animation.
pub struct Step {
    bar: ProgressBar,
    started: Instant,
}

/// Opens a step.
pub fn step(label: impl Into<String>) -> Step {
    let label = label.into();
    let bar = if attended() {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::with_template("{prefix}{spinner:.cyan} {msg}")
                .expect("gabarit d'indicateur")
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
        );
        bar.set_prefix(MARGIN);
        bar.enable_steady_tick(Duration::from_millis(80));
        bar
    } else {
        ProgressBar::hidden()
    };
    bar.set_message(label);
    Step {
        bar,
        started: Instant::now(),
    }
}

impl Step {
    /// Changes what the step says of itself without closing it.
    pub fn say(&self, message: impl Into<String>) {
        self.bar.set_message(message.into());
    }

    /// Closes the step on a success, followed by the time it took.
    pub fn done(&self, message: impl Display) {
        let elapsed = self.close();
        line(format!(
            "{MARGIN}{} {message}  {}",
            style(TICK).green().bold(),
            style(elapsed).dim()
        ));
    }

    /// Closes the step on neither success nor failure: there was nothing to do.
    pub fn skip(&self, message: impl Display) {
        self.close();
        skipped(message);
    }

    /// Gives the step up without saying anything.
    ///
    /// The why is already in the error on its way up, and [`report`] will write it at the end of
    /// the run. A failure line here would repeat it almost word for word — two crosses for a
    /// single problem.
    pub fn abandon(&self) {
        self.close();
    }

    fn close(&self) -> String {
        self.bar.finish_and_clear();
        elapsed(self.started.elapsed())
    }
}

/// Runs a tool behind a spinner; its output only comes back up if it fails.
///
/// A `cargo build` that succeeds teaches nothing — and its hundreds of lines drown the little the
/// CLI has to say. Let it fail, and it is the other way round: everything it wrote is what one is
/// after. `--verbose` streams the output live, for long builds one wants to watch make progress.
pub fn command(label: &str, cmd: &mut Command) -> Result<()> {
    let step = step(label.to_owned());

    if verbose() {
        keep_stdout_clean(cmd);
        let status = cmd.status().with_context(|| format!("run {label}"))?;
        if !status.success() {
            step.abandon();
            bail!("{label} failed");
        }
        step.done(label);
        return Ok(());
    }

    let output = cmd.output().with_context(|| format!("run {label}"))?;
    if !output.status.success() {
        step.abandon();
        emit_captured(&output.stderr);
        emit_captured(&output.stdout);
        bail!("{label} failed");
    }
    step.done(label);
    Ok(())
}

/// Renders a tool's output without dressing it up: it is what one reads in order to fix things.
pub(crate) fn emit_captured(bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_end();
    if text.is_empty() {
        return;
    }
    for raw in text.lines() {
        eline(format!("{}{raw}", indent()));
    }
    blank();
}

/// A duration as one reads it, not as it is measured.
pub fn elapsed(duration: Duration) -> String {
    let seconds = duration.as_secs_f64();
    if seconds < 1.0 {
        format!("{}ms", duration.as_millis())
    } else if seconds < 60.0 {
        format!("{seconds:.1}s")
    } else {
        format!(
            "{}m {:02}s",
            duration.as_secs() / 60,
            duration.as_secs() % 60
        )
    }
}

/// A countdown, for a delay the server imposes.
pub fn countdown(remaining: Duration) -> String {
    let seconds = remaining.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// Opens a URL in the user's browser.
///
/// Detached: `open` hands control back straight away instead of waiting for the browser to close,
/// which would block the polling right after it. A failure is not really one — the URL stays on
/// screen, to be opened by hand.
///
/// `https` only (plain `http` to this machine for a local platform): the URL comes from the
/// server, and `open` hands anything else — `file:`, a custom scheme — to whatever handles it.
pub fn open_browser(url: &str) -> bool {
    crate::auth::secure_or_loopback(url) && open::that_detached(url).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In bare output, nothing that addresses the eye survives: no margin, no glyph. That is
    /// the mode's promise, and it is checked on the functions that decide.
    ///
    /// The flag is global to the process: it is put back to its original value before leaving,
    /// without which this test would dictate the rendering of every test after it.
    #[test]
    fn plain_output_drops_what_only_an_eye_reads() {
        set_plain(true);
        assert_eq!(indent(), "");
        assert_eq!(glyphed("*".to_string(), "done"), "done");

        set_plain(false);
        assert_eq!(indent(), "    ");
        assert_eq!(glyphed("*".to_string(), "done"), format!("{MARGIN}* done"));
    }

    #[test]
    fn a_short_duration_reads_in_milliseconds() {
        assert_eq!(elapsed(Duration::from_millis(420)), "420ms");
    }

    #[test]
    fn a_build_reads_in_seconds() {
        assert_eq!(elapsed(Duration::from_millis(12_400)), "12.4s");
    }

    /// Past the minute, a duration in seconds no longer reads.
    #[test]
    fn a_long_build_reads_in_minutes() {
        assert_eq!(elapsed(Duration::from_secs(124)), "2m 04s");
    }

    /// A column settled on the group: short names tighten to the floor, middling ones push it
    /// out just as far as needed.
    /// The licence, the copyright holder and the no-warranty clause travel with the binary,
    /// which often circulates without its repository. A renamed `Cargo.toml` field would make
    /// them disappear without breaking anything — hence the assertion.
    #[test]
    fn the_version_screen_carries_what_protects_the_project() {
        let screen = long_version();

        assert!(screen.contains("Apache-2.0"));
        assert!(screen.contains("Syntax Labs"));
        assert!(screen.contains("AS IS"));
        assert!(screen.contains(env!("CARGO_PKG_REPOSITORY")));
    }

    #[test]
    fn the_help_footer_names_the_licence_and_the_source() {
        let footer = legal();

        assert!(footer.contains("Apache-2.0"));
        assert!(footer.contains("Copyright 2026 Syntax Labs"));
        assert!(footer.contains(env!("CARGO_PKG_HOMEPAGE")));
    }

    /// The wordmark's dot is the logo's only colour, and it sits on the baseline.
    #[test]
    fn the_wordmark_carries_its_dot() {
        assert!(DOT_ROW < LOGO.len());
        assert!(banner().contains("the module toolchain"));
        assert_eq!(LOGO.len(), 7);
    }

    #[test]
    fn a_column_settles_on_the_widest_name() {
        assert_eq!(column_width(&[("a", "x"), ("bb", "y")]), Some(COLUMN_FLOOR));
        assert_eq!(
            column_width(&[("portaki dev --watch", "x")]),
            Some("portaki dev --watch".len())
        );
    }

    /// Past the cap, the second column would head off towards the right edge: it wraps to the
    /// next line rather than counting on the terminal's width.
    #[test]
    fn an_overlong_name_gives_up_the_column() {
        assert_eq!(
            column_width(&[("cargo doc --workspace --no-deps --open", "x")]),
            None
        );
    }

    #[test]
    fn an_empty_list_has_no_column() {
        assert_eq!(column_width(&[]), None);
    }

    #[test]
    fn bytes_stay_readable_at_every_scale() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(2048), "2.0 KB");
        assert_eq!(bytes(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn a_countdown_always_pads_the_seconds() {
        assert_eq!(countdown(Duration::from_secs(598)), "9:58");
        assert_eq!(countdown(Duration::from_secs(61)), "1:01");
        assert_eq!(countdown(Duration::from_secs(9)), "0:09");
    }
}
