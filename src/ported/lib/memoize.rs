// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/memoize.py`.
//!
//! Time-based memoization primitive — caches a function's return value
//! keyed by kwargs for `timeout` seconds (monotonic clock). Used by VCS
//! segments, weather/battery segments, and any other segment whose
//! data source is expensive to query on every render tick.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from functools import wraps                                                              // py:4
// from powerline.lib.monotonic import monotonic                                            // py:6

use crate::ported::lib::monotonic::monotonic;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Port of `default_cache_key()` from `powerline/lib/memoize.py:9`.
///
/// Python: `return frozenset(kwargs.items())`.
///
/// Builds a stable identity from kwargs that survives dict re-iteration
/// order (frozenset is hashable + order-insensitive). In Rust we
/// produce a sorted-and-joined string since arbitrary `serde_json::Value`
/// is `Eq` but not `Hash` without an explicit derive — the join shape
/// gives the same identity property without a custom Hash impl.
pub fn default_cache_key(kwargs: &Map<String, Value>) -> String {
    // py:10  frozenset(kwargs.items())
    let mut pairs: Vec<(String, String)> = kwargs
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\0")
}

/// Cache entry — mirrors Python's `{'result': ..., 'time': ...}` dict
/// stored per-key in `self.cache`.
#[derive(Clone)]
pub struct CacheEntry {
    pub result: Value,
    pub time: f64,
}

/// Port of `class memoize` from `powerline/lib/memoize.py:13`.
///
/// Memoization decorator with timeout.
///
/// Python is a decorator class (callable returning a wrapped fn). The
/// Rust port carries the same configuration fields and provides
/// `get_or_compute` as the `__call__`-replacement entry point:
/// callers pass the kwargs map + a closure that produces the value if
/// the cache misses or expires.
#[allow(non_camel_case_types)]
pub struct memoize {
    // py:13
    /// Python: `self.timeout` (seconds) — py:16
    pub timeout: f64,
    /// Python: `self.cache_key` — opaque key fn. The Rust port stores
    /// a function pointer mirroring `default_cache_key`'s signature.
    pub cache_key: fn(&Map<String, Value>) -> String,
    /// Python: `self.cache` (`dict`) — py:18.
    /// Module-level mutable shared state per memoize-instance —
    /// bucket-2 in PORT_PLAN.md. Wrapped in `Arc<Mutex<HashMap>>`
    /// because rendering threads call into the cache in parallel.
    pub cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
}

impl memoize {
    /// Port of `memoize.__init__()` from `powerline/lib/memoize.py:15`.
    pub fn new(timeout: f64) -> Self {
        // py:15
        Self {
            timeout,                                     // py:16
            cache_key: default_cache_key,                // py:17
            cache: Arc::new(Mutex::new(HashMap::new())), // py:18
        }
    }

    /// Port of `memoize.__call__()` body from
    /// `powerline/lib/memoize.py:21` — the wrapper that the decorator
    /// returns.
    ///
    /// `kwargs` is the per-call arg dict; `compute` is the underlying
    /// function (Python: `func(**kwargs)` at py:32/38).
    ///
    /// Equivalent to invoking the closure returned by Python's
    /// `__call__`. Convenience alias retained for parity with
    /// upstream's `__call__` → wrapped-fn idiom.
    pub fn get_or_compute<F>(&self, kwargs: &Map<String, Value>, compute: F) -> Value
    where
        F: FnOnce(&Map<String, Value>) -> Value,
    {
        self.decorated_function(kwargs, compute)
    }

