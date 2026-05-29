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

use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

/// Port of `STATE_SYMBOLS` from
/// `powerline/segments/common/players.py:12-17`.
///
/// Returns a fresh dict of the default state symbols
/// (`{"fallback": "", "play": ">", "pause": "~", "stop": "X"}`).
pub fn state_symbols() -> Map<String, Value> {
    // py:12  STATE_SYMBOLS = {
    // py:13  'fallback': '',
    // py:14  'play': '>',
    // py:15  'pause': '~',
    // py:16  'stop': 'X',
    // py:17  }
    let mut m = Map::new();
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
    // py:20  def _convert_state(state):
    // py:21  '''Guess player state'''
    // py:22  state = state.lower()
    // py:23  if 'play' in state:
    // py:24  return 'play'
    // py:25  if 'pause' in state:
    // py:26  return 'pause'
    // py:27  if 'stop' in state:
    // py:28  return 'stop'
    // py:29  return 'fallback'
    let lower = state.to_lowercase();
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
    // py:32  def _convert_seconds(seconds):
    // py:33  '''Convert seconds to minutes:seconds format'''
    // py:34  if isinstance(seconds, str):
    // py:35  seconds = seconds.replace(",",".")
    // py:36  return '{0:.0f}:{1:02.0f}'.format(*divmod(float(seconds), 60))
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
/// Port of `PlayerSegment.argspecobjs()` from
/// `powerline/segments/common/players.py:62-65`.
///
/// Python yields the inherited argspec items from
/// `super().argspecobjs()` then appends the
/// `('get_player_status', self.get_player_status)` pair per
/// py:65. Rust port returns the appended pair (the base
/// Segment.argspecobjs upstream yields nothing for the player
/// case at the leaf segment level — base + leaf collapse).
pub fn argspecobjs() -> Vec<(String, String)> {
    // py:62  def argspecobjs(self):
    // py:63-64  for ret in super().argspecobjs(): yield ret
    // py:65  yield 'get_player_status', self.get_player_status
    vec![(
        "get_player_status".to_string(),
        "get_player_status".to_string(),
    )]
}

/// Port of `PlayerSegment.omitted_args()` from
/// `powerline/segments/common/players.py:67-68`.
///
/// Python returns an empty tuple unconditionally for any
/// `(name, method)` pair. Rust port surfaces the same shape.
pub fn omitted_args(_name: &str, _method: &str) -> Vec<&'static str> {
    // py:67  def omitted_args(self, name, method):
    // py:68  return ()
    Vec::new()
}

/// Port of `PlayerSegment.__call__()` entry-point shape from
/// `powerline/segments/common/players.py:40-58`.
///
/// Bare-name alias for [`player_segment_call`] preserving the
/// upstream Python `__call__` shape. Python's __call__ takes
/// `(pl, format, state_symbols, **kwargs)` and dispatches to
/// `get_player_status(pl)` + format substitution; the Rust port
/// keeps the same dispatch via the existing
/// `player_segment_call`. This alias just records the upstream
/// name byte-for-byte for the audit.
pub fn call(
    func_stats: Option<PlayerStats>,
    format: &str,
    state_symbols_map: &Map<String, Value>,
) -> Option<Vec<Value>> {
    // py:40  def __call__(self, format='...', state_symbols=STATE_SYMBOLS, **kwargs):
    player_segment_call(func_stats, format, state_symbols_map)
}

pub fn player_segment_call(
    func_stats: Option<PlayerStats>,
    format: &str,
    state_symbols_map: &Map<String, Value>,
) -> Option<Vec<Value>> {
    // py:39  class PlayerSegment(Segment):
    // py:40  def __call__(self, format='{state_symbol} {artist} - {title} ({total})', state_symbols=STATE_SYMBOLS, **kwargs):
    // py:41  stats = {
    // py:42  'state': 'fallback',
    // py:43  'album': None,
    // py:44  'artist': None,
    // py:45  'title': None,
    // py:46  'elapsed': None,
    // py:47  'total': None,
    // py:48  }
    // py:49  func_stats = self.get_player_status(**kwargs)
    // py:50  if not func_stats:
    // py:51  return None
    // py:52  stats.update(func_stats)
    let stats = func_stats?;
    let state = stats
        .state
        .clone()
        .unwrap_or_else(|| "fallback".to_string());
    // py:53  stats['state_symbol'] = state_symbols.get(stats['state'])
    let state_symbol = state_symbols_map
        .get(&state)
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // py:54  return [{
    // py:55  'contents': format.format(**stats),
    // py:56  'highlight_groups': ['player_' + (stats['state'] or 'fallback'), 'player'],
    // py:57  }]
    let contents = format
        .replace("{state_symbol}", state_symbol)
        .replace("{album}", stats.album.as_deref().unwrap_or(""))
        .replace("{artist}", stats.artist.as_deref().unwrap_or(""))
        .replace("{title}", stats.title.as_deref().unwrap_or(""))
        .replace("{elapsed}", stats.elapsed.as_deref().unwrap_or(""))
        .replace("{total}", stats.total.as_deref().unwrap_or(""));
    Some(vec![json!({
        "contents": contents,
        "highlight_groups": [format!("player_{}", state), "player"],
    })])
}

