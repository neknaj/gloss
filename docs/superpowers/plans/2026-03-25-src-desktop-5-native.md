# src-desktop-native (Plan 5 of 7) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the `src-desktop-native` crate — a `std` crate that bridges the `no_std` trait layer (`src-desktop-types`) to the concrete native implementations (`std::fs` and `GlossPluginHost`), and also fix `src-plugin::GlossPluginHost::run_lint_rule` to accept `&[PluginEvent]` (already-converted events) instead of `&[Event<'_>]`.

**Architecture:** Two changes. First, `src-plugin/src/host.rs` changes the `run_lint_rule` signature from `&[Event<'a>]` to `&[PluginEvent]` and removes the internal `to_plugin_events()` call — the caller (`AppCore`) now does the conversion. Second, a new `src-desktop-native` crate provides: `NativeFs` (FileSystem via std::fs), `impl PluginHost for GlossPluginHost` (thin delegation), `make_plugin_host()` (constructs GlossPluginHost from PluginEntrySpec), and `load_app_config()` (reads gloss.toml into AppConfig).

**Tech Stack:** Rust `std`, `src-desktop-types`, `src-plugin`, `src-plugin-types`, `serde_json`.

**Spec:** `docs/superpowers/specs/2026-03-24-src-desktop-design.md` §7

---

## File Map

| File | Change | Responsibility |
|------|--------|----------------|
| `src-plugin/src/host.rs` | Modify | Change `run_lint_rule` to accept `&[PluginEvent]`; remove `to_plugin_events` call |
| `Cargo.toml` (root) | Modify | Add `src-desktop-native` to workspace members |
| `src-desktop-native/Cargo.toml` | Create | `std` crate; deps: src-desktop-types, src-plugin, src-plugin-types, serde_json |
| `src-desktop-native/src/lib.rs` | Create | Module declarations + re-exports |
| `src-desktop-native/src/fs.rs` | Create | `NativeFs` struct implementing `FileSystem` |
| `src-desktop-native/src/plugin_host.rs` | Create | `impl PluginHost for GlossPluginHost` + `make_plugin_host()` |
| `src-desktop-native/src/config.rs` | Create | `load_app_config()` — TOML → AppConfig |

---

## Task 1: Fix `src-plugin::run_lint_rule` signature

**Files:**
- Modify: `src-plugin/src/host.rs`

**Why:** `AppCore::render_full` calls `to_plugin_events(&events)` and passes the result to `plugin_host.run_lint_rule(...)`. The `PluginHost` trait (`src-desktop-types`) already expects `&[PluginEvent]`. But `GlossPluginHost::run_lint_rule` still takes `&[Event<'a>]` and calls `to_plugin_events` internally — double-converting. Fix the source.

- [ ] **Step 1.1: Write failing test that uses PluginEvent directly**

Add to `src-plugin/src/host.rs` tests module (it already has tests — add to the existing `#[cfg(test)]` block):

```rust
#[test]
fn run_lint_rule_accepts_plugin_events() {
    use src_plugin_types::PluginEvent;
    let mut host = GlossPluginHost { plugins: vec![] };
    let events: Vec<PluginEvent> = vec![];
    let result = host.run_lint_rule("test.n.md", "# hi", &[], &events);
    assert!(result.is_empty());
}
```

Run: `cd /mnt/d/project/gloss && cargo test -p src-plugin run_lint_rule_accepts_plugin_events 2>&1 | tail -5`

Expected: **compile error** — `run_lint_rule` currently takes `&[Event<'a>]`, not `&[PluginEvent]`.

- [ ] **Step 1.2: Change `run_lint_rule` signature in `host.rs`**

In `src-plugin/src/host.rs`, make the following changes:

1. Remove (or comment out) these two imports if no longer needed after the change:
   ```rust
   use crate::convert::to_plugin_events;
   use src_core::parser::Event;
   ```

