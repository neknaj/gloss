use extism::{Plugin, Manifest, Wasm};
use src_plugin_types::{
    CodeHighlightInput, CodeHighlightOutput,
    CardLinkInput, CardLinkOutput,
    LintRuleInput, LintRuleOutput,
    FrontMatterInput, FrontMatterOutput,
    PluginWarning, PluginFrontMatterField,
};
use crate::convert::to_plugin_events;
use src_core::parser::Event;

pub struct LoadedPlugin {
    pub id: String,
    pub hooks: Vec<String>,
    pub config: serde_json::Value,
    pub instance: Plugin,
}

pub struct GlossPluginHost {
    pub plugins: Vec<LoadedPlugin>,
}

impl GlossPluginHost {
    /// Create a new host. Plugins that fail to load print an error and are skipped.
    ///
    /// Security settings per spec §6.1:
    /// - WASI disabled (wasi=false in Plugin::new)
    /// - 16 MB memory limit per plugin (256 Wasm pages of 64 KiB each)
    /// - Network access blocked via disallow_all_hosts()
    pub fn new(entries: &[crate::config::PluginEntry]) -> Self {
        let mut plugins = Vec::new();
        for entry in entries {
            let wasm = Wasm::file(&entry.path);
            let manifest = Manifest::new([wasm])
                .with_memory_max(256)
                .disallow_all_hosts();
            match Plugin::new(manifest, [], false) {
                Ok(instance) => plugins.push(LoadedPlugin {
                    id: entry.id.clone(),
                    hooks: entry.hooks.clone(),
                    config: entry.config.clone(),
                    instance,
                }),
                Err(e) => {
                    eprintln!("[gloss-plugin:{}] load failed: {e}", entry.id);
                }
            }
        }
        Self { plugins }
    }

    /// `code-highlight` hook — first-wins. Returns `None` if no plugin handles it.
    pub fn run_code_highlight(
        &mut self,
        lang: &str,
        code: &str,
        filename: &str,
        config: serde_json::Value,
    ) -> Option<String> {
        let input = CodeHighlightInput {
            lang: lang.to_string(),
            code: code.to_string(),
            filename: filename.to_string(),
            config,
        };
        let json = serde_json::to_string(&input).ok()?;

        for p in &mut self.plugins {
            if !p.hooks.iter().any(|h| h == "code-highlight") {
                continue;
            }
            match p.instance.call::<&str, String>("code_highlight", &json) {
                Ok(raw) => {
                    match serde_json::from_str::<CodeHighlightOutput>(&raw) {
                        Ok(out) => if out.html.is_some() { return out.html; }
                        Err(e) => eprintln!("[gloss-plugin:{}] code-highlight failed: {e}", p.id),
                    }
                }
                Err(e) => eprintln!("[gloss-plugin:{}] code-highlight failed: {e}", p.id),
            }
        }
        None
    }

    /// `card-link` hook — first-wins. Returns `None` if no plugin handles it.
    pub fn run_card_link(
        &mut self,
        url: &str,
        config: serde_json::Value,
    ) -> Option<CardLinkOutput> {
        let input = CardLinkInput { url: url.to_string(), config };
        let json = serde_json::to_string(&input).ok()?;

        for p in &mut self.plugins {
            if !p.hooks.iter().any(|h| h == "card-link") {
                continue;
            }
            match p.instance.call::<&str, String>("card_link", &json) {
                Ok(raw) => {
                    match serde_json::from_str::<CardLinkOutput>(&raw) {
                        Ok(out) => {
                            if out.html.is_some()
                                || out.title.is_some()
                                || out.description.is_some()
                                || out.image_url.is_some()
                            {
                                return Some(out);
                            }
                        }
                        Err(e) => eprintln!("[gloss-plugin:{}] card-link failed: {e}", p.id),
                    }
                }
                Err(e) => eprintln!("[gloss-plugin:{}] card-link failed: {e}", p.id),
            }
        }
        None
    }

    /// `lint-rule` hook — all plugins run, warnings merged.
    pub fn run_lint_rule<'a>(
        &mut self,
        source: &str,
        markdown: &str,
        existing_warnings: &[PluginWarning],
        events: &[Event<'a>],
    ) -> Vec<PluginWarning> {
        let plugin_events = to_plugin_events(events);
        let input = LintRuleInput {
            source: source.to_string(),
            markdown: markdown.to_string(),
            existing_warnings: existing_warnings.to_vec(),
            events: plugin_events,
        };
        let json = match serde_json::to_string(&input) {
            Ok(j) => j,
            Err(_) => return vec![],
        };
        let mut all_warnings = Vec::new();
        for p in &mut self.plugins {
            if !p.hooks.iter().any(|h| h == "lint-rule") {
                continue;
            }
            match p.instance.call::<&str, String>("lint_rule", &json) {
                Ok(raw) => {
                    match serde_json::from_str::<LintRuleOutput>(&raw) {
                        Ok(out) => all_warnings.extend(out.warnings),
                        Err(e) => eprintln!("[gloss-plugin:{}] lint-rule failed: {e}", p.id),
                    }
                }
                Err(e) => eprintln!("[gloss-plugin:{}] lint-rule failed: {e}", p.id),
            }
        }
        all_warnings
    }

    /// `front-matter` hook — first-wins. Returns `None` if no plugin handles it.
    pub fn run_front_matter(
        &mut self,
        fields: &[PluginFrontMatterField],
        source: &str,
        config: serde_json::Value,
    ) -> Option<String> {
        let input = FrontMatterInput {
            fields: fields.to_vec(),
            source: source.to_string(),
            config,
        };
        let json = serde_json::to_string(&input).ok()?;

        for p in &mut self.plugins {
            if !p.hooks.iter().any(|h| h == "front-matter") {
                continue;
            }
            match p.instance.call::<&str, String>("front_matter", &json) {
                Ok(raw) => {
                    match serde_json::from_str::<FrontMatterOutput>(&raw) {
                        Ok(out) => if out.html.is_some() { return out.html; }
                        Err(e) => eprintln!("[gloss-plugin:{}] front-matter failed: {e}", p.id),
                    }
                }
                Err(e) => eprintln!("[gloss-plugin:{}] front-matter failed: {e}", p.id),
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_host_returns_none_for_all_hooks() {
        let mut host = GlossPluginHost { plugins: vec![] };
        assert!(host.run_code_highlight("rust", "fn main(){}", "", serde_json::Value::Null).is_none());
        assert!(host.run_card_link("https://example.com", serde_json::Value::Null).is_none());
        assert!(host.run_front_matter(&[], "", serde_json::Value::Null).is_none());
    }

    #[test]
    fn empty_host_lint_returns_empty() {
        let mut host = GlossPluginHost { plugins: vec![] };
        let result = host.run_lint_rule("test.n.md", "# hi", &[], &[]);
        assert!(result.is_empty());
    }
}
