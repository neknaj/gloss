use src_desktop_types::{AppConfig, LintRules, PluginEntrySpec, VfsPath};
use src_plugin::config::GlossConfig;

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
