use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum Tag<'a> {
    Paragraph,
    Heading(u32),
    Section(u32),
    Ruby(&'a str),
    Gloss(Vec<&'a str>),
    List(bool), // true = ordered
    Item,
    Code,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    Start(Tag<'a>),
    End(Tag<'a>),
    Text(&'a str),
    MathDisplay(&'a str),
    MathInline(&'a str),
    SoftBreak,
    HardBreak,
    Rule,
}

pub struct Parser<'a> {
    events: alloc::vec::IntoIter<Event<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Self {
        let mut events = Vec::new();
        // Simplified block parsing
        let lines: Vec<&str> = text.lines().collect();
        let mut section_stack = Vec::new();
        let mut in_paragraph = false;
        let mut in_list: Option<bool> = None;
        
        let close_paragraph = |events: &mut Vec<Event<'a>>, in_p: &mut bool| {
            if *in_p {
                events.push(Event::End(Tag::Paragraph));
                *in_p = false;
            }
        };

        let close_list = |events: &mut Vec<Event<'a>>, in_l: &mut Option<bool>| {
            if let Some(ordered) = *in_l {
                events.push(Event::End(Tag::List(ordered)));
                *in_l = None;
            }
        };

        let pop_section = |events: &mut Vec<Event<'a>>, stack: &mut Vec<u32>| {
            if let Some(level) = stack.pop() {
                events.push(Event::End(Tag::Section(level)));
            }
        };

        let close_sections_until = |events: &mut Vec<Event<'a>>, stack: &mut Vec<u32>, level: u32| {
            while let Some(&top) = stack.last() {
                if top >= level {
                    pop_section(events, stack);
                } else {
                    break;
                }
            }
        };

        for line in lines {
            let tline = line.trim_start();
            if tline.is_empty() {
                close_paragraph(&mut events, &mut in_paragraph);
                close_list(&mut events, &mut in_list);
                continue;
            }
            
            // Check headings
            if tline.starts_with("#") {
                let bytes = tline.as_bytes();
                let mut level = 0;
                while level < bytes.len() && bytes[level] == b'#' {
                    level += 1;
                }
                if level > 0 && level <= 6 && (level == bytes.len() || bytes[level] == b' ') {
                    close_paragraph(&mut events, &mut in_paragraph);
                    close_list(&mut events, &mut in_list);
                    close_sections_until(&mut events, &mut section_stack, level as u32);
                    events.push(Event::Start(Tag::Section(level as u32)));
                    section_stack.push(level as u32);
                    
                    let content = tline[level..].trim();
                    events.push(Event::Start(Tag::Heading(level as u32)));
                    parse_inline(content, &mut events);
                    events.push(Event::End(Tag::Heading(level as u32)));
                    continue;
                }
            }
            
            // Checks rules
            if line.starts_with("---") {
                if line.chars().all(|c| c == '-') {
                    close_paragraph(&mut events, &mut in_paragraph);
                    close_list(&mut events, &mut in_list);
                    if line.len() == 3 {
                        // thematic break + 1 section close
                        pop_section(&mut events, &mut section_stack);
                        events.push(Event::Rule);
                    } else {
                        // Just regular rule if more than 3
                        events.push(Event::Rule);
                    }
                    continue;
                }
            }
            
            if line.starts_with(";;;") {
                close_paragraph(&mut events, &mut in_paragraph);
                close_list(&mut events, &mut in_list);
                let count = line.matches(";;;").count();
                for _ in 0..count {
                    pop_section(&mut events, &mut section_stack);
                }
                continue;
            }

            // Lists
            let is_unordered_item = tline.starts_with("- ") || tline.starts_with("* ");
            let digits_count = tline.chars().take_while(|c| c.is_ascii_digit()).count();
            let is_ordered_item = digits_count > 0 && tline[digits_count..].starts_with(". ");

            if is_unordered_item || is_ordered_item {
                close_paragraph(&mut events, &mut in_paragraph);
                let ordered = is_ordered_item;
                if in_list != Some(ordered) {
                    close_list(&mut events, &mut in_list);
                    events.push(Event::Start(Tag::List(ordered)));
                    in_list = Some(ordered);
                }
                events.push(Event::Start(Tag::Item));
                
                let content = if is_unordered_item {
                    &tline[2..]
                } else {
                    &tline[digits_count + 2..]
                };
                parse_inline(content, &mut events);
                events.push(Event::End(Tag::Item));
                continue;
            } else if !tline.is_empty() {
                // If not deeply indented, end list
                if in_list.is_some() && !line.starts_with("  ") && !line.starts_with('\t') {
                    close_list(&mut events, &mut in_list);
                }
            }
            
            // Paragraph text
            if !in_paragraph && in_list.is_none() {
                events.push(Event::Start(Tag::Paragraph));
                in_paragraph = true;
            } else if in_paragraph {
                // .n.md specifies literal line breaks become actual line breaks `<br>`
                events.push(Event::HardBreak);
            }
            
            parse_inline(tline, &mut events);
        }
        
        close_paragraph(&mut events, &mut in_paragraph);
        close_list(&mut events, &mut in_list);
        while !section_stack.is_empty() {
            pop_section(&mut events, &mut section_stack);
        }

        Parser {
            events: events.into_iter(),
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Event<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.events.next()
    }
}

fn parse_inline<'a>(mut text: &'a str, events: &mut Vec<Event<'a>>) {
    while !text.is_empty() {
        if text.starts_with("$$") {
            if let Some(end) = text[2..].find("$$") {
                events.push(Event::MathDisplay(&text[2..2 + end]));
                text = &text[2 + end + 2..];
                continue;
            }
        }
        if text.starts_with('$') {
            if let Some(end) = text[1..].find('$') {
                events.push(Event::MathInline(&text[1..1 + end]));
                text = &text[1 + end + 1..];
                continue;
            }
        }
        if text.starts_with('`') {
            if let Some(end) = text[1..].find('`') {
                events.push(Event::Start(Tag::Code));
                events.push(Event::Text(&text[1..1 + end]));
                events.push(Event::End(Tag::Code));
                text = &text[1 + end + 1..];
                continue;
            }
        }
        if text.starts_with("\\n") {
            events.push(Event::HardBreak);
            text = &text[2..];
            continue;
        }
        if text.starts_with('\\') && text.len() >= 2 {
            let ch = text[1..].chars().next().unwrap();
            let len = ch.len_utf8();
            events.push(Event::Text(&text[1..1 + len]));
            text = &text[1 + len..];
            continue;
        }
        if text.starts_with('[') {
            if let Some(end) = text[1..].find(']') {
                let content = &text[1..1 + end];
                if let Some(slash) = content.find('/') {
                    let base = &content[..slash];
                    let ruby = &content[slash + 1..];
                    events.push(Event::Start(Tag::Ruby(ruby)));
                    parse_inline(base, events); // Recurse inside base if needed
                    events.push(Event::End(Tag::Ruby(ruby)));
                    text = &text[1 + end + 1..];
                    continue;
                }
            }
        }
        if text.starts_with('{') {
            if let Some(end) = text[1..].find('}') {
                let content = &text[1..1 + end];
                let parts: Vec<&str> = content.split('/').collect();
                if parts.len() >= 2 {
                    let base = parts[0];
                    let glosses = parts[1..].to_vec();
                    events.push(Event::Start(Tag::Gloss(glosses.clone())));
                    parse_inline(base, events);
                    events.push(Event::End(Tag::Gloss(glosses)));
                    text = &text[1 + end + 1..];
                    continue;
                }
            }
        }
        
        // Find next special char
        let next_special = text.find(|c| c == '$' || c == '[' || c == '{' || c == '`' || c == '\\').unwrap_or(text.len());
        if next_special == 0 {
            // Unmatched special char
            let ch = text.chars().next().unwrap();
            let len = ch.len_utf8();
            events.push(Event::Text(&text[..len]));
            text = &text[len..];
        } else {
            events.push(Event::Text(&text[..next_special]));
            text = &text[next_special..];
        }
    }
}
