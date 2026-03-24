use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::path::VfsPath;

#[derive(Clone, Debug)]
pub struct LintRules(pub BTreeMap<String, bool>);

impl LintRules {
    pub fn is_enabled(&self, code: &str) -> bool {
        self.0.get(code).copied().unwrap_or(true)
    }
}

impl Default for LintRules {
    fn default() -> Self { LintRules(BTreeMap::new()) }
}

#[derive(Clone, Debug)]
pub struct PluginEntrySpec {
    pub id: String, pub path: VfsPath, pub hooks: Vec<String>,
    pub config: String, // raw JSON string
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub lint: LintRules,
    pub plugins: Vec<PluginEntrySpec>,
}

impl Default for AppConfig {
    fn default() -> Self { AppConfig { lint: LintRules::default(), plugins: Vec::new() } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_rules_default_enabled() {
        assert!(LintRules::default().is_enabled("W001"));
    }

    #[test]
    fn lint_rules_explicit_disable() {
        let mut map = BTreeMap::new();
        map.insert("W001".into(), false);
        let rules = LintRules(map);
        assert!(!rules.is_enabled("W001"));
        assert!(rules.is_enabled("W002"));
    }
}
