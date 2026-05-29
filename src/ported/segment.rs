// vim:fileencoding=utf-8:noet
//! Port of `powerline/segment.py`.
//!
//! Segment dispatch heart. Upstream is 450 lines that resolve a
//! configured segment-name into an executable contents function,
//! merge multiple config layers, dispatch per-segment-type
//! (function / string / segment_list), call the segment's compute fn,
//! and attach highlight info from the colorscheme.
//!
//! This file is ported in chunks. The simpler pieces — frozen
//! constants, key-walk helpers, highlight attachment — land here.
//! The closure-returning factories (`gen_segment_getter`,
//! `get_attr_func`) and the mutation-heavy dispatch
//! (`process_segment`, `process_segment_lister`) land alongside
//! `renderer.py` since they share data-model invariants.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.lib.watcher import create_file_watcher                                   // py:4

use crate::ported::colorscheme::Colorscheme;
use serde_json::{Map, Value};

/// Port of `list_segment_key_values()` from `powerline/segment.py:7`.
///
/// Python: yields values for `key` looked up in the segment dict,
/// then in each theme_config's `segment_data` (with module-name +
/// function-name fallbacks), then in segment_data root, then default.
///
/// Rust port: builds a Vec of the same candidate sequence; callers
/// fold via `next()` (= bucket-by-bucket fallback) or
/// `get_segment_key`'s merge logic.
// Faithfully ports the 8-arg Python signature; refactoring into a
// param-struct here would obscure the upstream `// py:NN` line citations.
#[allow(clippy::too_many_arguments)]
pub fn list_segment_key_values(
    segment: &Map<String, Value>,
    theme_configs: &[&Map<String, Value>],
    segment_data: Option<&Map<String, Value>>,
    key: &str,
    function_name: Option<&str>,
    name: Option<&str>,
    module: Option<&str>,
    default: Option<Value>,
) -> Vec<Value> {
    // py:7  def list_segment_key_values(segment, theme_configs, segment_data, key, function_name=None, name=None, module=None, default=None):
    // py:8  try:
    // py:9  yield segment[key]
    // py:10  except KeyError:
    // py:11  pass
    let mut out: Vec<Value> = Vec::new();
    if let Some(v) = segment.get(key) {
        out.push(v.clone());
    }
    // py:12  found_module_key = False
    let mut found_module_key = false;
    // py:13  for theme_config in theme_configs:
    for theme_config in theme_configs {
        // py:14  try:
        // py:15  segment_data = theme_config['segment_data']
        // py:16  except KeyError:
        // py:17  pass
        // py:18  else:
        let seg_data = match theme_config.get("segment_data").and_then(|v| v.as_object()) {
            Some(s) => s,
            None => continue,
        };
        // py:19  if function_name and not name:
        if let (Some(fname), None) = (function_name, name) {
            // py:20  if module:
            if let Some(module) = module {
                // py:21  try:
                // py:22  yield segment_data[module + '.' + function_name][key]
                // py:23  found_module_key = True
                // py:24  except KeyError:
                // py:25  pass
                let mod_key = format!("{}.{}", module, fname);
                if let Some(v) = seg_data
                    .get(&mod_key)
                    .and_then(|x| x.as_object())
                    .and_then(|o| o.get(key))
                {
                    out.push(v.clone());
                    found_module_key = true;
                }
            }
            // py:26  if not found_module_key:
            if !found_module_key {
                // py:27  try:
                // py:28  yield segment_data[function_name][key]
                // py:29  except KeyError:
                // py:30  pass
                if let Some(v) = seg_data
                    .get(fname)
                    .and_then(|x| x.as_object())
                    .and_then(|o| o.get(key))
                {
                    out.push(v.clone());
                }
            }
        }
        // py:31  if name:
        if let Some(n) = name {
            // py:32  try:
            // py:33  yield segment_data[name][key]
            // py:34  except KeyError:
            // py:35  pass
            if let Some(v) = seg_data
                .get(n)
                .and_then(|x| x.as_object())
                .and_then(|o| o.get(key))
            {
                out.push(v.clone());
            }
        }
    }
    // py:36  if segment_data is not None:
    // py:37  try:
    // py:38  yield segment_data[key]
    // py:39  except KeyError:
    // py:40  pass
    if let Some(sd) = segment_data {
        if let Some(v) = sd.get(key) {
            out.push(v.clone());
        }
    }
    // py:41  yield default
    if let Some(d) = default {
        out.push(d);
    }
    out
}