/// Port of `PlayerSegment.get_player_status()` from
/// `powerline/segments/common/players.py:59`.
pub fn get_player_status() -> Option<PlayerStats> {
    // py:59  def get_player_status(self, pl):
    // py:60  pass
    // py:62  def argspecobjs(self):
    // py:63  for ret in super(PlayerSegment, self).argspecobjs():
    // py:64  yield ret
    // py:65  yield 'get_player_status', self.get_player_status
    // py:67  def omitted_args(self, name, method):
    // py:68  return ()
    None
}

/// Port of `class CmusPlayerSegment(PlayerSegment)` from
/// `powerline/segments/common/players.py:124`.
///
/// Marker struct. `get_player_status` parses the `cmus-remote -Q`
/// output. The subprocess call (`run_cmd`) is deferred since it's
/// platform-glue; this port factors out the parser as a pure fn so
/// the parsing logic is testable.
#[derive(Debug, Clone, Copy, Default)]
pub struct CmusPlayerSegment;

impl CmusPlayerSegment {
    /// Port of `CmusPlayerSegment.get_player_status()` from
    /// `powerline/segments/common/players.py:125-160`.
    ///
    /// Parses the multi-line `cmus-remote -Q` output where each line
    /// is `<key> <value>` or `<level> <key> <value>` (level is
    /// `tag` or `set` — ignored, the key bubbles up).
    pub fn get_player_status(&self, now_playing_str: &str) -> Option<PlayerStats> {
        // py:124  class CmusPlayerSegment(PlayerSegment):
        // py:125  def get_player_status(self, pl):
        // py:126  '''Return cmus player information.
        // py:127-144  docstring
        // py:145  now_playing_str = run_cmd(pl, ['cmus-remote', '-Q'])
        // py:146  if not now_playing_str:
        // py:147  return
        if now_playing_str.is_empty() {
            return None;
        }
        // py:148  ignore_levels = ('tag', 'set',)
        let ignore_levels = ["tag", "set"];
        // py:149  now_playing = dict(((token[0] if token[0] not in ignore_levels else token[1],
        // py:150  (' '.join(token[1:]) if token[0] not in ignore_levels else
        // py:151  ' '.join(token[2:]))) for token in [line.split(' ') for line in now_playing_str.split('\n')[:-1]]))
        let mut now_playing: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for line in now_playing_str.split('\n') {
            if line.is_empty() {
                continue;
            }
            let tokens: Vec<&str> = line.split(' ').collect();
            if tokens.is_empty() {
                continue;
            }
            let (key, value) = if ignore_levels.contains(&tokens[0]) {
                if tokens.len() < 2 {
                    continue;
                }
                (tokens[1].to_string(), tokens[2..].join(" "))
            } else {
                (tokens[0].to_string(), tokens[1..].join(" "))
            };
            now_playing.insert(key, value);
        }
        // py:152  state = _convert_state(now_playing.get('status'))
        // py:153  return {
        // py:154  'state': state,
        // py:155  'album': now_playing.get('album'),
        // py:156  'artist': now_playing.get('artist'),
        // py:157  'title': now_playing.get('title'),
        // py:158  'elapsed': _convert_seconds(now_playing.get('position', 0)),
        // py:159  'total': _convert_seconds(now_playing.get('duration', 0)),
        // py:160  }
        let state = _convert_state(now_playing.get("status").map(|s| s.as_str()).unwrap_or(""));
        let parse_secs = |k: &str| {
            now_playing
                .get(k)
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0)
        };
        Some(PlayerStats {
            state: Some(state.to_string()),
            album: now_playing.get("album").cloned(),
            artist: now_playing.get("artist").cloned(),
            title: now_playing.get("title").cloned(),
            elapsed: Some(_convert_seconds(parse_secs("position"))),
            total: Some(_convert_seconds(parse_secs("duration"))),
        })
    }
}

/// Port of `class MpdPlayerSegment(PlayerSegment)` from
/// `powerline/segments/common/players.py:172`.
///
/// Marker struct. The Python class has two paths — `python-mpd`
/// module + `mpc` CLI fallback. The CLI parser is ported here since
/// the `mpd` Rust binding is a heavy dependency. The `mpd` Python
/// module path is deferred.
#[derive(Debug, Clone, Copy, Default)]
pub struct MpdPlayerSegment;

