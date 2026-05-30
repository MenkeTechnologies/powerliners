// vim:fileencoding=utf-8:noet
//! Bundled-config asset extraction — shared by `powerline-daemon`,
//! `powerline-render`, and `powerline-config` so all three find the
//! same baked-in colorscheme / theme / `config.json` tree regardless
//! of how the binary was installed.
//!
//! Upstream Python ships the JSON tree as `setuptools` `data_files`
//! resolved at install-time via the bindings directory, so `cargo
//! install --path .` accidentally Just Worked through 0.2.4 (the
//! `CARGO_MANIFEST_DIR` baked in pointed at the live checkout). Any
//! install built off-machine (release tarball, brew, `cargo install`
//! from a non-source dir) baked in an unreachable path — `is_dir()`
//! returned false, the bundled fallback was silently dropped, and
//! users with no `~/.config/powerline/colorschemes/<ext>/` got
//! `"no colorscheme for <ext>"` errors.
//!
//! Fix mirrors `TMUX_CONFIG_DIRECTORY()` in `src/ported/config.rs:76`:
//! embed every JSON file via `include_str!` and extract on first call
//! to `$XDG_CACHE_HOME/powerliners/config_files/`. Lives in
//! `src/extensions/` — sanctioned non-port location per
//! `docs/PORT.md` — because the asset-extraction layer has no Python
//! equivalent.

use std::path::PathBuf;
use std::sync::OnceLock;

static CELL: OnceLock<Option<PathBuf>> = OnceLock::new();

const BUNDLED: &[(&str, &str)] = &[
    (
        "colors.json",
        include_str!("../ported/config_files/colors.json"),
    ),
    (
        "config.json",
        include_str!("../ported/config_files/config.json"),
    ),
    (
        "colorschemes/default.json",
        include_str!("../ported/config_files/colorschemes/default.json"),
    ),
    (
        "colorschemes/solarized.json",
        include_str!("../ported/config_files/colorschemes/solarized.json"),
    ),
    (
        "colorschemes/ipython/__main__.json",
        include_str!("../ported/config_files/colorschemes/ipython/__main__.json"),
    ),
    (
        "colorschemes/pdb/__main__.json",
        include_str!("../ported/config_files/colorschemes/pdb/__main__.json"),
    ),
    (
        "colorschemes/pdb/default.json",
        include_str!("../ported/config_files/colorschemes/pdb/default.json"),
    ),
    (
        "colorschemes/pdb/solarized.json",
        include_str!("../ported/config_files/colorschemes/pdb/solarized.json"),
    ),
    (
        "colorschemes/shell/__main__.json",
        include_str!("../ported/config_files/colorschemes/shell/__main__.json"),
    ),
    (
        "colorschemes/shell/default.json",
        include_str!("../ported/config_files/colorschemes/shell/default.json"),
    ),
    (
        "colorschemes/shell/solarized.json",
        include_str!("../ported/config_files/colorschemes/shell/solarized.json"),
    ),
    (
        "colorschemes/tmux/default.json",
        include_str!("../ported/config_files/colorschemes/tmux/default.json"),
    ),
    (
        "colorschemes/tmux/solarized.json",
        include_str!("../ported/config_files/colorschemes/tmux/solarized.json"),
    ),
    (
        "colorschemes/vim/__main__.json",
        include_str!("../ported/config_files/colorschemes/vim/__main__.json"),
    ),
    (
        "colorschemes/vim/default.json",
        include_str!("../ported/config_files/colorschemes/vim/default.json"),
    ),
    (
        "colorschemes/vim/solarized.json",
        include_str!("../ported/config_files/colorschemes/vim/solarized.json"),
    ),
    (
        "colorschemes/vim/solarizedlight.json",
        include_str!("../ported/config_files/colorschemes/vim/solarizedlight.json"),
    ),
    (
        "themes/ascii.json",
        include_str!("../ported/config_files/themes/ascii.json"),
    ),
    (
        "themes/powerline.json",
        include_str!("../ported/config_files/themes/powerline.json"),
    ),
    (
        "themes/powerline_terminus.json",
        include_str!("../ported/config_files/themes/powerline_terminus.json"),
    ),
    (
        "themes/powerline_unicode7.json",
        include_str!("../ported/config_files/themes/powerline_unicode7.json"),
    ),
    (
        "themes/unicode.json",
        include_str!("../ported/config_files/themes/unicode.json"),
    ),
    (
        "themes/unicode_terminus.json",
        include_str!("../ported/config_files/themes/unicode_terminus.json"),
    ),
    (
        "themes/unicode_terminus_condensed.json",
        include_str!("../ported/config_files/themes/unicode_terminus_condensed.json"),
    ),
    (
        "themes/ipython/in.json",
        include_str!("../ported/config_files/themes/ipython/in.json"),
    ),
    (
        "themes/ipython/in2.json",
        include_str!("../ported/config_files/themes/ipython/in2.json"),
    ),
    (
        "themes/ipython/out.json",
        include_str!("../ported/config_files/themes/ipython/out.json"),
    ),
    (
        "themes/ipython/rewrite.json",
        include_str!("../ported/config_files/themes/ipython/rewrite.json"),
    ),
    (
        "themes/pdb/default.json",
        include_str!("../ported/config_files/themes/pdb/default.json"),
    ),
    (
        "themes/shell/__main__.json",
        include_str!("../ported/config_files/themes/shell/__main__.json"),
    ),
    (
        "themes/shell/continuation.json",
        include_str!("../ported/config_files/themes/shell/continuation.json"),
    ),
    (
        "themes/shell/default.json",
        include_str!("../ported/config_files/themes/shell/default.json"),
    ),
    (
        "themes/shell/default_leftonly.json",
        include_str!("../ported/config_files/themes/shell/default_leftonly.json"),
    ),
    (
        "themes/shell/select.json",
        include_str!("../ported/config_files/themes/shell/select.json"),
    ),
    (
        "themes/tmux/default.json",
        include_str!("../ported/config_files/themes/tmux/default.json"),
    ),
    (
        "themes/vim/__main__.json",
        include_str!("../ported/config_files/themes/vim/__main__.json"),
    ),
    (
        "themes/vim/cmdwin.json",
        include_str!("../ported/config_files/themes/vim/cmdwin.json"),
    ),
    (
        "themes/vim/default.json",
        include_str!("../ported/config_files/themes/vim/default.json"),
    ),
    (
        "themes/vim/help.json",
        include_str!("../ported/config_files/themes/vim/help.json"),
    ),
    (
        "themes/vim/plugin_commandt.json",
        include_str!("../ported/config_files/themes/vim/plugin_commandt.json"),
    ),
    (
        "themes/vim/plugin_gundo-preview.json",
        include_str!("../ported/config_files/themes/vim/plugin_gundo-preview.json"),
    ),
    (
        "themes/vim/plugin_gundo.json",
        include_str!("../ported/config_files/themes/vim/plugin_gundo.json"),
    ),
    (
        "themes/vim/plugin_nerdtree.json",
        include_str!("../ported/config_files/themes/vim/plugin_nerdtree.json"),
    ),
    (
        "themes/vim/quickfix.json",
        include_str!("../ported/config_files/themes/vim/quickfix.json"),
    ),
    (
        "themes/vim/tabline.json",
        include_str!("../ported/config_files/themes/vim/tabline.json"),
    ),
    (
        "themes/wm/default.json",
        include_str!("../ported/config_files/themes/wm/default.json"),
    ),
];

