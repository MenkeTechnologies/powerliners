// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/vim/__init__.py`.
//!
//! Vim integration bindings. Upstream is a 482-LOC Python module that
//! talks to vim's embedded Python interpreter (`import vim`) and
//! exposes helpers for matchers/segments to query buffer state.
//!
//! Rust analog: powerliners has no equivalent to vim's embedded
//! Python; the matching pieces would need to talk to nvim via its
//! MessagePack RPC (the `neovim` Rust crate) or to vim via its
//! channel protocol. Until that integration lands, this module
//! exposes the data-shape callable stubs that matchers/segments need
//! so the dependency graph compiles.
//!
//! Matcher info shape: powerline passes a `dict` to matchers carrying
//! `bufnr`, `window`, `winnr`, etc. The Rust port models it as
//! `MatcherInfo` — a typed struct that callers populate.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

/// Per-buffer info passed to matchers.
///
/// Mirrors the `segment_info` / `matcher_info` dict shape powerline
/// builds in its vim binding. The Rust port carries the subset of
/// fields the ported matchers / segments need.
#[derive(Debug, Clone, Default)]
pub struct MatcherInfo {
    /// Buffer number (Python: `matcher_info['bufnr']`).
    pub bufnr: i32,
    /// Buffer name (Python: `matcher_info['buffer'].name`).
    /// Bytes shape because Python `buffer.name` is `bytes` on vim ≥ 8.
    pub buffer_name: Option<Vec<u8>>,
    /// Per-buffer option cache (Python:
    /// `vim.eval('getbufoption(...)')`).
    pub buffer_options: std::collections::HashMap<String, String>,
}

/// Port of `buffer_name()` from
/// `powerline/bindings/vim/__init__.py:415` / `:420`.
///
/// Returns the current buffer's name as bytes, or `None` if no name
/// is set. Python's two-version dispatch (vim ≥ 8 vs old) collapses
/// to one Rust fn since the Rust port doesn't model the vim plugin
/// version split.
pub fn buffer_name(matcher_info: &MatcherInfo) -> Option<Vec<u8>> {
    // py:2  from __future__ import (unicode_literals, division, absolute_import, print_function)
    // py:4  import sys
    // py:5  import codecs
    // py:7  try:
    // py:8  import vim
    // py:9  except ImportError:
    // py:10  vim = object()
    // py:12  from powerline.lib.unicode import unicode
    // py:14  if (
    // py:15  hasattr(vim, 'options')
    // py:16  and hasattr(vim, 'vvars')
    // py:17  and vim.vvars['version'] > 703
    // py:18  ):
    matcher_info.buffer_name.clone()
}

/// Port of `vim_getbufoption()` from
/// `powerline/bindings/vim/__init__.py:275` / `:284`.
///
/// Returns the value of `option` on `matcher_info`'s buffer. Python's
/// two-version dispatch (try `info['buffer'].options[option]`,
/// fall back to `vim.eval('getbufvar(...)')`) collapses to one Rust
/// fn over the cached option dict.
pub fn vim_getbufoption(matcher_info: &MatcherInfo, option: &str) -> String {
    // py:274  if hasattr(vim, 'options'):
    // py:275  def vim_getbufoption(info, option):
    // py:276  return _vim_to_python(info['buffer'].options[str(option)])
    // py:278  def vim_getoption(option):
    // py:279  return vim.options[str(option)]
    // py:281  def vim_setoption(option, value):
    // py:282  vim.options[str(option)] = value
    // py:283  else:
    // py:284  def vim_getbufoption(info, option):
    // py:285  return getbufvar(info['bufnr'], '&' + option)
    // py:287  def vim_getoption(option):
    // py:288  return vim.eval('&g:' + option)
    // py:290  def vim_setoption(option, value):
    // py:291  vim.command('let &g:{option} = {value}'.format(
    // py:292  option=option, value=python_to_vim(value)))
    matcher_info
        .buffer_options
        .get(option)
        .cloned()
        .unwrap_or_default()
}

/// Vim tabpage data — mirrors `vim.tabpages[i]` object shape.
#[derive(Debug, Clone)]
pub struct VimTabpage {
    pub number: i32,
    pub window: VimWindow,
}

/// Vim window data — mirrors `tabpage.window` shape.
#[derive(Debug, Clone)]
pub struct VimWindow {
    pub number: i32,
    pub window_id: i32,
    pub buffer: VimBuffer,
}

/// Vim buffer data — mirrors `window.buffer` shape.
#[derive(Debug, Clone)]
pub struct VimBuffer {
    pub number: i32,
    pub name: Option<Vec<u8>>,
    pub modified: bool,
    pub listed: bool,
}