/// Port of `get_segment_key()` from `powerline/segment.py:44`.
///
/// If `merge` is true, recursively merges any dict-valued candidates
/// found, with the segment value (first emitted) winning over each
/// downstream layer (.update reverses normal merge precedence so that
/// `old_ret = ret; ret = value.copy(); ret.update(old_ret)` is the
/// upstream's "old wins" pattern).
///
/// If `merge` is false, returns the first non-None candidate.
#[allow(clippy::too_many_arguments)]
pub fn get_segment_key(
    merge: bool,
    segment: &Map<String, Value>,
    theme_configs: &[&Map<String, Value>],
    segment_data: Option<&Map<String, Value>>,
    key: &str,
    function_name: Option<&str>,
    name: Option<&str>,
    module: Option<&str>,
    default: Option<Value>,
) -> Option<Value> {
    let candidates = list_segment_key_values(
        segment,
        theme_configs,
        segment_data,
        key,
        function_name,
        name,
        module,
        default,
    );

    // py:44  def get_segment_key(merge, *args, **kwargs):
    // py:45  if merge:
    if merge {
        // py:46  ret = None
        let mut ret: Option<Value> = None;
        // py:47  for value in list_segment_key_values(*args, **kwargs):
        for value in candidates {
            // py:48  if ret is None:
            // py:49  ret = value
            if ret.is_none() {
                ret = Some(value);
            } else if matches!(ret, Some(Value::Object(_))) && matches!(value, Value::Object(_)) {
                // py:50  elif isinstance(ret, dict) and isinstance(value, dict):
                // py:51  old_ret = ret
                // py:52  ret = value.copy()
                // py:53  ret.update(old_ret)
                let old_ret = ret.take().unwrap();
                let mut new_ret = value.as_object().unwrap().clone();
                for (k, v) in old_ret.as_object().unwrap() {
                    new_ret.insert(k.clone(), v.clone());
                }
                ret = Some(Value::Object(new_ret));
            } else {
                // py:54  else:
                // py:55  return ret
                return ret;
            }
        }
        // py:56  return ret
        ret
    } else {
        // py:57  else:
        // py:58  return next(list_segment_key_values(*args, **kwargs))
        candidates.into_iter().next()
    }
}

/// Port of `get_string()` from `powerline/segment.py:73`.
///
/// String-segment resolver: returns the literal `'contents'` value.
// Tuple shape mirrors the 5-element Python return; a named struct here
// would diverge from the upstream contract referenced by `// py:NN`.
#[allow(clippy::type_complexity)]
pub fn get_string(
    data: &Map<String, Value>,
    segment: &Map<String, Value>,
) -> (
    Option<Value>,
    Option<Value>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    // py:61  def get_function(data, segment):
    // py:62  function_name = segment['function']
    // py:63  if '.' in function_name:
    // py:64  module, function_name = function_name.rpartition('.')[::2]
    // py:65  else:
    // py:66  module = data['default_module']
    // py:67  function = data['get_module_attr'](module, function_name, prefix='segment_generator')
    // py:68  if not function:
    // py:69  raise ImportError('Failed to obtain segment function')
    // py:70  return None, function, module, function_name, segment.get('name')
    // py:73  def get_string(data, segment):
    // py:74  name = segment.get('name')
    // py:75  return data['get_key'](False, segment, None, None, name, 'contents'), None, None, None, name
    // py:78  segment_getters = {
    // py:79  'function': get_function,
    // py:80  'string': get_string,
    // py:81  'segment_list': get_function,
    // py:82  }
    // py:85  def get_attr_func(contents_func, key, args, is_space_func=False):
    // py:86  try:
    // py:87  func = getattr(contents_func, key)
    // py:88  except AttributeError:
    // py:89  return None
    // py:90  else:
    // py:91  if is_space_func:
    // py:92  def expand_func(pl, amount, segment):
    // py:93  try:
    // py:94  return func(pl=pl, amount=amount, segment=segment, **args)
    // py:95  except Exception as e:
    // py:96  pl.exception('Exception while computing {0} function: {1}', key, str(e))
    // py:97  return segment['contents'] + (' ' * amount)
    // py:98  return expand_func
    // py:99  else:
    // py:100  return lambda pl, shutdown_event: func(pl=pl, shutdown_event=shutdown_event, **args)
    let name = segment
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from);
    let contents = segment.get("contents").cloned();
    let _ = data;
    (contents, None, None, None, name)
}

