use src_core::parser::{Event, Tag, Warning};
use src_plugin_types::{PluginEvent, PluginFrontMatterField, PluginWarning};

/// Convert a slice of core events to plugin events.
/// `BlockId` events are dropped (internal renderer state, not meaningful to plugins).
pub fn to_plugin_events<'a>(events: &[Event<'a>]) -> Vec<PluginEvent> {
    events.iter().filter_map(|e| to_plugin_event(e)).collect()
}

fn to_plugin_event<'a>(event: &Event<'a>) -> Option<PluginEvent> {
    Some(match event {
        Event::Text(t)           => PluginEvent::Text { content: t.to_string() },
        Event::SoftBreak         => PluginEvent::SoftBreak,
        Event::HardBreak         => PluginEvent::HardBreak,
        Event::Rule              => PluginEvent::Rule,
        Event::MathInline(m)     => PluginEvent::MathInline { latex: m.to_string() },
        Event::MathDisplay(m)    => PluginEvent::MathDisplay { latex: m.to_string() },
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
        Event::BlockId(_)        => return None,  // internal — drop
    })
}

/// Convert core `Warning` slice to plugin `PluginWarning` vec.
pub fn to_plugin_warnings(warnings: &[Warning]) -> Vec<PluginWarning> {
    warnings.iter().map(|w| PluginWarning {
        code: w.code.to_string(),
        message: w.message.clone(),
        line: w.line,
        col: w.col,
    }).collect()
}

/// Produce a human-readable string for a `Tag` (used in `PluginEvent`).
pub fn tag_to_string(tag: &Tag) -> String {
    match tag {
        Tag::Paragraph           => "Paragraph".to_string(),
        Tag::Heading(n)          => format!("Heading({n})"),
        Tag::Section(n)          => format!("Section({n})"),
        Tag::List(true)          => "OrderedList".to_string(),
        Tag::List(false)         => "UnorderedList".to_string(),
        Tag::Item                => "Item".to_string(),
        Tag::Code                => "Code".to_string(),
        Tag::CodeBlock(l, f)     => format!("CodeBlock({l},{f})"),
        Tag::Blockquote          => "Blockquote".to_string(),
        Tag::Table(_)            => "Table".to_string(),
        Tag::TableHead           => "TableHead".to_string(),
        Tag::TableRow            => "TableRow".to_string(),
        Tag::TableCell(_)        => "TableCell".to_string(),
        Tag::Strong              => "Strong".to_string(),
        Tag::Emphasis            => "Emphasis".to_string(),
        Tag::Strikethrough       => "Strikethrough".to_string(),
        Tag::Link(_)             => "Link".to_string(),
        Tag::Image(_, _)         => "Image".to_string(),
        Tag::Ruby(_)             => "Ruby".to_string(),
        Tag::Anno(_)             => "Anno".to_string(),
        Tag::AnnoNote            => "AnnoNote".to_string(),
        Tag::FootnoteSection     => "FootnoteSection".to_string(),
        Tag::FootnoteItem(n)     => format!("FootnoteItem({n})"),
    }
}
