// vim:fileencoding=utf-8:noet
//! Shared MenkeTechnologies house `--help` renderer for the powerliners
//! binaries.
//!
//! Included by each `src/bin/*.rs` entry via
//! `#[path = "shared/house_help.rs"] mod house_help;`. Provides the
//! banner + runtime-padded status box + SYSTEM footer (identical across
//! all five bins); each bin supplies only its own subtitle, USAGE line,
//! and option/mode sections.
//!
//! This is a display-only helper: it never touches the segment-render
//! path. Bins call [`help`] and `print!` its return before running any
//! normal logic when `-h` / `--help` is present.

#![allow(dead_code)]

/// ANSI-Shadow "POWERLINE" wordmark, cyan -> magenta -> red gradient.
/// Each row carries a leading space so its left edge lines up with the
/// status box (` ┌…┐`). Generated with figlet `ansi_shadow.flf`.
const BANNER: &str = concat!(
    "\x1b[36m ██████╗  ██████╗ ██╗    ██╗███████╗██████╗ ██╗     ██╗███╗   ██╗███████╗\x1b[0m\n",
    "\x1b[36m ██╔══██╗██╔═══██╗██║    ██║██╔════╝██╔══██╗██║     ██║████╗  ██║██╔════╝\x1b[0m\n",
    "\x1b[35m ██████╔╝██║   ██║██║ █╗ ██║█████╗  ██████╔╝██║     ██║██╔██╗ ██║█████╗\x1b[0m\n",
    "\x1b[35m ██╔═══╝ ██║   ██║██║███╗██║██╔══╝  ██╔══██╗██║     ██║██║╚██╗██║██╔══╝\x1b[0m\n",
    "\x1b[31m ██║     ╚██████╔╝╚███╔███╔╝███████╗██║  ██║███████╗██║██║ ╚████║███████╗\x1b[0m\n",
    "\x1b[31m ╚═╝      ╚═════╝  ╚══╝╚══╝ ╚══════╝╚═╝  ╚═╝╚══════╝╚═╝╚═╝  ╚═══╝╚══════╝\x1b[0m\n",
);

/// Inner width of the status box rule. The banner is 72 display columns
/// wide with a leading space (73); ` ┌` + 70×`─` + `┐` is also 73, so the
/// box right border lines up with the banner. Padded at runtime so the
/// border never drifts as VERSION grows.
const BOX_W: usize = 70;

/// Width the cyan `── SECTION ──` rules and the footer rule fill to.
const RULE_W: usize = 72;

/// One labelled group of option / mode lines. `items` are
/// `(left, comment)` pairs; the left columns are padded to a common
/// width at render time so every green `//` lines up.
pub struct Section<'a> {
    pub title: &'a str,
    pub items: &'a [(&'a str, &'a str)],
}

/// Build a cyan `  ── TITLE ─────…` rule padded to `RULE_W` columns.
fn section_rule(title: &str) -> String {
    // "  ── " (5) + title + " " (1) then fill with ─ up to RULE_W.
    let used = 5 + title.chars().count() + 1;
    let fill = RULE_W.saturating_sub(used);
    format!("\x1b[36m  ── {} {}\x1b[0m\n", title, "─".repeat(fill))
}

/// Render the full house `--help` screen.
///
/// `subtitle` is the plain one-line description under the tagline;
/// `tagline` is the magenta `>> … <<` banner strip; `usage` is the text
/// after the yellow `USAGE:` label; `sections` are the option/mode
/// groups; `footer_tagline` and `footer_jack` fill the SYSTEM footer.
pub fn help(
    subtitle: &str,
    tagline: &str,
    usage: &str,
    sections: &[Section<'_>],
    footer_tagline: &str,
    footer_jack: &str,
) -> String {
    let ver = env!("CARGO_PKG_VERSION");
    let rule = "─".repeat(BOX_W);
    let status = format!(" STATUS: ONLINE  // SIGNAL: ████████░░ // v{ver}");
    let space = " ".repeat(BOX_W.saturating_sub(status.chars().count()));

    // Widest left column across every section, so all green `//` align.
    let left_w = sections
        .iter()
        .flat_map(|s| s.items.iter())
        .map(|(l, _)| l.chars().count())
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    out.push('\n');
    out.push_str(BANNER);
    out.push_str(&format!(" \x1b[36m┌{rule}┐\x1b[0m\n"));
    out.push_str(&format!(
        " \x1b[36m│\x1b[0m{status}{space}\x1b[36m│\x1b[0m\n"
    ));
    out.push_str(&format!(" \x1b[36m└{rule}┘\x1b[0m\n"));
    out.push_str(&format!("\x1b[35m  >> {tagline} <<\x1b[0m\n\n"));
    out.push_str(&format!("  {subtitle}\n\n"));
    out.push_str(&format!("\x1b[33m  USAGE:\x1b[0m {usage}\n\n"));

    for sec in sections {
        out.push_str(&section_rule(sec.title));
        for (left, comment) in sec.items {
            let pad = " ".repeat(left_w.saturating_sub(left.chars().count()) + 2);
            out.push_str(&format!("  {left}{pad}\x1b[32m//\x1b[0m {comment}\n"));
        }
        out.push('\n');
    }

    out.push_str(&section_rule("SYSTEM"));
    out.push_str(&format!(
        "  \x1b[35mv{ver} \x1b[0m// \x1b[33m(c) Jacob Menke and contributors\x1b[0m\n"
    ));
    out.push_str(&format!("  \x1b[35m{footer_tagline}\x1b[0m\n"));
    out.push_str(&format!("  \x1b[33m>>> {footer_jack} <<<\x1b[0m\n"));
    out.push_str(&format!(" \x1b[36m{}\x1b[0m\n", "░".repeat(RULE_W)));
    out
}

/// True when `argv` requests the help screen (`-h` / `--help`).
pub fn wants_help(argv: &[String]) -> bool {
    argv.iter().any(|a| a == "-h" || a == "--help")
}

/// True when `argv` requests the version (`-V` / `--version`).
pub fn wants_version(argv: &[String]) -> bool {
    argv.iter().any(|a| a == "-V" || a == "--version")
}

/// `<bin> <VERSION>` line for `-V` / `--version`.
pub fn version_line(bin: &str) -> String {
    format!("{bin} {}", env!("CARGO_PKG_VERSION"))
}
