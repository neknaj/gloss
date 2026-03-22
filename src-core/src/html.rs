use alloc::string::{String, ToString};
use alloc::format;
use crate::parser::{Event, Tag};

pub fn push_html<'a, I>(out: &mut String, iter: I)
where
    I: Iterator<Item = Event<'a>>,
{
    for event in iter {
        match event {
            Event::Start(Tag::Paragraph) => out.push_str("<p>"),
            Event::End(Tag::Paragraph) => out.push_str("</p>\n"),
            Event::Start(Tag::Heading(level)) => out.push_str(&format!("<h{}>", level)),
            Event::End(Tag::Heading(level)) => out.push_str(&format!("</h{}>\n", level)),
            Event::Start(Tag::Section(level)) => out.push_str(&format!("<div class=\"md-section section level-{} indent\">\n", level)),
            Event::End(Tag::Section(_)) => out.push_str("</div>\n"),
            Event::Start(Tag::Ruby(rt)) => out.push_str("<ruby>"),
            Event::End(Tag::Ruby(rt)) => out.push_str(&format!("<rt>{}</rt></ruby>", rt)),
            Event::Start(Tag::Gloss(parts)) => {
                out.push_str("<ruby class=\"gloss\">");
                // Base text will be inside Event::Text
            }
            Event::End(Tag::Gloss(parts)) => {
                for part in parts {
                    out.push_str(&format!("<rt>{}</rt>", part));
                }
                out.push_str("</ruby>");
            }
            Event::Text(t) => out.push_str(&escape_html(t)),
            Event::MathInline(m) => {
                let mathml = latex2mathml::latex_to_mathml(m, latex2mathml::DisplayStyle::Inline)
                    .unwrap_or_else(|_| m.to_string());
                out.push_str(&mathml);
            }
            Event::MathDisplay(m) => {
                let mathml = latex2mathml::latex_to_mathml(m, latex2mathml::DisplayStyle::Block)
                    .unwrap_or_else(|_| m.to_string());
                out.push_str(&mathml);
            }
            Event::SoftBreak => out.push_str("\n"),
            Event::HardBreak => out.push_str("<br>\n"),
            Event::Rule => out.push_str("<hr>\n"),
        }
    }
}

fn escape_html(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
}
