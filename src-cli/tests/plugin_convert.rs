use src_plugin::convert::{to_plugin_events, to_plugin_warnings, tag_to_string};
use src_plugin_types::PluginEvent;
use src_core::parser::{Event, Tag, Warning};

#[test]
fn converts_text_event() {
    let events = vec![Event::Text("hello")];
    let result = to_plugin_events(&events);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], PluginEvent::Text { content: "hello".to_string() });
}

#[test]
fn skips_block_id_events() {
    let events = vec![
        Event::BlockId(42),
        Event::Text("x"),
    ];
    let result = to_plugin_events(&events);
    assert_eq!(result.len(), 1); // BlockId dropped
}

#[test]
fn converts_warnings() {
    let w = Warning {
        code: "kanji-no-ruby",
        message: "kanji without ruby".to_string(),
        source: "test.n.md".to_string(),
        line: 3,
        col: 5,
    };
    let result = to_plugin_warnings(&[w]);
    assert_eq!(result[0].code, "kanji-no-ruby");
    assert_eq!(result[0].line, 3);
}

#[test]
fn tag_to_string_paragraph() {
    assert_eq!(tag_to_string(&Tag::Paragraph), "Paragraph");
}

#[test]
fn converts_card_link_event() {
    let events = vec![Event::CardLink("https://example.com")];
    let result = to_plugin_events(&events);
    assert_eq!(result[0], PluginEvent::CardLink { url: "https://example.com".to_string() });
}

#[test]
fn converts_front_matter_event() {
    use src_core::parser::FrontMatterField;
    let fields = vec![FrontMatterField { key: "title", raw: "\"My Post\"" }];
    let events = vec![Event::FrontMatter(fields)];
    let result = to_plugin_events(&events);
    match &result[0] {
        PluginEvent::FrontMatter { fields } => {
            assert_eq!(fields[0].key, "title");
            assert_eq!(fields[0].raw, "\"My Post\"");
        }
        other => panic!("Expected FrontMatter, got {:?}", other),
    }
}