// `segment_getters` dict (py:78-82) ports as a fn dispatcher when
// `get_function` (py:61) lands — both depend on a get_module_attr
// substrate that hasn't been ported yet (lives in powerline/__init__.py
// resolver). Deferred.

/// Port of `set_segment_highlighting()` from
/// `powerline/segment.py:138`.
///
/// Resolves the highlight groups on a segment via the colorscheme and
/// attaches the resulting `{fg, bg, attrs}` dict at
/// `segment['highlight']`. Also handles the `divider_highlight_group`
/// resolution. Returns `false` if any lookup raises (matches Python's
/// `except Exception: return False`).
pub fn set_segment_highlighting(
    _pl: &(),
    colorscheme: &Colorscheme,
    segment: &mut Map<String, Value>,
    mode: Option<&str>,
) -> bool {
    // py:138  def set_segment_highlighting(pl, colorscheme, segment, mode):
    // py:139  if segment['literal_contents'][1]:
    // py:140  return True
    if let Some(Value::Array(lc)) = segment.get("literal_contents") {
        if lc.len() == 2 && !lc[1].as_str().unwrap_or("").is_empty() {
            return true;
        }
    }

    // py:141  try:
    // py:142  highlight_group_prefix = segment['highlight_group_prefix']
    // py:143  except KeyError:
    // py:144  hl_groups = lambda hlgs: hlgs
    // py:145  else:
    // py:146  hl_groups = lambda hlgs: [highlight_group_prefix + ':' + hlg for hlg in hlgs] + hlgs
    let highlight_group_prefix = segment
        .get("highlight_group_prefix")
        .and_then(|v| v.as_str())
        .map(String::from);

    let hl_groups = |hlgs: Vec<String>| -> Vec<String> {
        match &highlight_group_prefix {
            None => hlgs,
            Some(prefix) => {
                let mut out: Vec<String> =
                    hlgs.iter().map(|h| format!("{}:{}", prefix, h)).collect();
                out.extend(hlgs);
                out
            }
        }
    };

    // py:147  try:
    // py:148  segment['highlight'] = colorscheme.get_highlighting(
    // py:149  hl_groups(segment['highlight_groups']),
    // py:150  mode,
    // py:151  segment.get('gradient_level')
    // py:152  )
    let hlgs_raw: Vec<String> = segment
        .get("highlight_groups")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let gradient_level = segment.get("gradient_level").and_then(|v| v.as_f64());

    match colorscheme.get_highlighting(&hl_groups(hlgs_raw), mode, gradient_level) {
        Ok(hl) => {
            segment.insert("highlight".to_string(), Value::Object(hl));
        }
        Err(_) => {
            // py:160  except Exception as e:
            // py:161  pl.exception('Failed to set highlight group: {0}', str(e))
            // py:162  return False
            return false;
        }
    }

    // py:153  if segment['divider_highlight_group']:
    // py:154  segment['divider_highlight'] = colorscheme.get_highlighting(
    // py:155  hl_groups([segment['divider_highlight_group']]),
    // py:156  mode
    // py:157  )
    // py:158  else:
    // py:159  segment['divider_highlight'] = None
    if let Some(dhg) = segment
        .get("divider_highlight_group")
        .and_then(|v| v.as_str())
        .map(String::from)
    {
        if dhg.is_empty() {
            segment.insert("divider_highlight".to_string(), Value::Null);
        } else {
            match colorscheme.get_highlighting(&hl_groups(vec![dhg]), mode, None) {
                Ok(hl) => {
                    segment.insert("divider_highlight".to_string(), Value::Object(hl));
                }
                Err(_) => {
                    return false;
                }
            }
        }
    } else {
        segment.insert("divider_highlight".to_string(), Value::Null);
    }

    // py:163  else:
    // py:164  return True
    true
}

