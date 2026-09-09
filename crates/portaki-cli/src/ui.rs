//! Tout ce que la CLI montre passe par ici.
//!
//! Une commande ne fait pas de `println!`. Elle décrit ce qu'elle est en train de faire — une
//! étape, un champ, un échec — et cette couche décide de la forme. C'est ce qui garde la sortie
//! cohérente d'une commande à l'autre, et surtout ce qui permet de l'éteindre : redirigée dans
//! un fichier ou lue par une CI, la même commande écrit du texte nu, sans couleur ni animation.
//!
//! Rien n'est laissé à `NO_COLOR` seul : `--no-color` force l'extinction, `--verbose` remplace
//! les indicateurs par la sortie brute des outils pilotés.

use std::fmt::Display;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use console::{style, Emoji, Term};
use indicatif::{ProgressBar, ProgressStyle};

/// La marge de gauche commune à toutes les lignes : la sortie respire, et un bloc de texte se
/// distingue de ce qu'un outil appelé écrit sans marge.
const MARGIN: &str = "  ";

/// Repli ASCII compris : sortie redirigée, terminal Windows ancien, `TERM=dumb`.
static TICK: Emoji<'_, '_> = Emoji("✔", "+");
static CROSS: Emoji<'_, '_> = Emoji("✖", "x");
static BANG: Emoji<'_, '_> = Emoji("▲", "!");
static DOT: Emoji<'_, '_> = Emoji("·", "-");
static ARROW: Emoji<'_, '_> = Emoji("→", "->");

static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Fixe le mode de rendu pour tout le processus, avant la première ligne écrite.
pub fn init(no_color: bool, verbose: bool) {
    if no_color {
        console::set_colors_enabled(false);
        console::set_colors_enabled_stderr(false);
    }
    VERBOSE.store(verbose, Ordering::Relaxed);
}

/// L'utilisateur veut voir la sortie brute des outils pilotés.
pub fn verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

/// Quelqu'un regarde-t-il vraiment ? Sinon : pas d'animation, pas de réécriture de ligne.
fn attended() -> bool {
    console::user_attended() && !verbose()
}

/// Une ligne vide.
pub fn blank() {
    println!();
}

/// Le titre de la commande, une fois, en tête de sortie.
pub fn header(command: &str) {
    blank();
    println!(
        "{MARGIN}{}  {}",
        style(command).bold(),
        style(format!("v{}", env!("CARGO_PKG_VERSION"))).dim()
    );
    blank();
}

/// Une chose faite.
pub fn success(message: impl Display) {
    println!("{MARGIN}{} {message}", style(TICK).green().bold());
}

/// Une chose qui n'avait pas lieu d'être faite.
pub fn skipped(message: impl Display) {
    println!("{MARGIN}{} {}", style(DOT).dim(), style(message).dim());
}

/// Une chose à savoir, qui n'empêche rien.
pub fn warn(message: impl Display) {
    println!("{MARGIN}{} {message}", style(BANG).yellow().bold());
}

/// Un résultat renvoyé par ailleurs — la réponse d'une opération, un corps de trace.
pub fn result(message: impl Display) {
    println!("{MARGIN}{} {message}", style(ARROW).cyan());
}

/// Une précision sous la ligne qui précède.
pub fn detail(message: impl Display) {
    println!("{MARGIN}  {}", style(message).dim());
}

/// Un fichier produit : ce que c'est, puis où il est.
pub fn wrote(kind: &str, where_: impl Display) {
    println!(
        "{MARGIN}{} {} {}",
        style(TICK).green().bold(),
        style(format!("{kind:<10}")),
        style(where_).dim()
    );
}

/// Un couple étiquette / valeur, aligné avec ses voisins.
pub fn field(label: &str, value: impl Display) {
    println!("{MARGIN}  {} {value}", style(format!("{label:<11}")).dim());
}

/// La suite : ce que l'utilisateur tapera après.
pub fn next(steps: &[&str]) {
    blank();
    println!("{MARGIN}{}", style("next").dim());
    for step in steps {
        println!("{MARGIN}  {}", style(step).cyan());
    }
}

/// Un trait de séparation, pour marquer une reprise dans une session qui dure.
pub fn rule(label: &str) {
    let width = Term::stdout().size().1.clamp(20, 100) as usize;
    let filler = width.saturating_sub(label.chars().count() + MARGIN.len() + 4);
    let dash = if Term::stdout().is_term() { '─' } else { '-' };
    let rule = dash.to_string().repeat(filler);
    println!(
        "{MARGIN}{} {}",
        style(format!("{dash}{dash} {label}")).dim(),
        style(rule).dim()
    );
}

/// Une taille d'octets telle qu'on la lit.
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