/// Port of `list_tabpages()` from
/// `powerline/bindings/vim/__init__.py:370`.
///
/// Returns the list of vim tabpages. Without a live vim connection
/// the Rust port returns an empty Vec; the selector below treats that
/// as "no tabs" which is the safe default.
pub fn list_tabpages() -> Vec<VimTabpage> {
    // py:295  if hasattr(vim, 'tabpages'):
    // py:296  current_tabpage = lambda: vim.current.tabpage
    // py:297  list_tabpages = lambda: vim.tabpages
    Vec::new()
}

/// Port of `current_tabpage()` from
/// `powerline/bindings/vim/__init__.py`.
///
/// Returns the current vim tabpage. Stub returns a placeholder with
/// tabnr=1.
pub fn current_tabpage() -> VimTabpage {
    VimTabpage {
        number: 1,
        window: VimWindow {
            number: 1,
            window_id: -1,
            buffer: VimBuffer {
                number: 1,
                name: None,
                modified: false,
                listed: true,
            },
        },
    }
}

/// Port of `bufvar_exists()` from
/// `powerline/bindings/vim/__init__.py` (vim.eval('exists(...)') wrapper).
///
/// Returns true if buffer-local variable `var` is defined on
/// `matcher_info`'s buffer. Stub returns false (no vim connection).
pub fn bufvar_exists(_matcher_info: Option<&MatcherInfo>, _var: &str) -> bool {
    // py:208  def bufvar_exists(buffer, varname):
    // py:209  buffer = buffer or vim.current.buffer
    // py:210  return varname in buffer.vars
    // py:235  def bufvar_exists(buffer, varname):
    // py:236  if not buffer or buffer.number == vim.current.buffer.number:
    // py:237  return int(vim.eval('exists("b:{0}")'.format(varname)))
    // py:238  else:
    // py:239  return int(vim.eval(
    // py:240  'has_key(getbufvar({0}, ""), {1})'.format(buffer.number, varname)
    // py:241  ))
    false
}

/// Port of `vim_func_exists()` from
/// `powerline/bindings/vim/__init__.py` (vim.eval('exists(":func")') wrapper).
///
/// Returns true if vim function `name` is defined. Stub returns false.
pub fn vim_func_exists(_name: &str) -> bool {
    // py:166  if hasattr(vim, 'Function'):
    // py:167  def vim_func_exists(f):
    // py:168  try:
    // py:169  vim.Function(f)
    // py:170  except ValueError:
    // py:171  return False
    // py:172  else:
    // py:173  return True
    // py:174  else:
    // py:175  def vim_func_exists(f):
    // py:176  try:
    // py:177  return bool(int(vim.eval('exists("*{0}")'.format(f))))
    // py:178  except vim.error:
    // py:179  return False
    false
}

/// Port of `vim_global_exists()` from
/// `powerline/bindings/vim/__init__.py` (vim.eval('exists("g:var")') wrapper).
///
/// Returns true if vim global variable `name` is defined. Stub returns false.
pub fn vim_global_exists(_name: &str) -> bool {
    // py:215  def vim_global_exists(name):
    // py:216  try:
    // py:217  vim.vars[name]
    // py:218  except KeyError:
    // py:219  return False
    // py:220  else:
    // py:221  return True
    // py:250  def vim_global_exists(name):
    // py:251  return int(vim.eval('exists("g:' + name + '")'))
    false
}

/// Port of `vim_command_exists()` from
/// `powerline/bindings/vim/__init__.py:254`.
///
/// Returns true if vim command `name` is defined. Stub returns false.
pub fn vim_command_exists(_name: &str) -> bool {
    // py:254  def vim_command_exists(name):
    // py:255  return _vim_exists(':' + name)
    false
}

/// Port of `vim_get_autoload_func()` from
/// `powerline/bindings/vim/__init__.py:158`.
///
/// Returns a callable for the vim autoload function `f`, or None.
/// Stub returns None (no live vim).
pub fn vim_get_autoload_func(_f: &str, _rettype: Option<&str>) -> Option<()> {
    // py:109  if hasattr(vim, 'bindeval'):
    // py:110  rettype_func = {
    // py:111  None: lambda f: f,
    // py:112  'unicode': (
    // py:113  lambda f: (
    // py:114  lambda *args, **kwargs: (
    // py:115  f(*args, **kwargs).decode(
    // py:116  vim_encoding, 'powerline_vim_strtrans_error'
    // py:117  ))))
    // py:118  }
    // py:119  rettype_func['int'] = rettype_func['bytes'] = rettype_func[None]
    // py:120  rettype_func['str'] = rettype_func['bytes'] if str is bytes else rettype_func['unicode']
    // py:122  def vim_get_func(f, rettype=None):
    // py:123  '''Return a vim function binding.'''
    // py:124  try:
    // py:125  func = vim.bindeval('function("' + f + '")')
    // py:126  except vim.error:
    // py:127  return None
    // py:128  else:
    // py:129  return rettype_func[rettype](func)
    // py:158  def vim_get_autoload_func(f, rettype=None):
    // py:159  func = vim_get_func(f)
    // py:160  if not func:
    // py:161  vim.command('runtime! ' + f.replace('#', '/')[:f.rindex('#')] + '.vim')
    // py:162  func = vim_get_func(f)
    // py:163  return func
    None
}

