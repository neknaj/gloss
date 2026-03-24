// Shared types between plugin host (src-plugin) and plugin PDK authors.
// No Extism dependency — pure serde.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginWarning {
    pub code: String,
    pub message: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginFrontMatterField {
    pub key: String,
    pub raw: String,
}

// Note: adjacently tagged serde (`tag = "type", content = "data"`) does not
// support unit variants. We use internally tagged (`tag = "type"`) instead,
// which handles both struct variants and unit variants correctly.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum PluginEvent {
    Start { tag: String },
    End { tag: String },
    Text { content: String },
    MathInline { latex: String },
    MathDisplay { latex: String },
    FrontMatter { fields: Vec<PluginFrontMatterField> },
    CardLink { url: String },
    FootnoteRef { number: u32 },
    SoftBreak,
    HardBreak,
    Rule,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CodeHighlightInput {
    pub lang: String,      // "" when no language specifier
    pub code: String,      // raw (unescaped) source text
    pub filename: String,  // "" when no filename label
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CodeHighlightOutput {
    /// Returning `None` signals "I don't handle this — use default".
    pub html: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CardLinkInput {
    pub url: String,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CardLinkOutput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    /// Full HTML override. When Some, title/description/image_url are ignored.
    pub html: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LintRuleInput {
    pub source: String,
    pub markdown: String,
    pub existing_warnings: Vec<PluginWarning>,
    pub events: Vec<PluginEvent>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LintRuleOutput {
    pub warnings: Vec<PluginWarning>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FrontMatterInput {
    pub fields: Vec<PluginFrontMatterField>,
    pub source: String,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FrontMatterOutput {
    /// Returning `None` signals "use default rendering".
    pub html: Option<String>,
}