/// First-call extraction of the 46 baked-in JSON files to
/// `$XDG_CACHE_HOME/powerliners/config_files/`. Returns `Some(path)`
/// when the cache dir exists post-extraction, `None` on disk error.
/// Memoized via `OnceLock` so subsequent calls in the same process
/// are zero-cost.
///
/// The write loop overwrites on every fresh process so an upgraded
/// binary picks up new JSON contents without a manual cache purge —
/// cheap because the total tree is ~50 KB.
pub fn bundled_config_dir() -> Option<PathBuf> {
    CELL.get_or_init(extract).clone()
}

fn extract() -> Option<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("powerliners")
        .join("config_files");
    if std::fs::create_dir_all(&cache).is_err() {
        return None;
    }
    for (rel, content) in BUNDLED {
        let target = cache.join(rel);
        if let Some(parent) = target.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&target, content);
    }
    cache.join("config.json").exists().then_some(cache)
}

/// Extract the vim driver script
/// (`src/ported/bindings/vim/powerline.vim`) to
/// `$XDG_CACHE_HOME/powerliners/vim/powerline.vim` and return the
/// resulting path. Mirrors `bundled_config_dir` but with one file
/// instead of 46 — the plugin is small enough that paying the
/// `OnceLock` for parity isn't worth the indirection.
pub fn bundled_vim_plugin_path() -> Option<PathBuf> {
    const VIM_PLUGIN: &str = include_str!("../ported/bindings/vim/powerline.vim");
    let dir = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("powerliners")
        .join("vim");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("powerline.vim");
    std::fs::write(&path, VIM_PLUGIN).ok()?;
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_lands_load_bearing_shell_files() {
        let dir = bundled_config_dir().expect("cache dir extractable");
        // The shell colorscheme tree is the one whose absence
        // triggered the 0.2.4 regression. Pin it explicitly.
        assert!(dir.join("colorschemes/shell/default.json").exists());
        assert!(dir.join("colorschemes/shell/__main__.json").exists());
        assert!(dir.join("themes/shell/default.json").exists());
        // Daemon-required roots.
        assert!(dir.join("config.json").exists());
        assert!(dir.join("colors.json").exists());
    }

    #[test]
    fn all_46_files_extracted() {
        let dir = bundled_config_dir().expect("cache dir extractable");
        for (rel, _) in BUNDLED {
            assert!(dir.join(rel).exists(), "expected extracted file at {rel}");
        }
        assert_eq!(BUNDLED.len(), 46);
    }
}