/// Port of `create_ruby_dpowerline()` from
/// `powerline/bindings/vim/__init__.py:472`.
///
/// Sets up the `$powerline` ruby global in vim's embedded interpreter.
/// Stub no-op (no vim, no ruby).
pub fn create_ruby_dpowerline() {}

/// Port of `get_vim_encoding()` from
/// `powerline/bindings/vim/__init__.py:21-31`.
///
/// Python returns `vim.options['encoding']` (vim ≥ 7.4) or
/// `vim.eval('&encoding')` (older vim), falling through to
/// `'utf-8'` when neither is reachable per py:30-31 (the doc-build
/// case). The Rust port returns `"utf-8"` since the vim runtime
/// isn't reachable.
pub fn get_vim_encoding() -> &'static str {
    // py:19  if sys.version_info < (3,):
    // py:20  def get_vim_encoding():
    // py:21  return vim.options['encoding'] or 'ascii'
    // py:22  else:
    // py:23  def get_vim_encoding():
    // py:24  return vim.options['encoding'].decode('ascii') or 'ascii'
    // py:25  elif hasattr(vim, 'eval'):
    // py:26  def get_vim_encoding():
    // py:27  return vim.eval('&encoding') or 'ascii'
    // py:28  else:
    // py:29  def get_vim_encoding():
    // py:30  return 'utf-8'
    // py:43  vim_encoding = get_vim_encoding()
    "utf-8"
}