2. Change the method signature from:
   ```rust
   pub fn run_lint_rule<'a>(
       &mut self,
       source: &str,
       markdown: &str,
       existing_warnings: &[PluginWarning],
       events: &[Event<'a>],
   ) -> Vec<PluginWarning> {
       let plugin_events = to_plugin_events(events);
   ```
   To:
   ```rust
   pub fn run_lint_rule(
       &mut self,
       source: &str,
       markdown: &str,
       existing_warnings: &[PluginWarning],
       events: &[src_plugin_types::PluginEvent],
   ) -> Vec<PluginWarning> {
       let plugin_events = events;
   ```
   (Keep the rest of the method body unchanged — it already uses `plugin_events` as a local variable.)

   Add the import at the top of the file:
   ```rust
   use src_plugin_types::PluginEvent;
   ```
   Then use `events` directly in the body (rename `plugin_events` → `events` or keep as `let plugin_events = events;`).

   Full updated signature block (also change `events: plugin_events.clone()` → `events: events.to_vec()` inside the loop body — `.clone()` on a `&[PluginEvent]` returns a reference, not a `Vec`):
   ```rust
   pub fn run_lint_rule(
       &mut self,
       source: &str,
       markdown: &str,
       existing_warnings: &[PluginWarning],
       events: &[PluginEvent],
   ) -> Vec<PluginWarning> {
       let mut all_warnings = Vec::new();
       for p in &mut self.plugins {
           if !p.hooks.iter().any(|h| h == "lint-rule") {
               continue;
           }
           let input = LintRuleInput {
               source: source.to_string(),
               markdown: markdown.to_string(),
               existing_warnings: existing_warnings.to_vec(),
               events: events.to_vec(),   // ← was plugin_events.clone() — must use .to_vec()
               config: p.config.clone(),
           };
           // ... rest of method unchanged
   ```

- [ ] **Step 1.3: Run tests to confirm fix**

```
cd /mnt/d/project/gloss && cargo test -p src-plugin 2>&1 | grep "test result"
cd /mnt/d/project/gloss && cargo test --workspace 2>&1 | grep -E "FAILED|error\[" | head -10
```

Expected: `test result: ok. 3 passed` for src-plugin, no FAILEDs workspace-wide.

- [ ] **Step 1.4: Commit**

```bash
cd /mnt/d/project/gloss
git add src-plugin/src/host.rs
git commit -m "$(cat <<'EOF'
fix(src-plugin): change run_lint_rule to accept &[PluginEvent] instead of &[Event]

AppCore already calls to_plugin_events() before invoking run_lint_rule via the
PluginHost trait. Remove the redundant internal conversion.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Create `src-desktop-native` crate

**Files:**
- Modify: `Cargo.toml` (root workspace)
- Create: `src-desktop-native/Cargo.toml`
- Create: `src-desktop-native/src/lib.rs`
- Create: `src-desktop-native/src/fs.rs`
- Create: `src-desktop-native/src/plugin_host.rs`
- Create: `src-desktop-native/src/config.rs`

- [ ] **Step 2.1: Add `src-desktop-native` to workspace**

Edit root `Cargo.toml`, add `"src-desktop-native"` to members:

```toml
[workspace]
members = [
    "src-core", "src-web", "src-cli",
    "src-plugin-types", "src-plugin",
    "src-desktop-types", "src-editor",
    "src-desktop-layout", "src-desktop-core",
    "src-desktop-native",
]
```

- [ ] **Step 2.2: Create `src-desktop-native/Cargo.toml`**

```toml
[package]
name = "src-desktop-native"
version = "0.1.0"
edition = "2021"

[dependencies]
src-desktop-types = { path = "../src-desktop-types" }
src-plugin         = { path = "../src-plugin" }
src-plugin-types   = { path = "../src-plugin-types" }
serde_json         = "1"
```

- [ ] **Step 2.3: Create `src-desktop-native/src/lib.rs`**

```rust
pub mod config;
pub mod fs;
pub mod plugin_host;

