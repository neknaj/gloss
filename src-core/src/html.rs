use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use crate::parser::{Event, Tag, Parser, Alignment, FrontMatterField};

/// Stateful HTML emitter. Feed events one at a time with `feed()`,
/// then call `finish()` after the last event to flush pending front matter.
pub struct HtmlRenderer {
    block_ids: bool,
    in_thead: bool,
    in_anno: bool,
    anno_rb_closed: bool,
    pending_bid: Option<u64>,
    pending_fm: Option<String>,
    fm_emitted: bool,
}

impl HtmlRenderer {
    pub fn new(block_ids: bool) -> Self {
        Self {
            block_ids,
            in_thead: false,
            in_anno: false,
            anno_rb_closed: false,
            pending_bid: None,
            pending_fm: None,
            fm_emitted: false,
        }
    }

    /// Emit HTML for one event into `out`.
    pub fn feed<'a>(&mut self, event: Event<'a>, out: &mut String) {
        self.handle_event(event, out);
    }

    /// Call after all events. If no H1 was seen, prepends buffered
    /// front matter to the slice of `out` starting at `start_len`.
    pub fn finish(&mut self, out: &mut String, start_len: usize) {
        if let Some(fm) = self.pending_fm.take() {
            let content = out.split_off(start_len);
            out.push_str(&fm);
            out.push_str(&content);
        }
    }

    fn take_bid(&mut self) -> String {
        if self.block_ids {
            if let Some(id) = self.pending_bid.take() {
                return format!(" data-bid=\"{:x}\"", id);
            }
        } else {
            self.pending_bid.take();
        }
        String::new()
    }

    fn handle_event<'a>(&mut self, event: Event<'a>, out: &mut String) {
        match event {
            Event::FrontMatter(fields) => {
                self.pending_fm = Some(render_frontmatter(&fields));
            }
            Event::BlockId(id) => {
                self.pending_bid = Some(id);
            }
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
                out.push_str(&format!(
                    "<span class=\"math-display\"><span class=\"native-mathml\">{}</span><span class=\"math-tex\" style=\"display:none\">{}</span></span>\n",
                    mathml, escape_html(m)
                ));
            }
            Event::MathInline(m) => {
                let mathml = latex2mathml::latex_to_mathml(m, latex2mathml::DisplayStyle::Inline)
                    .unwrap_or_else(|_| escape_html(m).to_string());
                out.push_str(&format!(
                    "<span class=\"math-inline\"><span class=\"native-mathml\">{}</span><span class=\"math-tex\" style=\"display:none\">{}</span></span>",
                    mathml, escape_html(m)
                ));
            }
            Event::Start(Tag::Paragraph) => {
                out.push_str(&format!("<p{}>", self.take_bid()));
            }
            Event::End(Tag::Paragraph) => out.push_str("</p>\n"),
            Event::Start(Tag::Heading(level)) => {
                out.push_str(&format!("<h{}{}>", level, self.take_bid()));
            }
            Event::End(Tag::Heading(level)) => {
                out.push_str(&format!("</h{}>\n", level));
                // Emit front matter directly after the first H1
                if level == 1 && !self.fm_emitted {
                    self.fm_emitted = true;
                    if let Some(fm) = self.pending_fm.take() {
                        out.push_str(&fm);
                    }
                }
            }
            Event::Start(Tag::Section(level)) => {
                out.push_str(&format!("<section class=\"nm-sec level-{}\">\n", level));
            }
            Event::End(Tag::Section(_)) => out.push_str("</section>\n"),
            Event::Start(Tag::List(true)) => {
                out.push_str(&format!("<ol{}>\n", self.take_bid()));
            }
            Event::End(Tag::List(true)) => out.push_str("</ol>\n"),
            Event::Start(Tag::List(false)) => {
                out.push_str(&format!("<ul{}>\n", self.take_bid()));
            }
            Event::End(Tag::List(false)) => out.push_str("</ul>\n"),
            Event::Start(Tag::Item) => out.push_str("<li>"),
            Event::End(Tag::Item) => out.push_str("</li>\n"),
            Event::Start(Tag::Code) => out.push_str("<code class=\"nm-code-inline\">"),
            Event::End(Tag::Code) => out.push_str("</code>"),
            Event::Start(Tag::CodeBlock(lang, filename)) => {
                let has_header = !lang.is_empty() || !filename.is_empty();
                let bid = self.take_bid();
                if has_header {
                    out.push_str(&format!(
                        "<div class=\"nm-code-container\"{bid}><div class=\"nm-code-header\">"
                    ));
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
                // If no header, attach bid to the <pre> itself
                let pre_bid = if has_header { String::new() } else { bid };
                out.push_str(&format!("<pre class=\"nm-code\"{pre_bid}><code class=\"{cls}\">"));
            }
            Event::End(Tag::CodeBlock(lang, filename)) => {
                let has_header = !lang.is_empty() || !filename.is_empty();
                out.push_str("</code></pre>");
                if has_header { out.push_str("</div></div>"); }
                out.push('\n');
            }
            Event::Start(Tag::Blockquote) => {
                out.push_str(&format!(
                    "<blockquote class=\"nm-blockquote\"{}>\n",
                    self.take_bid()
                ));
            }
            Event::End(Tag::Blockquote) => out.push_str("</blockquote>\n"),
            Event::Start(Tag::Table(_)) => {
                out.push_str(&format!(
                    "<div class=\"nm-table-wrap\"{} ><table class=\"nm-table\">\n",
                    self.take_bid()
                ));
            }
            Event::End(Tag::Table(_)) => out.push_str("</tbody>\n</table></div>\n"),
            Event::Start(Tag::TableHead) => {
                self.in_thead = true;
                out.push_str("<thead>");
            }
            Event::End(Tag::TableHead) => {
                self.in_thead = false;
                out.push_str("</thead>\n<tbody>\n");
            }
            Event::Start(Tag::TableRow) => out.push_str("<tr>"),
            Event::End(Tag::TableRow) => out.push_str("</tr>\n"),
            Event::Start(Tag::TableCell(align)) => {
                let style = match align {
                    Alignment::Left   => " style=\"text-align:left\"",
                    Alignment::Center => " style=\"text-align:center\"",
                    Alignment::Right  => " style=\"text-align:right\"",
                    Alignment::None   => "",
                };
                if self.in_thead { out.push_str(&format!("<th{}>", style)); }
                else              { out.push_str(&format!("<td{}>", style)); }
            }
            Event::End(Tag::TableCell(_)) => {
                if self.in_thead { out.push_str("</th>"); }
                else              { out.push_str("</td>"); }
            }
            Event::Start(Tag::Strong)        => out.push_str("<strong>"),
            Event::End(Tag::Strong)          => out.push_str("</strong>"),
            Event::Start(Tag::Emphasis)      => out.push_str("<em>"),
            Event::End(Tag::Emphasis)        => out.push_str("</em>"),
            Event::Start(Tag::Strikethrough) => out.push_str("<del>"),
            Event::End(Tag::Strikethrough)   => out.push_str("</del>"),
            Event::Start(Tag::Link(href)) => {
                out.push_str(&format!("<a href=\"{}\">", escape_html(href)));
            }
            Event::End(Tag::Link(_)) => out.push_str("</a>"),
            Event::Start(Tag::Image(src, alt)) => {
                out.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\" class=\"nm-image\"/>",
                    escape_html(src), escape_html(alt)
                ));
            }
            Event::End(Tag::Image(_, _)) => {}
            // Ruby
            Event::Start(Tag::Ruby(_)) => out.push_str("<ruby class=\"nm-ruby\"><rb>"),
            Event::End(Tag::Ruby(rt)) => {
                out.push_str(&format!("</rb><rt>{}</rt></ruby>", escape_html(rt)));
            }
            // Anno
            Event::Start(Tag::Anno(_)) => {
                self.in_anno = true;
                self.anno_rb_closed = false;
                out.push_str("<ruby class=\"nm-anno\"><rb>");
            }
            Event::End(Tag::Anno(_)) => {
                if self.in_anno && !self.anno_rb_closed {
                    out.push_str("</rb><rt>");
                }
                self.in_anno = false;
                out.push_str("</rt></ruby>");
            }
            Event::Start(Tag::AnnoNote) => {
                if !self.anno_rb_closed {
                    out.push_str("</rb><rt>");
                    self.anno_rb_closed = true;
                }
                out.push_str("<span class=\"nm-anno-note\">");
            }
            Event::End(Tag::AnnoNote) => out.push_str("</span>"),
            Event::FootnoteRef(n) => {
                out.push_str(&format!(
                    "<sup class=\"nm-fn-ref\"><a href=\"#fn-{n}\" id=\"fnref-{n}\">{n}</a></sup>"
                ));
            }
            Event::Start(Tag::FootnoteSection) => {
                out.push_str("<section class=\"nm-footnotes\"><ol>\n");
            }
            Event::End(Tag::FootnoteSection) => out.push_str("</ol></section>\n"),
            Event::Start(Tag::FootnoteItem(n)) => {
                out.push_str(&format!("<li id=\"fn-{n}\">"));
            }
            Event::End(Tag::FootnoteItem(n)) => {
                out.push_str(&format!(
                    " <a href=\"#fnref-{n}\" class=\"nm-fn-back\">↩</a></li>\n"
                ));
            }
        }
    }
}

