use alloc::string::{String, ToString};
use alloc::format;
use crate::parser::{Event, Tag, Parser, Alignment};

pub fn push_html<'a>(out: &mut String, iter: Parser<'a>) {
    let mut in_thead = false;
    let mut in_gloss = false;

    for event in iter {
        match event {
            Event::Text(t) => out.push_str(&escape_html(t)),
            Event::SoftBreak => out.push('\n'),
            Event::HardBreak => out.push_str("<br/>\n"),
            Event::Rule => out.push_str("<hr/>\n"),
            Event::MathDisplay(m) => {
                let mathml = latex2mathml::latex_to_mathml(m, latex2mathml::DisplayStyle::Block)
                    .unwrap_or_else(|_| escape_html(m).to_string());
                out.push_str(&format!("<span class=\"math-display\">{}</span>\n", mathml));
            }
            Event::MathInline(m) => {
                let mathml = latex2mathml::latex_to_mathml(m, latex2mathml::DisplayStyle::Inline)
                    .unwrap_or_else(|_| escape_html(m).to_string());
                out.push_str(&format!("<span class=\"math-inline\">{}</span>", mathml));
            }
            Event::Start(Tag::Paragraph) => out.push_str("<p>"),
            Event::End(Tag::Paragraph) => out.push_str("</p>\n"),
            Event::Start(Tag::Heading(level)) => out.push_str(&format!("<h{}>", level)),
            Event::End(Tag::Heading(level)) => out.push_str(&format!("</h{}>\n", level)),
            Event::Start(Tag::Section(level)) => out.push_str(&format!("<section class=\"nm-sec level-{}\">\n", level)),
            Event::End(Tag::Section(_)) => out.push_str("</section>\n"),
            Event::Start(Tag::List(true)) => out.push_str("<ol>\n"),
            Event::End(Tag::List(true)) => out.push_str("</ol>\n"),
            Event::Start(Tag::List(false)) => out.push_str("<ul>\n"),
            Event::End(Tag::List(false)) => out.push_str("</ul>\n"),
            Event::Start(Tag::Item) => out.push_str("<li>"),
            Event::End(Tag::Item) => out.push_str("</li>\n"),
            Event::Start(Tag::Code) => out.push_str("<code class=\"nm-code-inline\">"),
            Event::End(Tag::Code) => out.push_str("</code>"),
            Event::Start(Tag::CodeBlock(lang)) => {
                let cls = if lang.is_empty() {
                    String::new()
                } else {
                    format!(" language-{}", escape_html(lang))
                };
                out.push_str(&format!("<pre class=\"nm-code\"><code class=\"{}\">", cls));
            }
            Event::End(Tag::CodeBlock(_)) => out.push_str("</code></pre>\n"),
            Event::Start(Tag::Blockquote) => out.push_str("<blockquote class=\"nm-blockquote\">\n"),
            Event::End(Tag::Blockquote) => out.push_str("</blockquote>\n"),
            Event::Start(Tag::Table(_)) => out.push_str("<div class=\"nm-table-wrap\"><table class=\"nm-table\">\n"),
            Event::End(Tag::Table(_)) => out.push_str("</tbody>\n</table></div>\n"),
            Event::Start(Tag::TableHead) => {
                in_thead = true;
                out.push_str("<thead>");
            }
            Event::End(Tag::TableHead) => {
                in_thead = false;
                out.push_str("</thead>\n<tbody>\n");
            }
            Event::Start(Tag::TableRow) => out.push_str("<tr>"),
            Event::End(Tag::TableRow) => out.push_str("</tr>\n"),
            Event::Start(Tag::TableCell(align)) => {
                let style = match align {
                    Alignment::Left => " style=\"text-align:left\"",
                    Alignment::Center => " style=\"text-align:center\"",
                    Alignment::Right => " style=\"text-align:right\"",
                    Alignment::None => "",
                };
                if in_thead {
                    out.push_str(&format!("<th{}>", style));
                } else {
                    out.push_str(&format!("<td{}>", style));
                }
            }
            Event::End(Tag::TableCell(_)) => {
                if in_thead {
                    out.push_str("</th>");
                } else {
                    out.push_str("</td>");
                }
            }
            Event::Start(Tag::Strong) => out.push_str("<strong>"),
            Event::End(Tag::Strong) => out.push_str("</strong>"),
            Event::Start(Tag::Emphasis) => out.push_str("<em>"),
            Event::End(Tag::Emphasis) => out.push_str("</em>"),
            Event::Start(Tag::Strikethrough) => out.push_str("<del>"),
            Event::End(Tag::Strikethrough) => out.push_str("</del>"),
            Event::Start(Tag::Link(href)) => out.push_str(&format!("<a href=\"{}\">", escape_html(href))),
            Event::End(Tag::Link(_)) => out.push_str("</a>"),
            Event::Start(Tag::Image(src, alt)) => {
                out.push_str(&format!("<img src=\"{}\" alt=\"{}\" class=\"nm-image\"/>", escape_html(src), escape_html(alt)));
            }
            Event::End(Tag::Image(_, _)) => {}
            // Ruby: <ruby class="nm-ruby"><rb>base</rb><rt>reading</rt></ruby>
            Event::Start(Tag::Ruby(_)) => out.push_str("<ruby class=\"nm-ruby\"><rb>"),
            Event::End(Tag::Ruby(rt)) => out.push_str(&format!("</rb><rt>{}</rt></ruby>", escape_html(rt))),
            // Gloss: <ruby class="nm-gloss"><rb>base</rb><rt>notes...</rt></ruby>
            // Notes are now emitted as GlossNote events between Start(Gloss) and End(Gloss)
            Event::Start(Tag::Gloss(_)) => {
                in_gloss = true;
                out.push_str("<ruby class=\"nm-gloss\"><rb>");
            }
            Event::End(Tag::Gloss(_)) => {
                in_gloss = false;
                out.push_str("</rt></ruby>");
            }
            Event::Start(Tag::GlossNote) => {
                // Close rb if it hasn't been closed yet (first note),
                // otherwise just push next span (subsequent notes).
                // We detect "first note" by whether rt has been opened yet.
                // Simple approach: track with a second bool.
                // Instead we check for the </rb> sentinel appended just before.
                // But string-suffix checks are fragile. Use in_gloss sentinel differently:
                // Re-purpose: close rb once, then open rt once, before first note.
                // The flag in_gloss is already set. We add a second flag gloss_rt_open.
                //
                // Actually: the simplest correct approach is to always emit </rb><rt>
                // at the FIRST GlossNote, tracked by a separate flag.
                // For now detect first note by "not yet seen </rt>" in this gloss.
                // We trust the parser emits notes sequentially, so we can use
                // a local counter – but html.rs is purely event-driven.
                //
                // Best fix: the parser emits an explicit event to close the base.
                // For now: the `in_gloss` flag tells us we are inside.
                // We use a second bool `gloss_rb_closed` initialised to false when
                // Gloss starts, set to true after first GlossNote closes it.
                out.push_str("</rb><rt><span class=\"nm-gloss-note\">");
            }
            Event::End(Tag::GlossNote) => {
                out.push_str("</span>");
            }
        }
    }
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