pub use config::load_app_config;
pub use fs::NativeFs;
pub use plugin_host::make_plugin_host;
```

- [ ] **Step 2.4: Write failing test for `NativeFs`**

Create `src-desktop-native/src/fs.rs` with just the test:

```rust
#[cfg(test)]
mod tests {
    use super::NativeFs;
    use src_desktop_types::{FileSystem, VfsPath};

    #[test]
    fn native_fs_write_and_read() {
        let mut fs = NativeFs;
        let dir = std::env::temp_dir();
        let path = VfsPath::from(
            dir.join("gloss_native_test.txt").to_string_lossy().as_ref()
        );
        fs.write(&path, b"hello").unwrap();
        let bytes = fs.read(&path).unwrap();
        assert_eq!(bytes, b"hello");
        fs.delete(&path).unwrap();
    }

    #[test]
    fn native_fs_exists_returns_false_for_missing() {
        let fs = NativeFs;
        assert!(!fs.exists(&VfsPath::from("/nonexistent/gloss_test_xyz.txt")));
    }

    #[test]
    fn native_fs_list_dir_returns_entries() {
        let mut fs = NativeFs;
        let dir = std::env::temp_dir();
        let sub = dir.join("gloss_list_test");
        let sub_path = VfsPath::from(sub.to_string_lossy().as_ref());
        fs.create_dir(&sub_path).unwrap();
        let file_path = VfsPath::from(sub.join("a.txt").to_string_lossy().as_ref());
        fs.write(&file_path, b"x").unwrap();
        let entries = fs.list_dir(&sub_path).unwrap();
        assert!(entries.iter().any(|e| e.name == "a.txt"));
        fs.delete(&sub_path).unwrap();
    }
}
```

Run: `cd /mnt/d/project/gloss && cargo test -p src-desktop-native 2>&1 | tail -5`

Expected: compile error — `NativeFs` not defined.

- [ ] **Step 2.5: Implement `src-desktop-native/src/fs.rs`**

```rust
use std::fs;
use src_desktop_types::{FileSystem, VfsPath, DirEntry, FsError};

pub struct NativeFs;

impl FileSystem for NativeFs {
    fn read(&self, path: &VfsPath) -> Result<Vec<u8>, FsError> {
        fs::read(path.as_str())
            .map_err(|e| map_io_err(e, path))
    }

    fn write(&mut self, path: &VfsPath, data: &[u8]) -> Result<(), FsError> {
        // Auto-create parent directories
        if let Some(parent) = std::path::Path::new(path.as_str()).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| FsError::Io(e.to_string()))?;
            }
        }
        fs::write(path.as_str(), data)
            .map_err(|e| map_io_err(e, path))
    }

    fn list_dir(&self, path: &VfsPath) -> Result<Vec<DirEntry>, FsError> {
        let rd = fs::read_dir(path.as_str())
            .map_err(|e| map_io_err(e, path))?;
        let mut result = Vec::new();
        for entry in rd {
            let entry = entry.map_err(|e| FsError::Io(e.to_string()))?;
            let name   = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let ep     = entry.path().to_string_lossy().into_owned();
            result.push(DirEntry { name, path: VfsPath::from(ep.as_str()), is_dir });
        }
        Ok(result)
    }

    fn exists(&self, path: &VfsPath) -> bool {
        std::path::Path::new(path.as_str()).exists()
    }

    fn create_dir(&mut self, path: &VfsPath) -> Result<(), FsError> {
        fs::create_dir_all(path.as_str())
            .map_err(|e| FsError::Io(e.to_string()))
    }

    fn delete(&mut self, path: &VfsPath) -> Result<(), FsError> {
        let p = std::path::Path::new(path.as_str());
        if p.is_dir() {
            fs::remove_dir_all(p).map_err(|e| FsError::Io(e.to_string()))
        } else {
            fs::remove_file(p).map_err(|e| map_io_err(e, path))
        }
    }

    fn rename(&mut self, from: &VfsPath, to: &VfsPath) -> Result<(), FsError> {
        fs::rename(from.as_str(), to.as_str())
            .map_err(|e| FsError::Io(e.to_string()))
    }

    fn is_dir(&self, path: &VfsPath) -> bool {
        std::path::Path::new(path.as_str()).is_dir()
    }
}

