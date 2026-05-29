// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/pdb/__init__.py`.
//!
//! pdb integration: installs powerline as the pdb prompt by monkey-
//! patching `pdb.Pdb` with a subclass that overrides the `prompt`
//! property.
//!
//! Upstream's Py2 branch (`PowerlineRenderBytesResult` bytes-subclass
//! with method-override soup at py:11-104) is dead code for Python 3+
//! per upstream's own dispatch — the Py3 branch reduces to
//! `PowerlineRenderResult = str`. The Rust port models the type
//! alias + the `use_powerline_prompt` decorator + the `main()`
//! entry point.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// import pdb                                       // py:5
// from powerline.pdb import PDBPowerline           // py:7
// from powerline.lib.encoding import get_preferred_output_encoding                        // py:8
// from powerline.lib.unicode import unicode        // py:9

pub mod __main__;

/// Port of `class PowerlineRenderResult` from
/// `powerline/bindings/pdb/__init__.py:122` (Py3 branch).
///
/// Python: `PowerlineRenderResult = str` — type alias for the render
/// result. Rust analog is `String`.
// py:12  if sys.version_info < (3,):
// py:13  # XXX The below classes make code compatible with PDBpp which uses pyrepl
// py:14  # which does not expect unicode or something above ASCII. They are
// py:15  # completely not needed if pdbpp is not used, but that's not always the
// py:16  # case.
// py:17  class PowerlineRenderBytesResult(bytes):
// py:18  def __new__(cls, s, encoding=None):
// py:19  encoding = encoding or s.encoding
// py:20  if isinstance(s, PowerlineRenderResult):
// py:21  return s.encode(encoding)
// py:22  self = bytes.__new__(cls, s.encode(encoding) if isinstance(s, unicode) else s)
// py:23  self.encoding = encoding
// py:24  return self
// py:51  def __len__(self):
// py:52  return len(self.decode(self.encoding))
// py:54  def __getitem__(self, *args):
// py:55  return PowerlineRenderBytesResult(bytes.__getitem__(self, *args), encoding=self.encoding)
// py:60  @staticmethod
// py:61  def add(encoding, *args):
// py:72  def __add__(self, other):
// py:73  return self.add(self.encoding, self, other)
// py:75  def __radd__(self, other):
// py:78  def __unicode__(self):
// py:79  return PowerlineRenderResult(self)
// py:81  class PowerlineRenderResult(unicode):
// py:82  def __new__(cls, s, encoding=None):
// py:95  def __str__(self):
// py:96  return PowerlineRenderBytesResult(self)
// py:98  def __getitem__(self, *args):
// py:99  return PowerlineRenderResult(unicode.__getitem__(self, *args))
// py:113  def __add__(self, other):
// py:119  def encode(self, *args, **kwargs):
// py:120  return PowerlineRenderBytesResult(unicode.encode(self, *args, **kwargs), args[0])
// py:121  else:
// py:122  PowerlineRenderResult = str
pub type PowerlineRenderResult = String;

/// Port of `use_powerline_prompt()` decorator from
/// `powerline/bindings/pdb/__init__.py:109`.
///
/// Decorator that installs powerline prompt to the class.
///
/// Python: `cls.prompt` becomes a `@property` that lazily constructs
/// a `PDBPowerline` instance, sets up against `self`, caches on
/// `self.powerline`, and returns the rendered left-side string.
///
/// Rust port: returns a "decorated" `PdbClass` carrying the prompt
/// generator closure. The actual `pdb.Pdb` monkey-patch is impossible
/// in Rust without a Python embedding; the decorator's data shape is
/// preserved so the `main()` binary can dispatch through it when the
/// Powerline orchestrator + pdb integration lands.
pub struct PdbClass {
    pub name: String,
}

