# Upstream Snapshot

This directory is a vendored snapshot of [powerline/powerline](https://github.com/powerline/powerline).

**Do not edit files under `vendor/powerline/`.** This is the canonical Python source that the Rust port mirrors. Every `// py:NNN` citation in `src/ported/` refers to a line in this tree.

To refresh the snapshot:

```sh
rm -rf vendor/powerline
git clone --depth 1 https://github.com/powerline/powerline.git vendor/powerline
python3 scripts/extract_py_names.py  # regenerate allowlists
```

Snapshot date: 2026-05-28
