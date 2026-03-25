use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use src_core::{HtmlRenderer, Parser, fnv1a};
use src_editor::{
    EditorState, Highlighter, HighlightContext,
    cursor::Cursor,
};
use src_desktop_types::{
    AppCmd, AppConfig, AppEvent, CursorDisplay, DirEntry, DocId, EditorLine, EditorViewModel,
    FileSystem, FsError, HtmlPatch, ImeEvent, KeyCode, KeyEvent, PaneId, PluginHost,
    PreeditDraw, VfsPath,
};
use src_plugin_types::{
    PluginFrontMatterField, PluginWarning as PW,
    to_plugin_events,
};

use crate::renderer::PluginAwareHtmlRenderer;

// ── DocumentState ─────────────────────────────────────────────────────────────

pub struct DocumentState {
    pub path:              VfsPath,
    pub editor:            EditorState,
    pub last_rendered_ver: u64,
    pub last_html:         Option<String>,
    pub last_block_hashes: Vec<u64>,
    pub warnings:          Vec<PW>,
    pub fm_config:         Option<AppConfig>,
}

// ── AppCore ───────────────────────────────────────────────────────────────────

pub struct AppCore<Fs: FileSystem, Ph: PluginHost> {
    pub fs:          Fs,
    pub plugin_host: Ph,
    pub config:      AppConfig,
    docs:            BTreeMap<DocId, DocumentState>,
    next_doc_id:     u64,
}

impl<Fs: FileSystem, Ph: PluginHost> AppCore<Fs, Ph> {

    pub fn new(fs: Fs, plugin_host: Ph, config: AppConfig) -> Self {
        Self { fs, plugin_host, config, docs: BTreeMap::new(), next_doc_id: 1 }
    }

    pub fn doc_count(&self) -> usize { self.docs.len() }

    // ── Helper ────────────────────────────────────────────────────────────────

    fn alloc_doc_id(&mut self) -> DocId {
        let id = DocId(self.next_doc_id);
        self.next_doc_id += 1;
        id
    }

    fn make_view_model(doc_id: DocId, doc: &mut DocumentState) -> EditorViewModel {
        let buf_str = doc.editor.buffer.as_str();
        let total_lines = doc.editor.buffer.line_count() as u32;

        let mut ctx = HighlightContext::Normal;
        let mut visible_lines: Vec<EditorLine> = Vec::new();

        for (line_no, line) in buf_str.lines().enumerate() {
            let next_ctx = update_ctx(line, ctx);
            let spans = Highlighter::highlight_line(line, ctx);
            ctx = next_ctx;
            visible_lines.push(src_desktop_types::EditorLine {
                line_no: line_no as u32,
                spans,
            });
        }
        if visible_lines.is_empty() {
            visible_lines.push(src_desktop_types::EditorLine { line_no: 0, spans: alloc::vec![] });
        }

        let cursor_display = CursorDisplay {
            line:       doc.editor.cursor.line as u32,
            visual_col: doc.editor.cursor.preferred_visual_col,
            blink:      true,
        };

        let preedit = doc.editor.ime.composing.as_ref().map(|p| PreeditDraw {
            text:            p.text.clone(),
            underline_range: p.cursor,
        });

        EditorViewModel {
            doc_id,
            visible_lines,
            total_lines: total_lines.max(1),
            cursor:    cursor_display,
            selection: None,
            preedit,
            scroll:    doc.editor.scroll,
            dirty:     doc.editor.version > 0,
        }
    }

    // ── Document management ───────────────────────────────────────────────────

    pub fn open_bytes(&mut self, path: VfsPath, content: Vec<u8>) -> (DocId, EditorViewModel) {
        let text = String::from_utf8(content).unwrap_or_default();
        let editor = EditorState::from_str(&text);
        let doc_id = self.alloc_doc_id();
        let mut doc = DocumentState {
            path,
            editor,
            last_rendered_ver: u64::MAX,
            last_html:         None,
            last_block_hashes: Vec::new(),
            warnings:          Vec::new(),
            fm_config:         None,
        };
        let vm = Self::make_view_model(doc_id, &mut doc);
        self.docs.insert(doc_id, doc);
        (doc_id, vm)
    }

    pub fn open_file(&mut self, path: &VfsPath) -> Result<(DocId, EditorViewModel), FsError> {
        let bytes = self.fs.read(path)?;
        Ok(self.open_bytes(path.clone(), bytes))
    }

    pub fn close_doc(&mut self, doc_id: DocId) {
        self.docs.remove(&doc_id);
    }

    pub fn save_doc(&mut self, doc_id: DocId) -> Result<(), FsError> {
        let doc = self.docs.get(&doc_id).ok_or(FsError::NotFound(VfsPath::from("")))?;
        let content = doc.editor.buffer.as_str();
        let path = doc.path.clone();
        self.fs.write(&path, content.as_bytes())
    }

