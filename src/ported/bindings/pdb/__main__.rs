// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/pdb/__main__.py`.
//!
//! Upstream source in full:
//!
//! ```python
//! #!/usr/bin/env python
//! # vim:fileencoding=utf-8:noet
//! from __future__ import (unicode_literals, division, absolute_import, print_function)
//!
//! from powerline.bindings.pdb import main
//!
//! if __name__ == '__main__':
//!     main()
//! ```
//!
//! Python convention: `__main__.py` is the entry point invoked by
//! `python -m powerline.bindings.pdb`. It delegates to `bindings/pdb/__init__.py:main`.
//!
//! **Status:** deferred — `crate::ported::bindings::pdb::main` lives in
//! `bindings/pdb/mod.rs` (currently a scaffold stub). When that module
//! is ported, the actual binary entry point will live in
//! `src/bin/powerliners-pdb.rs` and call into it; this file remains as
//! the 1:1 mirror of the Python `__main__.py` marker.