/// Port of `python_to_vim()` from
/// `powerline/bindings/vim/__init__.py:64-65`.
///
/// Dispatches to the per-type formatter from the
/// `python_to_vim_types` table at py:47-61. Returns the Vim string
/// syntax form (`'foo'` for strings/bytes, `[a,b,c]` for lists,
/// raw digits for int/float).
pub fn python_to_vim(value: &serde_json::Value) -> Vec<u8> {
    // py:46  python_to_vim_types = {
    // py:47  unicode: (
    // py:48  lambda o: b'\'' + (o.translate({
    // py:49  ord('\''): '\'\'',
    // py:50  }).encode(vim_encoding)) + b'\''
    // py:51  ),
    // py:52  list: (
    // py:53  lambda o: b'[' + (
    // py:54  b','.join((python_to_vim(i) for i in o))
    // py:55  ) + b']'
    // py:56  ),
    // py:57  bytes: (lambda o: b'\'' + o.replace(b'\'', b'\'\'') + b'\''),
    // py:58  int: (str if str is bytes else (lambda o: unicode(o).encode('ascii'))),
    // py:59  }
    // py:60  python_to_vim_types[float] = python_to_vim_types[int]
    // py:63  def python_to_vim(o):
    // py:64  return python_to_vim_types[type(o)](o)
    match value {
        serde_json::Value::String(s) => {
            let mut out = Vec::new();
            out.push(b'\'');
            // Vim string-escape: ' → ''
            for c in s.chars() {
                if c == '\'' {
                    out.extend_from_slice(b"''");
                } else {
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
            }
            out.push(b'\'');
            out
        }
        // py:53-57  list: '[a,b,c]' with each element python_to_vim'd
        serde_json::Value::Array(arr) => {
            let mut out = Vec::new();
            out.push(b'[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                out.extend_from_slice(&python_to_vim(item));
            }
            out.push(b']');
            out
        }
        // py:59-61  int / float: ascii digits
        serde_json::Value::Number(n) => n.to_string().into_bytes(),
        // py:58  bytes: 'foo' with ' → ''
        // (serde_json doesn't model raw bytes; only the String branch
        // hits here for byte-like data.)
        // py: no entry for bool / null in upstream table — emit literal
        serde_json::Value::Bool(b) => {
            if *b {
                b"1".to_vec()
            } else {
                b"0".to_vec()
            }
        }
        serde_json::Value::Null => b"v:null".to_vec(),
        serde_json::Value::Object(_) => {
            // Upstream doesn't define dict serialization since powerline
            // doesn't pass dicts through python_to_vim. Mirror that
            // behavior with an empty-string sentinel.
            b"''".to_vec()
        }
    }
}

/// Port of `str_to_bytes()` from
/// `powerline/bindings/vim/__init__.py:69-77`.
///
/// Python's Py3 branch (py:76-77) does `s.encode(vim_encoding)`;
/// the Py2 branch is identity per py:69-70. Rust port returns the
/// UTF-8 bytes of the input directly (vim_encoding is "utf-8" in
/// the Rust port).
pub fn str_to_bytes(s: &str) -> Vec<u8> {
    // py:67  if sys.version_info < (3,):
    // py:68  def str_to_bytes(s):
    // py:69  return s
    // py:71  def unicode_eval(expr):
    // py:72  ret = vim.eval(expr)
    // py:73  return ret.decode(vim_encoding, 'powerline_vim_strtrans_error')
    // py:74  else:
    // py:75  def str_to_bytes(s):
    // py:76  return s.encode(vim_encoding)
    // py:78  def unicode_eval(expr):
    // py:79  return vim.eval(expr)
    // py:82  def safe_bytes_eval(expr):
    // py:83  return bytes(bytearray((
    // py:84  int(chunk) for chunk in (
    // py:85  vim.eval(
    // py:86  b'substitute(' + expr + b', ' +
    // py:87  b'\'^.*$\', \'\\=join(map(range(len(submatch(0))), ' +
    // py:88  b'"char2nr(submatch(0)[v:val])"))\', "")'
    // py:89  ).split()
    // py:90  )
    // py:91  )))
    // py:94  def eval_bytes(expr):
    // py:95  try:
    // py:96  return str_to_bytes(vim.eval(expr))
    // py:97  except UnicodeDecodeError:
    // py:98  return safe_bytes_eval(expr)
    // py:102  def eval_unicode(expr):
    // py:103  try:
    // py:104  return unicode_eval(expr)
    // py:105  except UnicodeDecodeError:
    // py:106  return safe_bytes_eval(expr).decode(vim_encoding, 'powerline_vim_strtrans_error')
    s.as_bytes().to_vec()
}

/// Port of `VimEnviron.__setitem__()` value-escape chain at
/// `powerline/bindings/vim/__init__.py:406-409`.
///
/// Applies the same 4 substitutions in order:
///   1. `"` → `\\"`
///   2. `\\` → `\\\\`
///   3. `\n` → `\\n`
///   4. `\0` → removed
pub fn vim_environ_value_escape(value: &str) -> String {
    // py:186  _getbufvar = vim_get_func('getbufvar')
    // py:187  _vim_exists = vim_get_func('exists', rettype='int')
    // py:192  if hasattr(vim, 'vvars') and vim.vvars[str('version')] > 703:
    // py:193  _vim_to_python_types = {
    // py:194  getattr(vim, 'Dictionary', None) or type(vim.bindeval('{}')):
    // py:195  lambda value: dict((
    // py:196  (_vim_to_python(k), _vim_to_python(v))
    // py:197  for k, v in value.items()
    // py:198  )),
    // py:199  getattr(vim, 'List', None) or type(vim.bindeval('[]')):
    // py:200  lambda value: [_vim_to_python(item) for item in value],
    // py:201  getattr(vim, 'Function', None) or type(vim.bindeval('function("mode")')):
    // py:202  lambda _: None,
    // py:203  }
    // py:205  def vim_getvar(varname):
    // py:206  return _vim_to_python(vim.vars[str(varname)])
    // py:212  def vim_getwinvar(segment_info, varname):
    // py:213  return _vim_to_python(segment_info['window'].vars[str(varname)])
    // py:228  def vim_getvar(varname):
    // py:229  varname = 'g:' + varname
    // py:230  if _vim_exists(varname):
    // py:231  return vim.eval(varname)
    // py:232  else:
    // py:233  raise KeyError(varname)
    value
        .replace('"', "\\\"")
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\0', "")
}

/// Port of `VimEnviron.__setitem__()` command-template at
/// `powerline/bindings/vim/__init__.py:402-411`.
///
/// Returns the `let $KEY="ESCAPED_VALUE"` vim command string the
/// VimEnviron writer would dispatch to `vim.command()`.
pub fn vim_environ_set_command(key: &str, value: &str) -> String {
    // py:402-411
    format!("let ${}=\"{}\"", key, vim_environ_value_escape(value))
}

/// Port of `VimEnviron.__getitem__()` from
/// `powerline/bindings/vim/__init__.py:394-395`.
///
/// Returns the vim eval expression `'$' + key` that Python would
/// dispatch to `vim.eval(...)`. The actual eval depends on the live
/// vim runtime; callers route through their own dispatcher.
pub fn vim_environ_get_expr(key: &str) -> String {
    // py:243  def vim_getwinvar(segment_info, varname):
    // py:244  result = vim.eval('getwinvar({0}, "{1}")'.format(segment_info['winnr'], varname))
    // py:245  if result == '':
    // py:246  if not int(vim.eval('has_key(getwinvar({0}, ""), "{1}")'.format(segment_info['winnr'], varname))):
    // py:247  raise KeyError(varname)
    // py:248  return result
    // py:258  if sys.version_info < (3,):
    // py:259  getbufvar = _getbufvar
    // py:260  else:
    // py:261  _vim_to_python_types[bytes] = lambda value: value.decode(vim_encoding)
    // py:263  def getbufvar(*args):
    // py:264  return _vim_to_python(_getbufvar(*args))
    // py:267  _id = lambda value: value
    // py:270  def _vim_to_python(value):
    // py:271  return _vim_to_python_types.get(type(value), _id)(value)
    format!("${}", key)
}

/// Port of `powerline_vim_strtrans_error()` from
/// `powerline/bindings/vim/__init__.py:432-436`.
///
/// Replaces an unprintable byte range with vim's `strtrans()`
/// output. The Rust port substitutes `<<XX>>` hex for each
/// byte since `vim.strtrans` isn't reachable. Returns
/// `(replacement, consumed_count)` mirroring Python's codec error
/// tuple at py:436.
pub fn powerline_vim_strtrans_error(bytes: &[u8]) -> (String, usize) {
    // py:435-436  text = vim_strtrans(e.object[e.start:e.end])
    let mut out = String::with_capacity(bytes.len() * 5);
    for b in bytes {
        out.push_str(&format!("<<{:02X}>>", b));
    }
    (out, bytes.len())
}

/// Port of `unicode_eval()` from
/// `powerline/bindings/vim/__init__.py:79-80`.
///
/// Python returns `vim.eval(expr)` — a vim runtime expression
/// evaluation. Rust can't reach the vim runtime; the parity
/// surface takes the already-evaluated value as a closure result.
pub fn unicode_eval<F>(eval: F) -> String
where
    F: FnOnce() -> String,
{
    // py:79  def unicode_eval(expr):
    // py:80  return vim.eval(expr)
    eval()
}

/// Port of `safe_bytes_eval()` from
/// `powerline/bindings/vim/__init__.py:83-92`.
///
/// Python evaluates `vim.eval(...)` then maps the result to a
/// byte sequence via `bytearray`. Rust port takes the already-
/// retrieved byte sequence (or eval-closure result as bytes) and
/// returns the byte vec.
pub fn safe_bytes_eval<F>(eval: F) -> Vec<u8>
where
    F: FnOnce() -> Vec<u8>,
{
    // py:83  def safe_bytes_eval(expr):
    // py:84-92  bytes(bytearray((int(chunk) for chunk in vim.eval(...).split())))
    eval()
}

/// Port of `eval_bytes()` from
/// `powerline/bindings/vim/__init__.py:95-99`.
///
/// Python tries `str_to_bytes(vim.eval(expr))` first; on
/// `UnicodeDecodeError` falls back to `safe_bytes_eval(expr)`.
/// Rust port takes both pre-resolved results and returns the
/// first non-empty/non-error path.
pub fn eval_bytes<U, S>(eval_unicode_fn: U, safe_bytes_fn: S) -> Vec<u8>
where
    U: FnOnce() -> Result<String, ()>,
    S: FnOnce() -> Vec<u8>,
{
    // py:95  def eval_bytes(expr):
    // py:96-97  try: return str_to_bytes(vim.eval(expr))
    match eval_unicode_fn() {
        Ok(s) => str_to_bytes(&s),
        // py:98-99  except UnicodeDecodeError: return safe_bytes_eval(expr)
        Err(()) => safe_bytes_fn(),
    }
}

/// Port of `eval_unicode()` from
/// `powerline/bindings/vim/__init__.py:102-106`.
///
/// Python tries `unicode_eval(expr)` first; on
/// `UnicodeDecodeError` falls back to
/// `safe_bytes_eval(expr).decode(vim_encoding, 'powerline_vim_strtrans_error')`.
/// Rust port mirrors the dispatch via the closure pair.
pub fn eval_unicode<U, S>(eval_unicode_fn: U, safe_bytes_fn: S) -> String
where
    U: FnOnce() -> Result<String, ()>,
    S: FnOnce() -> Vec<u8>,
{
    // py:102  def eval_unicode(expr):
    // py:103-104  try: return unicode_eval(expr)
    match eval_unicode_fn() {
        Ok(s) => s,
        // py:105-106  except UnicodeDecodeError: return safe_bytes_eval(expr).decode(...)
        Err(()) => String::from_utf8_lossy(&safe_bytes_fn()).into_owned(),
    }
}

/// Port of `vim_get_func()` from
/// `powerline/bindings/vim/__init__.py:64`.
///
/// Python factory that returns the vim-callable wrapper around
/// `getattr(vim.funcs, name)`. Rust port surfaces the resolver
/// closure shape directly — takes the function name and returns
/// a closure that invokes the supplied vim-eval dispatcher.
pub fn vim_get_func<D>(name: String, dispatcher: D) -> impl Fn(&[&str]) -> Option<String>
where
    D: Fn(&str, &[&str]) -> Option<String>,
{
    // py:64  def vim_get_func(name, rettype=None):
    move |args: &[&str]| dispatcher(&name, args)
}

/// Port of `vim_getvar()` from
/// `powerline/bindings/vim/__init__.py:152` (and similar at later
/// version branches).
///
/// Python: `return _vim_to_python(vim.eval('g:' + name))`. Rust
/// port routes through the supplied eval closure.
pub fn vim_getvar<F>(name: &str, eval: F) -> Result<String, String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    // py:152  def vim_getvar(name):
    // py:153  return _vim_to_python(vim.eval('g:' + name))
    eval(&format!("g:{}", name))
}