    // ── Edit operations ───────────────────────────────────────────────────────

    pub fn apply_key(&mut self, doc_id: DocId, key: &KeyEvent) -> Option<EditorViewModel> {
        let doc = self.docs.get_mut(&doc_id)?;
        let editor = &mut doc.editor;

        match &key.key {
            KeyCode::Char(c) if !key.mods.ctrl && !key.mods.alt => {
                let mut s = String::new();
                s.push(*c);
                editor.insert_at_cursor(&s);
            }
            KeyCode::Backspace => { editor.backspace(); }
            KeyCode::Enter => { editor.insert_at_cursor("\n"); }
            KeyCode::Tab if !key.mods.ctrl => { editor.insert_at_cursor("    "); }
            KeyCode::ArrowRight => { editor.cursor.move_right(&mut editor.buffer); }
            KeyCode::ArrowLeft  => { editor.cursor.move_left(&mut editor.buffer); }
            KeyCode::ArrowDown  => { editor.cursor.move_down(&mut editor.buffer); }
            KeyCode::ArrowUp    => { editor.cursor.move_up(&mut editor.buffer); }
            KeyCode::Home       => { editor.cursor.move_line_start(&mut editor.buffer); }
            KeyCode::End        => { editor.cursor.move_line_end(&mut editor.buffer); }
            _ => return None,
        }

        Some(Self::make_view_model(doc_id, doc))
    }

    pub fn apply_ime(&mut self, doc_id: DocId, ime: ImeEvent) -> Option<EditorViewModel> {
        let doc = self.docs.get_mut(&doc_id)?;
        doc.editor.ime.apply(ime, &mut doc.editor.buffer, &mut doc.editor.cursor);
        Some(Self::make_view_model(doc_id, doc))
    }

    pub fn undo(&mut self, doc_id: DocId) -> Option<EditorViewModel> {
        let doc = self.docs.get_mut(&doc_id)?;
        if let Some(cursor_pos) = doc.editor.undo.undo(&mut doc.editor.buffer) {
            doc.editor.cursor.byte_pos = cursor_pos;
            doc.editor.cursor.sync_line_col(&mut doc.editor.buffer);
        }
        Some(Self::make_view_model(doc_id, doc))
    }

    pub fn redo(&mut self, doc_id: DocId) -> Option<EditorViewModel> {
        let doc = self.docs.get_mut(&doc_id)?;
        if let Some(cursor_pos) = doc.editor.undo.redo(&mut doc.editor.buffer) {
            doc.editor.cursor.byte_pos = cursor_pos;
            doc.editor.cursor.sync_line_col(&mut doc.editor.buffer);
        }
        Some(Self::make_view_model(doc_id, doc))
    }

    // ── Front matter config override ──────────────────────────────────────────