/// Port of the `mpc` output regex at
/// `powerline/segments/common/players.py:192-195`.
///
/// Pattern: `(.*) - (.*)\n\[([a-z]+)\] +[#0-9\/]+ +([0-9\:]+)\/([0-9\:]+)`.
/// Captures: artist, title, state, elapsed, total.
#[allow(non_snake_case)]
pub fn MPC_OUTPUT_RE() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?s)(.*) - (.*)\n\[([a-z]+)\] +[#0-9/]+ +([0-9:]+)/([0-9:]+)").unwrap()
    })
}

impl MpdPlayerSegment {
    /// Port of `MpdPlayerSegment.get_player_status()` from
    /// `powerline/segments/common/players.py:173-203` (CLI branch).
    ///
    /// `now_playing` is the raw output of `mpc -h <host> -p <port>`.
    /// `album` is the separate `mpc current -f %album%` output.
    /// Returns None when the output doesn't have exactly 3 newlines
    /// per py:190.
    pub fn get_player_status(&self, now_playing: &str, album: Option<&str>) -> Option<PlayerStats> {
        // py:172  class MpdPlayerSegment(PlayerSegment):
        // py:173  def get_player_status(self, pl, host='localhost', password=None, port=6600):
        // py:174  try:
        // py:175  import mpd
        // py:176  except ImportError:
        // py:177  if password:
        // py:178  host = password + '@' + host
        // py:179  now_playing = run_cmd(pl, [
        // py:180  'mpc',
        // py:181  '-h', host,
        // py:182  '-p', str(port)
        // py:183  ], strip=False)
        // py:184  album = run_cmd(pl, [
        // py:185  'mpc', 'current',
        // py:186  '-f', '%album%',
        // py:187  '-h', host,
        // py:188  '-p', str(port)
        // py:189  ])
        // py:190  if not now_playing or now_playing.count("\n") != 3:
        // py:191  return
        if now_playing.is_empty() || now_playing.matches('\n').count() != 3 {
            return None;
        }
        // py:192  now_playing = re.match(
        // py:193  r"(.*) - (.*)\n\[([a-z]+)\] +[#0-9\/]+ +([0-9\:]+)\/([0-9\:]+)",
        // py:194  now_playing
        // py:195  )
        let caps = MPC_OUTPUT_RE().captures(now_playing)?;
        // py:196  return {
        // py:197  'state': _convert_state(now_playing[3]),
        // py:198  'album': album,
        // py:199  'artist': now_playing[1],
        // py:200  'title': now_playing[2],
        // py:201  'elapsed': now_playing[4],
        // py:202  'total': now_playing[5]
        // py:203  }
        // py:204  else:
        // py:205  try:
        // py:206  client = mpd.MPDClient(use_unicode=True)
        // py:207  except TypeError:
        // py:208  # python-mpd 1.x does not support use_unicode
        // py:209  client = mpd.MPDClient()
        // py:210  client.connect(host, port)
        // py:211  if password:
        // py:212  client.password(password)
        // py:213  now_playing = client.currentsong()
        // py:214  if not now_playing:
        // py:215  return
        // py:216  status = client.status()
        // py:217  client.close()
        // py:218  client.disconnect()
        // py:219  return {
        // py:220  'state': status.get('state'),
        // py:221  'album': now_playing.get('album'),
        // py:222  'artist': now_playing.get('artist'),
        // py:223  'title': now_playing.get('title'),
        // py:224  'elapsed': _convert_seconds(status.get('elapsed', 0)),
        // py:225  'total': _convert_seconds(now_playing.get('time', 0)),
        // py:226  }
        let state = _convert_state(caps.get(3)?.as_str());
        Some(PlayerStats {
            state: Some(state.to_string()),
            album: album.map(str::to_string),
            artist: Some(caps.get(1)?.as_str().to_string()),
            title: Some(caps.get(2)?.as_str().to_string()),
            elapsed: Some(caps.get(4)?.as_str().to_string()),
            total: Some(caps.get(5)?.as_str().to_string()),
        })
    }
}