fn map_io_err(e: std::io::Error, path: &VfsPath) -> FsError {
    match e.kind() {
        std::io::ErrorKind::NotFound        => FsError::NotFound(path.clone()),
        std::io::ErrorKind::PermissionDenied => FsError::PermissionDenied,
        std::io::ErrorKind::AlreadyExists   => FsError::AlreadyExists(path.clone()),
        _                                   => FsError::Io(e.to_string()),
    }
}
```

- [ ] **Step 2.6: Run NativeFs tests**

```
cd /mnt/d/project/gloss && cargo test -p src-desktop-native 2>&1 | grep "test result"
```

Expected: `test result: ok. 3 passed` (NativeFs tests only — plugin_host tests don't exist yet).

- [ ] **Step 2.7: Write failing test for `make_plugin_host`**

Create `src-desktop-native/src/plugin_host.rs` with just the test:

```rust
#[cfg(test)]
mod tests {
    use src_desktop_types::{PluginHost, PluginEntrySpec, VfsPath};

    #[test]
    fn make_plugin_host_empty_specs_returns_noop_host() {
        let mut host = super::make_plugin_host(&[]);
        assert!(host.run_code_highlight("rust", "fn main(){}", "").is_none());
        assert!(host.run_card_link("https://example.com").is_none());
        assert!(host.run_front_matter(&[], "test.n.md").is_none());
        assert!(host.run_lint_rule("s", "m", &[], &[]).is_empty());
    }
}
```

Run: `cd /mnt/d/project/gloss && cargo test -p src-desktop-native make_plugin_host 2>&1 | tail -5`

Expected: compile error — `make_plugin_host` not defined.

- [ ] **Step 2.8: Implement `src-desktop-native/src/plugin_host.rs`**

```rust
use src_desktop_types::{PluginHost, PluginEntrySpec};
use src_plugin::host::GlossPluginHost;
use src_plugin_types::{CardLinkOutput, PluginEvent, PluginFrontMatterField, PluginWarning};

// ── PluginHost impl for GlossPluginHost ──────────────────────────────────────

impl PluginHost for GlossPluginHost {
    fn run_code_highlight(&mut self, lang: &str, code: &str, filename: &str) -> Option<String> {
        self.run_code_highlight(lang, code, filename)
    }

    fn run_card_link(&mut self, url: &str) -> Option<CardLinkOutput> {
        self.run_card_link(url)
    }

    fn run_lint_rule(
        &mut self,
        src: &str,
        md: &str,
        existing: &[PluginWarning],
        events: &[PluginEvent],
    ) -> Vec<PluginWarning> {
        self.run_lint_rule(src, md, existing, events)
    }

    fn run_front_matter(&mut self, fields: &[PluginFrontMatterField], src: &str) -> Option<String> {
        self.run_front_matter(fields, src)
    }
}

// ── Constructor ───────────────────────────────────────────────────────────────

/// Build a `GlossPluginHost` from `PluginEntrySpec` values (from `AppConfig`).
/// Plugins that fail to load are skipped with an error printed to stderr.
pub fn make_plugin_host(specs: &[PluginEntrySpec]) -> GlossPluginHost {
    let entries: Vec<src_plugin::config::PluginEntry> = specs.iter().map(|s| {
        src_plugin::config::PluginEntry {
            id:     s.id.clone(),
            path:   s.path.as_str().to_string(),
            hooks:  s.hooks.clone(),
            config: serde_json::from_str(&s.config).unwrap_or(serde_json::Value::Null),
        }
    }).collect();
    GlossPluginHost::new(&entries)
}

#[cfg(test)]
mod tests {
    use src_desktop_types::{PluginHost, PluginEntrySpec};

