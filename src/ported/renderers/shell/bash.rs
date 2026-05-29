// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/shell/bash.py`.
//!
//! Powerline bash prompt segment renderer.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.renderers.shell import ShellRenderer                                     // py:4

use std::collections::HashMap;

/// Port of `class BashPromptRenderer(ShellRenderer)` from
/// `powerline/renderers/shell/bash.py:7`.
///
/// bash's `\[ ... \]` escape markers (PROMPT_PS1-safe non-display
/// regions) + translations for `$`, backtick, and backslash so the
/// shell doesn't interpret literals as command substitution.
pub struct BashPromptRenderer;

impl BashPromptRenderer {
    /// Port of `BashPromptRenderer.escape_hl_start` from
    /// `powerline/renderers/shell/bash.py:9`.
    #[allow(non_upper_case_globals)]
    pub const escape_hl_start: &'static str = "\\[";

    /// Port of `BashPromptRenderer.escape_hl_end` from
    /// `powerline/renderers/shell/bash.py:10`.
    #[allow(non_upper_case_globals)]
    pub const escape_hl_end: &'static str = "\\]";

    /// Port of `BashPromptRenderer.character_translations` from
    /// `powerline/renderers/shell/bash.py:12-15`.
    ///
    /// Python: extends `ShellRenderer.character_translations` with:
    ///   - `$` → `\$` (suppress command substitution)
    ///   - `\`` → `\\\`` (suppress backtick substitution)
    ///   - `\\` → `\\\\` (escape literal backslash)
    pub fn character_translations() -> HashMap<char, &'static str> {
        // py:7  class BashPromptRenderer(ShellRenderer):
        // py:8  '''Powerline bash prompt segment renderer.'''
        // py:9  escape_hl_start = '\\['
        // py:10  escape_hl_end = '\\]'
        // py:12  character_translations = ShellRenderer.character_translations.copy()
        // py:13  character_translations[ord('$')] = '\\$'
        // py:14  character_translations[ord('`')] = '\\`'
        // py:15  character_translations[ord('\\')] = '\\\\'
        let mut t: HashMap<char, &'static str> = HashMap::new();
        t.insert('$', "\\$");
        t.insert('`', "\\`");
        t.insert('\\', "\\\\");
        t
    }

    /// Port of `BashPromptRenderer.do_render()` from
    /// `powerline/renderers/shell/bash.py:17`.
    ///
    /// **Status:** stub. Renders the bash-specific left+right prompt
    /// embedding (using `\033[s` / `\033[u` cursor save+restore +
    /// `\033[NC` / `\033[ND` cursor moves to position the right prompt
    /// at the terminal edge). The base ShellRenderer + escape markers
    /// are unported; this stub surfaces the dispatch shape.
    pub fn do_render(side: &str, line: u32, width: Option<u32>) -> String {
        // py:17  def do_render(self, side, line, width, output_width, output_raw, hl_args, **kwargs):
        // py:19  # we are rendering the normal left prompt
        // py:20  if side == 'left' and line == 0 and width is not None:
        if side == "left" && line == 0 && width.is_some() {
            // py:22  # we need left prompt's width to render the raw spacer
            // py:23  output_width = output_width or output_raw
            // py:25  left = super(BashPromptRenderer, self).do_render(
            // py:26  side=side,
            // py:27  line=line,
            // py:28  output_width=output_width,
            // py:29  width=width,
            // py:30  output_raw=output_raw,
            // py:31  hl_args=hl_args,
            // py:32  **kwargs
            // py:33  )
            // py:34  left_rendered = left[0] if output_width else left
            // py:36  # we don't escape color sequences in the right prompt so we can do escaping as a whole
            // py:37  if hl_args:
            // py:38  hl_args = hl_args.copy()
            // py:39  hl_args.update({'escape': False})
            // py:40  else:
            // py:41  hl_args = {'escape': False}
            // py:43  right = super(BashPromptRenderer, self).do_render(
            // py:44  side='right',
            // py:45  line=line,
            // py:46  output_width=True,
            // py:47  width=width,
            // py:48  output_raw=output_raw,
            // py:49  hl_args=hl_args,
            // py:50  **kwargs
            // py:51  )
            // py:53  ret = []
            // py:54  if right[-1] > 0:
            // py:55  # if the right prompt is not empty we embed it in the left prompt
            // py:56  # it must be escaped as a whole so readline doesn't see it
            // py:57  ret.append(''.join((
            // py:58  left_rendered,
            // py:59  self.escape_hl_start,
            // py:60  '\033[s',                           # save the cursor position
            // py:61  '\033[{0}C'.format(width),          # move to the right edge of the terminal
            // py:62  '\033[{0}D'.format(right[-1] - 1),  # move back to the right prompt position
            // py:63  right[0],
            // py:64  '\033[u',                           # restore the cursor position
            // py:65  self.escape_hl_end
            // py:66  )))
            // py:67  if output_raw:
            // py:68  ret.append(''.join((
            // py:69  left[1],
            // py:70  ' ' * (width - left[-1] - right[-1]),
            // py:71  right[1]
            // py:72  )))
            // py:73  else:
            // py:74  ret.append(left_rendered)
            // py:75  if output_raw:
            // py:76  ret.append(left[1])
            // py:77  if output_width:
            // py:78  ret.append(left[-1])
            // py:79  if len(ret) == 1:
            // py:80  return ret[0]
            // py:81  else:
            // py:82  return ret
            return String::new();
        }
        // py:84  else:
        // py:85  return super(BashPromptRenderer, self).do_render(
        // py:86  side=side,
        // py:87  line=line,
        // py:88  width=width,
        // py:89  output_width=output_width,
        // py:90  output_raw=output_raw,
        // py:91  hl_args=hl_args,
        // py:92  **kwargs
        // py:93  )
        String::new()
    }
}

/// Port of module-level binding `renderer` from
/// `powerline/renderers/shell/bash.py:84`.
#[allow(non_camel_case_types)]
pub type renderer = BashPromptRenderer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_escape_markers_are_prompt_safe() {
        assert_eq!(BashPromptRenderer::escape_hl_start, "\\[");
        assert_eq!(BashPromptRenderer::escape_hl_end, "\\]");
    }

    #[test]
    fn bash_translations_escape_shell_specials() {
        let t = BashPromptRenderer::character_translations();
        assert_eq!(t.get(&'$'), Some(&"\\$"));
        assert_eq!(t.get(&'`'), Some(&"\\`"));
        assert_eq!(t.get(&'\\'), Some(&"\\\\"));
    }
}
