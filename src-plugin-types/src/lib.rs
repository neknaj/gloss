#![no_std]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;

use serde::{Deserialize, Serialize};
use src_core::parser::{Event, Tag};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PluginWarning {
    pub code:    String,
    pub message: String,
    pub line:    u32,
    pub col:     u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PluginFrontMatterField {
    pub key: String,
    pub raw: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum PluginEvent {
    Start        { tag: String },
    End          { tag: String },
    Text         { content: String },
    MathInline   { latex: String },
    MathDisplay  { latex: String },
    FrontMatter  { fields: Vec<PluginFrontMatterField> },
    CardLink     { url: String },
    FootnoteRef  { number: u32 },
    SoftBreak,
    HardBreak,
    Rule,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CodeHighlightInput {
    pub lang:     String,
    pub code:     String,
    pub filename: String,
    pub config:   serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CodeHighlightOutput {
    /// None signals "I don't handle this — use default".
    pub html: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CardLinkInput {
    pub url:    String,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CardLinkOutput {
    pub title:       Option<String>,
    pub description: Option<String>,
    pub image_url:   Option<String>,
    /// Full HTML override. When Some, title/description/image_url are ignored.
    pub html:        Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LintRuleInput {
    pub source:             String,
    pub markdown:           String,
    pub existing_warnings:  Vec<PluginWarning>,   // keep field name (used by src-plugin)
    pub events:             Vec<PluginEvent>,
    pub config:             serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LintRuleOutput {
    pub warnings: Vec<PluginWarning>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FrontMatterInput {
    pub fields: Vec<PluginFrontMatterField>,
    pub source: String,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FrontMatterOutput {
    /// None = use default rendering. Keep field name `html` (used by src-plugin).
    pub html: Option<String>,
}

// ── Conversion: src-core Event → PluginEvent ──────────────────────────────

/// Convert a slice of core `Event`s to `PluginEvent`s.
/// Used by AppCore before calling `PluginHost::run_lint_rule`.
/// `BlockId` events are dropped (internal renderer state, not meaningful to plugins).
pub fn to_plugin_events(events: &[Event<'_>]) -> Vec<PluginEvent> {
    events.iter().filter_map(to_plugin_event).collect()
}

fn to_plugin_event(event: &Event<'_>) -> Option<PluginEvent> {
    Some(match event {
        Event::Text(t)           => PluginEvent::Text { content: t.to_string() },
        Event::SoftBreak         => PluginEvent::SoftBreak,
        Event::HardBreak         => PluginEvent::HardBreak,
        Event::Rule              => PluginEvent::Rule,
        Event::MathInline(m)    => PluginEvent::MathInline { latex: m.to_string() },
        Event::MathDisplay(m)   => PluginEvent::MathDisplay { latex: m.to_string() },
        Event::CardLink(url)     => PluginEvent::CardLink { url: url.to_string() },
        Event::FootnoteRef(n)    => PluginEvent::FootnoteRef { number: *n },
        Event::FrontMatter(flds) => PluginEvent::FrontMatter {
            fields: flds.iter().map(|f| PluginFrontMatterField {
                key: f.key.to_string(),
                raw: f.raw.to_string(),
            }).collect(),
        },
        Event::Start(tag)        => PluginEvent::Start { tag: tag_to_string(tag) },
        Event::End(tag)          => PluginEvent::End   { tag: tag_to_string(tag) },
        Event::BlockId(_)        => return None,
    })
}

pub fn tag_to_string(tag: &Tag) -> String {
    match tag {
        Tag::Paragraph           => "Paragraph".into(),
        Tag::Heading(n)          => format!("Heading({n})"),
        Tag::Section(n)          => format!("Section({n})"),
        Tag::List(true)          => "OrderedList".into(),
        Tag::List(false)         => "UnorderedList".into(),
        Tag::Item                => "Item".into(),
        Tag::Code                => "Code".into(),
        Tag::CodeBlock(l, f)     => format!("CodeBlock({l},{f})"),
        Tag::Blockquote          => "Blockquote".into(),
        Tag::Table(_)            => "Table".into(),
        Tag::TableHead           => "TableHead".into(),
        Tag::TableRow            => "TableRow".into(),
        Tag::TableCell(_)        => "TableCell".into(),
        Tag::Strong              => "Strong".into(),
        Tag::Emphasis            => "Emphasis".into(),
        Tag::Strikethrough       => "Strikethrough".into(),
        Tag::Link(_)             => "Link".into(),
        Tag::Image(_, _)         => "Image".into(),
        Tag::Ruby(_)             => "Ruby".into(),
        Tag::Anno(_)             => "Anno".into(),
        Tag::AnnoNote            => "AnnoNote".into(),
        Tag::FootnoteSection     => "FootnoteSection".into(),
        Tag::FootnoteItem(n)     => format!("FootnoteItem({n})"),
    }
}
