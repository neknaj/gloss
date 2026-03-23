use alloc::string::{String, ToString};
use alloc::format;
use crate::parser::{Event, Tag, Parser, Alignment};

pub fn push_html<'a>(out: &mut String, iter: Parser<'a>) {
    let mut in_thead = false;
    let mut in_gloss = false;
    let mut gloss_rb_closed = false;

    for event in iter {
        match event {
            Event::Text(t) => out.push_str(&escape_html(t)),
            Event::SoftBreak => out.push('\n'),
            Event::HardBreak => out.push_str("<br/>\n"),
            Event::Rule => out.push_str("<hr/>\n"),
            Event::CardLink(url) => {
                let escaped = escape_html(url);
                out.push_str(&format!(
                    "<a href=\"{escaped}\" class=\"nm-card-link\" target=\"_blank\" rel=\"noopener noreferrer\"><span class=\"nm-card-url\">{escaped}</span></a>\n"
                ));
            }
            Event::MathDisplay(m) => {
                let mathml = latex2mathml::latex_to_mathml(m, latex2mathml::DisplayStyle::Block)
                    .unwrap_or_else(|_| escape_html(m).to_string());
                out.push_str(&format!("<span class=\"math-display\"><span class=\"native-mathml\">{}</span><span class=\"math-tex\" style=\"display:none\">{}</span></span>\n", mathml, escape_html(m)));
            }
            Event::MathInline(m) => {
                let mathml = latex2mathml::latex_to_mathml(m, latex2mathml::DisplayStyle::Inline)
                    .unwrap_or_else(|_| escape_html(m).to_string());
                out.push_str(&format!("<span class=\"math-inline\"><span class=\"native-mathml\">{}</span><span class=\"math-tex\" style=\"display:none\">{}</span></span>", mathml, escape_html(m)));
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
            Event::Start(Tag::CodeBlock(lang, filename)) => {
                let has_header = !lang.is_empty() || !filename.is_empty();
                if has_header {
                    out.push_str("<div class=\"nm-code-container\"><div class=\"nm-code-header\">");
                    if !lang.is_empty() {
                        out.push_str(&format!("<span class=\"nm-badge-main\">{}</span>", escape_html(lang)));
                    }
                    if !filename.is_empty() {
                        out.push_str(&format!("<span class=\"nm-badge-flag\">{}</span>", escape_html(filename)));
                    }
                    out.push_str("</div><div class=\"nm-code-content\">");
                }
                let cls = if lang.is_empty() {
                    String::new()
                } else {
                    format!(" language-{}", escape_html(lang))
                };
                out.push_str(&format!("<pre class=\"nm-code\"><code class=\"{}\">", cls));
            }
            Event::End(Tag::CodeBlock(lang, filename)) => {
                let has_header = !lang.is_empty() || !filename.is_empty();
                out.push_str("</code></pre>");
                if has_header {
                    out.push_str("</div></div>");
                }
                out.push('\n');
            }
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
                gloss_rb_closed = false;
                out.push_str("<ruby class=\"nm-gloss\"><rb>");
            }
            Event::End(Tag::Gloss(_)) => {
                if in_gloss && !gloss_rb_closed {
                    // Empty gloss or no notes? close it anyway
                    out.push_str("</rb><rt>");
                }
                in_gloss = false;
                out.push_str("</rt></ruby>");
            }
            Event::Start(Tag::GlossNote) => {
                if !gloss_rb_closed {
                    out.push_str("</rb><rt>");
                    gloss_rb_closed = true;
                }
                out.push_str("<span class=\"nm-gloss-note\">");
            }
            Event::End(Tag::GlossNote) => {
                out.push_str("</span>");
            }
            Event::FootnoteRef(n) => {
                out.push_str(&format!(
                    "<sup class=\"nm-fn-ref\"><a href=\"#fn-{n}\" id=\"fnref-{n}\">{n}</a></sup>"
                ));
            }
            Event::Start(Tag::FootnoteSection) => {
                out.push_str("<section class=\"nm-footnotes\"><ol>\n");
            }
            Event::End(Tag::FootnoteSection) => {
                out.push_str("</ol></section>\n");
            }
            Event::Start(Tag::FootnoteItem(n)) => {
                out.push_str(&format!("<li id=\"fn-{n}\">"));
            }
            Event::End(Tag::FootnoteItem(n)) => {
                out.push_str(&format!(" <a href=\"#fnref-{n}\" class=\"nm-fn-back\">↩</a></li>\n"));
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