/// Un code court, mis en évidence pour être recopié sans erreur.
pub fn code_block(code: &str) {
    let width = code.chars().count() + 4;
    let (tl, tr, bl, br, h, v) = if Term::stdout().is_term() {
        ('┌', '┐', '└', '┘', '─', '│')
    } else {
        ('+', '+', '+', '+', '-', '|')
    };
    let rule = h.to_string().repeat(width);

    println!("{MARGIN}{}", style(format!("{tl}{rule}{tr}")).dim());
    println!(
        "{MARGIN}{}  {}  {}",
        style(v).dim(),
        style(code).bold().cyan(),
        style(v).dim()
    );
    println!("{MARGIN}{}", style(format!("{bl}{rule}{br}")).dim());
}

/// L'échec, en dernier mot du processus : le message, puis la chaîne des causes.
///
/// `anyhow` empile le contexte du plus proche de l'appelant au plus profond. Déplié plutôt
/// qu'affiché en `{:#}`, on lit d'abord ce qui a échoué, puis pourquoi.
pub fn report(failure: &anyhow::Error) {
    blank();
    eprintln!("{MARGIN}{} {failure}", style(CROSS).red().bold());
    for cause in failure.chain().skip(1) {
        eprintln!(
            "{MARGIN}  {} {}",
            style("caused by").dim(),
            style(cause).dim()
        );
    }
    blank();
}

/// Une étape en cours, qui deviendra une ligne de résultat.
///
/// L'indicateur ne survit pas à l'étape : `done`, `skip` et `fail` l'effacent avant d'écrire, si
/// bien qu'une sortie relue ne contient jamais une image d'animation figée.
pub struct Step {
    bar: ProgressBar,
    started: Instant,
}

/// Ouvre une étape.
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
    /// Change ce que l'étape dit d'elle-même sans la clore.
    pub fn say(&self, message: impl Into<String>) {
        self.bar.set_message(message.into());
    }

    /// Clôt l'étape sur un succès, suivi du temps qu'elle a pris.
    pub fn done(&self, message: impl Display) {
        let elapsed = self.close();
        println!(
            "{MARGIN}{} {message} {}",
            style(TICK).green().bold(),
            style(elapsed).dim()
        );
    }

    /// Clôt l'étape sans succès ni échec : il n'y avait rien à faire.
    pub fn skip(&self, message: impl Display) {
        self.close();
        skipped(message);
    }

    /// Clôt l'étape sur un échec. L'erreur elle-même est rendue par [`report`].
    pub fn fail(&self, message: impl Display) {
        self.close();
        eprintln!("{MARGIN}{} {message}", style(CROSS).red().bold());
    }

    fn close(&self) -> String {
        self.bar.finish_and_clear();
        elapsed(self.started.elapsed())
    }
}

/// Lance un outil derrière un indicateur ; sa sortie ne remonte que s'il échoue.
///
/// Un `cargo build` qui réussit n'apprend rien — et ses centaines de lignes noient le peu que la
/// CLI a à dire. Qu'il échoue, et c'est l'inverse : tout ce qu'il a écrit est ce qu'on cherche.
/// `--verbose` rend la sortie en direct, pour les compilations longues qu'on veut voir avancer.
pub fn command(label: &str, cmd: &mut Command) -> Result<()> {
    let step = step(label.to_owned());

    if verbose() {
        let status = cmd.status().with_context(|| format!("run {label}"))?;
        if !status.success() {
            step.fail(label);
            bail!("{label} failed");
        }
        step.done(label);
        return Ok(());
    }

    let output = cmd.output().with_context(|| format!("run {label}"))?;
    if !output.status.success() {
        step.fail(label);
        emit_captured(&output.stderr);
        emit_captured(&output.stdout);
        bail!("{label} failed");
    }
    step.done(label);
    Ok(())
}

/// Rend la sortie d'un outil sans la maquiller : c'est elle qu'on lit pour corriger.
fn emit_captured(bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_end();
    if text.is_empty() {
        return;
    }
    blank();
    for line in text.lines() {
        eprintln!("{MARGIN}  {line}");
    }
    blank();
}

/// Une durée telle qu'on la lit, pas telle qu'elle est mesurée.
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

/// Un compte à rebours, pour un délai que le serveur impose.
pub fn countdown(remaining: Duration) -> String {
    let seconds = remaining.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// Ouvre une URL dans le navigateur de l'utilisateur.
///
/// Détaché : `open` rend la main tout de suite au lieu d'attendre la fermeture du navigateur, ce
/// qui bloquerait le sondage juste après. Un échec n'en est pas vraiment un — il reste l'URL
/// affichée, à ouvrir à la main.
pub fn open_browser(url: &str) -> bool {
    open::that_detached(url).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_duration_reads_in_milliseconds() {
        assert_eq!(elapsed(Duration::from_millis(420)), "420ms");
    }

    #[test]
    fn a_build_reads_in_seconds() {
        assert_eq!(elapsed(Duration::from_millis(12_400)), "12.4s");
    }

    /// Au-delà de la minute, une durée en secondes ne se lit plus.
    #[test]
    fn a_long_build_reads_in_minutes() {
        assert_eq!(elapsed(Duration::from_secs(124)), "2m 04s");
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