/// Port of `_get_dbus_player_status()` from
/// `powerline/segments/common/players.py:258-312`.
///
/// Python builds the result by querying the dbus interface for
/// `Metadata` + `PlaybackStatus` + `Position`. Rust port takes the
/// already-extracted values (string status, microsecond elapsed,
/// optional microsecond length, optional album/title/artist) since
/// dbus IPC is platform-glue.
#[allow(clippy::too_many_arguments)]
pub fn _get_dbus_player_status(
    status: &str,
    album: Option<&str>,
    title: Option<&str>,
    artist: Option<&str>,
    elapsed_micros: Option<i64>,
    length_micros: Option<i64>,
) -> Option<PlayerStats> {
    // py:251  try:
    // py:252  import dbus
    // py:253  except ImportError:
    // py:254  def _get_dbus_player_status(pl, player_name, **kwargs):
    // py:255  pl.error('Could not add {0} segment: requires dbus module', player_name)
    // py:256  return
    // py:257  else:
    // py:258  def _get_dbus_player_status(pl,
    // py:259  bus_name=None,
    // py:260  iface_prop='org.freedesktop.DBus.Properties',
    // py:261  iface_player='org.mpris.MediaPlayer2.Player',
    // py:262  player_path='/org/mpris/MediaPlayer2',
    // py:263  player_name='player'):
    // py:264  bus = dbus.SessionBus()
    // py:266  if bus_name is None:
    // py:267  for service in bus.list_names():
    // py:268  if re.match('org.mpris.MediaPlayer2.', service):
    // py:269  bus_name = service
    // py:270  break
    // py:272  try:
    // py:273  player = bus.get_object(bus_name, player_path)
    // py:274  iface = dbus.Interface(player, iface_prop)
    // py:275  info = iface.Get(iface_player, 'Metadata')
    // py:276  status = iface.Get(iface_player, 'PlaybackStatus')
    // py:277  except dbus.exceptions.DBusException:
    // py:278  return
    // py:279  if not info:
    // py:280  return
    // py:282  try:
    // py:283  elapsed = iface.Get(iface_player, 'Position')
    // py:284  except dbus.exceptions.DBusException:
    // py:285  pl.warning('Missing player elapsed time')
    // py:286  elapsed = None
    // py:287  else:
    // py:288  elapsed = _convert_seconds(elapsed / 1e6)
    // py:289  album = info.get('xesam:album')
    // py:290  title = info.get('xesam:title')
    // py:291  artist = info.get('xesam:artist')
    // py:292  state = _convert_state(status)
    // py:293  if album:
    // py:294  album = out_u(album)
    // py:295  if title:
    // py:296  title = out_u(title)
    // py:297  if artist:
    // py:298  artist = out_u(artist[0])
    // py:300  length = info.get('mpris:length')
    // py:301  # avoid parsing `None` length values, that would
    // py:302  # raise an error otherwise
    // py:303  parsed_length = length and _convert_seconds(length / 1e6)
    // py:305  return {
    // py:306  'state': state,
    // py:307  'album': album,
    // py:308  'artist': artist,
    // py:309  'title': title,
    // py:310  'elapsed': elapsed,
    // py:311  'total': parsed_length,
    // py:312  }
    let elapsed = elapsed_micros.map(|m| _convert_seconds((m as f64) / 1_000_000.0));
    let state = _convert_state(status);
    let total = length_micros.map(|m| _convert_seconds((m as f64) / 1_000_000.0));
    Some(PlayerStats {
        state: Some(state.to_string()),
        album: album.map(str::to_string),
        title: title.map(str::to_string),
        artist: artist.map(str::to_string),
        elapsed,
        total,
    })
}

/// Port of `class DbusPlayerSegment(PlayerSegment)` from
/// `powerline/segments/common/players.py:315`.
///
/// Marker struct. `get_player_status = staticmethod(_get_dbus_player_status)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DbusPlayerSegment;

/// Port of `class SpotifyDbusPlayerSegment(PlayerSegment)` from
/// `powerline/segments/common/players.py:339`.
///
/// Marker struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpotifyDbusPlayerSegment;

/// Port of `class SpotifyAppleScriptPlayerSegment(PlayerSegment)` from
/// `powerline/segments/common/players.py:371`.
///
/// Marker struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpotifyAppleScriptPlayerSegment;

/// Port of the AppleScript field delimiter used at
/// `powerline/segments/common/players.py:373` / `:487` / `:535`.
///
/// Python: `status_delimiter = '-~`/='`.
pub const APPLESCRIPT_STATUS_DELIMITER: &str = "-~`/=";

