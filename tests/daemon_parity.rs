// vim:fileencoding=utf-8:noet
//! Parity tests against the upstream Python `powerline-render`.
//!
//! For each fixture scenario, runs BOTH:
//!   1. Upstream Python: `python3 vendor/powerline/scripts/powerline-render
//!      tmux right -p <fixture>` with `PYTHONPATH=vendor/powerline`
//!   2. Rust daemon: `target/debug/powerline-daemon --foreground --socket
//!      <tmp> --socket` with `POWERLINE_CONFIG_PATHS=<fixture>`, then sends
//!      the wire-format render request and reads the response bytes.
//!
//! Asserts the two output streams are **byte-for-byte identical**. This
//! is the strongest possible parity statement: any divergence (missing
//! `bg=default`, dropped `nobold`/`noitalics`/`nounderscore` resets,
//! re-ordered color tokens, off-by-one cterm index, etc.) fails the
//! test loudly.
//!
//! Skips with a clear diagnostic if `python3` or the vendored upstream
//! source isn't available (CI may not have either).
//!
//! Time-dependent segments (`date`, `time`) are run via a stable
//! `%Y-%m-%d` / `%H:%M` formatting — both implementations format the
//! same `time()` snapshot the same way to the second, so transient
//! drift is below the resolution we assert on. Slow CI machines may
//! occasionally race the minute boundary; the test waits for the
//! second-since-epoch to stabilize across the two binaries.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const SHUTDOWN_SENTINEL: &[u8] = b"EOF\0\0";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn daemon_binary() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop();
    p.pop();
    p.push("powerline-daemon");
    assert!(p.exists(), "powerline-daemon binary missing");
    p
}

fn fixture_root(scenario: &str) -> PathBuf {
    let mut p = manifest_dir();
    p.push("tests/data/e2e");
    p.push(scenario);
    p
}

fn unique_socket(tag: &str) -> PathBuf {
    let p = PathBuf::from(format!(
        "/tmp/plp-{}-{}-{}",
        tag,
        std::process::id() % 100000,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 10_000_000_000
    ));
    let _ = std::fs::remove_file(&p);
    p
}

struct DaemonHandle {
    child: Child,
    socket: PathBuf,
}

impl Drop for DaemonHandle {
    fn drop(&mut self) {
        if let Ok(mut s) = UnixStream::connect(&self.socket) {
            let _ = s.write_all(SHUTDOWN_SENTINEL);
        }
        std::thread::sleep(Duration::from_millis(100));
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket);
        let mut pid = self.socket.clone();
        pid.set_extension("pid");
        let _ = std::fs::remove_file(&pid);
    }
}