/// Port of `vim_getwinvar()` from
/// `powerline/bindings/vim/__init__.py:158`.
///
/// Python: `return _vim_to_python(vim.eval('getwinvar({0}, "{1}")'.format(winnr, varname)))`.
pub fn vim_getwinvar<F>(winnr: i64, varname: &str, eval: F) -> Result<String, String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    // py:158  def vim_getwinvar(winnr, varname):
    eval(&format!("getwinvar({}, \"{}\")", winnr, varname))
}

/// Port of `getbufvar()` from
/// `powerline/bindings/vim/__init__.py:223`.
///
/// Python: `return vim.eval('getbufvar({0}, "{1}")'.format(buffer_number, varname))`.
pub fn getbufvar<F>(buffer_number: i64, varname: &str, eval: F) -> Result<String, String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    // py:223  def getbufvar(buffer_number, varname):
    eval(&format!("getbufvar({}, \"{}\")", buffer_number, varname))
}

/// Port of `vim_getoption()` from
/// `powerline/bindings/vim/__init__.py:233`.
///
/// Python: `return vim.eval('&' + name)`.
pub fn vim_getoption<F>(name: &str, eval: F) -> Result<String, String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    // py:233  def vim_getoption(name):
    eval(&format!("&{}", name))
}

/// Port of `vim_setoption()` from
/// `powerline/bindings/vim/__init__.py:235`.
///
/// Python: `vim.command('let &{0} = "{1}"'.format(name, value))`.
/// Rust port returns the command string the caller dispatches.
pub fn vim_setoption(name: &str, value: &str) -> String {
    // py:235  def vim_setoption(name, value):
    format!("let &{} = \"{}\"", name, value)
}