impl SpotifyAppleScriptPlayerSegment {
    /// Port of `SpotifyAppleScriptPlayerSegment.get_player_status()` from
    /// `powerline/segments/common/players.py:372-413`.
    ///
    /// Parses the AppleScript stdout: 6 delimiter-separated fields —
    /// state, album, artist, title, total_ms, elapsed_seconds.
    /// Returns None for "stop" state per py:404-405.
    pub fn get_player_status(&self, spotify: &str) -> Option<PlayerStats> {
        // py:371  class SpotifyAppleScriptPlayerSegment(PlayerSegment):
        // py:372  def get_player_status(self, pl):
        // py:373  status_delimiter = '-~`/='
        // py:374  ascript = '''
        // py:375  tell application "System Events"
        // py:376  set process_list to (name of every process)
        // py:377  end tell
        // py:379  if process_list contains "Spotify" then
        // py:380  tell application "Spotify"
        // py:381  if player state is playing or player state is paused then
        // py:382  set track_name to name of current track
        // py:383  set artist_name to artist of current track
        // py:384  set album_name to album of current track
        // py:385  set track_length to duration of current track
        // py:386  set now_playing to "" & player state & "{0}" & album_name & "{0}" & artist_name & "{0}" & track_name & "{0}" & track_length & "{0}" & player position
        // py:387  return now_playing
        // py:388  else
        // py:389  return player state
        // py:390  end if
        // py:392  end tell
        // py:393  else
        // py:394  return "stopped"
        // py:395  end if
        // py:396  '''.format(status_delimiter)
        // py:398  spotify = asrun(pl, ascript)
        // py:399  if not asrun:
        // py:400  return None
        if spotify.is_empty() {
            return None;
        }
        // py:402  spotify_status = spotify.split(status_delimiter)
        let parts: Vec<&str> = spotify.split(APPLESCRIPT_STATUS_DELIMITER).collect();
        if parts.len() < 6 {
            return None;
        }
        // py:403  state = _convert_state(spotify_status[0])
        // py:404  if state == 'stop':
        // py:405  return None
        let state = _convert_state(parts[0]);
        if state == "stop" {
            return None;
        }
        // py:406  return {
        // py:407  'state': state,
        // py:408  'album': spotify_status[1],
        // py:409  'artist': spotify_status[2],
        // py:410  'title': spotify_status[3],
        // py:411  'total': _convert_seconds(int(spotify_status[4])/1000),
        // py:412  'elapsed': _convert_seconds(spotify_status[5]),
        // py:413  }
        let total_ms: f64 = parts[4].trim().parse().ok()?;
        let total = _convert_seconds(total_ms / 1000.0);
        let elapsed_secs: f64 = parts[5].trim().parse().ok()?;
        let elapsed = _convert_seconds(elapsed_secs);
        Some(PlayerStats {
            state: Some(state.to_string()),
            album: Some(parts[1].to_string()),
            artist: Some(parts[2].to_string()),
            title: Some(parts[3].to_string()),
            elapsed: Some(elapsed),
            total: Some(total),
        })
    }
}

/// Port of `class ClementinePlayerSegment(PlayerSegment)` from
/// `powerline/segments/common/players.py:436`.
///
/// Marker struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClementinePlayerSegment;

/// Port of `class RhythmboxPlayerSegment(PlayerSegment)` from
/// `powerline/segments/common/players.py:457`.
///
/// Marker struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct RhythmboxPlayerSegment;

impl RhythmboxPlayerSegment {
    /// Port of `RhythmboxPlayerSegment.get_player_status()` from
    /// `powerline/segments/common/players.py:458-473`.
    ///
    /// Parses the rhythmbox-client output:
    /// `%at\n%aa\n%tt\n%te\n%td` → album, artist, title, elapsed,
    /// total.
    pub fn get_player_status(&self, now_playing: &str) -> Option<PlayerStats> {
        // py:464-465  if not now_playing: return
        if now_playing.is_empty() {
            return None;
        }
        // py:466  now_playing.split('\n')
        let parts: Vec<&str> = now_playing.split('\n').collect();
        if parts.len() < 5 {
            return None;
        }
        // py:467-473  return dict
        Some(PlayerStats {
            state: None,
            album: Some(parts[0].to_string()),
            artist: Some(parts[1].to_string()),
            title: Some(parts[2].to_string()),
            elapsed: Some(parts[3].to_string()),
            total: Some(parts[4].to_string()),
        })
    }
}

/// Port of `class RDIOPlayerSegment(PlayerSegment)` from
/// `powerline/segments/common/players.py:485`.
///
/// Marker struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct RDIOPlayerSegment;

impl RDIOPlayerSegment {
    /// Port of `RDIOPlayerSegment.get_player_status()` from
    /// `powerline/segments/common/players.py:486-521`.
    ///
    /// Parses the AppleScript output: title, artist, album,
    /// elapsed_pct_str, total_secs_str, state_str.
    /// elapsed is computed as `elapsed_pct * total / 100`.
    pub fn get_player_status(&self, now_playing: &str) -> Option<PlayerStats> {
        // py:506-507  if not now_playing: return
        if now_playing.is_empty() {
            return None;
        }
        // py:508-510  split, len != 6 → None
        let parts: Vec<&str> = now_playing.split(APPLESCRIPT_STATUS_DELIMITER).collect();
        if parts.len() != 6 {
            return None;
        }
        // py:511  state = _convert_state(now_playing[5])
        let state = _convert_state(parts[5]);
        // py:512  total = _convert_seconds(now_playing[4])
        let total_secs: f64 = parts[4].trim().parse().ok()?;
        let total = _convert_seconds(total_secs);
        // py:513  elapsed = _convert_seconds(float(now_playing[3]) * float(now_playing[4]) / 100)
        let elapsed_pct: f64 = parts[3].trim().parse().ok()?;
        let elapsed = _convert_seconds(elapsed_pct * total_secs / 100.0);
        // py:514-520
        Some(PlayerStats {
            state: Some(state.to_string()),
            title: Some(parts[0].to_string()),
            artist: Some(parts[1].to_string()),
            album: Some(parts[2].to_string()),
            elapsed: Some(elapsed),
            total: Some(total),
        })
    }
}