    /// Port of the inner `decorated_function` closure from
    /// `powerline/lib/memoize.py:23-41` — the wrapped function the
    /// `@memoize(timeout)` decorator returns. Equivalent to invoking
    /// the closure Python's `__call__` returns.
    ///
    /// Python:
    /// ```python
    /// def __call__(self, func):
    ///     @wraps(func)
    ///     def decorated_function(**kwargs):
    ///         key = self.cache_key(**kwargs)
    ///         cached = self.cache.get(key, None)
    ///         if cached is None or not (cached['time'] < monotonic() <
    ///                                   cached['time'] + self.timeout):
    ///             cached = self.cache[key] = {
    ///                 'result': func(**kwargs),
    ///                 'time': monotonic(),
    ///             }
    ///         return cached['result']
    ///     return decorated_function
    /// ```
    pub fn decorated_function<F>(&self, kwargs: &Map<String, Value>, compute: F) -> Value
    where
        F: FnOnce(&Map<String, Value>) -> Value,
    {
        // py:21  def __call__(self, func):
        // py:22  @wraps(func)
        // py:23  def decorated_function(**kwargs):
        // py:24  if self.cache_reg_func:  (Rust port omits cache_reg_func)
        // py:25  self.cache_reg_func(self.cache)
        // py:26  self.cache_reg_func = None
        // py:28  key = self.cache_key(**kwargs)
        let key = (self.cache_key)(kwargs);
        let now = monotonic();

        // py:29  try:
        // py:30  cached = self.cache.get(key, None)
        // py:31  except TypeError:
        // py:32  return func(**kwargs)
        let cache = self.cache.lock().unwrap();
        if let Some(cached) = cache.get(&key) {
            // py:36  if cached is None or not (cached['time'] < monotonic() < cached['time'] + self.timeout):
            if cached.time < now && now < cached.time + self.timeout {
                // py:41  return cached['result']
                return cached.result.clone();
            }
        }

        // py:37  cached = self.cache[key] = {
        // py:38  'result': func(**kwargs),
        // py:39  'time': monotonic(),
        // py:40  }
        drop(cache);
        let result = compute(kwargs);
        let mut cache = self.cache.lock().unwrap();
        cache.insert(
            key,
            CacheEntry {
                result: result.clone(),
                time: now,
            },
        );
        // py:41  return cached['result']
        result
        // py:42  return decorated_function
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc as StdArc;

    #[test]
    fn default_cache_key_is_stable_across_iteration_order() {
        let mut m1 = Map::new();
        m1.insert("a".into(), json!(1));
        m1.insert("b".into(), json!(2));

        let mut m2 = Map::new();
        m2.insert("b".into(), json!(2));
        m2.insert("a".into(), json!(1));

        assert_eq!(default_cache_key(&m1), default_cache_key(&m2));
    }

    #[test]
    fn memoize_caches_within_timeout() {
        let m = memoize::new(60.0);
        let counter = StdArc::new(AtomicI32::new(0));
        let mut kwargs = Map::new();
        kwargs.insert("x".into(), json!(42));

        for _ in 0..3 {
            let c = counter.clone();
            m.get_or_compute(&kwargs, move |_| {
                c.fetch_add(1, Ordering::SeqCst);
                json!("computed")
            });
        }
        // Only the first call should have invoked compute.
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn memoize_recomputes_on_different_key() {
        let m = memoize::new(60.0);
        let counter = StdArc::new(AtomicI32::new(0));

        for x in [1, 2, 3] {
            let c = counter.clone();
            let mut kwargs = Map::new();
            kwargs.insert("x".into(), json!(x));
            m.get_or_compute(&kwargs, move |_| {
                c.fetch_add(1, Ordering::SeqCst);
                json!(x)
            });
        }
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn memoize_recomputes_after_timeout() {
        // Use a tiny timeout to exercise expiry without slowing tests.
        let m = memoize::new(0.01); // 10ms
        let counter = StdArc::new(AtomicI32::new(0));
        let mut kwargs = Map::new();
        kwargs.insert("x".into(), json!(42));

        for _ in 0..3 {
            let c = counter.clone();
            m.get_or_compute(&kwargs, move |_| {
                c.fetch_add(1, Ordering::SeqCst);
                json!("computed")
            });
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        // Each call should have re-computed (15ms > 10ms timeout).
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
