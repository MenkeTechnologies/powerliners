// vim:fileencoding=utf-8:noet
//! Kubernetes context segment — current context name + active
//! namespace. Shells out to `kubectl config` so the resolution
//! respects:
//!
//! - `$KUBECONFIG` (single path or colon-separated cascade),
//! - in-context namespace overrides (`kubens` / `kubectl config set-context --current --namespace=NS`),
//! - context renames done via `kubectx`,
//! - cloud-provider auth plugins that rewrite contexts at lookup.
//!
//! The two probes (`current-context` + `--minify -o jsonpath=...`)
//! are independent so a missing namespace doesn't disable the
//! segment.
//!
//! Returns `None` when:
//! - `kubectl` isn't on PATH, OR
//! - no current context is set (fresh kube install / cleared config).
//!
//! Theme JSON:
//! ```json
//! {
//!   "function": "powerliners.k8s.kubecontext",
//!   "args": {
//!     "format": "{context}:{namespace}",
//!     "default_namespace": "default",
//!     "hide_default": true
//!   }
//! }
//! ```
//!
//! Format tokens:
//! - `{context}`   — current kube context name (e.g. `gke_proj_us-central1_prod`)
//! - `{namespace}` — context's active namespace, or `default_namespace` arg
//!   when the context didn't pin one explicitly

use serde_json::{json, Value};
use std::process::Command;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct KubeContext {
    pub context: String,
    /// Empty when the context didn't set a namespace and the caller
    /// hasn't supplied a default — that's a legitimate "use whatever
    /// the resource manifest says" state.
    pub namespace: String,
}

/// Probe the active kube context. Returns `None` when kubectl is
/// missing or no current context is set.
pub fn read_kube_context(cli: &str) -> Option<KubeContext> {
    let ctx_out = Command::new(cli)
        .args(["config", "current-context"])
        .output()
        .ok()?;
    if !ctx_out.status.success() {
        return None;
    }
    let context = std::str::from_utf8(&ctx_out.stdout)
        .ok()?
        .trim()
        .to_string();
    if context.is_empty() {
        return None;
    }
    // `--minify` collapses to just the active context so the jsonpath
    // resolves against one entry. Empty namespace is fine.
    let ns_out = Command::new(cli)
        .args([
            "config",
            "view",
            "--minify",
            "-o",
            "jsonpath={..namespace}",
        ])
        .output()
        .ok()?;
    let namespace = if ns_out.status.success() {
        std::str::from_utf8(&ns_out.stdout)
            .unwrap_or_default()
            .trim()
            .to_string()
    } else {
        String::new()
    };
    Some(KubeContext { context, namespace })
}

/// Render the kube context segment.
///
/// `default_namespace`: substituted when the kube context didn't
/// pin one (typical for a freshly cloned config). `hide_default`:
/// when `true` AND the rendered namespace equals `default_namespace`,
/// the namespace fragment is dropped entirely so the segment shows
/// just the context name — reduces clutter on dev clusters where
/// most prompts sit in `default`.
pub fn kubecontext(
    cli: &str,
    format: &str,
    default_namespace: &str,
    hide_default: bool,
) -> Option<Vec<Value>> {
    let state = read_kube_context(cli)?;
    let ns = if state.namespace.is_empty() {
        default_namespace.to_string()
    } else {
        state.namespace.clone()
    };
    let contents = if hide_default && ns == default_namespace {
        // Strip the {namespace} token AND any single separator char
        // (':' or '/' or '@') immediately adjacent to it, so the
        // result reads cleanly without dangling punctuation.
        render_format_drop_ns(format, &state.context)
    } else {
        render_format(format, &state.context, &ns)
    };
    Some(vec![json!({
        "contents": contents,
        "highlight_groups": [
            "kubecontext",
            "k8s",
        ],
        "divider_highlight_group": "background:divider",
    })])
}

fn render_format(fmt: &str, context: &str, namespace: &str) -> String {
    fmt.replace("{context}", context)
        .replace("{namespace}", namespace)
}

/// Format renderer that elides the `{namespace}` token along with a
/// single adjacent separator char (`:` `/` `@`), so `foo:{namespace}`
/// → `foo` cleanly when the namespace is the default.
fn render_format_drop_ns(fmt: &str, context: &str) -> String {
    let mut s = fmt.replace("{context}", context);
    // Walk separators in priority order: the rendered form most
    // commonly uses `:`, then `/`, then `@`.
    for sep in [":{namespace}", "/{namespace}", "@{namespace}"] {
        if s.contains(sep) {
            s = s.replace(sep, "");
            return s;
        }
        // Also strip when separator follows the token.
        let trailing = format!("{}{}", "{namespace}", sep.chars().next().unwrap_or(':'));
        if s.contains(&trailing) {
            s = s.replace(&trailing, "");
            return s;
        }
    }
    s.replace("{namespace}", "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_kube_context_missing_kubectl_returns_none() {
        assert!(read_kube_context("/nonexistent/kubectl-xyz").is_none());
    }

    #[test]
    fn kubecontext_missing_kubectl_returns_none() {
        let r = kubecontext(
            "/nonexistent/kubectl-xyz",
            "{context}:{namespace}",
            "default",
            true,
        );
        assert!(r.is_none());
    }

    #[test]
    fn render_format_substitutes_context_and_namespace() {
        let s = render_format(
            "{context}:{namespace}",
            "prod-cluster",
            "kube-system",
        );
        assert_eq!(s, "prod-cluster:kube-system");
    }

    #[test]
    fn render_format_leaves_unknown_tokens_intact() {
        let s = render_format("{context}/{namespace} v{version}", "c", "ns");
        assert_eq!(s, "c/ns v{version}");
    }

    #[test]
    fn render_format_drop_ns_strips_colon_separator() {
        let s = render_format_drop_ns("{context}:{namespace}", "prod");
        assert_eq!(s, "prod");
    }

    #[test]
    fn render_format_drop_ns_strips_slash_separator() {
        let s = render_format_drop_ns("{context}/{namespace}", "prod");
        assert_eq!(s, "prod");
    }

    #[test]
    fn render_format_drop_ns_strips_at_separator() {
        let s = render_format_drop_ns("{namespace}@{context}", "prod");
        // The `@` follows the namespace token; the strip rules above
        // handle "{namespace}@" via the trailing-separator branch.
        assert_eq!(s, "prod");
    }

    #[test]
    fn render_format_drop_ns_handles_namespace_only() {
        // No surrounding separator → bare strip.
        let s = render_format_drop_ns("{namespace}", "prod");
        assert_eq!(s, "");
    }

    #[test]
    fn kubecontext_state_default_is_empty() {
        let k = KubeContext::default();
        assert!(k.context.is_empty());
        assert!(k.namespace.is_empty());
    }
}