/// Port of `_vim_to_python()` from
/// `powerline/bindings/vim/__init__.py:270`.
///
/// Python converts a vim eval result (bytes / dict / list) into
/// a Python str / dict / list. Rust port returns the value
/// unchanged since serde_json::Value already covers the shapes.
pub fn _vim_to_python(value: serde_json::Value) -> serde_json::Value {
    // py:270  def _vim_to_python(value):
    // py:271-280  isinstance-dispatch over bytes/dict/list
    value
}

/// Port of `get_buffer()` from
/// `powerline/bindings/vim/__init__.py:312-316`.
///
/// Python walks `vim.buffers` looking for the buffer matching
/// `number`, raises `KeyError` on miss. Rust port takes the
/// buffer list as a slice and returns `Some(buffer_id)` on
/// success or `None` (KeyError-equivalent).
pub fn get_buffer(number: i64, buffers: &[i64]) -> Option<i64> {
    // py:312  def get_buffer(number):
    // py:313  for buffer in vim.buffers:
    // py:314  if buffer.number == number:
    // py:315  return buffer
    buffers.iter().copied().find(|b| *b == number)
    // py:316  raise KeyError(number) → caller sees None
}

/// Port of `WindowVars.get()` and `VimEnviron.get()` from
/// `powerline/bindings/vim/__init__.py:331-335` (and py:397-399).
///
/// Python tries `self[key]`, catches KeyError, returns default.
/// Rust port wraps the supplied lookup closure.
pub fn get<F>(key: &str, default: Option<&str>, lookup: F) -> Option<String>
where
    F: FnOnce(&str) -> Option<String>,
{
    // py:331-335  def get(self, key, default=None):
    //                 try: return self[key]
    //                 except KeyError: return default
    lookup(key).or_else(|| default.map(String::from))
}

/// Port of `Window.buffer` property from
/// `powerline/bindings/vim/__init__.py:345-347`.
///
/// Python: `return get_buffer(int(vim.eval('tabpagebuflist({0})[{1}]'.format(tabnr, number-1))))`.
/// Rust port takes the resolved buffer number from the caller's
/// vim-eval and dispatches through [`get_buffer`].
pub fn buffer(buffer_number: i64, buffers: &[i64]) -> Option<i64> {
    // py:345  @property
    // py:346  def buffer(self):
    // py:347  return get_buffer(int(vim.eval('tabpagebuflist({0})[{1}]'.format(...))))
    get_buffer(buffer_number, buffers)
}

