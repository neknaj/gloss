use src_plugin::config::{GlossConfig, LintConfig};
use std::collections::HashMap;

#[test]
fn default_config_enables_all_lint() {
    let cfg = GlossConfig::default();
    assert!(cfg.lint.is_enabled("kanji-no-ruby"));
    assert!(cfg.lint.is_enabled("anything-unknown"));
}

#[test]
fn lint_config_disables_specific_rule() {
    let mut rules = HashMap::new();
    rules.insert("kanji-no-ruby".to_string(), false);
    let lint = LintConfig { rules };
    assert!(!lint.is_enabled("kanji-no-ruby"));
    assert!(lint.is_enabled("ruby-malformed")); // not in map → enabled
}

#[test]
fn config_from_missing_file_returns_default() {
    let cfg = GlossConfig::from_file("/nonexistent/gloss.toml");
    assert!(cfg.plugins.is_empty());
}

#[test]
fn front_matter_override_merges_lint() {
    use src_core::parser::FrontMatterField;
    let mut cfg = GlossConfig::default();
    cfg.lint.rules.insert("kanji-no-ruby".to_string(), true);

    let fields = vec![
        FrontMatterField { key: "plugins", raw: r#"{"lint":{"kanji-no-ruby":false}}"# },
    ];
    let merged = cfg.with_front_matter_override(&fields);
    assert!(!merged.lint.is_enabled("kanji-no-ruby"));
}

#[test]
fn front_matter_override_replaces_plugin_list_when_list_key_present() {
    use src_core::parser::FrontMatterField;
    let cfg = GlossConfig::default();
    let fields = vec![
        FrontMatterField { key: "plugins", raw: r#"{"list":[]}"# },
    ];
    let merged = cfg.with_front_matter_override(&fields);
    assert!(merged.plugins.is_empty());
}

#[test]
fn config_parse_error_returns_default() {
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(f, "not valid toml [[[").unwrap();
    let cfg = GlossConfig::from_file(f.path().to_str().unwrap());
    assert!(cfg.plugins.is_empty()); // fell back to default
}

#[test]
fn from_file_parses_toml_lint_rules() {
    // Verifies actual TOML format: lint rules are direct keys under [lint]
    // (e.g. `[lint]\nkanji-no-ruby = false`)
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(f, "[lint]\nkanji-no-ruby = false\nruby-malformed = false\n").unwrap();
    let cfg = GlossConfig::from_file(f.path().to_str().unwrap());
    assert!(!cfg.lint.is_enabled("kanji-no-ruby"));
    assert!(!cfg.lint.is_enabled("ruby-malformed"));
    assert!(cfg.lint.is_enabled("anno-malformed")); // not in file → enabled
}
