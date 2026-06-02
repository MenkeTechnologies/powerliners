// vim:fileencoding=utf-8:noet
//! stryke version segment — see `zshrs_version` for the shared
//! contract. Default binary is `stryke`; output shape is
//! `This is stryke vX.Y.Z — …`.

use crate::extensions::bin_version;
use serde_json::{json, Value};
use std::time::Duration;

pub fn version(bin: &str, format: &str, ttl_secs: u64) -> Option<Vec<Value>> {
    let v = bin_version::get(bin, &["--version"], Duration::from_secs(ttl_secs))?;
    let contents = format.replace("{version}", &v);
    Some(vec![json!({
        "contents": contents,
        "highlight_groups": ["stryke_version", "stryke", "information:regular"],
        "divider_highlight_group": "background:divider",
    })])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_missing_binary_returns_none() {
        let r = version("/nonexistent/stryke-xyz", "{version}", 30);
        assert!(r.is_none());
    }
}
