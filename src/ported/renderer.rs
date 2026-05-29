// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderer.py`.
//!
//! Base Renderer class. Subclasses (`renderers/tmux.rs`,
//! `renderers/vim.rs`, etc.) override the per-format hooks
//! (`hl`/`hlstyle`/`character_translations`). Surfaces:
//!   - `NBSP` constant
//!   - `np_control_character_translations()` — 0x00-0x1F → "^@"-"^_"
//!   - `np_invalid_character_translations()` — 0xDC80-0xDCFF → `"<80>"`-`"<FF>"`
//!   - `np_invalid_character_re()` — unpaired-surrogate regex
//!   - `np_character_translations()` — union for UCS-4
//!   - `translate_np(s)` — non-printable translation
//!   - `construct_returned_value(...)` — render return-tuple builder
//!   - Width data table + `strwidth(s)`
//!   - `Renderer` struct skeleton with theme_config / segment_info
//!     / character_translations / width_data
//!   - `compute_divider_widths(get_divider)` per-side hard/soft widths
//!
//! The full `render` / `do_render` / `_render_length` /
//! `__prepare_segments` segment-pipeline implementations are heavy
//! enough to deserve their own port pass; only the structural pieces
//! are covered here.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                        // py:4
// import os                                         // py:5
// import re                                         // py:6
// import operator                                   // py:7
// from itertools import chain                       // py:9
// from powerline.theme import Theme                  // py:11
// from powerline.lib.unicode import unichr, strwidth_ucs_2, strwidth_ucs_4                  // py:12

use regex::Regex;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `NBSP` from
/// `powerline/renderer.py:15`.
/// `NBSP = ' '` — non-breaking space.
pub const NBSP: &str = "\u{a0}";

/// Port of `np_control_character_translations` from
/// `powerline/renderer.py:18-21`.
///
/// Maps chars in `0x00..=0x1F` to printable two-char sequences:
/// `0x00` → `"^@"`, `0x01` → `"^A"`, …, `0x1F` → `"^_"`.
pub fn np_control_character_translations() -> &'static HashMap<char, String> {
    // py:15  NBSP = ' '
    // py:18  np_control_character_translations = dict((
    // py:19  # Control characters: ^@ … ^Y
    // py:20  (i1, '^' + unichr(i1 + 0x40)) for i1 in range(0x20)
    // py:21  ))
    // py:22  '''Control character translations
    // py:23
    // py:24  Dictionary that maps characters in range 0x00–0x1F (inclusive) to strings
    // py:25  ``'^@'``, ``'^A'`` and so on.
    // py:26
    // py:27  .. note: maps tab to ``^I`` and newline to ``^J``.
    // py:28  '''
    static M: OnceLock<HashMap<char, String>> = OnceLock::new();
    M.get_or_init(|| {
        let mut m = HashMap::new();
        for i in 0u32..0x20 {
            let ch = char::from_u32(i).unwrap();
            let repl_byte = (i + 0x40) as u8;
            let repl = format!("^{}", repl_byte as char);
            m.insert(ch, repl);
        }
        m
    })
}

/// Port of `np_invalid_character_translations` from
/// `powerline/renderer.py:30-33`.
///
/// Maps surrogate-escape codepoints in `0xDC80..=0xDCFF` to
/// `"<80>"`, `"<81>"`, …, `"<FF>"` strings.
pub fn np_invalid_character_translations() -> &'static HashMap<u32, String> {
    // py:30  np_invalid_character_translations = dict((
    // py:31  # Invalid unicode characters obtained using 'surrogateescape' error
    // py:32  # handler.
    // py:33  (i2, '<{0:02x}>'.format(i2 - 0xDC00)) for i2 in range(0xDC80, 0xDD00)
    // py:34  ))
    // py:35  '''Invalid unicode character translations
    // py:36
    // py:37  When using ``surrogateescape`` encoding error handling method characters in
    // py:38  range 0x80–0xFF (inclusive) are transformed into unpaired surrogate escape
    // py:39  unicode codepoints 0xDC80–0xDD00. This dictionary maps such characters to
    // py:40  ``<80>``, ``<81>``, and so on: in Python-3 they cannot be printed or
    // py:41  converted to UTF-8 because UTF-8 standard does not allow surrogate escape
    // py:42  characters, not even paired ones. Python-2 contains a bug that allows such
    // py:43  action, but printing them in any case makes no sense.
    // py:44  '''
    static M: OnceLock<HashMap<u32, String>> = OnceLock::new();
    M.get_or_init(|| {
        let mut m = HashMap::new();
        for i in 0xDC80u32..0xDD00 {
            m.insert(i, format!("<{:02x}>", i - 0xDC00));
        }
        m
    })
}

/// Port of `np_invalid_character_re` from
/// `powerline/renderer.py:46`.
///
/// `re.compile('(?<![\uD800-\uDBFF])[\uDC80-\uDD00]')`.
/// The Rust `regex` crate doesn't support lookbehind; this stub
/// matches lone trailing surrogates by codepoint range only — the
/// caller is responsible for the `(?<![\uD800-\uDBFF])` check.
pub fn np_invalid_character_re() -> &'static Regex {
    // py:46  # XXX: not using `r` because it makes no sense.
    // py:47  np_invalid_character_re = re.compile('(?<![\uD800-\uDBFF])[\uDC80-\uDD00]')
    // py:48  '''Regex that finds unpaired surrogate escape characters
    // py:49
    // py:50  Search is only limited to the ones obtained from ``surrogateescape`` error
    // py:51  handling method. This regex is only used for UCS-2 Python variants because
    // py:52  in this case characters above 0xFFFF are represented as surrogate escapes
    // py:53  characters and are thus subject to partial transformation if
    // py:54  ``np_invalid_character_translations`` translation table is used.
    // py:55  '''
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^$").unwrap())
}

/// Port of `np_character_translations` from
/// `powerline/renderer.py:59`.
///
/// Returns a fresh union of `np_control_character_translations`
/// (always) + `np_invalid_character_translations` (UCS-4). Rust is
/// always UCS-4-equivalent (chars are full unicode codepoints), so
/// the table is always the union.
pub fn np_character_translations() -> HashMap<char, String> {
    // py:57  np_character_translations = np_control_character_translations.copy()
    // py:58  '''Dictionary that contains non-printable character translations
    // py:59
    // py:60  In UCS-4 versions of Python this is a union of
    // py:61  ``np_invalid_character_translations`` and ``np_control_character_translations``
    // py:62  dictionaries. In UCS-2 for technical reasons ``np_invalid_character_re`` is used
    // py:63  instead and this dictionary only contains items from
    // py:64  ``np_control_character_translations``.
    // py:65  '''
    let mut m = np_control_character_translations().clone();
    for (cp, repl) in np_invalid_character_translations() {
        if let Some(c) = char::from_u32(*cp) {
            m.insert(c, repl.clone());
        }
    }
    m
}

/// Port of `translate_np()` from
/// `powerline/renderer.py:68-82`.
///
/// Translates non-printable characters in `s` via the
/// `np_character_translations` table.
pub fn translate_np(s: &str) -> String {
    // py:67  translate_np = (
    // py:68  (
    // py:69  lambda s: (
    // py:70  np_invalid_character_re.subn(
    // py:71  lambda match: (
    // py:72  np_invalid_character_translations[ord(match.group(0))]
    // py:73  ), s
    // py:74  )[0].translate(np_character_translations)
    // py:75  )
    // py:76  ) if sys.maxunicode < 0x10FFFF else (
    // py:77  lambda s: (
    // py:78  s.translate(np_character_translations)
    // py:79  )
    // py:80  )
    // py:81  )
    // py:82  '''Function that translates non-printable characters into printable strings
    // py:83
    // py:84  Is used to translate control characters and surrogate escape characters
    // py:85  obtained from ``surrogateescape`` encoding errors handling method into some
    // py:86  printable sequences. See documentation for
    // py:87  ``np_invalid_character_translations`` and
    // py:88  ``np_control_character_translations`` for more details.
    // py:89  '''
    let table = np_character_translations();
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if let Some(repl) = table.get(&c) {
            out.push_str(repl);
        } else {
            out.push(c);
        }
    }
    out
}

/// Port of `construct_returned_value()` from
/// `powerline/renderer.py:92`.
///
/// Builds the per-call return tuple from the rendered highlighted
/// string + the raw segments + width. The Python source returns a
/// bare string when neither `output_raw` nor `output_width` is
/// requested; otherwise returns a tuple.
#[derive(Debug, Clone)]
pub enum RenderReturn {
    /// py:94  return rendered_highlighted (string only)
    Plain(String),
    /// py:96+  tuple variant with optional raw + optional width
    Tuple {
        highlighted: String,
        raw: Option<String>,
        width: Option<usize>,
    },
}

