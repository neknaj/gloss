use alloc::string::{String, ToString};
use alloc::vec::Vec;

use src_core::{Event, HtmlRenderer, Tag, escape_html};
use src_desktop_types::PluginHost;
use src_plugin_types::{CardLinkOutput, PluginFrontMatterField};

pub struct PluginAwareHtmlRenderer<'a, Ph: PluginHost> {
    pub host: &'a mut Ph,
}

impl<'a, Ph: PluginHost> PluginAwareHtmlRenderer<'a, Ph> {
    pub fn new(host: &'a mut Ph) -> Self { Self { host } }

    pub fn render<'ev>(
        &mut self,
        events: &[Event<'ev>],
        out: &mut String,
        source: &str,
        _markdown: &str,
    ) {
        let start_len = out.len();
        let mut renderer = HtmlRenderer::new(false);
        let mut i = 0;

        while i < events.len() {
            match &events[i] {
                // ── front-matter hook ─────────────────────────────────────
                Event::FrontMatter(fields) => {
                    let pfields: Vec<PluginFrontMatterField> = fields.iter().map(|f| {
                        PluginFrontMatterField { key: f.key.into(), raw: f.raw.into() }
                    }).collect();
                    if let Some(html) = self.host.run_front_matter(&pfields, source) {
                        out.push_str(&html);
                    } else {
                        renderer.feed(events[i].clone(), out);
                    }
                    i += 1;
                }

                // ── code-highlight hook ───────────────────────────────────
                Event::Start(Tag::CodeBlock(_, _)) => {
                    let code_start = i;
                    let lang = match &events[i] {
                        Event::Start(Tag::CodeBlock(l, _)) => l.to_string(),
                        _ => unreachable!(),
                    };
                    let filename = match &events[i] {
                        Event::Start(Tag::CodeBlock(_, f)) => f.to_string(),
                        _ => unreachable!(),
                    };
                    i += 1;
                    let mut code_text = String::new();
                    while i < events.len() {
                        match &events[i] {
                            Event::End(Tag::CodeBlock(_, _)) => { i += 1; break; }
                            Event::Text(t) => { code_text.push_str(t); i += 1; }
                            _ => { i += 1; }
                        }
                    }
                    if let Some(html) = self.host.run_code_highlight(&lang, &code_text, &filename) {
                        out.push_str(&html);
                    } else {
                        // fallback: replay through HtmlRenderer
                        for j in code_start..i {
                            renderer.feed(events[j].clone(), out);
                        }
                    }
                }

                // ── card-link hook ────────────────────────────────────────
                Event::CardLink(_) => {
                    let url = match &events[i] {
                        Event::CardLink(u) => u.to_string(),
                        _ => unreachable!(),
                    };
                    if let Some(card_out) = self.host.run_card_link(&url) {
                        out.push_str(&render_card_output(&url, card_out));
                    } else {
                        renderer.feed(events[i].clone(), out);
                    }
                    i += 1;
                }

                // ── default ───────────────────────────────────────────────
                _ => {
                    renderer.feed(events[i].clone(), out);
                    i += 1;
                }
            }
        }

        renderer.finish(out, start_len);
    }
}

/// Render a `CardLinkOutput` to HTML.
/// Priority: `html` (full override) > structured metadata > plain URL fallback.
fn render_card_output(url: &str, out: CardLinkOutput) -> String {
    // Priority 1: full HTML override
    if let Some(html) = out.html {
        return html;
    }

    // Priority 2: structured metadata (any field present)
    if out.title.is_some() || out.description.is_some() || out.image_url.is_some() {
        let escaped_url = escape_html(url);
        let mut s = alloc::format!(
            "<a href=\"{escaped_url}\" class=\"nm-card-link\" target=\"_blank\" rel=\"noopener noreferrer\">"
        );
        if let Some(img) = out.image_url {
            s.push_str(&alloc::format!("<img src=\"{}\" class=\"nm-card-img\" alt=\"\">", escape_html(&img)));
        }
        s.push_str("<span class=\"nm-card-body\">");
        if let Some(title) = out.title {
            s.push_str(&alloc::format!("<span class=\"nm-card-title\">{}</span>", escape_html(&title)));
        }
        if let Some(desc) = out.description {
            s.push_str(&alloc::format!("<span class=\"nm-card-desc\">{}</span>", escape_html(&desc)));
        }
        s.push_str(&alloc::format!(
            "<span class=\"nm-card-url\">{escaped_url}</span></span></a>\n"
        ));
        return s;
    }

    // Priority 3: plain URL fallback
    let escaped = escape_html(url);
    alloc::format!(
        "<a href=\"{escaped}\" class=\"nm-card-link\" target=\"_blank\" rel=\"noopener noreferrer\"><span class=\"nm-card-url\">{escaped}</span></a>\n"
    )
}
