// Re-export conversion utilities that have moved to src-plugin-types.
pub use src_plugin_types::to_plugin_events;
pub use src_plugin_types::tag_to_string;

use src_plugin_types::PluginWarning;
use src_core::parser::Warning;

/// Convert core `Warning` slice to plugin `PluginWarning` vec.
pub fn to_plugin_warnings(warnings: &[Warning]) -> Vec<PluginWarning> {
    warnings.iter().map(|w| PluginWarning {
        code:    w.code.to_string(),
        message: w.message.clone(),
        line:    w.line,
        col:     w.col,
    }).collect()
}