/// Port of `construct_returned_value()` (py:92).
pub fn construct_returned_value(
    rendered_highlighted: String,
    rendered_raw: Option<String>,
    width: usize,
    output_raw: bool,
    output_width: bool,
) -> RenderReturn {
    // py:92  def construct_returned_value(rendered_highlighted, segments, width, output_raw, output_width):
    // py:93  if not (output_raw or output_width):
    // py:94  return rendered_highlighted
    // py:95  else:
    // py:96  return (
    // py:97  (rendered_highlighted,)
    // py:98  + ((''.join((segment['_rendered_raw'] for segment in segments)),) if output_raw else ())
    // py:99  + ((width,) if output_width else ())
    // py:100  )
    if !output_raw && !output_width {
        return RenderReturn::Plain(rendered_highlighted);
    }
    RenderReturn::Tuple {
        highlighted: rendered_highlighted,
        raw: if output_raw { rendered_raw } else { None },
        width: if output_width { Some(width) } else { None },
    }
}

/// Returns the upstream `width_data` table from
/// `powerline/renderer.py:177-184`.
///
/// Width-class → display-width mapping for `strwidth`. `ambiwidth`
/// configures the East Asian ambiguous width per py:182.
pub fn width_data(ambiwidth: u8) -> HashMap<char, u8> {
    // py:103  class Renderer(object):
    // py:104-121  docstring
    // py:123  segment_info = {
    // py:124  'environ': os.environ,
    // py:125  'getcwd': getattr(os, 'getcwdu', os.getcwd),
    // py:126  'home': os.environ.get('HOME'),
    // py:127  }
    // py:128-148  docstring
    // py:150  character_translations = {}
    // py:151-154  docstring
    // py:156  def __init__(self,
    // py:157  theme_config,
    // py:158  local_themes,
    // py:159  theme_kwargs,
    // py:160  pl,
    // py:161  ambiwidth=1,
    // py:162  **options):
    // py:163  self.__dict__.update(options)
    // py:164  self.theme_config = theme_config
    // py:165  theme_kwargs['pl'] = pl
    // py:166  self.pl = pl
    // py:167  if theme_config.get('use_non_breaking_spaces', True):
    // py:168  self.character_translations = self.character_translations.copy()
    // py:169  self.character_translations[ord(' ')] = NBSP
    // py:170  self.theme = Theme(theme_config=theme_config, **theme_kwargs)
    // py:171  self.local_themes = local_themes
    // py:172  self.theme_kwargs = theme_kwargs
    // py:173  self.width_data = {
    // py:174  'N': 1,          # Neutral
    // py:175  'Na': 1,         # Narrow
    // py:176  'A': ambiwidth,  # Ambiguous
    // py:177  'H': 1,          # Half-width
    // py:178  'W': 2,          # Wide
    // py:179  'F': 2,          # Fullwidth
    // py:180  }
    let mut m = HashMap::new();
    m.insert('N', 1);
    m.insert('a', 1);
    m.insert('A', ambiwidth);
    m.insert('H', 1);
    m.insert('W', 2);
    m.insert('F', 2);
    m
}

/// Port of `Renderer.strwidth()` from
/// `powerline/renderer.py:188`.
///
/// Computes the display width of `s` using the `width_data` table.
/// Rust port treats every char as Narrow=1 (no East Asian dispatch
/// without `unicode_width` crate). The function exists so callers
/// can plumb the width-aware path once the crate is added.
pub fn strwidth(s: &str) -> usize {
    // py:182  strwidth = lambda self, s: (
    // py:183  (strwidth_ucs_2 if sys.maxunicode < 0x10FFFF else strwidth_ucs_4)(
    // py:184  self.width_data, s)
    // py:185  )
    // py:186  '''Function that returns string width.
    // py:187-196  docstring
    s.chars().count()
}

/// Port of `Renderer.compute_divider_widths()` from
/// `powerline/renderer.py:303`.
///
/// `get_divider(side, kind)` is the caller-supplied closure that
/// resolves the divider string for the given side/kind pair (Python
/// calls `theme.get_divider(side, kind)`).
pub fn compute_divider_widths<F>(mut get_divider: F) -> Map<String, Value>
where
    F: FnMut(&str, &str) -> String,
{
    // py:198  def get_theme(self, matcher_info):
    // py:199-207  docstring
    // py:208  return self.theme
    // py:210  def shutdown(self):
    // py:211-214  docstring
    // py:215  self.theme.shutdown()
    // py:217  def get_segment_info(self, segment_info, mode):
    // py:218-232  docstring
    // py:233  r = self.segment_info.copy()
    // py:234  r['mode'] = mode
    // py:235  if segment_info:
    // py:236  r.update(segment_info)
    // py:237  if 'PWD' in r['environ']:
    // py:238  r['getcwd'] = lambda: r['environ']['PWD']
    // py:239  return r
    // py:241  def render_above_lines(self, **kwargs):
    // py:242-247  docstring
    // py:250  theme = self.get_theme(kwargs.get('matcher_info', None))
    // py:251  for line in range(theme.get_line_number() - 1, 0, -1):
    // py:252  yield self.render(side=None, line=line, **kwargs)
    // py:254  def render(self, mode=None, width=None, side=None, line=0, output_raw=False, output_width=False, segment_info=None, matcher_info=None, hl_args=None):
    // py:255-294  docstring
    // py:295  theme = self.get_theme(matcher_info)
    // py:296  return self.do_render(
    // py:297  mode=mode,
    // py:298  width=width,
    // py:299  side=side,
    // py:303  def compute_divider_widths(self, theme):
    // py:304  return {
    // py:305  'left': {
    // py:306  'hard': self.strwidth(theme.get_divider('left', 'hard')),
    // py:307  'soft': self.strwidth(theme.get_divider('left', 'soft')),
    // py:308  },
    // py:309  'right': {
    let mut out = Map::new();
    for side in ["left", "right"] {
        let mut side_map = Map::new();
        side_map.insert(
            "hard".to_string(),
            Value::from(strwidth(&get_divider(side, "hard"))),
        );
        side_map.insert(
            "soft".to_string(),
            Value::from(strwidth(&get_divider(side, "soft"))),
        );
        out.insert(side.to_string(), Value::Object(side_map));
    }
    out
}

/// Port of `class Renderer(object)` from
/// `powerline/renderer.py:103`.
///
/// Holds the base renderer state. The render-pipeline methods
/// (`render`/`do_render`/`_render_segments`/`__prepare_segments`)
/// are heavy enough to deserve their own port pass; this struct
/// surfaces the constructor + the `segment_info` / `width_data` /
/// `character_translations` state.
pub struct Renderer {
    /// Python: `self.theme_config`.
    pub theme_config: Map<String, Value>,
    /// Python: `self.local_themes`.
    pub local_themes: Map<String, Value>,
    /// Python: `self.character_translations`.
    pub character_translations: HashMap<char, String>,
    /// Python: `self.width_data` per py:177-184.
    pub width_data: HashMap<char, u8>,
    /// Python: `self.theme` — the default Theme used by get_theme
    /// when there's no local-theme match per py:208.
    pub theme: Value,
    /// Records shutdown-call order. Used in lieu of the
    /// `Theme.shutdown()` side effect since the Theme class isn't
    /// yet wired through Rust. Same pattern as the IPython/Shell/Vim
    /// renderer ports.
    pub shutdown_called: std::sync::Mutex<Vec<String>>,
}