/// Render to HTML without block IDs (for CLI / tests / snapshot comparison).
pub fn push_html<'a>(out: &mut String, iter: Parser<'a>) {
    push_html_inner(out, iter, false);
}

/// Render to HTML with `data-bid` attributes on block-level elements.
/// Used by the web playground for differential DOM patching.
pub fn push_html_with_ids<'a>(out: &mut String, iter: Parser<'a>) {
    push_html_inner(out, iter, true);
}

fn push_html_inner<'a>(out: &mut String, iter: Parser<'a>, block_ids: bool) {
    let start_len = out.len();
    let mut renderer = HtmlRenderer::new(block_ids);
    for event in iter {
        renderer.feed(event, out);
    }
    renderer.finish(out, start_len);
}

// ── Front matter rendering ────────────────────────────────────────────────────

/// Strip surrounding `"..."` or `'...'` quotes from a scalar value.
fn fm_scalar(raw: &str) -> &str {
    let t = raw.trim();
    if t.len() >= 2 &&
       ((t.starts_with('"') && t.ends_with('"')) ||
        (t.starts_with('\'') && t.ends_with('\'')))
    {
        &t[1..t.len() - 1]
    } else {
        t
    }
}

/// Parse a YAML flow sequence like `["a", "b", "c"]` into a Vec of strings.
fn fm_list(raw: &str) -> Vec<String> {
    let inner = raw.trim().trim_start_matches('[').trim_end_matches(']');
    inner.split(',')
        .map(|s| fm_scalar(s.trim()).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn render_frontmatter(fields: &[FrontMatterField]) -> String {
    let mut meta = String::new();
    let mut tags_html = String::new();

    for field in fields {
        if field.key == "tags" {
            for tag in fm_list(field.raw) {
                tags_html.push_str(&format!(
                    "<span class=\"nm-fm-tag\">{}</span>", escape_html(&tag)
                ));
            }
        } else {
            let val = fm_scalar(field.raw);
            meta.push_str(&format!(
                "<span class=\"nm-fm-field nm-fm-{k}\"><span class=\"nm-fm-key\">{k}</span><span class=\"nm-fm-val\">{v}</span></span>",
                k = escape_html(field.key),
                v = escape_html(val),
            ));
        }
    }

    let mut out = String::from("<div class=\"nm-frontmatter\">");
    if !meta.is_empty() {
        out.push_str("<div class=\"nm-fm-meta\">");
        out.push_str(&meta);
        out.push_str("</div>");
    }
    if !tags_html.is_empty() {
        out.push_str("<div class=\"nm-fm-tags\">");
        out.push_str(&tags_html);
        out.push_str("</div>");
    }
    out.push_str("</div>\n");
    out
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&'  => out.push_str("&amp;"),
            '<'  => out.push_str("&lt;"),
            '>'  => out.push_str("&gt;"),
            '"'  => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _    => out.push(c),
        }
    }
    out
}