/// Port of `class ITunesPlayerSegment(PlayerSegment)` from
/// `powerline/segments/common/players.py:533`.
///
/// Marker struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct ITunesPlayerSegment;

impl ITunesPlayerSegment {
    /// Port of `ITunesPlayerSegment.get_player_status()` from
    /// `powerline/segments/common/players.py:534-572`.
    ///
    /// Parses the AppleScript output: title, artist, album,
    /// elapsed_secs, total_secs, state_str.
    pub fn get_player_status(&self, now_playing: &str) -> Option<PlayerStats> {
        // py:556-557  if not now_playing: return
        if now_playing.is_empty() {
            return None;
        }
        // py:558-560  split, len != 6 → None
        let parts: Vec<&str> = now_playing.split(APPLESCRIPT_STATUS_DELIMITER).collect();
        if parts.len() != 6 {
            return None;
        }
        // py:561-572
        let state = _convert_state(parts[5]);
        let total_secs: f64 = parts[4].trim().parse().ok()?;
        let elapsed_secs: f64 = parts[3].trim().parse().ok()?;
        Some(PlayerStats {
            state: Some(state.to_string()),
            title: Some(parts[0].to_string()),
            artist: Some(parts[1].to_string()),
            album: Some(parts[2].to_string()),
            elapsed: Some(_convert_seconds(elapsed_secs)),
            total: Some(_convert_seconds(total_secs)),
        })
    }
}

/// Port of `class MocPlayerSegment(PlayerSegment)` from
/// `powerline/segments/common/players.py:584`.
///
/// Marker struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct MocPlayerSegment;