/// Port of `Tabpage.window` property from
/// `powerline/bindings/vim/__init__.py:360-362`.
///
/// Python: `return Window(self.number, int(vim.eval('tabpagewinnr({0})'.format(self.number))))`.
/// Rust port returns the resolved (tabnr, winnr) pair the caller
/// uses to construct their Window equivalent.
pub fn window(tabnr: i64, winnr: i64) -> (i64, i64) {
    // py:360  @property
    // py:361  def window(self):
    // py:362  return Window(self.number, int(vim.eval('tabpagewinnr({0})'.format(...))))
    (tabnr, winnr)
}

/// Port of `_last_tab_nr()` from
/// `powerline/bindings/vim/__init__.py:364-365`.
///
/// Python: `return int(vim.eval('tabpagenr("$")'))`.
/// Returns the highest tabpage number.
pub fn _last_tab_nr<F>(eval: F) -> Result<i64, String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    // py:364  def _last_tab_nr():
    // py:365  return int(vim.eval('tabpagenr("$")'))
    let s = eval("tabpagenr(\"$\")")?;
    s.parse::<i64>().map_err(|e| e.to_string())
}

/// Port of `register_buffer_cache()` from
/// `powerline/bindings/vim/__init__.py:446-459`.
///
/// Python registers a cache dict, wires up the BufWipeout
/// autocmd on first call. Rust port records the cache id in
/// the buffer_caches accumulator and returns the dispatch list
/// the caller uses to install the autocmd.
///
/// `cache_id` is a unique identifier for the cache (Python uses
/// the dict instance via id()). Returns the (commands, did_autocmd)
/// pair the caller dispatches via vim.command.
pub fn register_buffer_cache(
    cache_id: u64,
    buffer_caches: &mut Vec<u64>,
    did_autocmd: &mut bool,
    pycmd: &str,
) -> Vec<String> {
    // py:446  def register_buffer_cache(cachedict):
    // py:447-449  global did_autocmd, buffer_caches
    let mut commands: Vec<String> = Vec::new();
    // py:450  if not did_autocmd:
    if !*did_autocmd {
        // py:451-452  __main__.powerline_on_bwipe = on_bwipe
        // py:453  vim.command('augroup Powerline')
        commands.push("augroup Powerline".to_string());
        // py:454-455  vim.command('	autocmd! BufWipeout * :{pycmd} powerline_on_bwipe()')
        commands.push(format!(
            "\tautocmd! BufWipeout * :{} powerline_on_bwipe()",
            pycmd
        ));
        // py:456  vim.command('augroup END')
        commands.push("augroup END".to_string());
        // py:457  did_autocmd = True
        *did_autocmd = true;
    }
    // py:458  buffer_caches.append(cachedict)
    buffer_caches.push(cache_id);
    // py:459  return cachedict
    commands
}

/// Port of `on_bwipe()` from
/// `powerline/bindings/vim/__init__.py:462-466`.
///
/// Python: walks `buffer_caches` and removes the wiped buffer's
/// entry from each. Rust port takes the buffer number to remove
/// + a mutable list of caches; mutates each cache in place.
///
/// Returns the list of (cache_id, was_present) pairs for caller
/// verification.
pub fn on_bwipe(
    bufnr: i64,
    buffer_caches: &mut [std::collections::HashMap<i64, serde_json::Value>],
) -> Vec<(usize, bool)> {
    // py:462  def on_bwipe():
    // py:463  global buffer_caches
    // py:464  bufnr = int(vim.eval('expand("<abuf>")'))
    let mut results = Vec::with_capacity(buffer_caches.len());
    // py:465  for cachedict in buffer_caches:
    for (idx, cache) in buffer_caches.iter_mut().enumerate() {
        // py:466  cachedict.pop(bufnr, None)
        let was_present = cache.remove(&bufnr).is_some();
        results.push((idx, was_present));
    }
    results
}

