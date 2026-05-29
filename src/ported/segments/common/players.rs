// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/common/players.py`.
//!
//! Media-player segment helpers. Surfaces the pure transformation
//! functions (state translation, seconds → "M:SS", state-symbol
//! table) + the `PlayerSegment` render path. The concrete player
//! backends (cmus / mpd / dbus / mpris / spotify / rdio /
//! rhythmbox / clementine) shell out via `asrun` / `run_cmd` and
//! parse vendor-specific output; those are deferred since each
//! needs its own platform-specific subprocess wiring.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// import re                                        // py:5
// from powerline.lib.shell import asrun, run_cmd  // py:7
// from powerline.lib.unicode import out_u         // py:8
// from powerline.segments import Segment, with_docstring                                  // py:9

use serde_json::{json, Map, Value};

/// Port of `STATE_SYMBOLS` from
/// `powerline/segments/common/players.py:12-17`.
///
/// Returns a fresh dict of the default state symbols
/// (`{"fallback": "", "play": ">", "pause": "~", "stop": "X"}`).
pub fn state_symbols() -> Map<String, Value> {
    let mut m = Map::new();
    // py:12-17  STATE_SYMBOLS = {...}
    m.insert("fallback".to_string(), Value::String("".into()));
    m.insert("play".to_string(), Value::String(">".into()));
    m.insert("pause".to_string(), Value::String("~".into()));
    m.insert("stop".to_string(), Value::String("X".into()));
    m
}

/// Port of `_convert_state()` from
/// `powerline/segments/common/players.py:20`.
///
/// Guess the canonical player state from a raw status string.
/// Returns one of `"play"` / `"pause"` / `"stop"` / `"fallback"`.
pub fn _convert_state(state: &str) -> &'static str {
    // py:22  state = state.lower()
    let lower = state.to_lowercase();
    // py:23-29  substring matching
    if lower.contains("play") {
        "play"
    } else if lower.contains("pause") {
        "pause"
    } else if lower.contains("stop") {
        "stop"
    } else {
        "fallback"
    }
}

/// Port of `_convert_seconds()` from
/// `powerline/segments/common/players.py:31`.
///
/// Convert a `seconds` value to `"M:SS"` format. The Python
/// source accepts both `str` (replaces `,` with `.` and calls
/// `float()`) and numeric inputs; the Rust port takes the parsed
/// f64 directly via the `Into<f64>` conversion.
pub fn _convert_seconds(seconds: f64) -> String {
    // py:32-34  if isinstance(seconds, str): seconds = seconds.replace(',', '.')
    //           return '{0:.0f}:{1:02.0f}'.format(*divmod(float(seconds), 60))
    let s = seconds.max(0.0);
    let mins = (s / 60.0).floor();
    let secs = s - mins * 60.0;
    format!("{:.0}:{:02.0}", mins, secs)
}

/// Variant of `_convert_seconds` that accepts a string input matching
/// Python's `isinstance(seconds, str)` branch at py:33.
pub fn _convert_seconds_str(seconds: &str) -> Option<String> {
    // py:33  seconds = seconds.replace(',', '.')
    let normalized = seconds.replace(',', ".");
    let parsed: f64 = normalized.trim().parse().ok()?;
    Some(_convert_seconds(parsed))
}

/// Stats produced by a player backend's `get_player_status`. Mirrors
/// the dict initialised at `powerline/segments/common/players.py:
/// 41-48`.
#[derive(Debug, Clone, Default)]
pub struct PlayerStats {
    pub state: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub elapsed: Option<String>,
    pub total: Option<String>,
}

impl PlayerStats {
    /// Constructs a fresh `PlayerStats` with `state: "fallback"`
    /// matching py:42.
    pub fn fallback() -> Self {
        Self {
            state: Some("fallback".to_string()),
            ..Default::default()
        }
    }
}