impl MocPlayerSegment {
    /// Port of `MocPlayerSegment.get_player_status()` from
    /// `powerline/segments/common/players.py:585-627`.
    ///
    /// Parses `mocp -i` output where each line is `Key: Value`.
    pub fn get_player_status(&self, now_playing_str: &str) -> Option<PlayerStats> {
        // py:612-613  if not now_playing_str: return
        if now_playing_str.is_empty() {
            return None;
        }
        // py:615-618  dict from each `key: value` line
        let mut now_playing: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for line in now_playing_str.split('\n') {
            if line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once(": ") {
                now_playing.insert(k.to_string(), v.to_string());
            }
        }
        // py:619  state = _convert_state(now_playing.get('State', 'stop'))
        let state = _convert_state(
            now_playing
                .get("State")
                .map(|s| s.as_str())
                .unwrap_or("stop"),
        );
        // py:620-627
        let parse_secs = |k: &str| {
            now_playing
                .get(k)
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0)
        };
        Some(PlayerStats {
            state: Some(state.to_string()),
            album: Some(now_playing.get("Album").cloned().unwrap_or_default()),
            artist: Some(now_playing.get("Artist").cloned().unwrap_or_default()),
            title: Some(now_playing.get("SongTitle").cloned().unwrap_or_default()),
            elapsed: Some(_convert_seconds(parse_secs("CurrentSec"))),
            total: Some(_convert_seconds(parse_secs("TotalSec"))),
        })
    }
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

    #[test]
    fn cmus_parses_basic_status() {
        // py:145-160  cmus-remote -Q output
        let raw = concat!(
            "status playing\n",
            "file /home/user/song.mp3\n",
            "tag artist The Artist\n",
            "tag title The Title\n",
            "tag album The Album\n",
            "set continue true\n",
            "duration 245\n",
            "position 30\n",
        );
        let s = CmusPlayerSegment.get_player_status(raw).unwrap();
        assert_eq!(s.state.as_deref(), Some("play"));
        assert_eq!(s.artist.as_deref(), Some("The Artist"));
        assert_eq!(s.title.as_deref(), Some("The Title"));
        assert_eq!(s.album.as_deref(), Some("The Album"));
        assert_eq!(s.elapsed.as_deref(), Some("0:30"));
        assert_eq!(s.total.as_deref(), Some("4:05"));
    }

    #[test]
    fn cmus_returns_none_for_empty_input() {
        // py:146-147  if not now_playing_str: return
        assert!(CmusPlayerSegment.get_player_status("").is_none());
    }

    #[test]
    fn cmus_handles_paused_state() {
        let raw = "status paused\ntag artist X\nposition 10\nduration 100\n";
        let s = CmusPlayerSegment.get_player_status(raw).unwrap();
        assert_eq!(s.state.as_deref(), Some("pause"));
    }

    #[test]
    fn cmus_handles_set_level_keys() {
        // py:148-151  'set' level keys flatten same as 'tag' level
        let raw = "status stopped\nset shuffle true\n";
        let s = CmusPlayerSegment.get_player_status(raw).unwrap();
        // 'shuffle' is at set level → should be ignored in our key search
        // but state must still be derived
        assert_eq!(s.state.as_deref(), Some("stop"));
    }

    #[test]
    fn mpd_parses_mpc_output() {
        // py:192-203  mpc output regex
        // Real mpc emits exactly 3 newlines: "Artist - Title\n[state] #N/M  E:LL/T:OT\nflags: x\n"
        let raw = "The Artist - The Title\n[playing] #1/10   0:30/4:05\nrandom: off   repeat: on\n";
        let s = MpdPlayerSegment
            .get_player_status(raw, Some("The Album"))
            .unwrap();
        assert_eq!(s.state.as_deref(), Some("play"));
        assert_eq!(s.artist.as_deref(), Some("The Artist"));
        assert_eq!(s.title.as_deref(), Some("The Title"));
        assert_eq!(s.elapsed.as_deref(), Some("0:30"));
        assert_eq!(s.total.as_deref(), Some("4:05"));
        assert_eq!(s.album.as_deref(), Some("The Album"));
    }

    #[test]
    fn mpd_returns_none_when_wrong_newline_count() {
        // py:190  newline count != 3 → return
        let raw = "Artist - Title\nstuff";
        assert!(MpdPlayerSegment.get_player_status(raw, None).is_none());
    }

    #[test]
    fn mpd_returns_none_when_empty() {
        assert!(MpdPlayerSegment.get_player_status("", None).is_none());
    }

    #[test]
    fn dbus_player_status_converts_micros_to_mss() {
        // py:288  elapsed = _convert_seconds(elapsed / 1e6)
        // py:303  parsed_length = length and _convert_seconds(length / 1e6)
        let s = _get_dbus_player_status(
            "Playing",
            Some("Album"),
            Some("Title"),
            Some("Artist"),
            Some(60_000_000),  // 60s
            Some(245_000_000), // 245s = 4:05
        )
        .unwrap();
        assert_eq!(s.state.as_deref(), Some("play"));
        assert_eq!(s.elapsed.as_deref(), Some("1:00"));
        assert_eq!(s.total.as_deref(), Some("4:05"));
        assert_eq!(s.album.as_deref(), Some("Album"));
        assert_eq!(s.title.as_deref(), Some("Title"));
        assert_eq!(s.artist.as_deref(), Some("Artist"));
    }

    #[test]
    fn dbus_player_status_none_elapsed_and_length() {
        // py:285-287  elapsed = None when dbus get fails
        let s = _get_dbus_player_status("Paused", None, None, None, None, None).unwrap();
        assert_eq!(s.state.as_deref(), Some("pause"));
        assert!(s.elapsed.is_none());
        assert!(s.total.is_none());
    }

    #[test]
    fn applescript_delimiter_matches_python() {
        // py:373  status_delimiter = '-~`/='
        assert_eq!(APPLESCRIPT_STATUS_DELIMITER, "-~`/=");
    }

    #[test]
    fn spotify_applescript_parses_playing() {
        // py:402-413
        let raw = "playing-~`/=The Album-~`/=The Artist-~`/=The Track-~`/=180000-~`/=45.5";
        let s = SpotifyAppleScriptPlayerSegment
            .get_player_status(raw)
            .unwrap();
        assert_eq!(s.state.as_deref(), Some("play"));
        assert_eq!(s.album.as_deref(), Some("The Album"));
        assert_eq!(s.artist.as_deref(), Some("The Artist"));
        assert_eq!(s.title.as_deref(), Some("The Track"));
        // 180000ms / 1000 = 180s = 3:00
        assert_eq!(s.total.as_deref(), Some("3:00"));
        // 45.5s = 0:45
        assert_eq!(s.elapsed.as_deref(), Some("0:46"));
    }

    #[test]
    fn spotify_applescript_returns_none_for_stop() {
        // py:404-405  if state == 'stop': return None
        let raw = "stopped-~`/=-~`/=-~`/=-~`/=0-~`/=0";
        assert!(SpotifyAppleScriptPlayerSegment
            .get_player_status(raw)
            .is_none());
    }

    #[test]
    fn spotify_applescript_returns_none_for_empty() {
        assert!(SpotifyAppleScriptPlayerSegment
            .get_player_status("")
            .is_none());
    }

    #[test]
    fn rhythmbox_parses_5_field_output() {
        // py:466-473
        let raw = "Album X\nArtist Y\nTitle Z\n0:30\n4:05";
        let s = RhythmboxPlayerSegment.get_player_status(raw).unwrap();
        assert_eq!(s.album.as_deref(), Some("Album X"));
        assert_eq!(s.artist.as_deref(), Some("Artist Y"));
        assert_eq!(s.title.as_deref(), Some("Title Z"));
        assert_eq!(s.elapsed.as_deref(), Some("0:30"));
        assert_eq!(s.total.as_deref(), Some("4:05"));
        // py:467-473  no state field
        assert!(s.state.is_none());
    }

    #[test]
    fn rhythmbox_returns_none_for_empty() {
        assert!(RhythmboxPlayerSegment.get_player_status("").is_none());
    }

    #[test]
    fn rdio_parses_6_field_output_with_elapsed_pct() {
        // py:511-513  elapsed = pct * total / 100
        // 50% of 200s = 100s = 1:40
        let raw = "Title-~`/=Artist-~`/=Album-~`/=50-~`/=200-~`/=playing";
        let s = RDIOPlayerSegment.get_player_status(raw).unwrap();
        assert_eq!(s.state.as_deref(), Some("play"));
        assert_eq!(s.title.as_deref(), Some("Title"));
        assert_eq!(s.artist.as_deref(), Some("Artist"));
        assert_eq!(s.album.as_deref(), Some("Album"));
        assert_eq!(s.total.as_deref(), Some("3:20"));
        assert_eq!(s.elapsed.as_deref(), Some("1:40"));
    }

    #[test]
    fn rdio_returns_none_for_wrong_field_count() {
        // py:509-510  if len != 6: return
        assert!(RDIOPlayerSegment
            .get_player_status("only-~`/=three-~`/=fields")
            .is_none());
    }

    #[test]
    fn itunes_parses_6_field_output() {
        // py:561-572
        let raw = "Title-~`/=Artist-~`/=Album-~`/=30-~`/=245-~`/=playing";
        let s = ITunesPlayerSegment.get_player_status(raw).unwrap();
        assert_eq!(s.state.as_deref(), Some("play"));
        assert_eq!(s.title.as_deref(), Some("Title"));
        assert_eq!(s.artist.as_deref(), Some("Artist"));
        assert_eq!(s.album.as_deref(), Some("Album"));
        assert_eq!(s.elapsed.as_deref(), Some("0:30"));
        assert_eq!(s.total.as_deref(), Some("4:05"));
    }

    #[test]
    fn itunes_returns_none_for_wrong_field_count() {
        assert!(ITunesPlayerSegment.get_player_status("x").is_none());
    }

    #[test]
    fn mocp_parses_key_value_output() {
        // py:611-627
        let raw = concat!(
            "State: PLAY\n",
            "File: song.mp3\n",
            "Title: full title\n",
            "Artist: The Artist\n",
            "SongTitle: The Track\n",
            "Album: The Album\n",
            "TotalSec: 245\n",
            "CurrentSec: 30\n",
        );
        let s = MocPlayerSegment.get_player_status(raw).unwrap();
        assert_eq!(s.state.as_deref(), Some("play"));
        assert_eq!(s.artist.as_deref(), Some("The Artist"));
        assert_eq!(s.title.as_deref(), Some("The Track"));
        assert_eq!(s.album.as_deref(), Some("The Album"));
        assert_eq!(s.elapsed.as_deref(), Some("0:30"));
        assert_eq!(s.total.as_deref(), Some("4:05"));
    }

    #[test]
    fn mocp_defaults_state_to_stop_when_missing() {
        // py:619  now_playing.get('State', 'stop')
        let raw = "File: x.mp3\nTitle: x\n";
        let s = MocPlayerSegment.get_player_status(raw).unwrap();
        assert_eq!(s.state.as_deref(), Some("stop"));
    }

    #[test]
    fn mocp_returns_none_for_empty() {
        assert!(MocPlayerSegment.get_player_status("").is_none());
    }

    #[test]
    fn mpc_output_re_captures_5_groups() {
        // py:192-195
        let re = MPC_OUTPUT_RE();
        let s = "Artist - Title\n[playing] #1/10  0:30/4:05";
        let caps = re.captures(s).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "Artist");
        assert_eq!(caps.get(2).unwrap().as_str(), "Title");
        assert_eq!(caps.get(3).unwrap().as_str(), "playing");
        assert_eq!(caps.get(4).unwrap().as_str(), "0:30");
        assert_eq!(caps.get(5).unwrap().as_str(), "4:05");
    }

    #[test]
    fn argspecobjs_yields_get_player_status_pair() {
        // py:62-65
        let r = argspecobjs();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, "get_player_status");
    }

    #[test]
    fn omitted_args_returns_empty_tuple() {
        // py:67-68
        assert!(omitted_args("render", "render").is_empty());
        assert!(omitted_args("anything", "method").is_empty());
    }

    #[test]
    fn call_alias_dispatches_to_player_segment_call() {
        // py:40 alias
        let symbols = state_symbols();
        let r = call(None, "{state_symbol}", &symbols);
        assert!(r.is_none());
    }
}
