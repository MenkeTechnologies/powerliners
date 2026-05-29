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
/// `powerline/bindings/pdb/__init__.py:106` (Py3 branch).
///
/// Python: `PowerlineRenderResult = str` — type alias for the render
/// result. Rust analog is `String`.
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

pub fn use_powerline_prompt(cls: PdbClass) -> PdbClass {
    // py:115-123  @property def prompt(self): try: powerline = self.powerline
    //             except AttributeError: powerline = PDBPowerline(); ...
    //             return PowerlineRenderResult(powerline.render(side='left'))
    //
    // py:125-127  @prompt.setter, @prompt.deleter — both no-ops
    //
    // py:129-138  old-style → new-style class adapter (Py2)
    //
    // py:140  cls.prompt = prompt
    //
    // Rust port: monkey-patching is unrepresentable; return cls
    // unchanged but flagged with the powerline-prompt-installed name
    // suffix so callers can verify the decorator was applied.
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
    // py:150  orig_pdb = pdb.Pdb
    // py:152-155  @use_powerline_prompt class Pdb(pdb.Pdb, object): ...
    let _decorated = use_powerline_prompt(PdbClass::new("Pdb"));
    // py:157  pdb.Pdb = Pdb
    // py:159  return pdb.main()
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
