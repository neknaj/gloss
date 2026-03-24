use std::collections::HashMap;
use serde::Deserialize;
use src_core::parser::FrontMatterField;

const KNOWN_LINT_CODES: &[&str] = &[
    "kanji-no-ruby", "ruby-kana-base", "ruby-kanji-reading", "ruby-katakana-hiragana",
    "ruby-empty-base", "ruby-empty-reading", "ruby-self-referential", "ruby-malformed",
    "anno-looks-like-ruby", "anno-empty-base", "anno-malformed",
    "math-unclosed-inline", "math-unclosed-display",
    "footnote-undefined-ref", "footnote-unused-def",
    "card-non-http", "card-malformed", "card-unknown-type",
];

#[derive(Debug, Clone, Default)]
pub struct LintConfig {
    pub rules: HashMap<String, bool>,
}

impl LintConfig {
    /// Returns `true` unless the rule is explicitly set to `false`.
    pub fn is_enabled(&self, code: &str) -> bool {
        self.rules.get(code).copied().unwrap_or(true)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PluginEntry {
    pub id: String,
    pub path: String,
    pub hooks: Vec<String>,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct GlossConfig {
    pub lint: LintConfig,
    pub plugins: Vec<PluginEntry>,
}

// ── TOML deserialization types ───────────────────────────────────────────────

#[derive(Deserialize)]
struct TomlRoot {
    #[serde(default)]
    lint: TomlLint,
    #[serde(default)]
    plugins: Vec<TomlPlugin>,
}

#[derive(Deserialize, Default)]
struct TomlLint {
    #[serde(flatten)]
    rules: HashMap<String, bool>,
}

#[derive(Deserialize)]
struct TomlPlugin {
    id: String,
    path: String,
    #[serde(default)]
    hooks: Vec<String>,
    #[serde(default)]
    config: serde_json::Value,
}

impl GlossConfig {
    /// Load from a TOML file. Missing file → default. Parse error → stderr + default.
    /// Unknown lint rule keys are printed to stderr and ignored (§5.5).
    pub fn from_file(path: &str) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => return Self::default(), // missing file is normal
        };
        match toml::from_str::<TomlRoot>(&text) {
            Ok(root) => {
                // Validate lint rule keys
                for key in root.lint.rules.keys() {
                    if !KNOWN_LINT_CODES.contains(&key.as_str()) {
                        eprintln!("[gloss-plugin] unknown lint rule: {key}");
                    }
                }
                Self {
                    lint: LintConfig { rules: root.lint.rules },
                    plugins: root.plugins.into_iter().map(|p| PluginEntry {
                        id: p.id,
                        path: p.path,
                        hooks: p.hooks,
                        config: p.config,
                    }).collect(),
                }
            },
            Err(e) => {
                eprintln!("[gloss-plugin] config error: {e}");
                Self::default()
            }
        }
    }

    /// Returns a new config with per-file front matter overrides applied.
    /// The `plugins` front matter key must be inline JSON:
    /// `{"lint":{"rule":false},"list":[...]}`
    pub fn with_front_matter_override(&self, fields: &[FrontMatterField<'_>]) -> Self {
        let mut result = self.clone();

        for field in fields {
            if field.key != "plugins" {
                continue;
            }
            let val: serde_json::Value = match serde_json::from_str(field.raw) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[gloss-plugin] front matter 'plugins' parse error: {e}");
                    continue;
                }
            };

            // Merge lint rules
            if let Some(lint_obj) = val.get("lint").and_then(|v| v.as_object()) {
                for (k, v) in lint_obj {
                    if let Some(enabled) = v.as_bool() {
                        result.lint.rules.insert(k.clone(), enabled);
                    } else {
                        eprintln!("[gloss-plugin] unknown lint rule value for '{k}': expected bool");
                    }
                }
            }

            // Replace plugin list if `list` key present
            if let Some(list) = val.get("list").and_then(|v| v.as_array()) {
                result.plugins = list.iter().filter_map(|entry| {
                    let id = entry.get("id")?.as_str()?.to_string();
                    let path = entry.get("path")?.as_str()?.to_string();
                    let hooks = entry.get("hooks")
                        .and_then(|h| h.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();
                    let config = entry.get("config").cloned().unwrap_or(serde_json::Value::Null);
                    Some(PluginEntry { id, path, hooks, config })
                }).collect();
            }
        }

        result
    }
}
