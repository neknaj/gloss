use src_desktop_types::{PluginHost, PluginEntrySpec};
use src_plugin::host::GlossPluginHost;
use src_plugin_types::{CardLinkOutput, PluginEvent, PluginFrontMatterField, PluginWarning};

/// Newtype wrapper that bridges GlossPluginHost to the PluginHost trait.
pub struct NativePluginHost(pub GlossPluginHost);

impl PluginHost for NativePluginHost {
    fn run_code_highlight(&mut self, lang: &str, code: &str, filename: &str) -> Option<String> {
        self.0.run_code_highlight(lang, code, filename)
    }

    fn run_card_link(&mut self, url: &str) -> Option<CardLinkOutput> {
        self.0.run_card_link(url)
    }

    fn run_lint_rule(
        &mut self,
        src: &str,
        md: &str,
        existing: &[PluginWarning],
        events: &[PluginEvent],
    ) -> Vec<PluginWarning> {
        self.0.run_lint_rule(src, md, existing, events)
    }

    fn run_front_matter(&mut self, fields: &[PluginFrontMatterField], src: &str) -> Option<String> {
        self.0.run_front_matter(fields, src)
    }
}

pub fn make_plugin_host(specs: &[PluginEntrySpec]) -> NativePluginHost {
    let entries: Vec<src_plugin::config::PluginEntry> = specs.iter().map(|s| {
        src_plugin::config::PluginEntry {
            id:     s.id.clone(),
            path:   s.path.as_str().to_string(),
            hooks:  s.hooks.clone(),
            config: serde_json::from_str(&s.config).unwrap_or(serde_json::Value::Null),
        }
    }).collect();
    NativePluginHost(GlossPluginHost::new(&entries))
}

#[cfg(test)]
mod tests {
    use src_desktop_types::PluginHost;

    #[test]
    fn make_plugin_host_empty_specs_returns_noop_host() {
        let mut host = super::make_plugin_host(&[]);
        assert!(host.run_code_highlight("rust", "fn main(){}", "").is_none());
        assert!(host.run_card_link("https://example.com").is_none());
        assert!(host.run_front_matter(&[], "test.n.md").is_none());
        assert!(host.run_lint_rule("s", "m", &[], &[]).is_empty());
    }
}