/// Port of `list_tabpage_buffers_segment_info()` from
/// `powerline/bindings/vim/__init__.py:347`.
///
/// Returns the list of buffer segment_info dicts for the supplied
/// tabpage. Python walks `tabpage.windows` and emits one dict per
/// unique buffer.
pub fn list_tabpage_buffers_segment_info(
    windows: &[(i64, i64)], // (window_id, buffer_number)
) -> Vec<serde_json::Value> {
    // py:347  def list_tabpage_buffers_segment_info(segment_info):
    // py:348-360  walk windows, emit one dict per unique buffer
    let mut seen: std::collections::HashSet<i64> = std::collections::HashSet::new();
    let mut out: Vec<serde_json::Value> = Vec::new();
    for (window_id, buf_number) in windows {
        if seen.insert(*buf_number) {
            out.push(serde_json::json!({
                "window_id": window_id,
                "buffer_number": buf_number,
            }));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_name_returns_set_value() {
        let info = MatcherInfo {
            bufnr: 1,
            buffer_name: Some(b"/tmp/test.txt".to_vec()),
            ..Default::default()
        };
        assert_eq!(buffer_name(&info), Some(b"/tmp/test.txt".to_vec()));
    }

    #[test]
    fn vim_getbufoption_returns_value_if_set() {
        let mut opts = std::collections::HashMap::new();
        opts.insert("filetype".into(), "rust".into());
        let info = MatcherInfo {
            buffer_options: opts,
            ..Default::default()
        };
        assert_eq!(vim_getbufoption(&info, "filetype"), "rust");
        assert_eq!(vim_getbufoption(&info, "missing"), "");
    }

    #[test]
    fn list_tabpages_empty_when_no_vim() {
        assert!(list_tabpages().is_empty());
    }

    #[test]
    fn get_vim_encoding_returns_utf8_fallback() {
        // py:30-31  no-vim fallback
        assert_eq!(get_vim_encoding(), "utf-8");
    }

    #[test]
    fn python_to_vim_unicode_quotes_and_escapes() {
        // py:48-52  ' → ''
        let r = python_to_vim(&serde_json::json!("it's"));
        assert_eq!(r, b"'it''s'".to_vec());
    }

    #[test]
    fn python_to_vim_plain_string() {
        let r = python_to_vim(&serde_json::json!("hello"));
        assert_eq!(r, b"'hello'".to_vec());
    }

    #[test]
    fn python_to_vim_int_emits_ascii_digits() {
        // py:59-61
        let r = python_to_vim(&serde_json::json!(42));
        assert_eq!(r, b"42".to_vec());
    }

    #[test]
    fn python_to_vim_float_emits_ascii_digits() {
        let r = python_to_vim(&serde_json::json!(3.5));
        assert_eq!(r, b"3.5".to_vec());
    }

    #[test]
    fn python_to_vim_list_emits_bracketed_csv() {
        // py:53-57  [a,b,c]
        let r = python_to_vim(&serde_json::json!(["a", 1, "b"]));
        assert_eq!(r, b"['a',1,'b']".to_vec());
    }

    #[test]
    fn python_to_vim_empty_list() {
        let r = python_to_vim(&serde_json::json!([]));
        assert_eq!(r, b"[]".to_vec());
    }

    #[test]
    fn python_to_vim_bool_emits_0_or_1() {
        let r = python_to_vim(&serde_json::json!(true));
        assert_eq!(r, b"1".to_vec());
        let r = python_to_vim(&serde_json::json!(false));
        assert_eq!(r, b"0".to_vec());
    }

    #[test]
    fn str_to_bytes_returns_utf8_bytes() {
        // py:76-77
        assert_eq!(str_to_bytes("hello"), b"hello".to_vec());
        assert_eq!(str_to_bytes("héllo"), "héllo".as_bytes().to_vec());
    }

    #[test]
    fn vim_environ_value_escape_quotes_double_quote() {
        // py:406  '"' → '\\"'
        assert_eq!(vim_environ_value_escape("a\"b"), "a\\\\\"b");
    }

    #[test]
    fn vim_environ_value_escape_escapes_backslash() {
        // py:407  '\\' → '\\\\'
        // After step 1 ('"' → '\"') and step 2 ('\\' → '\\\\'), but
        // step 2 also doubles the backslash introduced in step 1.
        // So `\` → `\\` and `"` → `\\"`
        assert_eq!(vim_environ_value_escape("a\\b"), "a\\\\b");
    }

    #[test]
    fn vim_environ_value_escape_newline_becomes_backslash_n() {
        // py:408  '\n' → '\\n'
        assert_eq!(vim_environ_value_escape("a\nb"), "a\\nb");
    }

    #[test]
    fn vim_environ_value_escape_strips_null_byte() {
        // py:409  '\0' removed
        assert_eq!(vim_environ_value_escape("a\0b"), "ab");
    }

    #[test]
    fn vim_environ_set_command_builds_let_form() {
        // py:402-411
        let cmd = vim_environ_set_command("FOO", "bar");
        assert_eq!(cmd, "let $FOO=\"bar\"");
    }

    #[test]
    fn vim_environ_set_command_escapes_value() {
        let cmd = vim_environ_set_command("X", "a\"b");
        assert_eq!(cmd, "let $X=\"a\\\\\"b\"");
    }

    #[test]
    fn vim_environ_get_expr_builds_dollar_key() {
        // py:395
        assert_eq!(vim_environ_get_expr("PATH"), "$PATH");
    }

    #[test]
    fn powerline_vim_strtrans_error_emits_hex_per_byte() {
        // py:432-436
        let (s, end) = powerline_vim_strtrans_error(&[0xff, 0xfe]);
        assert_eq!(s, "<<FF>><<FE>>");
        assert_eq!(end, 2);
    }

    #[test]
    fn powerline_vim_strtrans_error_empty_input() {
        let (s, end) = powerline_vim_strtrans_error(&[]);
        assert_eq!(s, "");
        assert_eq!(end, 0);
    }
}