    pub fn apply_front_matter(&mut self, doc_id: DocId, _fields: &[PluginFrontMatterField]) {
        if let Some(doc) = self.docs.get_mut(&doc_id) {
            doc.fm_config = None;
        }
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    pub fn render_full(&mut self, doc_id: DocId)
        -> Option<(String, Vec<u64>, Vec<PW>)>
    {
        let (content, source, effective_lint) = {
            let doc = self.docs.get(&doc_id)?;
            let content = doc.editor.buffer.as_str();
            let source  = String::from(doc.path.as_str());
            let lint    = doc.fm_config.as_ref()
                .map(|c| c.lint.clone())
                .unwrap_or_else(|| self.config.lint.clone());
            (content, source, lint)
        };

        let mut parser = Parser::new_with_source(&content, &source);
        let events: Vec<src_core::Event<'_>> = (&mut parser).collect();
        let raw_warnings = core::mem::take(&mut parser.warnings);

        let filtered: Vec<PW> = raw_warnings.iter()
            .filter(|w| effective_lint.is_enabled(w.code))
            .map(|w| PW { code: w.code.into(), message: w.message.clone(), line: w.line, col: w.col })
            .collect();

        let plugin_events = to_plugin_events(&events);
        let mut extra = self.plugin_host.run_lint_rule(&source, &content, &filtered, &plugin_events);
        let mut all_warnings = filtered;
        all_warnings.append(&mut extra);

        let mut html = String::new();
        {
            let mut renderer = PluginAwareHtmlRenderer::new(&mut self.plugin_host);
            renderer.render(&events, &mut html, &source, &content);
        }

        let block_hashes = alloc::vec![fnv1a(&html)];

        let doc = self.docs.get_mut(&doc_id)?;
        doc.last_html         = Some(html.clone());
        doc.last_block_hashes = block_hashes.clone();
        doc.last_rendered_ver = doc.editor.version as u64;
        doc.warnings          = all_warnings.clone();

        Some((html, block_hashes, all_warnings))
    }

    pub fn render_diff(&mut self, doc_id: DocId)
        -> Option<(Vec<HtmlPatch>, Vec<PW>)>
    {
        let (html, hashes, warnings) = self.render_full(doc_id)?;
        let hash = hashes.into_iter().next().unwrap_or(0);
        Some((alloc::vec![HtmlPatch { block_id: hash, html }], warnings))
    }

    // ── FS helpers ────────────────────────────────────────────────────────────

    pub fn list_dir(&self, path: &VfsPath) -> Result<Vec<DirEntry>, FsError> {
        self.fs.list_dir(path)
    }

    pub fn create_file(&mut self, path: &VfsPath) -> Result<(), FsError> {
        self.fs.write(path, &[])
    }

    pub fn delete_file(&mut self, path: &VfsPath) -> Result<(), FsError> {
        self.fs.delete(path)
    }

    pub fn rename_file(&mut self, from: &VfsPath, to: &VfsPath) -> Result<(), FsError> {
        self.fs.rename(from, to)
    }

    pub fn apply_config(&mut self, config: AppConfig) {
        self.config = config;
    }

    // ── execute_cmds ──────────────────────────────────────────────────────────

    pub fn execute_cmds(&mut self, cmds: Vec<AppCmd>)
        -> (Vec<AppEvent>, Vec<AppCmd>)
    {
        let mut result_events: Vec<AppEvent> = Vec::new();
        let mut shell_cmds:    Vec<AppCmd>   = Vec::new();

        for cmd in cmds {
            match cmd {
                AppCmd::WriteFile { path, content } => {
                    let doc_content = if content.is_empty() {
                        self.docs.values()
                            .find(|d| d.path == path)
                            .map(|d| d.editor.buffer.as_str().into_bytes())
                            .unwrap_or(content)
                    } else {
                        content
                    };
                    match self.fs.write(&path, &doc_content) {
                        Ok(()) => result_events.push(AppEvent::FileSaved { path }),
                        Err(e) => result_events.push(AppEvent::FileError {
                            path, error: alloc::format!("{e}"),
                        }),
                    }
                }
                AppCmd::ListDir { path } => {
                    match self.fs.list_dir(&path) {
                        Ok(entries) => result_events.push(AppEvent::DirLoaded { path, entries }),
                        Err(e) => result_events.push(AppEvent::FileError {
                            path, error: alloc::format!("{e}"),
                        }),
                    }
                }
                AppCmd::RunRender { pane_id, doc_id } => {
                    if let Some((html, _hashes, warnings)) = self.render_full(doc_id) {
                        result_events.push(AppEvent::RenderComplete { pane_id, html, warnings });
                    }
                }
                AppCmd::RunLint { doc_id } => {
                    let (content, source, effective_lint) = match self.docs.get(&doc_id) {
                        None => continue,
                        Some(doc) => {
                            let content = doc.editor.buffer.as_str();
                            let source  = String::from(doc.path.as_str());
                            let lint    = doc.fm_config.as_ref()
                                .map(|c| c.lint.clone())
                                .unwrap_or_else(|| self.config.lint.clone());
                            (content, source, lint)
                        }
                    };
                    let mut parser = Parser::new_with_source(&content, &source);
                    let events: Vec<src_core::Event<'_>> = (&mut parser).collect();
                    let raw_warnings = core::mem::take(&mut parser.warnings);
                    let mut filtered: Vec<PW> = raw_warnings.iter()
                        .filter(|w| effective_lint.is_enabled(w.code))
                        .map(|w| PW { code: w.code.into(), message: w.message.clone(), line: w.line, col: w.col })
                        .collect();
                    let plugin_events = to_plugin_events(&events);
                    let mut extra = self.plugin_host.run_lint_rule(&source, &content, &filtered, &plugin_events);
                    filtered.append(&mut extra);
                    if let Some(doc) = self.docs.get_mut(&doc_id) {
                        doc.warnings = filtered.clone();
                    }
                    result_events.push(AppEvent::LintComplete { doc_id, warnings: filtered });
                }
                other => { shell_cmds.push(other); }
            }
        }

        (result_events, shell_cmds)
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn update_ctx(line: &str, ctx: HighlightContext) -> HighlightContext {
    match ctx {
        HighlightContext::Normal => {
            if line.starts_with("```") { HighlightContext::InCodeBlock { lang: "text" } }
            else if line.trim_start().starts_with("$$") { HighlightContext::InMathBlock }
            else { HighlightContext::Normal }
        }
        HighlightContext::InCodeBlock { .. } => {
            if line.trim_start().starts_with("```") { HighlightContext::Normal }
            else { ctx }
        }
        HighlightContext::InMathBlock => {
            if line.trim() == "$$" { HighlightContext::Normal }
            else { ctx }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;
    use src_desktop_types::{AppConfig, MemoryVfs, VfsPath, FileSystem};
    use src_desktop_types::noop::NoopPluginHost;
    use super::AppCore;

    fn make_core() -> AppCore<MemoryVfs, NoopPluginHost> {
        AppCore::new(MemoryVfs::default(), NoopPluginHost, AppConfig::default())
    }

    #[test]
    fn new_has_no_docs() {
        assert_eq!(make_core().doc_count(), 0);
    }

    #[test]
    fn open_bytes_returns_view_model() {
        let mut core = make_core();
        let (doc_id, vm) = core.open_bytes(VfsPath::from("a.n.md"), b"hello".to_vec());
        assert_eq!(core.doc_count(), 1);
        assert_eq!(vm.doc_id, doc_id);
    }

    #[test]
    fn close_doc_removes_it() {
        let mut core = make_core();
        let (doc_id, _) = core.open_bytes(VfsPath::from("a.n.md"), b"hi".to_vec());
        core.close_doc(doc_id);
        assert_eq!(core.doc_count(), 0);
    }

    #[test]
    fn open_file_reads_from_fs() {
        let mut vfs = MemoryVfs::default();
        vfs.write(&VfsPath::from("doc.n.md"), b"# Hello").unwrap();
        let mut core = AppCore::new(vfs, NoopPluginHost, AppConfig::default());
        let result = core.open_file(&VfsPath::from("doc.n.md"));
        assert!(result.is_ok());
        let (_, vm) = result.unwrap();
        assert!(vm.total_lines >= 1);
    }

    #[test]
    fn save_doc_writes_to_fs() {
        let mut core = make_core();
        let path = VfsPath::from("out.n.md");
        let (doc_id, _) = core.open_bytes(path.clone(), b"content".to_vec());
        assert!(core.save_doc(doc_id).is_ok());
        let bytes = core.fs.read(&path).unwrap();
        assert_eq!(bytes, b"content");
    }

    #[test]
    fn apply_key_char_updates_vm() {
        use src_desktop_types::{KeyCode, KeyEvent, Modifiers};
        let mut core = make_core();
        let (doc_id, _) = core.open_bytes(VfsPath::from("x.n.md"), b"".to_vec());
        let key = KeyEvent { key: KeyCode::Char('A'), mods: Modifiers::default(), text: None };
        let vm = core.apply_key(doc_id, &key);
        assert!(vm.is_some());
        assert!(vm.unwrap().dirty);
    }

    #[test]
    fn undo_redo_roundtrip() {
        use src_desktop_types::{KeyCode, KeyEvent, Modifiers};
        let mut core = make_core();
        let (doc_id, _) = core.open_bytes(VfsPath::from("x.n.md"), b"".to_vec());
        let key = KeyEvent { key: KeyCode::Char('X'), mods: Modifiers::default(), text: None };
        core.apply_key(doc_id, &key);
        core.undo(doc_id);
        core.redo(doc_id);
    }

    #[test]
    fn render_full_returns_html() {
        let mut core = make_core();
        let (doc_id, _) = core.open_bytes(VfsPath::from("x.n.md"), b"# Title".to_vec());
        let result = core.render_full(doc_id);
        assert!(result.is_some());
        let (html, _, _) = result.unwrap();
        assert!(html.contains("Title"));
    }

    #[test]
    fn execute_cmds_run_render_emits_render_complete() {
        use src_desktop_types::{AppCmd, AppEvent, PaneId};
        let mut core = make_core();
        let (doc_id, _) = core.open_bytes(VfsPath::from("x.n.md"), b"hi".to_vec());
        let cmds = vec![AppCmd::RunRender { pane_id: PaneId(1), doc_id }];
        let (events, shell): (Vec<AppEvent>, Vec<AppCmd>) = core.execute_cmds(cmds);
        assert!(events.iter().any(|e| matches!(e, AppEvent::RenderComplete { .. })));
        assert!(shell.is_empty());
    }

    #[test]
    fn execute_cmds_passes_unknown_to_shell() {
        use src_desktop_types::{AppCmd, VfsPath};
        let mut core = make_core();
        let cmds = vec![AppCmd::ReadFile { path: VfsPath::from("x.n.md") }];
        let (events, shell): (Vec<src_desktop_types::AppEvent>, Vec<AppCmd>) = core.execute_cmds(cmds);
        assert!(events.is_empty());
        assert_eq!(shell.len(), 1);
    }
}
