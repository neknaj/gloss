use alloc::vec::Vec;
use alloc::string::String;
use src_plugin_types::{CardLinkOutput, PluginWarning, PluginEvent, PluginFrontMatterField};
use crate::traits::PluginHost;

/// Stub that does nothing. For tests and WASM playground (no native plugin runtime).
pub struct NoopPluginHost;

impl PluginHost for NoopPluginHost {
    fn run_code_highlight(&mut self, _: &str, _: &str, _: &str) -> Option<String> { None }
    fn run_card_link(&mut self, _: &str) -> Option<CardLinkOutput> { None }
    fn run_lint_rule(&mut self, _: &str, _: &str, _: &[PluginWarning], _: &[PluginEvent]) -> Vec<PluginWarning> { Vec::new() }
    fn run_front_matter(&mut self, _: &[PluginFrontMatterField], _: &str) -> Option<String> { None }
}
