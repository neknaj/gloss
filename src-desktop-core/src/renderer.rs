use src_desktop_types::PluginHost;
use src_core::{Event, HtmlRenderer};

pub struct PluginAwareHtmlRenderer<'a, Ph: PluginHost> {
    pub host: &'a mut Ph,
}

impl<'a, Ph: PluginHost> PluginAwareHtmlRenderer<'a, Ph> {
    pub fn new(host: &'a mut Ph) -> Self { Self { host } }

    pub fn render<'ev>(
        &mut self,
        events: &[Event<'ev>],
        out: &mut alloc::string::String,
        _source: &str,
        _markdown: &str,
    ) {
        // Basic rendering via src-core HtmlRenderer; plugin integration added in Task 2
        let start_len = out.len();
        let mut renderer = HtmlRenderer::new(false);
        for event in events.iter().cloned() {
            renderer.feed(event, out);
        }
        renderer.finish(out, start_len);
    }
}
