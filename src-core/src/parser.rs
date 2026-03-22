use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tag<'a> {
    Paragraph,
    Heading(u32),
    Section(u32),
    Ruby(&'a str),
    Gloss(Vec<&'a str>),
    List(bool),
    Item,
    Code,
    CodeBlock(&'a str),
    Blockquote,
    Table(Vec<Alignment>),
    TableHead,
    TableRow,
    TableCell(Alignment),
    Strong,
    Emphasis,
    Strikethrough,
    Link(&'a str),
    Image(&'a str, &'a str),
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
        let lines: Vec<&str> = text.lines().collect();
        let mut events = Vec::new();
        parse_blocks(&lines, &mut events, true);
        Parser { events: events.into_iter() }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Event<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.events.next()
    }
}

fn parse_blocks<'a>(lines: &[&'a str], events: &mut Vec<Event<'a>>, root: bool) {
    let mut i = 0;
    let mut section_stack: Vec<u32> = Vec::new();

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

    while i < lines.len() {
        let line = lines[i];
        let tline = line.trim_start();

        // Blank
        if tline.is_empty() {
            i += 1;
            continue;
        }

        // Code block
        if tline.starts_with("```") {
            let lang = tline[3..].trim();
            events.push(Event::Start(Tag::CodeBlock(lang)));
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                events.push(Event::Text(lines[i]));
                events.push(Event::HardBreak);
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            events.push(Event::End(Tag::CodeBlock(lang)));
            continue;
        }

        // Headings
        if tline.starts_with("#") {
            let bytes = tline.as_bytes();
            let mut level = 0;
            while level < bytes.len() && bytes[level] == b'#' {
                level += 1;
            }
            if level > 0 && level <= 6 && (level == bytes.len() || bytes[level] == b' ') {
                if root {
                    close_sections_until(events, &mut section_stack, level as u32);
                    events.push(Event::Start(Tag::Section(level as u32)));
                    section_stack.push(level as u32);
                }
                events.push(Event::Start(Tag::Heading(level as u32)));
                parse_inline(tline[level..].trim(), events);
                events.push(Event::End(Tag::Heading(level as u32)));
                i += 1;
                continue;
            }
        }

        // Thematic break
        if line.starts_with("---") && line.chars().all(|c| c == '-') {
            if root && line.len() == 3 {
                pop_section(events, &mut section_stack);
            }
            events.push(Event::Rule);
            i += 1;
            continue;
        }

        // Section close (;;;)
        if line.starts_with(";;;") {
            if root {
                let count = line.matches(";;;").count();
                for _ in 0..count {
                    pop_section(events, &mut section_stack);
                }
            }
            i += 1;
            continue;
        }

        // Blockquote
        if line.starts_with('>') {
            let mut bq_lines = Vec::new();
            let mut j = i;
            while j < lines.len() {
                let ln = lines[j];
                if ln.starts_with('>') {
                    let mut content = &ln[1..];
                    if content.starts_with(' ') { content = &content[1..]; }
                    bq_lines.push(content);
                    j += 1;
                } else if ln.trim().is_empty() && j > i && j + 1 < lines.len() && lines[j + 1].starts_with('>') {
                    bq_lines.push("");
                    j += 1;
                } else {
                    break;
                }
            }
            events.push(Event::Start(Tag::Blockquote));
            parse_blocks(&bq_lines, events, false);
            events.push(Event::End(Tag::Blockquote));
            i = j;
            continue;
        }

        // Table
        let is_table_line = |l: &str| l.trim_start().starts_with('|');
        if is_table_line(line) && i + 1 < lines.len() && is_table_line(lines[i + 1]) {
            let sep_line = lines[i + 1].trim();
            // simple check for separator `|---|`
            if sep_line.contains("-|") || sep_line.contains("|-") {
                let parse_cells = |l: &'a str| -> Vec<&'a str> {
                    let t = l.trim();
                    let t = if t.starts_with('|') { &t[1..] } else { t };
                    let t = if t.ends_with('|') { &t[..t.len()-1] } else { t };
                    t.split('|').map(|s| s.trim()).collect()
                };
                let head = parse_cells(line);
                let sep = parse_cells(lines[i + 1]);
                let aligns: Vec<Alignment> = sep.iter().map(|s| {
                    let s = s.trim();
                    let left = s.starts_with(':');
                    let right = s.ends_with(':');
                    if left && right { Alignment::Center }
                    else if left { Alignment::Left }
                    else if right { Alignment::Right }
                    else { Alignment::None }
                }).collect();

                events.push(Event::Start(Tag::Table(aligns.clone())));
                events.push(Event::Start(Tag::TableHead));
                events.push(Event::Start(Tag::TableRow));
                for (ci, cell) in head.iter().enumerate() {
                    let a = aligns.get(ci).cloned().unwrap_or(Alignment::None);
                    events.push(Event::Start(Tag::TableCell(a.clone())));
                    parse_inline(cell, events);
                    events.push(Event::End(Tag::TableCell(a)));
                }
                events.push(Event::End(Tag::TableRow));
                events.push(Event::End(Tag::TableHead));

                let mut j = i + 2;
                while j < lines.len() && is_table_line(lines[j]) {
                    events.push(Event::Start(Tag::TableRow));
                    let row = parse_cells(lines[j]);
                    for (ci, cell) in row.iter().enumerate() {
                        let a = aligns.get(ci).cloned().unwrap_or(Alignment::None);
                        events.push(Event::Start(Tag::TableCell(a.clone())));
                        parse_inline(cell, events);
                        events.push(Event::End(Tag::TableCell(a)));
                    }
                    events.push(Event::End(Tag::TableRow));
                    j += 1;
                }
                events.push(Event::End(Tag::Table(aligns)));
                i = j;
                continue;
            }
        }

        // Ordered/Unordered list
        let is_ul = tline.starts_with("- ") || tline.starts_with("* ");
        let dig_count = tline.chars().take_while(|c| c.is_ascii_digit()).count();
        let is_ol = dig_count > 0 && tline[dig_count..].starts_with(". ");

        if is_ul || is_ol {
            events.push(Event::Start(Tag::List(is_ol)));
            let mut j = i;
            while j < lines.len() {
                let l2 = lines[j].trim_start();
                let is_ul2 = l2.starts_with("- ") || l2.starts_with("* ");
                let d2 = l2.chars().take_while(|c| c.is_ascii_digit()).count();
                let is_ol2 = d2 > 0 && l2[d2..].starts_with(". ");
                
                if (is_ol && is_ol2) || (!is_ol && is_ul2) {
                    let content = if is_ul2 { &l2[2..] } else { &l2[d2 + 2..] };
                    events.push(Event::Start(Tag::Item));
                    parse_inline(content, events);
                    events.push(Event::End(Tag::Item));
                    j += 1;
                } else {
                    break;
                }
            }
            events.push(Event::End(Tag::List(is_ol)));
            i = j;
            continue;
        }

        // Paragraph
        let mut para = Vec::new();
        let mut j = i;
        while j < lines.len() {
            let ln = lines[j];
            let t = ln.trim_start();
            if t.is_empty() || t.starts_with("```") || t.starts_with("#") ||
               (ln.starts_with("---") && ln.chars().all(|c| c == '-')) ||
               ln.starts_with(";;;") || ln.starts_with('>') ||
               is_table_line(ln) ||
               t.starts_with("- ") || t.starts_with("* ") ||
               (t.chars().take_while(|c| c.is_ascii_digit()).count() > 0 && t[t.chars().take_while(|c| c.is_ascii_digit()).count()..].starts_with(". "))
            {
                break;
            }
            para.push(ln);
            j += 1;
        }
        
        if !para.is_empty() {
            events.push(Event::Start(Tag::Paragraph));
            for (pidx, pline) in para.iter().enumerate() {
                parse_inline(pline, events);
                if pidx < para.len() - 1 {
                    events.push(Event::HardBreak);
                }
            }
            events.push(Event::End(Tag::Paragraph));
        }
        i = j;
    }

    if root {
        while !section_stack.is_empty() {
            pop_section(events, &mut section_stack);
        }
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
        if text.starts_with("~~") {
            if let Some(end) = text[2..].find("~~") {
                events.push(Event::Start(Tag::Strikethrough));
                parse_inline(&text[2..2 + end], events);
                events.push(Event::End(Tag::Strikethrough));
                text = &text[2 + end + 2..];
                continue;
            }
        }
        if text.starts_with("**") {
            if let Some(end) = text[2..].find("**") {
                events.push(Event::Start(Tag::Strong));
                parse_inline(&text[2..2 + end], events);
                events.push(Event::End(Tag::Strong));
                text = &text[2 + end + 2..];
                continue;
            }
        }
        if text.starts_with('*') && !text.starts_with("**") {
            if let Some(end) = text[1..].find('*') {
                events.push(Event::Start(Tag::Emphasis));
                parse_inline(&text[1..1 + end], events);
                events.push(Event::End(Tag::Emphasis));
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
        if text.starts_with("![") {
            if let Some(close_alt) = text.find(']') {
                if text.len() > close_alt + 1 && text.as_bytes()[close_alt + 1] == b'(' {
                    if let Some(close_src) = text[close_alt + 2..].find(')') {
                        let close_src = close_alt + 2 + close_src;
                        let alt = &text[2..close_alt];
                        let src = &text[close_alt + 2..close_src];
                        events.push(Event::Start(Tag::Image(src, alt)));
                        events.push(Event::End(Tag::Image(src, alt)));
                        text = &text[close_src + 1..];
                        continue;
                    }
                }
            }
        }
        if text.starts_with('[') {
            if let Some(close_bracket) = text.find(']') {
                let content = &text[1..close_bracket];
                if text.len() > close_bracket + 1 && text.as_bytes()[close_bracket + 1] == b'(' {
                    if let Some(close_paren) = text[close_bracket + 2..].find(')') {
                        let close_paren = close_bracket + 2 + close_paren;
                        let href = &text[close_bracket + 2..close_paren];
                        events.push(Event::Start(Tag::Link(href)));
                        parse_inline(content, events);
                        events.push(Event::End(Tag::Link(href)));
                        text = &text[close_paren + 1..];
                        continue;
                    }
                }
                if let Some(slash) = content.find('/') {
                    let base = &content[..slash];
                    let ruby = &content[slash + 1..];
                    events.push(Event::Start(Tag::Ruby(ruby)));
                    parse_inline(base, events); 
                    events.push(Event::End(Tag::Ruby(ruby)));
                    text = &text[close_bracket + 1..];
                    continue;
                }
            }
        }
        if text.starts_with('{') {
            if let Some(end) = text[1..].find('}') {
                // To accurately split gloss parts taking into account brackets as in original JS parser `splitGlossParts`
                // Simplified for now: just split by '/' assuming no nested {}
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
        
        let next_special = text.find(|c| c == '$' || c == '[' || c == '{' || c == '`' || c == '\\' || c == '*' || c == '~' || c == '!').unwrap_or(text.len());
        if next_special == 0 {
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