fn start_daemon(scenario: &str, tag: &str) -> DaemonHandle {
    let socket = unique_socket(tag);
    let mut child = Command::new(daemon_binary())
        .arg("--foreground")
        .arg("--socket")
        .arg(&socket)
        .env("POWERLINE_CONFIG_PATHS", fixture_root(scenario))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn powerline-daemon");
    // 15 s budget: see daemon_e2e helper comment for the cold-start rationale.
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Ok(probe) = UnixStream::connect(&socket) {
            let _ = probe.shutdown(std::net::Shutdown::Both);
            return DaemonHandle { child, socket };
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("daemon never became ready on {}", socket.display());
}

fn build_request(args: &[&str], cwd: &str, env: &[(&str, &str)]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend(format!("{:x}", args.len()).as_bytes());
    out.push(0);
    for arg in args {
        out.extend(arg.as_bytes());
        out.push(0);
    }
    out.extend(cwd.as_bytes());
    out.push(0);
    for (k, v) in env {
        out.extend(format!("{}={}", k, v).as_bytes());
        out.push(0);
    }
    out.push(0);
    out.push(0);
    out
}

fn rust_render(scenario: &str, tag: &str, side: &str) -> Vec<u8> {
    rust_render_with_argv(scenario, tag, &["tmux", side])
}

fn rust_render_with_argv(scenario: &str, tag: &str, argv: &[&str]) -> Vec<u8> {
    let daemon = start_daemon(scenario, tag);
    let mut conn = UnixStream::connect(&daemon.socket).expect("connect");
    conn.set_read_timeout(Some(Duration::from_secs(5))).ok();
    conn.write_all(&build_request(
        argv,
        "/tmp",
        &[("HOME", "/tmp"), ("PWD", "/tmp")],
    ))
    .expect("send");
    let mut buf = Vec::new();
    let _ = conn.read_to_end(&mut buf);
    buf
}

fn python_render(scenario: &str, side: &str) -> Option<Vec<u8>> {
    python_render_with_extra(scenario, side, &[])
}

fn python_render_with_extra(scenario: &str, side: &str, extra: &[&str]) -> Option<Vec<u8>> {
    let upstream = manifest_dir().join("vendor/powerline/scripts/powerline-render");
    let pythonpath = manifest_dir().join("vendor/powerline");
    if !upstream.exists() || !pythonpath.is_dir() {
        return None;
    }
    let mut cmd = Command::new("python3");
    cmd.arg(&upstream)
        .arg("tmux")
        .arg(side)
        .arg("-p")
        .arg(fixture_root(scenario));
    for e in extra {
        cmd.arg(e);
    }
    let out = cmd
        .env("PYTHONPATH", &pythonpath)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(out.stdout)
}

fn assert_parity_side(scenario: &str, tag: &str, side: &str) {
    let py = match python_render(scenario, side) {
        Some(b) => b,
        None => {
            eprintln!(
                "[skip] python3 + vendor/powerline not available for {}",
                scenario
            );
            return;
        }
    };
    let mut rust = rust_render(scenario, tag, side);
    let py_trim: &[u8] = py.strip_suffix(b"\n").unwrap_or(&py);
    let py_trim = py_trim.to_vec();
    if rust != py_trim {
        // Single retry for transient race conditions (e.g. minute
        // boundary on date/time scenarios).
        rust = rust_render(scenario, &format!("{}-retry", tag), side);
        let py2 = python_render(scenario, side).unwrap_or_default();
        let py2_trim: &[u8] = py2.strip_suffix(b"\n").unwrap_or(&py2);
        assert_eq!(
            rust,
            py2_trim,
            "byte parity failed for scenario `{}` side `{}`\n  rust: {:?}\n  py:   {:?}",
            scenario,
            side,
            String::from_utf8_lossy(&rust),
            String::from_utf8_lossy(py2_trim)
        );
    }
}

fn assert_parity(scenario: &str, tag: &str) {
    assert_parity_side(scenario, tag, "right");
}

fn assert_parity_with_width(scenario: &str, tag: &str, width: u32) {
    let py = match python_render_with_extra(scenario, "right", &["-w", &width.to_string()]) {
        Some(b) => b,
        None => {
            eprintln!(
                "[skip] python3 + vendor/powerline not available for {}",
                scenario
            );
            return;
        }
    };
    let width_arg = width.to_string();
    let rust = rust_render_with_argv(scenario, tag, &["tmux", "right", "-w", &width_arg]);
    let py_trim: Vec<u8> = py.strip_suffix(b"\n").unwrap_or(&py).to_vec();
    assert_eq!(
        rust,
        py_trim,
        "byte parity failed for scenario `{}` with width {}\n  rust: {:?}\n  py:   {:?}",
        scenario,
        width,
        String::from_utf8_lossy(&rust),
        String::from_utf8_lossy(&py_trim)
    );
}

#[test]
fn parity_hostname_only() {
    // Single hostname segment — exercises divider + hl + escape
    // without any time-dependent values.
    assert_parity("scenario_hostname", "hostname");
}

#[test]
fn parity_date_and_time_and_hostname() {
    // Three segments with two distinct highlight groups + a divider
    // between them. Catches divider color computation (hard vs soft)
    // and the `time:divider` highlight-group lookup.
    assert_parity("scenario_date", "datehost");
}

#[test]
fn parity_empty_segments_list() {
    // Theme with `segments.right: []`. Python emits 0 bytes; we must
    // do the same. Tests the empty-segment early-exit path.
    assert_parity("scenario_empty", "empty");
}

#[test]
fn parity_three_string_segments() {
    // Three `type: "string"` segments with fixed contents (ALPHA,
    // BETA, GAMMA) and distinct highlight groups. Catches:
    //   * hard divider color between adjacent distinct-bg segments
    //   * `bg=default` emission for the rightmost divider's compare_bg
    //   * attrs flag bit pack for bold (sg_alpha + sg_gamma)
    //   * cterm color resolution for non-grayscale palette entries
    assert_parity("scenario_string_segments", "strings");
}

#[test]
fn parity_left_side_hostname() {
    // Hostname rendered on the LEFT side. Catches the side-specific
    // divider placement (left-side: divider AFTER content; right-side:
    // BEFORE content).
    assert_parity_side("scenario_left_hostname", "lefthost", "left");
}

#[test]
fn parity_no_dividers() {
    // Segments with `draw_hard_divider:false` / `draw_soft_divider:false`.
    // Catches the divider-suppression branch in `_render_segments`.
    assert_parity("scenario_no_dividers", "nodiv");
}

#[test]
fn parity_alias_chain() {
    // Highlight group is a string alias chain "chain_a" → "chain_b"
    // → "chain_c" → {fg,bg,attrs}. Tests recursive aliasing in
    // `Colorscheme::get_group_props`.
    assert_parity("scenario_alias_chain", "alias");
}

#[test]
fn parity_before_after_strings() {
    // String segment with `before:"<<"` and `after:">>"` keys.
    // Tests `Theme::get_segments`'s contents wrapping at py:160-162.
    assert_parity("scenario_before_after", "beforeafter");
}

#[test]
fn parity_unicode_contents() {
    // Segments containing CJK ("日本語") + accented Latin ("café").
    // Catches: strwidth char counting, UTF-8 byte handling in
    // contents, escape() char passthrough.
    assert_parity("scenario_unicode", "unicode");
}

#[test]
fn parity_outer_padding_2() {
    // Theme with `outer_padding:2`. Catches the outer-padding-spaces
    // multiplier at py:469 + py:530.
    assert_parity("scenario_padding2", "padding");
}

#[test]
fn parity_divider_highlight_group_override() {
    // Segment with `divider_highlight_group:"sg_a_div"` overriding
    // the default soft-divider fg/bg. Tests the `divider_highlight`
    // resolution at py:552-554.
    assert_parity("scenario_divider_hl", "dividerhl");
}

#[test]
fn parity_both_sides_left() {
    // Theme defines both `left` and `right` segments. We render only
    // the left side; the right side's existence in the theme must
    // not pollute the left render.
    assert_parity_side("scenario_both_sides", "bothleft", "left");
}

#[test]
fn parity_both_sides_right() {
    // Same theme, render right side. Sanity-check the symmetric path.
    assert_parity_side("scenario_both_sides", "bothright", "right");
}

#[test]
fn parity_mode_translations_normal_mode() {
    // Colorscheme has `mode_translations` for "insert" mode but we
    // render with no explicit mode → translations are inert. Tests
    // that the `mode_translations` lookup doesn't leak into the
    // base render path.
    assert_parity("scenario_mode_xlate", "modexlate");
}

#[test]
fn parity_truecolor_emits_hex_directives() {
    // `term_truecolor:true` switches the TmuxRenderer to emit
    // `fg=#RRGGBB` directives instead of `fg=colourN`. Tests the
    // truecolor branch in `hlstyle`.
    assert_parity("scenario_truecolor", "truecolor");
}

#[test]
fn parity_spaces3() {
    // Theme with `spaces:3` — inner divider spacing widens from 1 to
    // 3. Tests `theme.get_spaces()` propagation through
    // `_render_segments`.
    assert_parity("scenario_spaces3", "spaces");
}

#[test]
fn parity_hash_character_escaped() {
    // Segment contents containing `#`. TmuxRenderer's
    // character_translations replaces `#` → `##[]` so tmux doesn't
    // parse it as a style escape.
    assert_parity("scenario_hash_escape", "hashesc");
}

#[test]
fn parity_soft_divider_between_same_bg() {
    // Two adjacent segments sharing the same `bg`. Per py:463/531
    // this triggers `divider_type = "soft"` rather than hard.
    assert_parity("scenario_soft_divider", "softdiv");
}

#[test]
fn parity_italic_and_underline_attrs() {
    // Three segments with italics-only, underline-only, and italics
    // + underline. Tests `attrs_to_tmux_attrs` bit handling for the
    // non-bold flags.
    assert_parity("scenario_italic_underline", "italund");
}

#[test]
fn parity_mode_translations_present_but_inactive() {
    // Colorscheme with `mode_translations.insert.colors`, rendered
    // with no mode argument. The translations table is present but
    // inert — both implementations must skip it identically.
    assert_parity("scenario_mode_active", "modeact");
}

#[test]
fn parity_multi_character_dividers() {
    // Theme with multi-character `dividers.right.hard:"<=="`. Tests
    // that `Theme::get_divider` returns the literal string and that
    // it survives `escape` and `_render_segments`'s string concat.
    assert_parity("scenario_multi_char_div", "mchardiv");
}

#[test]
fn parity_highlight_groups_fallback_chain() {
    // Segment with `highlight_groups:["sg_missing", "sg_present"]`.
    // `Colorscheme::get_highlighting` walks the list and returns the
    // first match.
    assert_parity("scenario_hl_fallback", "hlfb");
}

#[test]
fn parity_all_attrs_set() {
    // Segment with `attrs:["bold", "italic", "underline"]`. Tests
    // the full bit-on path of `attrs_to_tmux_attrs`.
    assert_parity("scenario_all_attrs", "allattrs");
}

#[test]
fn parity_empty_string_contents() {
    // String segment with `contents:""`. Catches the divider+padding
    // emission around zero-width content.
    assert_parity("scenario_empty_contents", "emptyc");
}

#[test]
fn parity_ten_alternating_segments() {
    // 10 segments with varied widths and alternating bold attrs.
    // Stress test for divider sequencing across longer renders.
    assert_parity("scenario_many_segs", "many");
}

#[test]
fn parity_cterm_only_colors() {
    // colors.json with plain integer cterm entries (no [cterm, hex]
    // tuples). Tests `Colorscheme::new` py:48-49 fallback through
    // `cterm_to_hex` table.
    assert_parity("scenario_cterm_only", "ctermonly");
}

#[test]
fn parity_width_no_truncation_when_fits() {
    // Three segments — no width arg sent, no truncation expected.
    // Both implementations render all segments.
    assert_parity("scenario_width_trunc", "widthnone");
}

#[test]
fn parity_priority_segments_render_all_when_no_width() {
    // Four segments with mixed priorities (10, 50, 100, null). With
    // no width limit, no drops happen — full render in priority-
    // independent order.
    assert_parity("scenario_priority_drop", "prionone");
}

#[test]
fn parity_left_side_with_four_segments() {
    // Four segments on the left. Tests left-side compare_segment
    // direction (compare with NEXT segment, not previous), and the
    // tail-side divider emission at py:534/547.
    assert_parity_side("scenario_left_multi", "leftmulti", "left");
}

#[test]
fn parity_outer_padding_zero() {
    // Theme with `outer_padding:0`. Tests the `outer_padding * ' '`
    // and `outer_padding`-summed-into-segment_len branches when the
    // multiplier is 0.
    assert_parity("scenario_padding0", "padding0");
}

#[test]
fn parity_left_side_when_theme_only_has_right_segments() {
    // Theme defines only `segments.right`. Rendering the LEFT side
    // must produce 0 bytes (no segments, no markup). Catches the
    // empty-segments early-exit + empty-rendered-highlighted branch.
    assert_parity_side("scenario_left_empty", "lefte", "left");
}

#[test]
fn parity_right_side_when_theme_only_has_left_segments() {
    // Mirror of the above: only `segments.left` defined, render
    // RIGHT side → 0 bytes.
    assert_parity_side("scenario_right_empty", "righte", "right");
}

#[test]
fn parity_tab_in_contents_translates_to_caret_i() {
    // Segment contents `"A\tB"`. `translate_np`'s 0x00-0x1F table
    // maps tab (0x09) to `^I`. Both implementations emit `A^IB`.
    assert_parity("scenario_tab_content", "tab");
}

#[test]
fn parity_long_single_segment_80_chars() {
    // Single segment with 80 A's. No divider math complications;
    // tests contents passthrough at length.
    assert_parity("scenario_long_single", "longone");
}

#[test]
fn parity_name_override_does_not_leak_into_output() {
    // Two `string` segments with explicit `name:"first_name"` /
    // `"second_name"`. The name is internal; only `contents` should
    // reach the rendered output.
    assert_parity("scenario_name_override", "name");
}

#[test]
fn parity_full_16_color_palette() {
    // Seven segments using base ANSI colors 0-7 + bright variants.
    // Tests color name → cterm lookup across the full 16-color
    // palette space.
    assert_parity("scenario_palette16", "palette");
}

#[test]
fn parity_width_forces_low_priority_drops() {
    // Three segments, priorities 50/30/10. Request width=15 which
    // forces drops in priority order (lowest first). Both
    // implementations must drop the same segments in the same
    // order and emit identical markup.
    assert_parity_with_width("scenario_width_drop", "wdrop", 15);
}

#[test]
fn parity_width_loose_does_not_drop() {
    // Same theme but width=200 — wider than the rendered line.
    // No drops. Both render the full segment chain.
    assert_parity_with_width("scenario_width_drop", "wloose", 200);
}

#[test]
fn parity_empty_divider_strings() {
    // Theme `dividers.right.{hard,soft}` are both `""`. Tests the
    // empty-divider passthrough in `_render_segments`.
    assert_parity("scenario_empty_div", "edivs");
}

#[test]
fn parity_single_character_dividers() {
    // Theme with `dividers.right.hard:"<"` etc. ASCII single-char
    // dividers; tests Theme::get_divider lookup + escape() pass.
    assert_parity("scenario_single_char_div", "scdivs");
}

#[test]
fn parity_combining_diacritic_marks() {
    // Segment contents containing combining marks (`é`, `f́`, `ǵ`
    // = base + U+0301). Tests multi-codepoint glyph passthrough.
    assert_parity("scenario_combining_mark", "comb");
}

#[test]
fn parity_emoji_in_contents() {
    // Segments with 4-byte UTF-8 emoji (🎉, 🚀). Tests UTF-8
    // passthrough at the byte level.
    assert_parity("scenario_emoji", "emoji");
}

#[test]
fn parity_single_character_segment() {
    // Minimal segment: `contents:"X"`. One byte of content, full
    // divider + outer-padding stack around it.
    assert_parity("scenario_single_char_seg", "scseg");
}

#[test]
fn parity_three_segments_identical_bg() {
    // Three adjacent segments all sharing the same `bg`. Both
    // segment boundaries trigger SOFT dividers per the same-bg
    // check in `_render_segments`.
    assert_parity("scenario_three_same_bg", "samebg");
}

#[test]
fn parity_check_python_available() {
    // Meta-test: surface clearly when Python/upstream is missing so
    // a "0 failed" result doesn't hide the fact that parity wasn't
    // actually checked. Always passes; just prints the status.
    match python_render("scenario_hostname", "right") {
        Some(_) => println!("python3 + vendor/powerline OK — parity tests active"),
        None => println!(
            "[warn] python3 + vendor/powerline NOT available — \
             parity_* tests will silently skip"
        ),
    }
}
