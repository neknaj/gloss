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
    /// ruby: `[base/reading]` - the reading text is stored here for the End event
    Ruby(&'a str),
    /// gloss: `{base/note1/note2}` - notes are stored for the End event (for backward compat)
    Gloss(Vec<&'a str>),
    GlossNote,
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
                events.push(Event::Text("\n"));
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            events.push(Event::End(Tag::CodeBlock(lang)));
            continue;
        }

        // Headings
        if tline.starts_with('#') {
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

        // Thematic break: first close ONE section (so hr lands in parent), then emit <hr/>
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

        // Paragraph: collect consecutive non-block lines
        let mut para = Vec::new();
        let mut j = i;
        while j < lines.len() {
            let ln = lines[j];
            let t = ln.trim_start();
            if t.is_empty()
                || t.starts_with("```")
                || t.starts_with('#')
                || (ln.starts_with("---") && ln.chars().all(|c| c == '-'))
                || ln.starts_with(";;;")
                || ln.starts_with('>')
                || is_table_line(ln)
                || t.starts_with("- ")
                || t.starts_with("* ")
                || (t.chars().take_while(|c| c.is_ascii_digit()).count() > 0
                    && t[t.chars().take_while(|c| c.is_ascii_digit()).count()..].starts_with(". "))
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
        // $$ math display
        if text.starts_with("$$") {
            if let Some(end) = text[2..].find("$$") {
                events.push(Event::MathDisplay(&text[2..2 + end]));
                text = &text[2 + end + 2..];
                continue;
            }
        }
        // $ math inline
        if text.starts_with('$') {
            if let Some(end) = text[1..].find('$') {
                events.push(Event::MathInline(&text[1..1 + end]));
                text = &text[1 + end + 1..];
                continue;
            }
        }
        // `code`
        if text.starts_with('`') {
            if let Some(end) = text[1..].find('`') {
                events.push(Event::Start(Tag::Code));
                events.push(Event::Text(&text[1..1 + end]));
                events.push(Event::End(Tag::Code));
                text = &text[1 + end + 1..];
                continue;
            }
        }
        // ~~strike~~
        if text.starts_with("~~") {
            if let Some(end) = text[2..].find("~~") {
                events.push(Event::Start(Tag::Strikethrough));
                parse_inline(&text[2..2 + end], events);
                events.push(Event::End(Tag::Strikethrough));
                text = &text[2 + end + 2..];
                continue;
            }
        }
        // **bold**
        if text.starts_with("**") {
            if let Some(end) = text[2..].find("**") {
                events.push(Event::Start(Tag::Strong));
                parse_inline(&text[2..2 + end], events);
                events.push(Event::End(Tag::Strong));
                text = &text[2 + end + 2..];
                continue;
            }
        }
        // *em* (not **)
        if text.starts_with('*') && !text.starts_with("**") {
            if let Some(end) = text[1..].find('*') {
                events.push(Event::Start(Tag::Emphasis));
                parse_inline(&text[1..1 + end], events);
                events.push(Event::End(Tag::Emphasis));
                text = &text[1 + end + 1..];
                continue;
            }
        }
        // \n  (literal backslash-n → hard break)
        if text.starts_with("\\n") {
            events.push(Event::HardBreak);
            text = &text[2..];
            continue;
        }
        // Escape: \X → literal X
        if text.starts_with('\\') && text.len() >= 2 {
            let ch = text[1..].chars().next().unwrap();
            let len = ch.len_utf8();
            events.push(Event::Text(&text[1..1 + len]));
            text = &text[1 + len..];
            continue;
        }
        // ![alt](src) image — must come before [
        if text.starts_with("![") {
            // find matching ] honouring bracket nesting inside alt
            let mut bracket = 0;
            let mut close_alt = None;
            for (idx, c) in text[1..].char_indices() {
                if c == '[' { bracket += 1; }
                else if c == ']' {
                    if bracket == 0 { close_alt = Some(idx + 1); break; }
                    bracket -= 1;
                }
            }
            if let Some(ca) = close_alt {
                if text.len() > ca + 1 && text.as_bytes()[ca + 1] == b'(' {
                    if let Some(cp) = text[ca + 2..].find(')') {
                        let close_src = ca + 2 + cp;
                        let alt = &text[2..ca];
                        let src = &text[ca + 2..close_src];
                        events.push(Event::Start(Tag::Image(src, alt)));
                        events.push(Event::End(Tag::Image(src, alt)));
                        text = &text[close_src + 1..];
                        continue;
                    }
                }
            }
        }
        // [content](url)  or  [base/ruby]
        if text.starts_with('[') {
            let mut bracket = 0;
            let mut close_bracket = None;
            for (idx, c) in text.char_indices() {
                if c == '[' { bracket += 1; }
                else if c == ']' {
                    bracket -= 1;
                    if bracket == 0 { close_bracket = Some(idx); break; }
                }
            }
            if let Some(cb) = close_bracket {
                let content = &text[1..cb];
                // [text](url) link
                if text.len() > cb + 1 && text.as_bytes()[cb + 1] == b'(' {
                    if let Some(cp) = text[cb + 2..].find(')') {
                        let close_paren = cb + 2 + cp;
                        let href = &text[cb + 2..close_paren];
                        events.push(Event::Start(Tag::Link(href)));
                        parse_inline(content, events);
                        events.push(Event::End(Tag::Link(href)));
                        text = &text[close_paren + 1..];
                        continue;
                    }
                }
                // [base/ruby]: find first '/' at bracket-level 0
                let slash_idx = {
                    let mut blk = 0i32;
                    let mut found = None;
                    for (idx, c) in content.char_indices() {
                        if c == '[' { blk += 1; }
                        else if c == ']' { blk -= 1; }
                        else if c == '/' && blk == 0 { found = Some(idx); break; }
                    }
                    found
                };
                if let Some(slash) = slash_idx {
                    let base = &content[..slash];
                    let ruby = &content[slash + 1..];
                    events.push(Event::Start(Tag::Ruby(ruby)));
                    parse_inline(base, events);
                    events.push(Event::End(Tag::Ruby(ruby)));
                    text = &text[cb + 1..];
                    continue;
                }
            }
        }
        // {base/note1/note2…}
        if text.starts_with('{') {
            let mut bracket = 0;
            let mut close_brace = None;
            for (idx, c) in text.char_indices() {
                if c == '{' { bracket += 1; }
                else if c == '}' {
                    bracket -= 1;
                    if bracket == 0 { close_brace = Some(idx); break; }
                }
            }
            if let Some(end) = close_brace {
                let content = &text[1..end];
                // split by '/' at bracket-level 0 (respecting nested [...])
                let mut parts: Vec<&str> = Vec::new();
                let mut last = 0;
                let mut blk = 0i32;
                for (idx, c) in content.char_indices() {
                    if c == '[' { blk += 1; }
                    else if c == ']' { blk -= 1; }
                    else if c == '/' && blk == 0 {
                        parts.push(&content[last..idx]);
                        last = idx + 1; // '/' is always 1 byte
                    }
                }
                parts.push(&content[last..]);

                if parts.len() >= 2 {
                    let base = parts[0];
                    let notes = &parts[1..];
                    // Emit: Start(Gloss), Start(GlossBase implied by rb in html), parse base,
                    // then for each note: Start(GlossNote), parse note, End(GlossNote)
                    // We keep Gloss(Vec) for the End event for html.rs symmetry
                    let notes_owned: Vec<&str> = notes.to_vec();
                    events.push(Event::Start(Tag::Gloss(notes_owned.clone())));
                    parse_inline(base, events);
                    for note in notes_owned.iter() {
                        events.push(Event::Start(Tag::GlossNote));
                        parse_inline(note, events);
                        events.push(Event::End(Tag::GlossNote));
                    }
                    events.push(Event::End(Tag::Gloss(notes_owned)));
                    text = &text[end + 1..];
                    continue;
                }
            }
        }

        // Plain text up to next special character
        let next_special = text.find(|c| matches!(c, '$' | '[' | '{' | '`' | '\\' | '*' | '~' | '!')).unwrap_or(text.len());
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