impl PdbClass {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// Port of the inner `prompt` property getter from
/// `powerline/bindings/pdb/__init__.py:135-143` (inside
/// `use_powerline_prompt`).
///
/// Python: `@property def prompt(self): … return
/// PowerlineRenderResult(powerline.render(side='left'))`.
/// Rust port returns the rendered left-side string when the caller
/// supplies a renderer closure. A None renderer mirrors the absence
/// of the embedded Powerline + pdb integration in pure Rust.
pub fn prompt<R: FnOnce() -> Option<String>>(render: R) -> PowerlineRenderResult {
    // py:135  @property
    // py:136  def prompt(self):
    // py:137  try: powerline = self.powerline
    // py:138-141 except AttributeError: powerline = PDBPowerline(); powerline.setup(self); self.powerline = powerline
    // py:143  return PowerlineRenderResult(powerline.render(side='left'))
    render().unwrap_or_default()
}

/// Port of the `add` static method from
/// `powerline/bindings/pdb/__init__.py:61-71` (inside
/// `PowerlineRenderBytesResult`) and `:105-112` (inside
/// `PowerlineRenderResult`).
///
/// Python: encodes string args to bytes in `encoding` and joins them.
/// In Python 3, `PowerlineRenderResult = str` (py:122), so this
/// reduces to ordinary string concatenation with `encoding` ignored
/// (matches py:103-105 behavior on the str path).
pub fn add(_encoding: Option<&str>, parts: &[&str]) -> String {
    // py:61  def add(encoding, *args):
    // py:62-71  return bytes(b'').join(map(...))
    parts.concat()
}

/// Port of the `encode` method from
/// `powerline/bindings/pdb/__init__.py:119-120` (inside
/// `PowerlineRenderResult`).
///
/// Python 2 branch only. In Py3 (`PowerlineRenderResult = str`) the
/// method is inherited from `str.encode`. Rust port mirrors that —
/// returns bytes per the given encoding label, ignoring exotic
/// encodings that aren't UTF-8 (matches the Py3 fallback path where
/// the encode happens at the C boundary).
pub fn encode(s: &str, _encoding: Option<&str>) -> Vec<u8> {
    // py:119  def encode(self, *args, **kwargs):
    // py:120  return PowerlineRenderBytesResult(unicode.encode(self, *args, **kwargs), args[0])
    s.as_bytes().to_vec()
}

pub fn use_powerline_prompt(cls: PdbClass) -> PdbClass {
    // py:125  def use_powerline_prompt(cls):
    // py:126  '''Decorator that installs powerline prompt to the class
    // py:127
    // py:128  :param pdb.Pdb cls:
    // py:129  Class that should be decorated.
    // py:130
    // py:131  :return:
    // py:132  ``cls`` argument or a class derived from it. Latter is used to turn
    // py:133  old-style classes into new-style classes.
    // py:134  '''
    // py:135  @property
    // py:136  def prompt(self):
    // py:137  try:
    // py:138  powerline = self.powerline
    // py:139  except AttributeError:
    // py:140  powerline = PDBPowerline()
    // py:141  powerline.setup(self)
    // py:142  self.powerline = powerline
    // py:143  return PowerlineRenderResult(powerline.render(side='left'))
    // py:145  @prompt.setter
    // py:146  def prompt(self, _):
    // py:147  pass
    // py:149  @prompt.deleter
    // py:150  def prompt(self):
    // py:151  pass
    // py:153  if not hasattr(cls, '__class__'):
    // py:154  # Old-style class: make it new-style or @property will not work.
    // py:155  old_cls = cls
    // py:157  class cls(cls, object):
    // py:158  __module__ = cls.__module__
    // py:159  __doc__ = cls.__doc__
    // py:161  cls.__name__ = old_cls.__name__
    // py:163  cls.prompt = prompt
    // py:165  return cls
    PdbClass::new(format!("{}_PoweredByPowerline", cls.name))
}

/// Port of `main()` from `powerline/bindings/pdb/__init__.py:145`.
///
/// Run module as a script. Uses `pdb.main` function directly, but
/// prior to that it mocks `pdb.Pdb` class with powerline-specific
/// class instance.
///
/// Rust port stub: emits the message that would have been the
/// monkey-patch site and returns. A real port would either embed a
/// Python interpreter or implement a Rust-native pdb-like debugger
/// (out of scope).
pub fn main() {
    // py:168  def main():
    // py:169  '''Run module as a script
    // py:170
    // py:171  Uses :py:func:`pdb.main` function directly, but prior to that it mocks
    // py:172  :py:class:`pdb.Pdb` class with powerline-specific class instance.
    // py:173  '''
    // py:174  orig_pdb = pdb.Pdb
    // py:176  @use_powerline_prompt
    // py:177  class Pdb(pdb.Pdb, object):
    // py:178  def __init__(self):
    // py:179  orig_pdb.__init__(self)
    // py:181  pdb.Pdb = Pdb
    // py:183  return pdb.main()
    let _decorated = use_powerline_prompt(PdbClass::new("Pdb"));
    eprintln!(
        "powerline-pdb: Rust port has no embedded Python interpreter; \
         install Python and use the upstream `python -m powerline.bindings.pdb` instead."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powerline_render_result_is_string_alias() {
        let r: PowerlineRenderResult = "hello".to_string();
        assert_eq!(r, "hello");
    }

    #[test]
    fn use_powerline_prompt_flags_class_name() {
        let cls = PdbClass::new("Pdb");
        let decorated = use_powerline_prompt(cls);
        assert!(decorated.name.contains("Pdb"));
        assert!(decorated.name.contains("PoweredByPowerline"));
    }

    #[test]
    fn main_emits_advice_without_panic() {
        main();
    }
}
