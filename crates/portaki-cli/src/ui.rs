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
use console::{style, Emoji, Style, Term};
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

/// Rien n'a encore été écrit depuis l'en-tête.
///
/// Une section pose une ligne vide devant elle pour se détacher de ce qui précède. Juste après
/// l'en-tête, qui en pose déjà une, ça en faisait deux — un trou qui se lit comme un oubli.
static FRESH: AtomicBool = AtomicBool::new(false);

/// Fixe le mode de rendu pour tout le processus, avant la première ligne écrite.
pub fn init(no_color: bool, verbose: bool) {
    set_colors(!no_color);
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

/// Le logo, en cinq lignes de blocs.
///
/// Écrit ici plutôt que généré : une police de blocs se lit à l'œil, pas à l'exécution, et un
/// générateur ferait dépendre l'identité de la marque d'une dépendance de plus.
const LOGO: [&str; 5] = [
    "██████   ██████  ██████  ████████  █████  ██   ██ ██",
    "██   ██ ██    ██ ██   ██    ██    ██   ██ ██  ██  ██",
    "██████  ██    ██ ██████     ██    ███████ █████   ██",
    "██      ██    ██ ██   ██    ██    ██   ██ ██  ██  ██",
    "██       ██████  ██   ██    ██    ██   ██ ██   ██ ██",
];

/// Le dégradé du logo, du cyan clair au bleu — une couleur par ligne.
///
/// En 256 couleurs : la palette de base n'a pas assez de bleus pour un dégradé, et un terminal
/// qui ne les gère pas verra le texte nu, jamais des codes en clair.
const LOGO_RAMP: [u8; 5] = [51, 45, 39, 33, 27];

/// Le logo peint, prêt à être posé en tête d'un écran d'aide.
///
/// Rendu en `String` plutôt qu'imprimé : `clap` le veut comme en-tête de son aide, et le même
/// texte sert à `--version`.
pub fn banner() -> String {
    let mut out = String::from("\n");
    for (row, shade) in LOGO.iter().zip(LOGO_RAMP) {
        out.push_str(&format!(
            "{MARGIN}{}\n",
            Style::new().color256(shade).apply_to(row)
        ));
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

/// Ce qui protège le projet et ceux qui s'en servent, en pied d'aide.
///
/// La licence et le détenteur du copyright ne sont pas de la décoration : ils voyagent avec le
/// binaire, qui circule souvent sans son dépôt. Les lire coûte deux lignes.
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

/// Ce que `--version` raconte, par opposition au `-V` que lit un script.
///
/// Un `-V` doit rester une ligne analysable ; c'est la forme longue qui porte la licence, la
/// clause de non-garantie et où retrouver la source.
pub fn long_version() -> String {
    let rows = [
        ("license", env!("CARGO_PKG_LICENSE")),
        ("copyright", "Copyright 2026 Syntax Labs"),
        ("homepage", env!("CARGO_PKG_HOMEPAGE")),
        ("source", env!("CARGO_PKG_REPOSITORY")),
    ];
    let mut out = format!("{}\n{}\n", env!("CARGO_PKG_VERSION"), banner());
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

/// Éteint la couleur avant que quoi que ce soit ne soit peint.
///
/// Séparé d'[`init`] parce que l'aide et `--version` sont rendues par `clap` pendant l'analyse,
/// donc avant qu'on sache autre chose des arguments.
pub fn set_colors(enabled: bool) {
    if !enabled {
        console::set_colors_enabled(false);
        console::set_colors_enabled_stderr(false);
    }
}

/// Une ligne vide.
pub fn blank() {
    println!();
}

/// Une ligne de sortie, et la trace qu'il s'en est écrit une.
///
/// Toute écriture passe par ici ou par [`eline`] : une seule qui y échappe, et le drapeau ment.
fn line(text: String) {
    FRESH.store(false, Ordering::Relaxed);
    println!("{text}");
}

/// La même chose sur la sortie d'erreur.
fn eline(text: String) {
    FRESH.store(false, Ordering::Relaxed);
    eprintln!("{text}");
}

/// Le titre de la commande, et en une ligne ce qu'elle fait.
///
/// La ligne de propos n'est pas de la décoration : `build`, `dev` et `publish` ne font pas ce
/// que leur nom laisse supposer — `dev` ne monte pas de passerelle locale, `publish` ne se
/// limite pas à pousser. Le dire en tête coûte une ligne et évite de le découvrir autrement.
pub fn header(command: &str, purpose: &str) {
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

/// La largeur au-delà de laquelle une colonne de noms repousse trop loin les explications.
const COLUMN_CAP: usize = 32;

/// En deçà, la colonne se serre au point que les deux moitiés se touchent.
const COLUMN_FLOOR: usize = 16;

/// Un intertitre discret, qui ouvre une liste : « next », « what you got ».
pub fn section(title: &str) {
    // Juste après l'en-tête, la ligne vide est déjà là.
    if !FRESH.swap(false, Ordering::Relaxed) {
        blank();
    }
    line(format!("{MARGIN}{}", style(title).dim()));
}

/// Une liste sous son intertitre : chaque nom, puis à quoi il sert.
///
/// Sans la seconde colonne, une liste de chemins ou de commandes suppose qu'on sache déjà les
/// lire — c'est-à-dire qu'on n'en ait pas besoin.
///
/// La colonne se règle sur le groupe, pas sur une constante : une liste de chemins courts se
/// serre, une liste de commandes longues respire. Passé [`COLUMN_CAP`], l'explication passe à la
/// ligne plutôt que de partir chercher le bord droit du terminal.
pub fn list(title: &str, rows: &[(&str, &str)]) {
    section(title);

    match column_width(rows) {
        Some(width) => {
            for (name, purpose) in rows {
                let padding = " ".repeat(width - name.chars().count());
                line(format!(
                    "{MARGIN}  {}{padding}  {}",
                    style(name).cyan(),
                    style(purpose).dim()
                ));
            }
        }
        None => {
            for (name, purpose) in rows {
                line(format!("{MARGIN}  {}", style(name).cyan()));
                line(format!("{MARGIN}    {}", style(purpose).dim()));
            }
        }
    }
}

/// La largeur de la colonne de gauche, ou `None` s'il faut passer à deux lignes.
///
/// Isolée de l'écriture pour être décidable sans terminal : c'est la seule partie de la mise en
/// page dont on puisse dire qu'elle a tort ou raison.
fn column_width(rows: &[(&str, &str)]) -> Option<usize> {
    let widest = rows.iter().map(|(name, _)| name.chars().count()).max()?;
    (widest <= COLUMN_CAP).then(|| widest.max(COLUMN_FLOOR))
}

/// Une chose faite.
pub fn success(message: impl Display) {
    line(format!("{MARGIN}{} {message}", style(TICK).green().bold()));
}

/// Une chose qui n'avait pas lieu d'être faite.
pub fn skipped(message: impl Display) {
    line(format!(
        "{MARGIN}{} {}",
        style(DOT).dim(),
        style(message).dim()
    ));
}

/// Un échec sans chaîne de causes — celui que `clap` rend, par exemple.
pub fn failure(message: impl Display) {
    eline(format!("{MARGIN}{} {message}", style(CROSS).red().bold()));
}

/// Une chose à savoir, qui n'empêche rien.
pub fn warn(message: impl Display) {
    line(format!("{MARGIN}{} {message}", style(BANG).yellow().bold()));
}

/// Un résultat renvoyé par ailleurs — la réponse d'une opération, un corps de trace.
pub fn result(message: impl Display) {
    line(format!("{MARGIN}{} {message}", style(ARROW).cyan()));
}

/// Une précision sous la ligne qui précède.
pub fn detail(message: impl Display) {
    line(format!("{MARGIN}  {}", style(message).dim()));
}

/// Un fichier produit : ce que c'est, puis où il est.
pub fn wrote(kind: &str, where_: impl Display) {
    line(format!(
        "{MARGIN}{} {} {}",
        style(TICK).green().bold(),
        style(format!("{kind:<10}")),
        style(where_).dim()
    ));
}

/// Un couple étiquette / valeur, aligné avec ses voisins.
pub fn field(label: &str, value: impl Display) {
    line(format!(
        "{MARGIN}  {} {value}",
        style(format!("{label:<11}")).dim()
    ));
}

/// La suite : ce que l'utilisateur tapera après, et ce que ça lui donnera.
pub fn next(steps: &[(&str, &str)]) {
    list("next", steps);
}

/// Un trait de séparation, pour marquer une reprise dans une session qui dure.
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

/// L'échec, en dernier mot du processus : le message, puis la chaîne des causes.
///
/// Sans ligne vide devant : ce qui précède en a déjà posé une — l'en-tête de la commande, le
/// résultat de l'étape qui vient d'échouer, ou le bloc de sortie capturée de l'outil piloté.
///
/// `anyhow` empile le contexte du plus proche de l'appelant au plus profond. Déplié plutôt
/// qu'affiché en `{:#}`, on lit d'abord ce qui a échoué, puis pourquoi.
pub fn report(failure: &anyhow::Error) {
    eline(format!("{MARGIN}{} {failure}", style(CROSS).red().bold()));
    for cause in failure.chain().skip(1) {
        eline(format!(
            "{MARGIN}  {} {}",
            style("caused by").dim(),
            style(cause).dim()
        ));
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
        line(format!(
            "{MARGIN}{} {message}  {}",
            style(TICK).green().bold(),
            style(elapsed).dim()
        ));
    }

    /// Clôt l'étape sans succès ni échec : il n'y avait rien à faire.
    pub fn skip(&self, message: impl Display) {
        self.close();
        skipped(message);
    }

    /// Abandonne l'étape sans rien dire.
    ///
    /// Le pourquoi est déjà dans l'erreur qui remonte, et [`report`] l'écrira en fin de course.
    /// Une ligne d'échec ici la répéterait à un mot près — deux croix pour un seul problème.
    pub fn abandon(&self) {
        self.close();
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

/// Rend la sortie d'un outil sans la maquiller : c'est elle qu'on lit pour corriger.
fn emit_captured(bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_end();
    if text.is_empty() {
        return;
    }
    for raw in text.lines() {
        eline(format!("{MARGIN}  {raw}"));
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

    /// Une colonne réglée sur le groupe : des noms courts se serrent au plancher, des noms
    /// moyens l'écartent juste ce qu'il faut.
    /// La licence, le détenteur du copyright et la clause de non-garantie voyagent avec le
    /// binaire, qui circule souvent sans son dépôt. Un champ de `Cargo.toml` renommé les ferait
    /// disparaître sans rien casser — d'où l'assertion.
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

    /// Le logo est rectangulaire : une ligne plus courte que les autres se voit tout de suite.
    #[test]
    fn every_logo_row_is_the_same_width() {
        let width = LOGO[0].chars().count();

        assert!(LOGO.iter().all(|row| row.chars().count() == width));
        assert_eq!(LOGO.len(), LOGO_RAMP.len());
    }

    #[test]
    fn a_column_settles_on_the_widest_name() {
        assert_eq!(column_width(&[("a", "x"), ("bb", "y")]), Some(COLUMN_FLOOR));
        assert_eq!(
            column_width(&[("portaki dev --watch", "x")]),
            Some("portaki dev --watch".len())
        );
    }

    /// Passé le plafond, la seconde colonne partirait chercher le bord droit : on passe à la
    /// ligne plutôt que de compter sur la largeur du terminal.
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