/// Port of `PlayerSegment.__call__()` from
/// `powerline/segments/common/players.py:40`.
///
/// `func_stats` is the result of the concrete backend's
/// `get_player_status` (Python returns a dict; Rust takes a
/// `PlayerStats`). Returns None when no stats are returned (py:50-51).
///
/// `format` uses str-format-like placeholders `{state_symbol}`,
/// `{album}`, `{artist}`, `{title}`, `{elapsed}`, `{total}`.
pub fn player_segment_call(
    func_stats: Option<PlayerStats>,
    format: &str,
    state_symbols_map: &Map<String, Value>,
) -> Option<Vec<Value>> {
    // py:50-51  if not func_stats: return None
    let stats = func_stats?;
    // py:43-44  start from default fallback stats, then update from func_stats
    let state = stats
        .state
        .clone()
        .unwrap_or_else(|| "fallback".to_string());
    // py:53  stats['state_symbol'] = state_symbols.get(stats['state'])
    let state_symbol = state_symbols_map
        .get(&state)
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // py:55  contents = format.format(**stats)
    let contents = format
        .replace("{state_symbol}", state_symbol)
        .replace("{album}", stats.album.as_deref().unwrap_or(""))
        .replace("{artist}", stats.artist.as_deref().unwrap_or(""))
        .replace("{title}", stats.title.as_deref().unwrap_or(""))
        .replace("{elapsed}", stats.elapsed.as_deref().unwrap_or(""))
        .replace("{total}", stats.total.as_deref().unwrap_or(""));
    // py:55-58  return [{contents, highlight_groups}]
    Some(vec![json!({
        "contents": contents,
        "highlight_groups": [format!("player_{}", state), "player"],
    })])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_symbols_table_matches_upstream() {
        // py:12-17  defaults
        let s = state_symbols();
        assert_eq!(s.get("fallback"), Some(&Value::String("".into())));
        assert_eq!(s.get("play"), Some(&Value::String(">".into())));
        assert_eq!(s.get("pause"), Some(&Value::String("~".into())));
        assert_eq!(s.get("stop"), Some(&Value::String("X".into())));
    }

    #[test]
    fn convert_state_play_returns_play() {
        // py:23-24  'play' in state.lower() → 'play'
        assert_eq!(_convert_state("playing"), "play");
        assert_eq!(_convert_state("Play"), "play");
        assert_eq!(_convert_state("PLAY"), "play");
    }

    #[test]
    fn convert_state_pause_returns_pause() {
        // py:25-26  'pause' in state.lower() → 'pause'
        assert_eq!(_convert_state("paused"), "pause");
        assert_eq!(_convert_state("Pause"), "pause");
    }

    #[test]
    fn convert_state_stop_returns_stop() {
        // py:27-28  'stop' in state.lower() → 'stop'
        assert_eq!(_convert_state("stopped"), "stop");
        assert_eq!(_convert_state("STOPPED"), "stop");
    }

    #[test]
    fn convert_state_unknown_returns_fallback() {
        // py:29  return 'fallback'
        assert_eq!(_convert_state("loading"), "fallback");
        assert_eq!(_convert_state(""), "fallback");
        assert_eq!(_convert_state("buffering"), "fallback");
    }

    #[test]
    fn convert_state_play_takes_precedence_over_pause() {
        // Python source: first match wins. "play" check runs first.
        assert_eq!(_convert_state("playpause"), "play");
    }

    #[test]
    fn convert_seconds_zero_emits_zero_zero_zero() {
        // py:32-34  divmod(0, 60) = (0, 0) → "0:00"
        assert_eq!(_convert_seconds(0.0), "0:00");
    }

    #[test]
    fn convert_seconds_under_a_minute_pads_to_two_digits() {
        // 45s → "0:45"
        assert_eq!(_convert_seconds(45.0), "0:45");
    }

    #[test]
    fn convert_seconds_one_minute_emits_one_zero_zero() {
        assert_eq!(_convert_seconds(60.0), "1:00");
    }

    #[test]
    fn convert_seconds_multi_minute_pads_seconds() {
        // 125 = 2:05
        assert_eq!(_convert_seconds(125.0), "2:05");
    }

    #[test]
    fn convert_seconds_handles_large_values() {
        // 3661 = 61 minutes, 1 second → "61:01"
        assert_eq!(_convert_seconds(3661.0), "61:01");
    }

    #[test]
    fn convert_seconds_str_accepts_dot_notation() {
        // py:33  seconds = seconds.replace(',', '.')
        assert_eq!(_convert_seconds_str("60.5"), Some("1:00".to_string()));
    }

    #[test]
    fn convert_seconds_str_accepts_comma_notation() {
        // py:33  comma-as-decimal-separator (locale-friendly)
        assert_eq!(_convert_seconds_str("60,5"), Some("1:00".to_string()));
    }

    #[test]
    fn convert_seconds_str_invalid_returns_none() {
        assert!(_convert_seconds_str("not a number").is_none());
    }

    #[test]
    fn player_stats_fallback_initial_state() {
        // py:42  state='fallback', all other fields None
        let s = PlayerStats::fallback();
        assert_eq!(s.state.as_deref(), Some("fallback"));
        assert!(s.album.is_none());
        assert!(s.artist.is_none());
        assert!(s.title.is_none());
        assert!(s.elapsed.is_none());
        assert!(s.total.is_none());
    }

    #[test]
    fn player_segment_call_no_stats_returns_none() {
        // py:50-51  if not func_stats: return None
        let symbols = state_symbols();
        let r = player_segment_call(None, "{state_symbol}", &symbols);
        assert!(r.is_none());
    }

    #[test]
    fn player_segment_call_emits_player_state_highlight_group() {
        // py:55-58  highlight_groups: ['player_<state>', 'player']
        let symbols = state_symbols();
        let stats = PlayerStats {
            state: Some("play".to_string()),
            artist: Some("Pink Floyd".to_string()),
            title: Some("Time".to_string()),
            ..Default::default()
        };
        let r = player_segment_call(Some(stats), "{state_symbol} {artist} - {title}", &symbols)
            .unwrap();
        assert_eq!(r[0]["highlight_groups"][0], "player_play");
        assert_eq!(r[0]["highlight_groups"][1], "player");
        assert_eq!(r[0]["contents"], "> Pink Floyd - Time");
    }

    #[test]
    fn player_segment_call_substitutes_all_placeholders() {
        let symbols = state_symbols();
        let stats = PlayerStats {
            state: Some("play".to_string()),
            album: Some("The Wall".to_string()),
            artist: Some("Pink Floyd".to_string()),
            title: Some("Time".to_string()),
            elapsed: Some("1:23".to_string()),
            total: Some("4:56".to_string()),
        };
        let r = player_segment_call(
            Some(stats),
            "{state_symbol}|{album}|{artist}|{title}|{elapsed}|{total}",
            &symbols,
        )
        .unwrap();
        assert_eq!(r[0]["contents"], ">|The Wall|Pink Floyd|Time|1:23|4:56");
    }

    #[test]
    fn player_segment_call_empty_fields_become_empty_strings() {
        // PlayerStats with None artist → empty substitution.
        let symbols = state_symbols();
        let stats = PlayerStats {
            state: Some("stop".to_string()),
            ..Default::default()
        };
        let r = player_segment_call(Some(stats), "{artist} - {title}", &symbols).unwrap();
        assert_eq!(r[0]["contents"], " - ");
    }

    #[test]
    fn player_segment_call_unknown_state_uses_fallback_symbol() {
        // py:53  state_symbols.get(state) — falls back to "" for unknown
        let mut symbols = state_symbols();
        symbols.remove("fallback");
        let stats = PlayerStats {
            state: Some("fallback".to_string()),
            ..Default::default()
        };
        let r = player_segment_call(Some(stats), "{state_symbol}|x", &symbols).unwrap();
        assert_eq!(r[0]["contents"], "|x");
    }

    #[test]
    fn player_segment_call_state_none_falls_back() {
        // PlayerStats with state=None: contents use 'fallback'
        let symbols = state_symbols();
        let stats = PlayerStats {
            state: None,
            ..Default::default()
        };
        let r = player_segment_call(Some(stats), "{state_symbol}", &symbols).unwrap();
        // fallback symbol is "" per py:13
        assert_eq!(r[0]["contents"], "");
        // highlight_groups[0] = "player_fallback"
        assert_eq!(r[0]["highlight_groups"][0], "player_fallback");
    }

    #[test]
    fn player_segment_call_with_custom_state_symbols() {
        // py:state_symbols arg overrides default symbols
        let mut custom = state_symbols();
        custom.insert("play".to_string(), Value::String("▶".into()));
        let stats = PlayerStats {
            state: Some("play".to_string()),
            ..Default::default()
        };
        let r = player_segment_call(Some(stats), "{state_symbol}", &custom).unwrap();
        assert_eq!(r[0]["contents"], "▶");
    }
}