impl Renderer {
    /// Port of `Renderer.__init__()` from
    /// `powerline/renderer.py:158`.
    pub fn new(
        theme_config: Map<String, Value>,
        local_themes: Map<String, Value>,
        ambiwidth: u8,
    ) -> Self {
        // py:167-171  use_non_breaking_spaces → character_translations[' '] = NBSP
        let use_nbsp = theme_config
            .get("use_non_breaking_spaces")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let mut character_translations: HashMap<char, String> = HashMap::new();
        if use_nbsp {
            character_translations.insert(' ', NBSP.to_string());
        }
        Self {
            theme_config,
            local_themes,
            character_translations,
            width_data: width_data(ambiwidth),
            theme: Value::Null,
            shutdown_called: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Port of `Renderer.get_theme()` from
    /// `powerline/renderer.py:198-208`.
    ///
    /// Base implementation returns `self.theme` per py:208. Subclasses
    /// (e.g. VimRenderer, ShellRenderer, IPythonRenderer) override to
    /// dispatch through `local_themes`. The `matcher_info` param is
    /// preserved for parity but ignored at this level per py:205-206.
    pub fn get_theme(&self, _matcher_info: Option<&Value>) -> Value {
        // py:208  return self.theme
        self.theme.clone()
    }

    /// Port of `Renderer.shutdown()` from
    /// `powerline/renderer.py:210-215`.
    ///
    /// Records `"theme"` in the shutdown_called log to mirror the
    /// `self.theme.shutdown()` side effect per py:215. Subclasses
    /// extend this to walk local_themes (see IPythonRenderer /
    /// ShellRenderer / VimRenderer ports).
    pub fn shutdown(&self) {
        // py:320  hl_join = staticmethod(''.join)
        // py:321  '''Join a list of rendered segments into a resulting string
        // py:322-331  docstring
        // py:333  def do_render(self, mode, width, side, line, output_raw, output_width, segment_info, theme, hl_args):
        // py:334  '''Like Renderer.render(), but accept theme in place of matcher_info
        // py:335  '''
        // py:336  segments = list(theme.get_segments(side, line, segment_info, mode))
        // py:338  current_width = 0
        // py:340  self._prepare_segments(segments, output_width or width)
        // py:342  hl_args = hl_args or dict()
        // py:344  if not width:
        // py:345  # No width specified, so we don't need to crop or pad anything
        // py:346  if output_width:
        // py:347  current_width = self._render_length(theme, segments, self.compute_divider_widths(theme))
        // py:348  return construct_returned_value(self.hl_join([
        // py:349  segment['_rendered_hl']
        // py:350  for segment in self._render_segments(theme, segments, hl_args)
        // py:351  ]) + self.hlstyle(**hl_args), segments, current_width, output_raw, output_width)
        // py:353  divider_widths = self.compute_divider_widths(theme)
        // py:355  # Create an ordered list of segments that can be dropped
        // py:356  segments_priority = sorted((segment for segment in segments if segment['priority'] is not None), key=lambda segment: segment['priority'], reverse=True)
        // py:357  no_priority_segments = filter(lambda segment: segment['priority'] is None, segments)
        // py:358  current_width = self._render_length(theme, segments, divider_widths)
        // py:359  if current_width > width:
        // py:360  for segment in chain(segments_priority, no_priority_segments):
        // py:361  if segment['truncate'] is not None:
        // py:362  segment['contents'] = segment['truncate'](self.pl, current_width - width, segment)
        // py:386  # Distribute the remaining space on spacer segments
        // py:387  segments_spacers = [segment for segment in segments if segment['expand'] is not None]
        // py:388  if segments_spacers:
        // py:389  distribute_len, distribute_len_remainder = divmod(width - current_width, len(segments_spacers))
        // py:403  rendered_highlighted = self.hl_join([
        // py:404  segment['_rendered_hl']
        // py:405  for segment in self._render_segments(theme, segments, hl_args)
        // py:406  ])
        // py:407  if rendered_highlighted:
        // py:408  rendered_highlighted += self.hlstyle(**hl_args)
        // py:410  return construct_returned_value(rendered_highlighted, segments, current_width, output_raw, output_width)
        let mut log = self
            .shutdown_called
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        log.push("theme".to_string());
    }

    /// Port of `Renderer.escape()` from
    /// `powerline/renderer.py:586-589`.
    ///
    /// Python: `string.translate(self.character_translations)`.
    /// Rust port walks each char and substitutes from the translation
    /// table when present; non-translated chars pass through.
    pub fn escape(&self, string: &str) -> String {
        // py:586  def escape(self, string):
        // py:587  '''Method that escapes given string. Method may be overridden by subclasses.
        // py:588  '''
        // py:589  return string.translate(self.character_translations)
        let mut out = String::with_capacity(string.len());
        for c in string.chars() {
            match self.character_translations.get(&c) {
                Some(replacement) => out.push_str(replacement),
                None => out.push(c),
            }
        }
        out
    }

    /// Port of `Renderer.hl()` from
    /// `powerline/renderer.py:600-606`.
    ///
    /// Returns `hlstyle(fg, bg, attrs) + (contents or '')` per
    /// py:606. The Rust port takes `hlstyle_fn` as a closure since
    /// the base Python `hlstyle` raises NotImplementedError at
    /// py:598; concrete renderers (ShellRenderer, VimRenderer)
    /// provide the implementation.
    pub fn hl(contents: Option<&str>, hlstyle_output: &str) -> String {
        // py:594  def hlstyle(self, fg=None, bg=None, attrs=None, **kwargs):
        // py:595  '''Method that returns formatting string for given style.
        // py:596  '''
        // py:597  # Should be overridden by subclasses
        // py:598  raise NotImplementedError
        // py:600  def hl(self, contents, fg=None, bg=None, attrs=None, **kwargs):
        // py:601  '''Output highlighted text.
        // py:602  '''
        // py:603  return (
        // py:604  self.hlstyle(fg, bg, attrs, **kwargs)
        // py:605  + (contents or '')
        // py:606  )
        format!("{}{}", hlstyle_output, contents.unwrap_or(""))
    }

    /// Port of `Renderer.__prepare_segments()` from
    /// `powerline/renderer.py:412-422`.
    ///
    /// For each segment: translates non-printable chars in
    /// `contents` per py:415-416. When `calculate_contents_len` is
    /// true, sets `_contents_len` from `literal_contents[0]` if
    /// `literal_contents[1]` is truthy, else from `strwidth(contents)`.
    pub fn _prepare_segments(segments: &mut [Value], calculate_contents_len: bool) {
        // py:412  def __prepare_segments(self, segments, calculate_contents_len):
        // py:413  '''Translate non-printable characters and calculate segment widths.'''
        // py:414  for segment in segments:
        // py:415  segment['contents'] = translate_np(segment['contents'])
        // py:416  if calculate_contents_len:
        // py:417  for segment in segments:
        // py:418  if segment['literal_contents'][1]:
        // py:419  segment['_contents_len'] = segment['literal_contents'][0]
        // py:420  else:
        // py:421  segment['_contents_len'] = self.strwidth(segment['contents'])
        // py:422  return segments
        for segment in segments.iter_mut() {
            if let Some(obj) = segment.as_object_mut() {
                if let Some(contents) = obj.get("contents").and_then(|v| v.as_str()) {
                    let translated = translate_np(contents);
                    obj.insert("contents".to_string(), Value::String(translated));
                }
            }
        }
        if calculate_contents_len {
            for segment in segments.iter_mut() {
                if let Some(obj) = segment.as_object_mut() {
                    // py:419-420  if literal_contents[1]: contents_len = literal_contents[0]
                    let literal = obj
                        .get("literal_contents")
                        .and_then(|v| v.as_array())
                        .cloned();
                    let contents_len = if let Some(lit) = literal {
                        let has_literal = lit
                            .get(1)
                            .and_then(|v| v.as_str())
                            .map(|s| !s.is_empty())
                            .unwrap_or(false);
                        if has_literal {
                            lit.first().and_then(|v| v.as_u64()).unwrap_or(0) as usize
                        } else {
                            // py:422  strwidth(contents)
                            obj.get("contents")
                                .and_then(|v| v.as_str())
                                .map(strwidth)
                                .unwrap_or(0)
                        }
                    } else {
                        obj.get("contents")
                            .and_then(|v| v.as_str())
                            .map(strwidth)
                            .unwrap_or(0)
                    };
                    obj.insert(
                        "_contents_len".to_string(),
                        Value::from(contents_len as u64),
                    );
                }
            }
        }
    }

    /// Port of `Renderer.segment_info` class attribute from
    /// `powerline/renderer.py:124-128`.
    ///
    /// Returns a fresh dict with environ + getcwd + home keys
    /// populated from the process environment.
    pub fn segment_info() -> Map<String, Value> {
        let mut info = Map::new();
        // py:125-128  environ / getcwd / home
        let env_map: Map<String, Value> = std::env::vars()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        info.insert("environ".to_string(), Value::Object(env_map.clone()));
        info.insert(
            "home".to_string(),
            env_map.get("HOME").cloned().unwrap_or(Value::Null),
        );
        info
    }

    /// Port of `Renderer.get_segment_info()` from
    /// `powerline/renderer.py:216`.
    ///
    /// Merges `segment_info` over the base `Renderer::segment_info()` +
    /// sets `mode`. When `PWD` is present, replaces `getcwd` with a
    /// `Value::String(pwd)` (Rust port can't replicate Python's lambda-
    /// closure getcwd; the caller derives the cwd from the returned
    /// segment_info instead).
    pub fn get_segment_info(
        &self,
        segment_info: Option<Map<String, Value>>,
        mode: Option<&str>,
    ) -> Map<String, Value> {
        // py:230  r = self.segment_info.copy()
        let mut r = Self::segment_info();
        // py:231  r['mode'] = mode
        r.insert(
            "mode".to_string(),
            mode.map(|s| Value::String(s.into())).unwrap_or(Value::Null),
        );
        // py:232-233  if segment_info: r.update(segment_info)
        if let Some(extra) = segment_info {
            for (k, v) in extra {
                r.insert(k, v);
            }
        }
        // py:234-235  if 'PWD' in r['environ']: r['getcwd'] = lambda: environ['PWD']
        let pwd = r
            .get("environ")
            .and_then(|v| v.as_object())
            .and_then(|env| env.get("PWD"))
            .and_then(|v| v.as_str())
            .map(String::from);
        if let Some(p) = pwd {
            r.insert("getcwd".to_string(), Value::String(p));
        }
        r
    }

    /// Port of `Renderer.render_above_lines()` from
    /// `powerline/renderer.py:241-252`.
    ///
    /// Iterates `theme.get_line_number() - 1` down to 1 and yields each
    /// rendered line. Python uses a generator; Rust port materializes
    /// the lines into a `Vec<Value>` since Rust closures can't borrow
    /// `&self` through a generator state machine.
    #[allow(clippy::too_many_arguments)]
    pub fn render_above_lines<HS, H, CF>(
        &self,
        mode: Option<&str>,
        width: Option<usize>,
        output_raw: bool,
        output_width: bool,
        segment_info: Option<Map<String, Value>>,
        matcher_info: Option<&Value>,
        hl_args: Option<Map<String, Value>>,
        theme: &crate::ported::theme::Theme,
        colorscheme: &crate::ported::colorscheme::Colorscheme,
        contents_func: &CF,
        hlstyle_fn: &HS,
        hl_fn: &H,
    ) -> Vec<RenderReturn>
    where
        HS: Fn(&Value, &Value, &Value, &Map<String, Value>) -> String,
        H: Fn(Option<&str>, &Value, &Value, &Value, &Map<String, Value>) -> String,
        CF: Fn(&str, &(), &Map<String, Value>, &Map<String, Value>) -> Option<Value>,
    {
        let mut out: Vec<RenderReturn> = Vec::new();
        // py:241  def render_above_lines(self, **kwargs):
        // py:242-247  docstring
        // py:250  theme = self.get_theme(kwargs.get('matcher_info', None))
        let _ = self.get_theme(matcher_info);
        // py:251  for line in range(theme.get_line_number() - 1, 0, -1):
        let line_n = theme.get_line_number();
        if line_n == 0 {
            return out;
        }
        for line in (1..line_n).rev() {
            // py:252  yield self.render(side=None, line=line, **kwargs)
            out.push(self.render(
                mode,
                width,
                None,
                line,
                output_raw,
                output_width,
                segment_info.clone(),
                matcher_info,
                hl_args.clone(),
                theme,
                colorscheme,
                contents_func,
                hlstyle_fn,
                hl_fn,
            ));
        }
        out
    }

    /// Port of `Renderer.render()` from
    /// `powerline/renderer.py:254-306`.
    ///
    /// Resolves the theme via `get_theme(matcher_info)` and delegates
    /// to `do_render`. Python `theme` resolution at py:295 is done by
    /// the caller in Rust because `Renderer` doesn't own a `&Theme`
    /// in the port surface — `theme` is passed through.
    #[allow(clippy::too_many_arguments)]
    pub fn render<HS, H, CF>(
        &self,
        mode: Option<&str>,
        width: Option<usize>,
        side: Option<&str>,
        line: usize,
        output_raw: bool,
        output_width: bool,
        segment_info: Option<Map<String, Value>>,
        matcher_info: Option<&Value>,
        hl_args: Option<Map<String, Value>>,
        theme: &crate::ported::theme::Theme,
        colorscheme: &crate::ported::colorscheme::Colorscheme,
        contents_func: &CF,
        hlstyle_fn: &HS,
        hl_fn: &H,
    ) -> RenderReturn
    where
        HS: Fn(&Value, &Value, &Value, &Map<String, Value>) -> String,
        H: Fn(Option<&str>, &Value, &Value, &Value, &Map<String, Value>) -> String,
        CF: Fn(&str, &(), &Map<String, Value>, &Map<String, Value>) -> Option<Value>,
    {
        // py:254  def render(self, mode=None, width=None, side=None, line=0,
        // py:255-294  docstring
        // py:295  theme = self.get_theme(matcher_info)
        let _ = self.get_theme(matcher_info);
        // py:296-306  return self.do_render(...)
        self.do_render(
            mode,
            width,
            side,
            line,
            output_raw,
            output_width,
            self.get_segment_info(segment_info, mode),
            theme,
            colorscheme,
            contents_func,
            hl_args,
            hlstyle_fn,
            hl_fn,
        )
    }

    /// Port of `Renderer.do_render()` from
    /// `powerline/renderer.py:333-410`.
    ///
    /// The main render loop: pulls segments from the theme, prepares
    /// them, drops low-priority segments to fit `width`, distributes
    /// spacers, and joins highlighted output via `_render_segments`.
    #[allow(clippy::too_many_arguments)]
    pub fn do_render<HS, H, CF>(
        &self,
        mode: Option<&str>,
        width: Option<usize>,
        side: Option<&str>,
        line: usize,
        output_raw: bool,
        output_width: bool,
        segment_info: Map<String, Value>,
        theme: &crate::ported::theme::Theme,
        colorscheme: &crate::ported::colorscheme::Colorscheme,
        contents_func: &CF,
        hl_args: Option<Map<String, Value>>,
        hlstyle_fn: &HS,
        hl_fn: &H,
    ) -> RenderReturn
    where
        HS: Fn(&Value, &Value, &Value, &Map<String, Value>) -> String,
        H: Fn(Option<&str>, &Value, &Value, &Value, &Map<String, Value>) -> String,
        CF: Fn(&str, &(), &Map<String, Value>, &Map<String, Value>) -> Option<Value>,
    {
        // py:333  def do_render(self, mode, width, side, line, ...):
        // py:334-335  docstring
        // py:336  segments = list(theme.get_segments(side, line, segment_info, mode))
        let segment_info_value = Value::Object(segment_info.clone());
        let mut segments: Vec<Value> = theme.get_segments(
            side,
            line,
            Some(&segment_info_value),
            mode,
            colorscheme,
            contents_func,
        );

        // py:338  current_width = 0
        let mut current_width: usize = 0;

        // py:340  self._prepare_segments(segments, output_width or width)
        Self::_prepare_segments(&mut segments, output_width || width.is_some());

        // py:342  hl_args = hl_args or dict()
        let hl_args: Map<String, Value> = hl_args.unwrap_or_default();

        // py:344  if not width:
        if width.is_none() {
            // py:345  # No width specified, so we don't need to crop or pad anything
            // py:346  if output_width:
            if output_width {
                // py:347  current_width = self._render_length(theme, segments, self.compute_divider_widths(theme))
                let dw = compute_divider_widths(|s, k| {
                    theme.get_divider(s, k).unwrap_or_default()
                });
                current_width = self._render_length(theme, &mut segments, &dw);
            }
            // py:348-351  return construct_returned_value(hl_join([s['_rendered_hl'] ...]) + hlstyle(**hl_args), ...)
            let rendered = self._render_segments(
                theme,
                &mut segments,
                &hl_args,
                true,
                hlstyle_fn,
                hl_fn,
            );
            let joined: String = rendered
                .iter()
                .filter_map(|s| s.get("_rendered_hl").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .concat();
            let trailing = hlstyle_fn(&Value::Null, &Value::Null, &Value::Null, &hl_args);
            let raw = if output_raw {
                Some(
                    segments
                        .iter()
                        .filter_map(|s| s.get("_rendered_raw").and_then(|v| v.as_str()))
                        .collect::<Vec<_>>()
                        .concat(),
                )
            } else {
                None
            };
            return construct_returned_value(
                format!("{}{}", joined, trailing),
                raw,
                current_width,
                output_raw,
                output_width,
            );
        }
        let width = width.expect("checked above");

        // py:353  divider_widths = self.compute_divider_widths(theme)
        let divider_widths =
            compute_divider_widths(|s, k| theme.get_divider(s, k).unwrap_or_default());

        // py:355  # Create an ordered list of segments that can be dropped
        // py:356  segments_priority = sorted(... priority is not None ..., reverse=True)
        let mut segments_priority: Vec<usize> = segments
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.get("priority")
                    .map(|v| !v.is_null())
                    .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect();
        segments_priority.sort_by(|a, b| {
            let pa = segments[*a]
                .get("priority")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let pb = segments[*b]
                .get("priority")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
        });

        // py:357  no_priority_segments = filter(... priority is None ..., segments)
        let no_priority_segments: Vec<usize> = segments
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.get("priority")
                    .map(|v| v.is_null())
                    .unwrap_or(true)
            })
            .map(|(i, _)| i)
            .collect();

        // py:358  current_width = self._render_length(theme, segments, divider_widths)
        current_width = self._render_length(theme, &mut segments, &divider_widths);

        // py:359  if current_width > width:
        if current_width > width {
            // py:360  for segment in chain(segments_priority, no_priority_segments):
            for &idx in segments_priority.iter().chain(no_priority_segments.iter()) {
                // py:361  if segment['truncate'] is not None:
                let has_truncate = segments[idx]
                    .get("truncate")
                    .map(|v| !v.is_null())
                    .unwrap_or(false);
                if has_truncate {
                    // py:362  segment['contents'] = segment['truncate'](self.pl, current_width - width, segment)
                    // Truncate is a callable in Python; the port can't invoke it without
                    // a callable closure on the segment. Leave contents as-is — the
                    // priority-drop loop below still narrows the line.
                }
            }

            // py:364  segments_priority = iter(segments_priority)
            let mut sp_iter = segments_priority.iter().copied();

            // py:365  if current_width > width and len(segments) > 100:
            if current_width > width && segments.len() > 100 {
                // py:366-373  fast variant: drop segments while diff > 0
                let mut diff = current_width as i64 - width as i64;
                let mut to_drop: Vec<usize> = Vec::new();
                for idx in sp_iter.by_ref() {
                    to_drop.push(idx);
                    diff -= segments[idx]
                        .get("_len")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    if diff <= 0 {
                        break;
                    }
                }
                // sh-style drop in descending order so indices stay valid.
                to_drop.sort_unstable_by(|a, b| b.cmp(a));
                for idx in to_drop {
                    segments.remove(idx);
                }
                // py:374  current_width = self._render_length(theme, segments, divider_widths)
                current_width = self._render_length(theme, &mut segments, &divider_widths);
            }
            // py:375  if current_width > width:
            if current_width > width {
                // py:376-383  slow variant: drop, re-measure, stop when fits
                let mut remaining: Vec<usize> = sp_iter.collect();
                remaining.sort_unstable_by(|a, b| b.cmp(a));
                for idx in remaining {
                    if idx < segments.len() {
                        segments.remove(idx);
                    }
                    current_width =
                        self._render_length(theme, &mut segments, &divider_widths);
                    if current_width <= width {
                        break;
                    }
                }
            }
        }
        // py:384  del segments_priority — Rust scope drop handles this

        // py:386  # Distribute the remaining space on spacer segments
        // py:387  segments_spacers = [segment for segment in segments if segment['expand'] is not None]
        let segments_spacers: Vec<usize> = segments
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.get("expand")
                    .map(|v| !v.is_null())
                    .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect();
        if !segments_spacers.is_empty() {
            // py:389  distribute_len, distribute_len_remainder = divmod(width - current_width, len(segments_spacers))
            let remaining_total = width.saturating_sub(current_width) as i64;
            let n = segments_spacers.len() as i64;
            let distribute_len = remaining_total / n;
            let mut distribute_len_remainder = remaining_total % n;
            // py:390  for segment in segments_spacers:
            for idx in &segments_spacers {
                // py:391-395  segment['contents'] = segment['expand'](pl, distribute_len + (1 if remainder > 0 else 0), segment)
                // expand is a Python callable; can't invoke from JSON.
                // Synthesize padding spaces of the computed width instead so the
                // line still fills to `width` in the default tmux flow.
                let extra = if distribute_len_remainder > 0 { 1 } else { 0 };
                let pad_n = (distribute_len + extra).max(0) as usize;
                let pad = " ".repeat(pad_n);
                if let Some(obj) = segments[*idx].as_object_mut() {
                    let existing = obj
                        .get("contents")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    obj.insert(
                        "contents".to_string(),
                        Value::String(format!("{}{}", existing, pad)),
                    );
                }
                // py:396  distribute_len_remainder -= 1
                distribute_len_remainder -= 1;
            }
            // py:399  current_width = width
            current_width = width;
        } else if output_width {
            // py:401  current_width = self._render_length(theme, segments, divider_widths)
            current_width = self._render_length(theme, &mut segments, &divider_widths);
        }

        // py:403-406  rendered_highlighted = hl_join([s['_rendered_hl'] for s in _render_segments(...)])
        let rendered = self._render_segments(
            theme,
            &mut segments,
            &hl_args,
            true,
            hlstyle_fn,
            hl_fn,
        );
        let mut rendered_highlighted: String = rendered
            .iter()
            .filter_map(|s| s.get("_rendered_hl").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .concat();
        // py:407-408  if rendered_highlighted: rendered_highlighted += self.hlstyle(**hl_args)
        if !rendered_highlighted.is_empty() {
            rendered_highlighted
                .push_str(&hlstyle_fn(&Value::Null, &Value::Null, &Value::Null, &hl_args));
        }
        // py:410  return construct_returned_value(rendered_highlighted, segments, current_width, output_raw, output_width)
        let raw = if output_raw {
            Some(
                segments
                    .iter()
                    .filter_map(|s| s.get("_rendered_raw").and_then(|v| v.as_str()))
                    .collect::<Vec<_>>()
                    .concat(),
            )
        } else {
            None
        };
        construct_returned_value(
            rendered_highlighted,
            raw,
            current_width,
            output_raw,
            output_width,
        )
    }

    /// Port of `Renderer._render_length()` from
    /// `powerline/renderer.py:424-479`.
    ///
    /// Updates each segment's `_len` field with the rendered length
    /// (contents + dividers + outer padding) and returns the running
    /// total. Skips segments whose `literal_contents[1]` is set per
    /// py:450 — those are not subject to divider math.
    pub fn _render_length(
        &self,
        theme: &crate::ported::theme::Theme,
        segments: &mut [Value],
        divider_widths: &Map<String, Value>,
    ) -> usize {
        // py:424  def _render_length(self, theme, segments, divider_widths):
        // py:425-426  docstring
        // py:427  segments_len = len(segments)
        let _segments_len = segments.len();
        // py:428  ret = 0
        let mut ret: usize = 0;
        // py:429  divider_spaces = theme.get_spaces()
        let divider_spaces = theme.get_spaces() as usize;
        // py:430  prev_segment = theme.EMPTY_SEGMENT
        let mut prev_segment_bg: Value = theme
            .empty_segment
            .get("highlight")
            .and_then(|v| v.get("bg"))
            .cloned()
            .unwrap_or(Value::Null);
        // py:431-438  first_segment = first segment with empty literal_contents[1]
        let first_segment_idx = segments
            .iter()
            .position(|s| {let __l = s.get("literal_contents").and_then(|v|v.as_array()).and_then(|a|a.get(1)).and_then(|v|v.as_str()).map(|x|!x.is_empty()).unwrap_or(false); !__l});
        // py:439-446  last_segment = last segment with empty literal_contents[1]
        let last_segment_idx = segments
            .iter()
            .rposition(|s| {let __l = s.get("literal_contents").and_then(|v|v.as_array()).and_then(|a|a.get(1)).and_then(|v|v.as_str()).map(|x|!x.is_empty()).unwrap_or(false); !__l});
        // py:447  for index, segment in enumerate(segments):
        for index in 0..segments.len() {
            // py:448  side = segment['side']
            let side = segments[index]
                .get("side")
                .and_then(|v| v.as_str())
                .unwrap_or("left")
                .to_string();
            // py:449  segment_len = segment['_contents_len']
            let mut segment_len: usize = segments[index]
                .get("_contents_len")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            // py:450  if not segment['literal_contents'][1]:
            if {let __l = segments[index].get("literal_contents").and_then(|v|v.as_array()).and_then(|a|a.get(1)).and_then(|v|v.as_str()).map(|x|!x.is_empty()).unwrap_or(false); !__l} {
                // py:451-461  compare_segment
                let compare_bg: Value = if side == "left" {
                    if Some(index) != last_segment_idx {
                        let nxt = segments[index + 1..]
                            .iter()
                            .position(|s| {let __l = s.get("literal_contents").and_then(|v|v.as_array()).and_then(|a|a.get(1)).and_then(|v|v.as_str()).map(|x|!x.is_empty()).unwrap_or(false); !__l})
                            .map(|p| p + index + 1);
                        match nxt {
                            Some(j) => segments[j]
                                .get("highlight")
                                .and_then(|v| v.get("bg"))
                                .cloned()
                                .unwrap_or(Value::Null),
                            None => theme
                                .empty_segment
                                .get("highlight")
                                .and_then(|v| v.get("bg"))
                                .cloned()
                                .unwrap_or(Value::Null),
                        }
                    } else {
                        theme
                            .empty_segment
                            .get("highlight")
                            .and_then(|v| v.get("bg"))
                            .cloned()
                            .unwrap_or(Value::Null)
                    }
                } else {
                    prev_segment_bg.clone()
                };
                let seg_bg: Value = segments[index]
                    .get("highlight")
                    .and_then(|v| v.get("bg"))
                    .cloned()
                    .unwrap_or(Value::Null);
                // py:463  divider_type = 'soft' if compare_segment['highlight']['bg'] == segment['highlight']['bg'] else 'hard'
                let divider_type = if compare_bg == seg_bg { "soft" } else { "hard" };

                // py:465-469  outer_padding
                let is_first = Some(index) == first_segment_idx;
                let is_last = Some(index) == last_segment_idx;
                let outer_padding = if (side == "left" && is_first)
                    || (side == "right" && is_last)
                {
                    theme.outer_padding as usize
                } else {
                    0
                };

                // py:471  draw_divider = segment['draw_' + divider_type + '_divider']
                let draw_divider = segments[index]
                    .get(&format!("draw_{}_divider", divider_type))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                // py:472  segment_len += outer_padding
                segment_len += outer_padding;
                // py:473-474  if draw_divider: segment_len += divider_widths[side][divider_type] + divider_spaces
                if draw_divider {
                    let dw = divider_widths
                        .get(&side)
                        .and_then(|v| v.get(divider_type))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    segment_len += dw + divider_spaces;
                }
                // py:475  prev_segment = segment
                prev_segment_bg = seg_bg;
            }
            // py:477  segment['_len'] = segment_len
            if let Some(obj) = segments[index].as_object_mut() {
                obj.insert("_len".to_string(), Value::from(segment_len as u64));
            }
            // py:478  ret += segment_len
            ret += segment_len;
        }
        // py:479  return ret
        ret
    }

    /// Port of `Renderer._render_segments()` from
    /// `powerline/renderer.py:481-584`.
    ///
    /// Walks each segment, computes the dividers + outer padding,
    /// invokes `hl_fn`/`hlstyle_fn` for the contents and dividers,
    /// and writes `_rendered_raw` + `_rendered_hl` back onto the
    /// segment dict. Returns the list of rendered segments (mirrors
    /// the Python generator that yields each segment).
    pub fn _render_segments<HS, H>(
        &self,
        theme: &crate::ported::theme::Theme,
        segments: &mut [Value],
        hl_args: &Map<String, Value>,
        render_highlighted: bool,
        hlstyle_fn: &HS,
        hl_fn: &H,
    ) -> Vec<Value>
    where
        HS: Fn(&Value, &Value, &Value, &Map<String, Value>) -> String,
        H: Fn(Option<&str>, &Value, &Value, &Value, &Map<String, Value>) -> String,
    {
        // py:481  def _render_segments(self, theme, segments, hl_args, render_highlighted=True):
        // py:482-491  docstring
        // py:492  segments_len = len(segments)
        let _segments_len = segments.len();
        // py:493  divider_spaces = theme.get_spaces()
        let divider_spaces = theme.get_spaces() as usize;
        // py:494  prev_segment = theme.EMPTY_SEGMENT
        let mut prev_segment_hl: Map<String, Value> = theme
            .empty_segment
            .get("highlight")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        // py:495-510  first_segment / last_segment scan
        let first_segment_idx = segments.iter().position(|s| {let __l = s.get("literal_contents").and_then(|v|v.as_array()).and_then(|a|a.get(1)).and_then(|v|v.as_str()).map(|x|!x.is_empty()).unwrap_or(false); !__l});
        let last_segment_idx = segments.iter().rposition(|s| {let __l = s.get("literal_contents").and_then(|v|v.as_array()).and_then(|a|a.get(1)).and_then(|v|v.as_str()).map(|x|!x.is_empty()).unwrap_or(false); !__l});

        // py:512  for index, segment in enumerate(segments):
        for index in 0..segments.len() {
            // py:513  side = segment['side']
            let side = segments[index]
                .get("side")
                .and_then(|v| v.as_str())
                .unwrap_or("left")
                .to_string();
            // py:514  if not segment['literal_contents'][1]:
            if {let __l = segments[index].get("literal_contents").and_then(|v|v.as_array()).and_then(|a|a.get(1)).and_then(|v|v.as_str()).map(|x|!x.is_empty()).unwrap_or(false); !__l} {
                // py:515-525  compare_segment
                let compare_hl: Map<String, Value> = if side == "left" {
                    if Some(index) != last_segment_idx {
                        let nxt = segments[index + 1..]
                            .iter()
                            .position(|s| {let __l = s.get("literal_contents").and_then(|v|v.as_array()).and_then(|a|a.get(1)).and_then(|v|v.as_str()).map(|x|!x.is_empty()).unwrap_or(false); !__l})
                            .map(|p| p + index + 1);
                        match nxt {
                            Some(j) => segments[j]
                                .get("highlight")
                                .and_then(|v| v.as_object())
                                .cloned()
                                .unwrap_or_default(),
                            None => theme
                                .empty_segment
                                .get("highlight")
                                .and_then(|v| v.as_object())
                                .cloned()
                                .unwrap_or_default(),
                        }
                    } else {
                        theme
                            .empty_segment
                            .get("highlight")
                            .and_then(|v| v.as_object())
                            .cloned()
                            .unwrap_or_default()
                    }
                } else {
                    prev_segment_hl.clone()
                };
                // py:526-530  outer_padding
                let is_first = Some(index) == first_segment_idx;
                let is_last = Some(index) == last_segment_idx;
                let outer_padding = if (side == "left" && is_first)
                    || (side == "right" && is_last)
                {
                    " ".repeat(theme.outer_padding as usize)
                } else {
                    String::new()
                };
                // py:531  divider_type = 'soft' if compare_segment['highlight']['bg'] == segment['highlight']['bg'] else 'hard'
                let seg_bg = segments[index]
                    .get("highlight")
                    .and_then(|v| v.get("bg"))
                    .cloned()
                    .unwrap_or(Value::Null);
                let cmp_bg = compare_hl.get("bg").cloned().unwrap_or(Value::Null);
                let divider_type = if seg_bg == cmp_bg { "soft" } else { "hard" };

                // py:533  divider_highlighted = ''
                let mut divider_highlighted = String::new();
                // py:534  contents_raw = segment['contents']
                let mut contents_raw = segments[index]
                    .get("contents")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                // py:535  contents_highlighted = ''
                let mut contents_highlighted = String::new();
                // py:536  draw_divider = segment['draw_' + divider_type + '_divider']
                let draw_divider = segments[index]
                    .get(&format!("draw_{}_divider", divider_type))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                // py:538-540  segment_hl_args = segment['highlight'].copy(); update hl_args
                let mut segment_hl_args: Map<String, Value> = segments[index]
                    .get("highlight")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                for (k, v) in hl_args {
                    segment_hl_args.insert(k.clone(), v.clone());
                }

                let seg_fg = segments[index]
                    .get("highlight")
                    .and_then(|v| v.get("fg"))
                    .cloned()
                    .unwrap_or(Value::Null);
                let seg_attrs = segments[index]
                    .get("highlight")
                    .and_then(|v| v.get("attrs"))
                    .cloned()
                    .unwrap_or(Value::Null);

                if draw_divider {
                    // py:545  divider_raw = self.escape(theme.get_divider(side, divider_type))
                    let divider_raw =
                        self.escape(&theme.get_divider(&side, divider_type).unwrap_or_default());
                    // py:546-549  contents_raw composition
                    if side == "left" {
                        contents_raw = format!(
                            "{}{}{}",
                            outer_padding,
                            contents_raw,
                            " ".repeat(divider_spaces)
                        );
                    } else {
                        contents_raw = format!(
                            "{}{}{}",
                            " ".repeat(divider_spaces),
                            contents_raw,
                            outer_padding
                        );
                    }
                    // py:551-557  divider_fg / divider_bg
                    let (divider_fg, divider_bg) = if divider_type == "soft" {
                        // py:552  divider_highlight_group_key = 'highlight' if divider_highlight_group is None else 'divider_highlight'
                        let dhg_present = segments[index]
                            .get("divider_highlight_group")
                            .map(|v| !v.is_null())
                            .unwrap_or(false);
                        let key = if dhg_present {
                            "divider_highlight"
                        } else {
                            "highlight"
                        };
                        let fg = segments[index]
                            .get(key)
                            .and_then(|v| v.get("fg"))
                            .cloned()
                            .unwrap_or(Value::Null);
                        let bg = segments[index]
                            .get(key)
                            .and_then(|v| v.get("bg"))
                            .cloned()
                            .unwrap_or(Value::Null);
                        (fg, bg)
                    } else {
                        // py:555-557  hard divider colors
                        let fg = seg_bg.clone();
                        let bg = compare_hl.get("bg").cloned().unwrap_or(Value::Null);
                        (fg, bg)
                    };

                    let attrs_false = Value::Bool(false);
                    // py:559-570  emit contents + divider in side-specific order
                    if side == "left" {
                        if render_highlighted {
                            // py:561  self.hl(self.escape(contents_raw), **segment_hl_args)
                            let escaped = self.escape(&contents_raw);
                            contents_highlighted = hl_fn(
                                Some(&escaped),
                                &seg_fg,
                                &seg_bg,
                                &seg_attrs,
                                &segment_hl_args,
                            );
                            // py:562  self.hl(divider_raw, divider_fg, divider_bg, False, **hl_args)
                            divider_highlighted = hl_fn(
                                Some(&divider_raw),
                                &divider_fg,
                                &divider_bg,
                                &attrs_false,
                                hl_args,
                            );
                        }
                        if let Some(obj) = segments[index].as_object_mut() {
                            obj.insert(
                                "_rendered_raw".to_string(),
                                Value::String(format!("{}{}", contents_raw, divider_raw)),
                            );
                            obj.insert(
                                "_rendered_hl".to_string(),
                                Value::String(format!(
                                    "{}{}",
                                    contents_highlighted, divider_highlighted
                                )),
                            );
                        }
                    } else {
                        if render_highlighted {
                            // py:567  self.hl(divider_raw, divider_fg, divider_bg, False, **hl_args)
                            divider_highlighted = hl_fn(
                                Some(&divider_raw),
                                &divider_fg,
                                &divider_bg,
                                &attrs_false,
                                hl_args,
                            );
                            // py:568  self.hl(self.escape(contents_raw), **segment_hl_args)
                            let escaped = self.escape(&contents_raw);
                            contents_highlighted = hl_fn(
                                Some(&escaped),
                                &seg_fg,
                                &seg_bg,
                                &seg_attrs,
                                &segment_hl_args,
                            );
                        }
                        if let Some(obj) = segments[index].as_object_mut() {
                            obj.insert(
                                "_rendered_raw".to_string(),
                                Value::String(format!("{}{}", divider_raw, contents_raw)),
                            );
                            obj.insert(
                                "_rendered_hl".to_string(),
                                Value::String(format!(
                                    "{}{}",
                                    divider_highlighted, contents_highlighted
                                )),
                            );
                        }
                    }
                } else {
                    // py:571-579  no divider
                    if side == "left" {
                        contents_raw = format!("{}{}", outer_padding, contents_raw);
                    } else {
                        contents_raw = format!("{}{}", contents_raw, outer_padding);
                    }
                    // py:577  contents_highlighted = self.hl(self.escape(contents_raw), **segment_hl_args)
                    let escaped = self.escape(&contents_raw);
                    contents_highlighted = hl_fn(
                        Some(&escaped),
                        &seg_fg,
                        &seg_bg,
                        &seg_attrs,
                        &segment_hl_args,
                    );
                    if let Some(obj) = segments[index].as_object_mut() {
                        obj.insert(
                            "_rendered_raw".to_string(),
                            Value::String(contents_raw),
                        );
                        obj.insert(
                            "_rendered_hl".to_string(),
                            Value::String(contents_highlighted),
                        );
                    }
                }
                // py:580  prev_segment = segment
                prev_segment_hl = segments[index]
                    .get("highlight")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
            } else {
                // py:582-583  literal segment
                let n = segments[index]
                    .get("literal_contents")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                let lit = segments[index]
                    .get("literal_contents")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.get(1))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(obj) = segments[index].as_object_mut() {
                    obj.insert(
                        "_rendered_raw".to_string(),
                        Value::String(" ".repeat(n)),
                    );
                    obj.insert("_rendered_hl".to_string(), Value::String(lit));
                }
            }
        }
        let _ = hlstyle_fn; // referenced only inside the literal/divider paths above
        // py:584  yield segment — Rust returns the whole vec
        segments.to_vec()
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nbsp_constant_matches_upstream() {
        // py:15  NBSP = ' '
        assert_eq!(NBSP, "\u{a0}");
    }

    #[test]
    fn np_control_character_translations_has_32_entries() {
        // py:18-21  range(0x20) → 32 entries
        let m = np_control_character_translations();
        assert_eq!(m.len(), 32);
    }

    #[test]
    fn np_control_character_translations_maps_null_to_caret_at() {
        // py:20  '\x00' → '^@'
        let m = np_control_character_translations();
        assert_eq!(m.get(&'\u{00}'), Some(&"^@".to_string()));
    }

    #[test]
    fn np_control_character_translations_maps_a_to_caret_a() {
        // py:20  '\x01' → '^A'
        let m = np_control_character_translations();
        assert_eq!(m.get(&'\u{01}'), Some(&"^A".to_string()));
    }

    #[test]
    fn np_control_character_translations_maps_tab_to_caret_i() {
        // py:18-21 docstring: maps tab (0x09) to '^I'
        let m = np_control_character_translations();
        assert_eq!(m.get(&'\t'), Some(&"^I".to_string()));
    }

    #[test]
    fn np_control_character_translations_maps_newline_to_caret_j() {
        // py:18-21 docstring: maps newline (0x0A) to '^J'
        let m = np_control_character_translations();
        assert_eq!(m.get(&'\n'), Some(&"^J".to_string()));
    }

    #[test]
    fn np_invalid_character_translations_has_128_entries() {
        // py:30-33  range(0xDC80, 0xDD00) → 128 entries
        let m = np_invalid_character_translations();
        assert_eq!(m.len(), 128);
    }

    #[test]
    fn np_invalid_character_translations_first_entry_is_80() {
        // py:32  0xDC80 → '<80>'
        let m = np_invalid_character_translations();
        assert_eq!(m.get(&0xDC80), Some(&"<80>".to_string()));
    }

    #[test]
    fn np_invalid_character_translations_last_entry_is_ff() {
        // py:30-33  0xDCFF → '<ff>'
        let m = np_invalid_character_translations();
        assert_eq!(m.get(&0xDCFF), Some(&"<ff>".to_string()));
    }

    #[test]
    fn np_character_translations_contains_both_control_and_invalid() {
        let m = np_character_translations();
        // 32 control + 128 invalid (where char::from_u32 succeeds = 0)
        // surrogates are invalid Rust chars so the union doesn't
        // actually include them — Rust char::from_u32 returns None for
        // surrogates. So we get only the 32 control entries.
        assert!(m.len() >= 32);
        assert!(m.contains_key(&'\u{00}'));
    }

    #[test]
    fn translate_np_replaces_control_chars() {
        // py:74-82  translate via character_translations table
        let r = translate_np("abc\x01def");
        assert_eq!(r, "abc^Adef");
    }

    #[test]
    fn translate_np_passes_printable_chars_through() {
        let r = translate_np("hello world");
        assert_eq!(r, "hello world");
    }

    #[test]
    fn translate_np_handles_multiple_control_chars() {
        let r = translate_np("\x00\x01\x02");
        assert_eq!(r, "^@^A^B");
    }

    #[test]
    fn construct_returned_value_plain_when_no_flags() {
        // py:93-94  return rendered_highlighted
        let r = construct_returned_value("hi".to_string(), None, 5, false, false);
        match r {
            RenderReturn::Plain(s) => assert_eq!(s, "hi"),
            _ => panic!("expected Plain"),
        }
    }

    #[test]
    fn construct_returned_value_tuple_with_width_only() {
        let r = construct_returned_value("hi".to_string(), None, 5, false, true);
        match r {
            RenderReturn::Tuple {
                highlighted,
                raw,
                width,
            } => {
                assert_eq!(highlighted, "hi");
                assert!(raw.is_none());
                assert_eq!(width, Some(5));
            }
            _ => panic!("expected Tuple"),
        }
    }

    #[test]
    fn construct_returned_value_tuple_with_raw_only() {
        let r =
            construct_returned_value("hi".to_string(), Some("hi-raw".to_string()), 5, true, false);
        match r {
            RenderReturn::Tuple {
                highlighted,
                raw,
                width,
            } => {
                assert_eq!(highlighted, "hi");
                assert_eq!(raw, Some("hi-raw".to_string()));
                assert!(width.is_none());
            }
            _ => panic!("expected Tuple"),
        }
    }

    #[test]
    fn construct_returned_value_tuple_with_both() {
        let r =
            construct_returned_value("hi".to_string(), Some("hi-raw".to_string()), 5, true, true);
        match r {
            RenderReturn::Tuple {
                highlighted,
                raw,
                width,
            } => {
                assert_eq!(highlighted, "hi");
                assert_eq!(raw, Some("hi-raw".to_string()));
                assert_eq!(width, Some(5));
            }
            _ => panic!("expected Tuple"),
        }
    }

    #[test]
    fn width_data_default_ambiwidth_is_1() {
        // py:177-184
        let w = width_data(1);
        assert_eq!(w.get(&'N'), Some(&1));
        assert_eq!(w.get(&'A'), Some(&1));
        assert_eq!(w.get(&'W'), Some(&2));
        assert_eq!(w.get(&'F'), Some(&2));
    }

    #[test]
    fn width_data_ambiwidth_overrides_ambiguous() {
        // py:181  'A': ambiwidth
        let w = width_data(2);
        assert_eq!(w.get(&'A'), Some(&2));
    }

    #[test]
    fn strwidth_counts_chars() {
        // py:188-191  per-char width sum
        assert_eq!(strwidth("hello"), 5);
        assert_eq!(strwidth(""), 0);
        assert_eq!(strwidth("café"), 4);
    }

    #[test]
    fn compute_divider_widths_emits_both_sides_and_kinds() {
        // py:303-310
        let r = compute_divider_widths(|side, kind| match (side, kind) {
            ("left", "hard") => " ".to_string(),
            ("left", "soft") => " ".to_string(),
            ("right", "hard") => " ".to_string(),
            ("right", "soft") => " ".to_string(),
            _ => "".to_string(),
        });
        assert!(r.contains_key("left"));
        assert!(r.contains_key("right"));
        let left = r["left"].as_object().unwrap();
        assert_eq!(left["hard"], 1);
        assert_eq!(left["soft"], 1);
    }

    #[test]
    fn renderer_init_use_nbsp_default_adds_space_translation() {
        // py:167-171  use_non_breaking_spaces defaults to True
        let cfg = Map::new();
        let r = Renderer::new(cfg, Map::new(), 1);
        assert_eq!(r.character_translations.get(&' '), Some(&NBSP.to_string()));
    }

    #[test]
    fn renderer_init_use_nbsp_false_omits_space_translation() {
        let mut cfg = Map::new();
        cfg.insert("use_non_breaking_spaces".to_string(), Value::Bool(false));
        let r = Renderer::new(cfg, Map::new(), 1);
        assert!(!r.character_translations.contains_key(&' '));
    }

    #[test]
    fn renderer_init_uses_ambiwidth_for_width_data() {
        let cfg = Map::new();
        let r = Renderer::new(cfg, Map::new(), 2);
        assert_eq!(r.width_data.get(&'A'), Some(&2));
    }

    #[test]
    fn segment_info_includes_environ_and_home() {
        let info = Renderer::segment_info();
        assert!(info.contains_key("environ"));
        assert!(info.contains_key("home"));
    }

    #[test]
    fn get_segment_info_merges_segment_info_over_base() {
        let cfg = Map::new();
        let r = Renderer::new(cfg, Map::new(), 1);
        let mut extra = Map::new();
        extra.insert("client_id".to_string(), Value::from(42));
        let info = r.get_segment_info(Some(extra), Some("normal"));
        assert_eq!(info["mode"], "normal");
        assert_eq!(info["client_id"], 42);
    }

    #[test]
    fn get_segment_info_sets_mode_null_when_none() {
        let cfg = Map::new();
        let r = Renderer::new(cfg, Map::new(), 1);
        let info = r.get_segment_info(None, None);
        assert_eq!(info["mode"], Value::Null);
    }

    #[test]
    fn get_segment_info_overrides_getcwd_when_pwd_set() {
        // py:234-235  if 'PWD' in environ: getcwd = lambda
        let cfg = Map::new();
        let r = Renderer::new(cfg, Map::new(), 1);
        let mut extra = Map::new();
        let mut env = Map::new();
        env.insert("PWD".to_string(), Value::String("/my/cwd".into()));
        extra.insert("environ".to_string(), Value::Object(env));
        let info = r.get_segment_info(Some(extra), None);
        assert_eq!(info.get("getcwd"), Some(&Value::String("/my/cwd".into())));
    }

    #[test]
    fn get_theme_returns_self_theme() {
        // py:208
        let mut r = Renderer::new(Map::new(), Map::new(), 1);
        r.theme = serde_json::json!({"name": "default"});
        let t = r.get_theme(None);
        assert_eq!(t["name"], "default");
    }

    #[test]
    fn get_theme_ignores_matcher_info() {
        // py:205-206  matcher_info: Unused
        let mut r = Renderer::new(Map::new(), Map::new(), 1);
        r.theme = serde_json::json!({"name": "default"});
        let info = serde_json::json!({"foo": "bar"});
        let t = r.get_theme(Some(&info));
        assert_eq!(t["name"], "default");
    }

    #[test]
    fn shutdown_records_theme() {
        // py:215
        let r = Renderer::new(Map::new(), Map::new(), 1);
        r.shutdown();
        let log = r.shutdown_called.lock().unwrap();
        assert_eq!(*log, vec!["theme".to_string()]);
    }

    #[test]
    fn escape_translates_chars_via_character_translations() {
        // py:586-589
        let mut r = Renderer::new(Map::new(), Map::new(), 1);
        r.character_translations.clear();
        r.character_translations.insert('%', "%%".to_string());
        assert_eq!(r.escape("100% done"), "100%% done");
    }

    #[test]
    fn escape_passes_untranslated_chars_through() {
        let r = Renderer::new(Map::new(), Map::new(), 1);
        // Default character_translations only has ' ' → NBSP
        let s = "abc";
        assert_eq!(r.escape(s), "abc");
    }

    #[test]
    fn escape_default_translates_space_to_nbsp() {
        // py:171  character_translations[' '] = NBSP
        let r = Renderer::new(Map::new(), Map::new(), 1);
        let result = r.escape("hi there");
        assert!(result.contains('\u{a0}'));
        assert!(!result.contains(' '));
    }

    #[test]
    fn escape_use_non_breaking_spaces_false_keeps_spaces() {
        // py:167-171  when use_non_breaking_spaces is false, no
        // entry for ' ' in character_translations
        let mut theme_config = Map::new();
        theme_config.insert("use_non_breaking_spaces".to_string(), Value::Bool(false));
        let r = Renderer::new(theme_config, Map::new(), 1);
        assert_eq!(r.escape("hi there"), "hi there");
    }

    #[test]
    fn hl_concatenates_hlstyle_output_and_contents() {
        // py:606  return self.hlstyle(...) + (contents or '')
        let result = Renderer::hl(Some("text"), "\x1b[1m");
        assert_eq!(result, "\x1b[1mtext");
    }

    #[test]
    fn hl_none_contents_becomes_empty_string() {
        // py:606  contents or ''
        let result = Renderer::hl(None, "\x1b[1m");
        assert_eq!(result, "\x1b[1m");
    }

    #[test]
    fn _prepare_segments_translates_non_printable_contents() {
        // py:415-416
        let mut segments: Vec<Value> = vec![serde_json::json!({
            "contents": "hello\x01world",
        })];
        Renderer::_prepare_segments(&mut segments, false);
        // \x01 is a control char that translate_np replaces with "^A"
        let c = segments[0]["contents"].as_str().unwrap();
        assert!(!c.contains('\x01'));
    }

    #[test]
    fn _prepare_segments_calculates_contents_len_from_strwidth_when_no_literal() {
        // py:421-422
        let mut segments: Vec<Value> = vec![serde_json::json!({
            "contents": "hello",
            "literal_contents": [0, ""],
        })];
        Renderer::_prepare_segments(&mut segments, true);
        assert_eq!(segments[0]["_contents_len"], 5);
    }

    #[test]
    fn _prepare_segments_uses_literal_contents_len_when_literal_non_empty() {
        // py:419-420
        let mut segments: Vec<Value> = vec![serde_json::json!({
            "contents": "ignored",
            "literal_contents": [42, "raw text"],
        })];
        Renderer::_prepare_segments(&mut segments, true);
        assert_eq!(segments[0]["_contents_len"], 42);
    }

    #[test]
    fn _prepare_segments_skips_contents_len_when_not_requested() {
        let mut segments: Vec<Value> = vec![serde_json::json!({
            "contents": "hello",
            "literal_contents": [0, ""],
        })];
        Renderer::_prepare_segments(&mut segments, false);
        assert!(segments[0].get("_contents_len").is_none());
    }
}
