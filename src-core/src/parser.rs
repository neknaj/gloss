use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum Tag<'a> {
    Paragraph,
    Heading(u32),
    Section(u32),
    Ruby(&'a str),
    Gloss(Vec<&'a str>),
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
        
        let close_paragraph = |events: &mut Vec<Event<'a>>, in_p: &mut bool| {
            if *in_p {
                events.push(Event::End(Tag::Paragraph));
                *in_p = false;
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
                continue;
            }
            if tline.starts_with("#") {
                let bytes = tline.as_bytes();
                let mut level = 0;
                while level < bytes.len() && bytes[level] == b'#' {
                    level += 1;
                }
                if level > 0 && level <= 6 && (level == bytes.len() || bytes[level] == b' ') {
                    close_paragraph(&mut events, &mut in_paragraph);
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
            if line.starts_with("---") {
                if line.chars().all(|c| c == '-') {
                    close_paragraph(&mut events, &mut in_paragraph);
                    if line.len() == 3 {
                        // thematic break + 1 section close
                        events.push(Event::Rule);
                        pop_section(&mut events, &mut section_stack);
                    } else {
                        // Just regular rule if more than 3
                        events.push(Event::Rule);
                    }
                    continue;
                }
            }
            if line.starts_with(";;;") {
                close_paragraph(&mut events, &mut in_paragraph);
                let count = line.matches(";;;").count();
                for _ in 0..count {
                    pop_section(&mut events, &mut section_stack);
                }
                continue;
            }
            
            // Paragraph text
            if !in_paragraph {
                events.push(Event::Start(Tag::Paragraph));
                in_paragraph = true;
            } else {
                events.push(Event::SoftBreak);
            }
            parse_inline(line, &mut events);
        }
        
        close_paragraph(&mut events, &mut in_paragraph);
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
        let next_special = text.find(|c| c == '$' || c == '[' || c == '{').unwrap_or(text.len());
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