    #[test]
    fn make_plugin_host_empty_specs_returns_noop_host() {
        let mut host = super::make_plugin_host(&[]);
        assert!(host.run_code_highlight("rust", "fn main(){}", "").is_none());
        assert!(host.run_card_link("https://example.com").is_none());
        assert!(host.run_front_matter(&[], "test.n.md").is_none());
        assert!(host.run_lint_rule("s", "m", &[], &[]).is_empty());
    }
}
```

- [ ] **Step 2.9: Write failing test for `load_app_config`**

Create `src-desktop-native/src/config.rs` with just the test:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn load_app_config_missing_file_returns_default() {
        let cfg = super::load_app_config("/nonexistent/gloss_xyz.toml");
        assert!(cfg.plugins.is_empty());
    }

    #[test]
    fn load_app_config_parses_lint_rules() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "[lint]\nkanji-no-ruby = false").unwrap();
        let cfg = super::load_app_config(tmp.path().to_str().unwrap());
        assert_eq!(cfg.lint.is_enabled("kanji-no-ruby"), false);
        assert_eq!(cfg.lint.is_enabled("card-non-http"), true);
    }
}
```

Add `tempfile` to `[dev-dependencies]` in `src-desktop-native/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

Run: `cd /mnt/d/project/gloss && cargo test -p src-desktop-native load_app_config 2>&1 | tail -5`

Expected: compile error — `load_app_config` not defined.

- [ ] **Step 2.10: Implement `src-desktop-native/src/config.rs`**

```rust
use src_desktop_types::{AppConfig, LintRules, PluginEntrySpec, VfsPath};
use src_plugin::config::GlossConfig;

/// Load `AppConfig` from a TOML file (e.g. `gloss.toml`).
/// Missing file → default config (no plugins, all lint rules enabled).
/// Parse error → logs to stderr + default.
pub fn load_app_config(path: &str) -> AppConfig {
    let gc = GlossConfig::from_file(path);
    AppConfig {
        lint: LintRules(gc.lint.rules.into_iter().collect()),
        plugins: gc.plugins.into_iter().map(|p| PluginEntrySpec {
            id:     p.id,
            path:   VfsPath::from(p.path.as_str()),
            hooks:  p.hooks,
            config: serde_json::to_string(&p.config).unwrap_or_default(),
        }).collect(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn load_app_config_missing_file_returns_default() {
        let cfg = super::load_app_config("/nonexistent/gloss_xyz.toml");
        assert!(cfg.plugins.is_empty());
    }

    #[test]
    fn load_app_config_parses_lint_rules() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "[lint]\nkanji-no-ruby = false").unwrap();
        let cfg = super::load_app_config(tmp.path().to_str().unwrap());
        assert_eq!(cfg.lint.is_enabled("kanji-no-ruby"), false);
        assert_eq!(cfg.lint.is_enabled("card-non-http"), true);
    }
}
```

- [ ] **Step 2.11: Run all src-desktop-native tests**

```
cd /mnt/d/project/gloss && cargo test -p src-desktop-native 2>&1 | grep "test result"
```

Expected: `test result: ok. 6 passed` (3 NativeFs + 1 make_plugin_host + 2 load_app_config = 6 total).

If count differs, run without filter to see all tests:
```
cd /mnt/d/project/gloss && cargo test -p src-desktop-native 2>&1 | grep -E "^test |test result"
```

- [ ] **Step 2.12: Run full workspace tests**

```
cd /mnt/d/project/gloss && cargo test --workspace 2>&1 | grep -E "FAILED|error\[" | head -10
```

Expected: no FAILEDs, no errors.

- [ ] **Step 2.13: Commit**

```bash
cd /mnt/d/project/gloss
git add Cargo.toml Cargo.lock \
    src-desktop-native/Cargo.toml \
    src-desktop-native/src/lib.rs \
    src-desktop-native/src/fs.rs \
    src-desktop-native/src/plugin_host.rs \
    src-desktop-native/src/config.rs
git commit -m "$(cat <<'EOF'
feat(src-desktop-native): NativeFs, PluginHost impl, make_plugin_host, load_app_config

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```