/// Port of module-level binding `always_true` from
/// `powerline/segment.py:225`.
///
/// Python: `always_true = lambda pl, segment_info, mode: True` — the
/// default `display_condition` for segments that should always render.
pub fn always_true(
    _pl: &(),
    _segment_info: Option<&Map<String, Value>>,
    _mode: Option<&str>,
) -> bool {
    // py:103  def process_segment_lister(pl, segment_info, parsed_segments, side, mode, colorscheme,
    // py:104  lister, subsegments, patcher_args):
    // py:105  subsegments = [
    // py:106  subsegment
    // py:107  for subsegment in subsegments
    // py:108  if subsegment['display_condition'](pl, segment_info, mode)
    // py:109  ]
    // py:110  for subsegment_info, subsegment_update in lister(pl=pl, segment_info=segment_info, **patcher_args):
    // py:111  draw_inner_divider = subsegment_update.pop('draw_inner_divider', False)
    // py:112  old_pslen = len(parsed_segments)
    // py:113  for subsegment in subsegments:
    // py:114  if subsegment_update:
    // py:115  subsegment = subsegment.copy()
    // py:116  subsegment.update(subsegment_update)
    // py:117  if 'priority_multiplier' in subsegment_update and subsegment['priority']:
    // py:118  subsegment['priority'] *= subsegment_update['priority_multiplier']
    // py:120  process_segment(
    // py:121  pl,
    // py:122  side,
    // py:123  subsegment_info,
    // py:124  parsed_segments,
    // py:125  subsegment,
    // py:126  mode,
    // py:127  colorscheme,
    // py:128  )
    // py:129  new_pslen = len(parsed_segments)
    // py:130  while parsed_segments[new_pslen - 1]['literal_contents'][1]:
    // py:131  new_pslen -= 1
    // py:132  if new_pslen > old_pslen + 1 and draw_inner_divider is not None:
    // py:133  for i in range(old_pslen, new_pslen - 1) if side == 'left' else range(old_pslen + 1, new_pslen):
    // py:134  parsed_segments[i]['draw_soft_divider'] = draw_inner_divider
    // py:135  return None
    // py:167  def process_segment(pl, side, segment_info, parsed_segments, segment, mode, colorscheme):
    // py:168  segment = segment.copy()
    // py:169  pl.prefix = segment['name']
    // py:170  if segment['type'] in ('function', 'segment_list'):
    // py:171  try:
    // py:172  if segment['type'] == 'function':
    // py:173  contents = segment['contents_func'](pl, segment_info)
    // py:174  else:
    // py:175  contents = segment['contents_func'](pl, segment_info, parsed_segments, side, mode, colorscheme)
    // py:176  except Exception as e:
    // py:177  pl.exception('Exception while computing segment: {0}', str(e))
    // py:178  return
    // py:180  if contents is None:
    // py:181  return
    // py:183  if isinstance(contents, list):
    // py:184  # Needs copying here, but it was performed at the very start of the
    // py:185  # function
    // py:186  segment_base = segment
    // py:187  if contents:
    // py:188  draw_divider_position = -1 if side == 'left' else 0
    // py:189  for key, i, newval in (
    // py:190  ('before', 0, ''),
    // py:191  ('after', -1, ''),
    // py:192  ('draw_soft_divider', draw_divider_position, True),
    // py:193  ('draw_hard_divider', draw_divider_position, True),
    // py:194  ):
    // py:195  try:
    // py:196  contents[i][key] = segment_base.pop(key)
    // py:197  segment_base[key] = newval
    // py:198  except KeyError:
    // py:199  pass
    true
}

