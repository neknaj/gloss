use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;
use src_desktop_types::TextSpan;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HighlightContext {
    Normal,
    InCodeBlock { lang: &'static str },
    InMathBlock,
}

// ARGB color constants
const COLOR_DEFAULT:  u32 = 0xFF_D4D4D4;
const COLOR_HEADING:  u32 = 0xFF_569CD6;
const COLOR_RUBY:     u32 = 0xFF_CE9178;
const COLOR_ANNO:     u32 = 0xFF_9CDCFE;
const COLOR_MATH:     u32 = 0xFF_C586C0;
const COLOR_CODE:     u32 = 0xFF_4EC9B0;
const COLOR_KEYWORD:  u32 = 0xFF_569CD6;
const COLOR_COMMENT:  u32 = 0xFF_6A9955;

pub struct Highlighter;

impl Highlighter {
    /// Highlight one line. Returns a Vec of TextSpan covering the whole line.
    pub fn highlight_line(line: &str, ctx: HighlightContext) -> Vec<TextSpan> {
        match ctx {
            HighlightContext::InCodeBlock { .. } => highlight_code_line(line),
            HighlightContext::InMathBlock  => vec![span(line, COLOR_MATH, false, false)],
            HighlightContext::Normal       => highlight_normal_line(line),
        }
    }
}

fn highlight_normal_line(line: &str) -> Vec<TextSpan> {
    // Heading: starts with one or more `#`
    if line.starts_with('#') {
        let hashes = line.chars().take_while(|&c| c == '#').count();
        let rest = &line[hashes..];
        return vec![
            span(&line[..hashes], COLOR_HEADING, true, false),
            span(rest, COLOR_HEADING, true, false),
        ];
    }
    // Fallback: scan for inline constructs
    let mut spans = Vec::new();
    let mut chars = line.char_indices().peekable();
    let mut text_start = 0usize;

    macro_rules! flush {
        ($end:expr) => {
            if text_start < $end {
                spans.push(span(&line[text_start..$end], COLOR_DEFAULT, false, false));
            }
        };
    }

    while let Some((i, c)) = chars.next() {
        match c {
            '[' => {
                // Ruby: [base/ruby] — scan forward for matching ]
                flush!(i);
                let rest = &line[i..];
                if let Some(end) = rest.find(']') {
                    let _inner = &rest[1..end];
                    spans.push(span(&rest[..end + 1], COLOR_RUBY, false, false));
                    // Skip consumed chars
                    let consumed = end + 1;
                    for _ in 0..rest[1..consumed].chars().count() { chars.next(); }
                    text_start = i + consumed;
                } else {
                    text_start = i;
                }
            }
            '{' => {
                // Anno: {word/anno}
                flush!(i);
                let rest = &line[i..];
                if let Some(end) = rest.find('}') {
                    spans.push(span(&rest[..end + 1], COLOR_ANNO, false, false));
                    let consumed = end + 1;
                    for _ in 0..rest[1..consumed].chars().count() { chars.next(); }
                    text_start = i + consumed;
                } else {
                    text_start = i;
                }
            }
            '$' => {
                // Inline math: $...$
                flush!(i);
                let rest = &line[i + 1..];
                if let Some(end) = rest.find('$') {
                    let full_len = 1 + end + 1;
                    spans.push(span(&line[i..i + full_len], COLOR_MATH, false, true));
                    for _ in 0..end { chars.next(); }
                    chars.next(); // closing $
                    text_start = i + full_len;
                } else {
                    text_start = i;
                }
            }
            '`' => {
                // Inline code: `...`
                flush!(i);
                let rest = &line[i + 1..];
                if let Some(end) = rest.find('`') {
                    let full_len = 1 + end + 1;
                    spans.push(span(&line[i..i + full_len], COLOR_CODE, false, false));
                    for _ in 0..end { chars.next(); }
                    chars.next();
                    text_start = i + full_len;
                } else {
                    text_start = i;
                }
            }
            _ => {}
        }
    }
    flush!(line.len());
    if spans.is_empty() {
        spans.push(span(line, COLOR_DEFAULT, false, false));
    }
    spans
}

fn highlight_code_line(line: &str) -> Vec<TextSpan> {
    // Minimal: comments start with // or #
    if line.trim_start().starts_with("//") || line.trim_start().starts_with('#') {
        return vec![span(line, COLOR_COMMENT, false, true)];
    }
    vec![span(line, COLOR_DEFAULT, false, false)]
}

fn span(text: &str, color: u32, bold: bool, italic: bool) -> TextSpan {
    TextSpan { text: String::from(text), color, bold, italic }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn plain_line_is_default_color() {
        let spans = Highlighter::highlight_line("hello world", HighlightContext::Normal);
        assert!(spans.iter().all(|s| s.color == COLOR_DEFAULT));
    }
    #[test] fn heading_gets_heading_color() {
        let spans = Highlighter::highlight_line("## Title", HighlightContext::Normal);
        assert!(spans.iter().any(|s| s.color == COLOR_HEADING));
    }
    #[test] fn ruby_bracket_colored() {
        let spans = Highlighter::highlight_line("[漢字/かんじ]", HighlightContext::Normal);
        assert!(spans.iter().any(|s| s.color == COLOR_RUBY));
    }
    #[test] fn anno_brace_colored() {
        let spans = Highlighter::highlight_line("{word/gloss}", HighlightContext::Normal);
        assert!(spans.iter().any(|s| s.color == COLOR_ANNO));
    }
    #[test] fn inline_math_colored() {
        let spans = Highlighter::highlight_line("result is $x^2$", HighlightContext::Normal);
        assert!(spans.iter().any(|s| s.color == COLOR_MATH));
    }
    #[test] fn inline_code_colored() {
        let spans = Highlighter::highlight_line("use `cargo test`", HighlightContext::Normal);
        assert!(spans.iter().any(|s| s.color == COLOR_CODE));
    }
    #[test] fn math_block_whole_line() {
        let spans = Highlighter::highlight_line("x^2 + y^2", HighlightContext::InMathBlock);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].color, COLOR_MATH);
    }
    #[test] fn code_block_comment_italic() {
        let spans = Highlighter::highlight_line("// comment", HighlightContext::InCodeBlock { lang: "rust" });
        assert!(spans.iter().any(|s| s.italic));
    }
    #[test] fn spans_cover_full_line() {
        let line = "hello [漢字/かんじ] world";
        let spans = Highlighter::highlight_line(line, HighlightContext::Normal);
        let total: usize = spans.iter().map(|s| s.text.len()).sum();
        assert_eq!(total, line.len());
    }
}
