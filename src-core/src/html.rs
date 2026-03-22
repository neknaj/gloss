use alloc::string::{String, ToString};
use alloc::format;
use crate::parser::{Event, Tag, Parser, Alignment};

pub fn push_html<'a>(out: &mut String, iter: Parser<'a>) {
    let mut align_stack: alloc::vec::Vec<alloc::vec::Vec<Alignment>> = alloc::vec::Vec::new();
    let mut tr_index = 0; // to track td alignment index

    for event in iter {
        match event {
            Event::Text(t) => out.push_str(&escape_html(t)),
            Event::SoftBreak => out.push_str("\n"),
            Event::HardBreak => out.push_str("<br/>\n"),
            Event::Rule => out.push_str("<hr/>\n"),
            Event::MathDisplay(m) => {
                let mathml = latex2mathml::latex_to_mathml(m, latex2mathml::DisplayStyle::Block)
                    .unwrap_or_else(|_| m.to_string()); // Fallback
                out.push_str(&format!("<span class=\"math-display\">{}</span>\n", mathml));
            }
            Event::MathInline(m) => {
                let mathml = latex2mathml::latex_to_mathml(m, latex2mathml::DisplayStyle::Inline)
                    .unwrap_or_else(|_| m.to_string());
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
                let cls = if lang.is_empty() { String::new() } else { format!(" language-{}", escape_html(lang)) };
                out.push_str(&format!("<pre class=\"nm-code\"><code class=\"{}\">", cls));
            }
            Event::End(Tag::CodeBlock(_)) => out.push_str("</code></pre>\n"),
            Event::Start(Tag::Blockquote) => out.push_str("<blockquote class=\"nm-blockquote\">\n"),
            Event::End(Tag::Blockquote) => out.push_str("</blockquote>\n"),
            Event::Start(Tag::Table(aligns)) => {
                align_stack.push(aligns);
                out.push_str("<div class=\"nm-table-wrap\"><table class=\"nm-table\">\n");
            }
            Event::End(Tag::Table(_)) => {
                align_stack.pop();
                out.push_str("</table></div>\n");
            }
            Event::Start(Tag::TableHead) => out.push_str("<thead>"),
            Event::End(Tag::TableHead) => out.push_str("</thead>\n<tbody>\n"),
            Event::Start(Tag::TableRow) => {
                tr_index = 0;
                out.push_str("<tr>");
            }
            Event::End(Tag::TableRow) => out.push_str("</tr>\n"),
            Event::Start(Tag::TableCell(align)) => {
                let style = match align {
                    Alignment::Left => " style=\"text-align:left\"",
                    Alignment::Center => " style=\"text-align:center\"",
                    Alignment::Right => " style=\"text-align:right\"",
                    Alignment::None => "",
                };
                if out.ends_with("<thead><tr>") || out.ends_with("</th>") {
                    out.push_str(&format!("<th{}>", style));
                } else {
                    out.push_str(&format!("<td{}>", style));
                }
            }
            Event::End(Tag::TableCell(_)) => {
                if out.contains("<thead><tr>") && !out.contains("</thead>") {
                    out.push_str("</th>");
                } else {
                    out.push_str("</td>");
                }
                tr_index += 1;
            }
            Event::Start(Tag::Strong) => out.push_str("<strong>"),
            Event::End(Tag::Strong) => out.push_str("</strong>"),
            Event::Start(Tag::Emphasis) => out.push_str("<em>"),
            Event::End(Tag::Emphasis) => out.push_str("</em>"),
            Event::Start(Tag::Strikethrough) => out.push_str("<del>"),
            Event::End(Tag::Strikethrough) => out.push_str("</del>"),
            Event::Start(Tag::Link(href)) => out.push_str(&format!("<a href=\"{}\">", escape_html(href))),
            Event::End(Tag::Link(_)) => out.push_str("</a>"),
            Event::Start(Tag::Image(src, alt)) => out.push_str(&format!("<img src=\"{}\" alt=\"{}\" class=\"nm-image\"/>", escape_html(src), escape_html(alt))),
            Event::End(Tag::Image(_, _)) => {}
            Event::Start(Tag::Ruby(_)) => out.push_str("<ruby class=\"nm-ruby\"><rb>"),
            Event::End(Tag::Ruby(rt)) => out.push_str(&format!("</rb><rt>{}</rt></ruby>", escape_html(rt))),
            Event::Start(Tag::Gloss(_)) => out.push_str("<ruby class=\"nm-gloss\"><rb>"),
            Event::End(Tag::Gloss(parts)) => {
                out.push_str("</rb><rt>");
                for p in parts {
                    out.push_str(&format!("<span class=\"nm-gloss-note\">{}</span>", escape_html(p)));
                }
                out.push_str("</rt></ruby>");
            }
        }
    }
    // ensure tbody close if table was there
    if out.contains("<tbody>") {
        *out = out.replace("</table>", "</tbody>\n</table>");
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