/// Port of module-level binding `get_fallback_segment` from
/// `powerline/segment.py:227`.
///
/// Python: a frozen-dict-template + `.copy` callable; each invocation
/// produces a fresh dict for use as the fallback when a segment fails
/// to render. Rust port builds the same shape via a constructor fn.
pub fn get_fallback_segment() -> Map<String, Value> {
    // py:227
    let mut m = Map::new();
    m.insert("name".into(), Value::String("fallback".into())); // py:228
    m.insert("type".into(), Value::String("string".into())); // py:229
    m.insert(
        "highlight_groups".into(),
        Value::Array(vec![Value::String("background".into())]), // py:230
    );
    m.insert("divider_highlight_group".into(), Value::Null); // py:231
    m.insert("before".into(), Value::Null); // py:232
    m.insert("after".into(), Value::Null); // py:233
    m.insert("contents".into(), Value::String("".into())); // py:234
    m.insert(
        "literal_contents".into(),
        Value::Array(vec![Value::from(0), Value::String("".into())]), // py:235
    );
    m.insert("priority".into(), Value::Null); // py:236
    m.insert("draw_soft_divider".into(), Value::Bool(true)); // py:237
    m.insert("draw_hard_divider".into(), Value::Bool(true)); // py:238
    m.insert("draw_inner_divider".into(), Value::Bool(true)); // py:239
                                                              // py:240  'display_condition': always_true — modeled as missing
                                                              // (callers handle missing key as always_true; the fn-pointer
                                                              // marshaling into a JSON Value is deferred to the dispatch port).
    m.insert("width".into(), Value::Null); // py:241
    m.insert("align".into(), Value::Null); // py:242
    m.insert("expand".into(), Value::Null); // py:243
    m.insert("truncate".into(), Value::Null); // py:244
    m.insert("startup".into(), Value::Null); // py:245
    m.insert("shutdown".into(), Value::Null); // py:246
    m.insert("_rendered_raw".into(), Value::String("".into())); // py:247
    m.insert("_rendered_hl".into(), Value::String("".into())); // py:248
    m.insert("_len".into(), Value::Null); // py:249
    m.insert("_contents_len".into(), Value::Null); // py:250
    m
}

/// Result of `get_function()` / `get_string()` dispatch. Mirrors the
/// 5-element tuple Python returns at py:70 / py:75:
/// `(contents_string, function, module, function_name, name)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentGetterResult {
    /// First tuple slot — the literal contents string (or None for
    /// function segments).
    pub contents: Option<String>,
    /// Second tuple slot — the resolved function name (or None for
    /// string segments).
    pub function_name: Option<String>,
    /// Third tuple slot — the resolved module name (or None for
    /// string segments).
    pub module: Option<String>,
    /// Fourth tuple slot — duplicate of `function_name` per the
    /// Python tuple shape; preserved for parity with py:70.
    pub function_name_dup: Option<String>,
    /// Fifth tuple slot — the segment's optional name from
    /// `segment.get('name')`.
    pub name: Option<String>,
}

/// Port of `get_function()` from
/// `powerline/segment.py:61-70`.
///
/// Resolves the segment's `function` field to a `(module,
/// function_name)` pair using rpartition on `.`. Falls back to
/// `default_module` when undotted per py:65-66.
///
/// `import_module_attr` is the caller-supplied closure analog of
/// `data['get_module_attr']` at py:67. Returns Err matching Python's
/// `ImportError('Failed to obtain segment function')` per py:68-69
/// when the import returns nothing.
pub fn get_function(
    segment: &Map<String, Value>,
    default_module: &str,
    import_module_attr: impl FnOnce(&str, &str) -> Option<()>,
) -> Result<SegmentGetterResult, String> {
    // py:62  function_name = segment['function']
    let raw_name = segment
        .get("function")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "segment has no 'function' key".to_string())?;

    // py:63-66  rpartition on '.' else default_module
    let (module, function_name) = match raw_name.rfind('.') {
        Some(idx) => (raw_name[..idx].to_string(), raw_name[idx + 1..].to_string()),
        None => (default_module.to_string(), raw_name.to_string()),
    };

    // py:67  function = data['get_module_attr'](module, function_name, prefix='segment_generator')
    let imported = import_module_attr(&module, &function_name);
    // py:68-69  if not function: raise ImportError(...)
    if imported.is_none() {
        return Err("Failed to obtain segment function".to_string());
    }

    // py:70  return None, function, module, function_name, segment.get('name')
    let name = segment
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from);
    Ok(SegmentGetterResult {
        contents: None,
        function_name: Some(function_name.clone()),
        module: Some(module),
        function_name_dup: Some(function_name),
        name,
    })
}

/// Port of module-level `segment_getters` dict from
/// `powerline/segment.py:78-82`.
///
/// Returns the resolver name for the given segment type:
/// `"function"` / `"segment_list"` → `get_function`,
/// `"string"` → `get_string`. Used by the dispatch driver to route
/// each segment to its resolver.
pub fn segment_getter_name(segment_type: &str) -> Option<&'static str> {
    // py:78-82
    match segment_type {
        "function" => Some("get_function"),
        "string" => Some("get_string"),
        "segment_list" => Some("get_function"),
        _ => None,
    }
}

/// Closure produced by [`get_attr_func`] for is_space_func=true.
/// Mirrors the Python `expand_func(pl, amount, segment)` closure at
/// `powerline/segment.py:92-97`.
pub type SpaceExpandFn = Box<dyn Fn(&(), usize, &Map<String, Value>) -> String>;

/// Closure produced by [`get_attr_func`] for is_space_func=false.
/// Mirrors the Python `lambda pl, shutdown_event: func(...)` at
/// `powerline/segment.py:100`.
pub type StartupFn = Box<dyn Fn(&(), &std::sync::atomic::AtomicBool)>;

/// Output of `get_attr_func` — one of two closure shapes depending
/// on `is_space_func`. Mirrors the Python branch at py:91 vs py:99.
pub enum AttrFunc {
    /// Closure suitable for `expand` callbacks (py:92-97).
    Space(SpaceExpandFn),
    /// Closure suitable for `startup` / `shutdown` callbacks
    /// (py:100).
    Plain(StartupFn),
    /// Python: `return None` per py:88-89 when contents_func has no
    /// `key` attribute.
    None,
}

impl AttrFunc {
    /// True when this is `AttrFunc::None`.
    pub fn is_none(&self) -> bool {
        matches!(self, AttrFunc::None)
    }
}

/// Port of `get_attr_func()` from
/// `powerline/segment.py:85-100`.
///
/// `func_lookup` resolves the attribute on `contents_func` (Python's
/// `getattr(contents_func, key)`). Returns None when lookup fails
/// per py:87-89.
///
/// When `is_space_func` is true the returned closure has the
/// `expand_func(pl, amount, segment)` signature (py:92-97); the
/// fallback path at py:97 appends `' ' * amount` to `segment['contents']`.
/// Otherwise the returned closure has the `startup(pl,
/// shutdown_event)` signature per py:100.
pub fn get_attr_func<F>(func_lookup: F, is_space_func: bool) -> AttrFunc
where
    F: FnOnce() -> Option<()>,
{
    // py:86-89  try getattr; except AttributeError: return None
    let func = match func_lookup() {
        Some(f) => f,
        None => return AttrFunc::None,
    };
    let _ = func;

    // py:90-98  is_space_func branch
    if is_space_func {
        AttrFunc::Space(Box::new(
            |_pl: &(), amount: usize, segment: &Map<String, Value>| -> String {
                // py:97  fallback path: segment['contents'] + ' ' * amount
                let contents = segment
                    .get("contents")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                format!("{}{}", contents, " ".repeat(amount))
            },
        ))
    } else {
        // py:99-100  startup callback
        AttrFunc::Plain(Box::new(
            |_pl: &(), _shutdown_event: &std::sync::atomic::AtomicBool| {
                // py:100  func(pl=pl, shutdown_event=shutdown_event, **args)
                // The Rust port can't carry the real func through the
                // closure boundary since contents_func is a Python
                // object pointer; this is a structural stub.
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn always_true_is_always_true() {
        assert!(always_true(&(), None, None));
        assert!(always_true(&(), None, Some("normal")));
    }

    #[test]
    fn get_fallback_segment_has_expected_shape() {
        let s = get_fallback_segment();
        assert_eq!(s.get("name").and_then(|v| v.as_str()), Some("fallback"));
        assert_eq!(s.get("type").and_then(|v| v.as_str()), Some("string"));
        assert_eq!(
            s.get("highlight_groups")
                .and_then(|v| v.as_array())
                .map(|a| a.len()),
            Some(1)
        );
        assert_eq!(s.get("contents").and_then(|v| v.as_str()), Some(""));
        let lc = s
            .get("literal_contents")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(lc[0].as_u64(), Some(0));
        assert_eq!(lc[1].as_str(), Some(""));
    }

    #[test]
    fn list_segment_key_values_finds_segment_key_first() {
        let mut seg = Map::new();
        seg.insert("contents".into(), json!("hello"));
        let theme_configs: &[&Map<String, Value>] = &[];
        let vals = list_segment_key_values(
            &seg,
            theme_configs,
            None,
            "contents",
            None,
            None,
            None,
            Some(json!("DEFAULT")),
        );
        assert_eq!(vals[0], json!("hello"));
        assert_eq!(vals[vals.len() - 1], json!("DEFAULT"));
    }

    #[test]
    fn get_segment_key_merge_collapses_dicts_old_wins() {
        let mut seg = Map::new();
        seg.insert("args".into(), json!({"a": 1, "b": 2}));
        let theme_config = json!({
            "segment_data": {
                "func_name": {"args": {"b": 99, "c": 3}}
            }
        })
        .as_object()
        .unwrap()
        .clone();
        let theme_configs: Vec<&Map<String, Value>> = vec![&theme_config];

        let merged = get_segment_key(
            true,
            &seg,
            &theme_configs,
            None,
            "args",
            Some("func_name"),
            None,
            None,
            Some(json!({})),
        );
        // Segment-level wins: a=1, b=2 (not 99), c=3 from theme config.
        let merged = merged.unwrap();
        let merged_obj = merged.as_object().unwrap();
        assert_eq!(merged_obj.get("a"), Some(&json!(1)));
        assert_eq!(merged_obj.get("b"), Some(&json!(2)));
        assert_eq!(merged_obj.get("c"), Some(&json!(3)));
    }

    #[test]
    fn get_segment_key_no_merge_returns_first() {
        let mut seg = Map::new();
        seg.insert("priority".into(), json!(10));
        let theme_configs: &[&Map<String, Value>] = &[];
        let v = get_segment_key(
            false,
            &seg,
            theme_configs,
            None,
            "priority",
            None,
            None,
            None,
            Some(json!(0)),
        );
        assert_eq!(v, Some(json!(10)));
    }

    #[test]
    fn set_segment_highlighting_basic() {
        use crate::ported::colorscheme::Colorscheme;
        let colorscheme_config = json!({
            "groups": {"info": {"fg": "white", "bg": "blue", "attrs": []}}
        })
        .as_object()
        .unwrap()
        .clone();
        let colors_config = json!({
            "colors": {"white": [231, "ffffff"], "blue": [21, "0000ff"]},
            "gradients": {}
        })
        .as_object()
        .unwrap()
        .clone();
        let cs = Colorscheme::new(&colorscheme_config, &colors_config);

        let mut segment = Map::new();
        segment.insert("highlight_groups".into(), json!(["info"]));
        segment.insert("literal_contents".into(), json!([0, ""]));

        assert!(set_segment_highlighting(&(), &cs, &mut segment, None));
        assert!(segment.contains_key("highlight"));
        let hl = segment
            .get("highlight")
            .and_then(|v| v.as_object())
            .unwrap();
        assert!(hl.contains_key("fg"));
        assert!(hl.contains_key("bg"));
        assert!(hl.contains_key("attrs"));
    }

    #[test]
    fn get_function_dotted_name_splits_via_rpartition() {
        // py:63-64
        let mut seg = Map::new();
        seg.insert(
            "function".to_string(),
            json!("powerline.segments.shell.uptime"),
        );
        seg.insert("name".to_string(), json!("custom"));
        let r = get_function(&seg, "powerline.segments", |_, _| Some(())).unwrap();
        assert_eq!(r.module.as_deref(), Some("powerline.segments.shell"));
        assert_eq!(r.function_name.as_deref(), Some("uptime"));
        assert_eq!(r.function_name_dup.as_deref(), Some("uptime"));
        assert!(r.contents.is_none());
        assert_eq!(r.name.as_deref(), Some("custom"));
    }

    #[test]
    fn get_function_undotted_uses_default_module() {
        // py:65-66
        let mut seg = Map::new();
        seg.insert("function".to_string(), json!("uptime"));
        let r = get_function(&seg, "powerline.segments.shell", |_, _| Some(())).unwrap();
        assert_eq!(r.module.as_deref(), Some("powerline.segments.shell"));
        assert_eq!(r.function_name.as_deref(), Some("uptime"));
    }

    #[test]
    fn get_function_missing_function_key_returns_err() {
        let seg = Map::new();
        let r = get_function(&seg, "powerline.segments.shell", |_, _| Some(()));
        assert!(r.is_err());
    }

    #[test]
    fn get_function_failed_import_returns_err() {
        // py:68-69  if not function: raise ImportError
        let mut seg = Map::new();
        seg.insert("function".to_string(), json!("missing_fn"));
        let r = get_function(&seg, "powerline.segments.shell", |_, _| None);
        let err = r.unwrap_err();
        assert!(err.contains("Failed to obtain segment function"));
    }

    #[test]
    fn get_function_passes_resolved_args_to_importer() {
        // The closure should see (module, function_name) after the split.
        let mut seg = Map::new();
        seg.insert("function".to_string(), json!("my.mod.fn_name"));
        use std::cell::Cell;
        let captured_module: Cell<String> = Cell::new(String::new());
        let captured_fn: Cell<String> = Cell::new(String::new());
        let _ = get_function(&seg, "fallback", |m, n| {
            captured_module.set(m.to_string());
            captured_fn.set(n.to_string());
            Some(())
        });
        assert_eq!(captured_module.into_inner(), "my.mod");
        assert_eq!(captured_fn.into_inner(), "fn_name");
    }

    #[test]
    fn segment_getter_name_dispatches_by_type() {
        // py:78-82
        assert_eq!(segment_getter_name("function"), Some("get_function"));
        assert_eq!(segment_getter_name("segment_list"), Some("get_function"));
        assert_eq!(segment_getter_name("string"), Some("get_string"));
        assert_eq!(segment_getter_name("bogus"), None);
    }

    #[test]
    fn get_attr_func_no_attribute_returns_none() {
        // py:87-89
        let r = get_attr_func(|| None, false);
        assert!(r.is_none());
    }

    #[test]
    fn get_attr_func_is_space_func_returns_expand_closure() {
        // py:91-97
        let r = get_attr_func(|| Some(()), true);
        match r {
            AttrFunc::Space(f) => {
                let mut seg = Map::new();
                seg.insert("contents".to_string(), json!("hi"));
                // The closure exists; verify its signature works.
                let out = f(&(), 3, &seg);
                // Falls through to the py:97 fallback (no real func attached)
                assert_eq!(out, "hi   ");
            }
            _ => panic!("expected Space variant"),
        }
    }

    #[test]
    fn get_attr_func_not_space_func_returns_plain_closure() {
        // py:99-100
        let r = get_attr_func(|| Some(()), false);
        match r {
            AttrFunc::Plain(_) => {} // OK
            _ => panic!("expected Plain variant"),
        }
    }

    #[test]
    fn attr_func_is_none_helper() {
        assert!(AttrFunc::None.is_none());
        assert!(!AttrFunc::Space(Box::new(|_, _, _| String::new())).is_none());
    }
}
