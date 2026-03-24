# src-desktop 設計仕様

> **日付:** 2026-03-24
> **対象:** Gloss プロジェクトへの Tauri ベース GUI エディタ追加
> **ステータス:** Draft

---

## 1. 概要

Gloss プロジェクトに VSCode 風パネル/タブレイアウトを持つ GUI エディタ `src-desktop` を追加する。
既存の no_std 哲学を継承し、IO を極限まで抽象化することで究極のマルチプラットフォーム設計とする。

**主要方針：**

- I/O（FS・プラグイン・クリップボード・IME）をトレイトで抽象化し、ロジック層をすべて no_std + alloc に保つ
- Tauri は「薄いシェル」として機能し、トレイトの native 実装を提供するだけ
- 同じロジック層を WASM ターゲットでも使用し `src-web` を廃止・統合する
- 日本語 IME 対応のカスタムテキストエディタを実装する
- `src-cli` は機能を維持しつつ内部実装を `src-desktop-core` ベースに刷新する（後方互換なし）

---

## 2. クレート構成

```
src-core              (no_std + alloc)  パーサー・HtmlRenderer [変更なし]
src-plugin-types      (no_std + alloc)  プラグインフック I/O 型 [serde_json alloc mode に修正]
src-plugin            (std)             Extism ホスト [変更なし・native only]

src-desktop-types     (no_std + alloc)  プロトコル定義層
src-editor            (no_std + alloc)  テキストエディタコア
src-desktop-layout    (no_std + alloc)  Elm アーキテクチャ
src-desktop-core      (no_std + alloc)  アプリケーションロジック

src-desktop           (std + Tauri)     Tauri プラットフォームシェル
src-desktop-wasm      (std + wasm32)    WASM シェル（src-web 後継）

src-cli               (std)             CLI 変換ツール [src-desktop-core ベースに刷新]
```

### 依存グラフ

```
src-core ──────────────────────────────────────────────┐
src-plugin-types ──────────────────────────────────────┤
src-desktop-types ─────────────────────────────────────┤
                                                        ↓
src-editor          ──→ src-desktop-core ──→ src-desktop  (std+Tauri)
src-desktop-layout  ──→                  ──→ src-desktop-wasm (wasm32)
                                          ──→ src-cli         (std)

src-plugin (std, native only)
  ← src-desktop のみが依存（ExtismHost として）
  ← src-cli が依存（TOML パース・ExtismHost として）
```

### no_std 境界

| クレート | no_std | 備考 |
|---------|--------|------|
| `src-core` | ✅ | 変更なし |
| `src-plugin-types` | ✅ | `serde_json = { default-features=false, features=["alloc"] }` |
| `src-desktop-types` | ✅ | MemoryVfs・トレイト定義含む |
| `src-editor` | ✅ | GapBuffer・IME・undo |
| `src-desktop-layout` | ✅ | Elm update/view 純粋関数 |
| `src-desktop-core` | ✅ | トレイト境界で I/O 注入 |
| `src-desktop` | ❌ | std + Tauri |
| `src-desktop-wasm` | ❌ | std + wasm32 |
| `src-cli` | ❌ | std |

---

## 3. `src-desktop-types` — プロトコル定義層

### 3.1 コアトレイト

```rust
pub trait FileSystem {
    fn read(&self, path: &VfsPath) -> Result<Vec<u8>, FsError>;
    fn write(&mut self, path: &VfsPath, data: &[u8]) -> Result<(), FsError>;
    fn list_dir(&self, path: &VfsPath) -> Result<Vec<DirEntry>, FsError>;
    fn exists(&self, path: &VfsPath) -> bool;
    fn create_dir(&mut self, path: &VfsPath) -> Result<(), FsError>;
    fn delete(&mut self, path: &VfsPath) -> Result<(), FsError>;
    fn rename(&mut self, from: &VfsPath, to: &VfsPath) -> Result<(), FsError>;
    fn is_dir(&self, path: &VfsPath) -> bool;
}

pub trait PluginHost {
    fn run_code_highlight(&mut self, lang: &str, code: &str, filename: &str) -> Option<String>;
    fn run_card_link(&mut self, url: &str) -> Option<CardLinkOutput>;
    fn run_lint_rule(&mut self, src: &str, md: &str,
        existing: &[PluginWarning], events: &[PluginEvent]) -> Vec<PluginWarning>;
    fn run_front_matter(&mut self, fields: &[PluginFrontMatterField], src: &str) -> Option<String>;
}

pub trait Clipboard {
    fn get_text(&self) -> Option<String>;
    fn set_text(&mut self, text: &str);
}

pub trait ImeSource {
    fn poll_event(&mut self) -> Option<ImeEvent>;
}
```

### 3.2 IME イベント型

```rust
pub enum ImeEvent {
    Start,
    Update { preedit: String, cursor: Option<(usize, usize)> },
    Commit { text: String },
    Cancel,
}
```

### 3.3 `AppEvent` — 全入力イベント

```rust
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Ime(ImeEvent),
    Resize { width: u32, height: u32 },
    // FS 完了通知
    FileLoaded  { path: VfsPath, content: Vec<u8> },
    FileSaved   { path: VfsPath },
    FileError   { path: VfsPath, error: FsError },
    DirLoaded   { path: VfsPath, entries: Vec<DirEntry> },
    // レンダリング完了
    RenderComplete { pane_id: PaneId, html: String, warnings: Vec<PluginWarning> },
    // 設定ロード完了
    ConfigLoaded(AppConfig),
    // クリップボード
    ClipboardText(String),
    // UI 操作
    TabSelected       { pane_id: PaneId, tab_index: usize },
    PaneSplitRequested { pane_id: PaneId, direction: SplitDirection },
    // プレビュー
    PreviewLinkClicked { pane_id: PaneId, url: String, new_tab: bool },
    PreviewScrolled    { pane_id: PaneId, offset_y: f32 },
    Quit,
}
```

### 3.4 `AppCmd` — 非同期サイドエフェクト命令

```rust
pub enum AppCmd {
    ReadFile      { path: VfsPath },
    WriteFile     { path: VfsPath, content: Vec<u8> },
    ListDir       { path: VfsPath },
    LoadConfig    { path: VfsPath },
    RunRender     { pane_id: PaneId, doc_id: DocId, markdown: String },
    RunLint       { doc_id: DocId, markdown: String },
    ScheduleRender { pane_id: PaneId, delay_ms: u32 },
    CopyToClipboard { text: String },
    PasteRequest,
    SetImeCursorArea { rect: Rect },
    ShowOpenFileDialog,
    ShowSaveFileDialog { suggested: Option<VfsPath> },
    OpenUrl { url: String },
    Quit,
}
```

### 3.5 `DrawCmd` — レンダリング命令

パネル/タブの境界ボックスは Rust 側で計算し、JS 側で `position: absolute` として配置する。
パネル内コンテンツ（ファイルツリーの項目等）の詳細描画は HTML/CSS に委譲する。
エディタペインのみ Canvas 精密描画を行う。

```rust
pub enum DrawCmd {
    // ── レイアウト（絶対座標）──
    SetLayout {
        panels:   Vec<PanelLayout>,
        dividers: Vec<DividerLayout>,
    },
    SetTabBar {
        pane_id:    PaneId,
        tabs:       Vec<TabInfo>,
        active_tab: usize,
    },

    // ── エディタペイン（Canvas 精密描画）──
    EditorFrame {
        pane_id:   PaneId,
        bounds:    Rect,
        lines:     Vec<EditorLine>,
        cursor:    CursorDraw,
        selection: Option<SelectionDraw>,
        preedit:   Option<PreeditDraw>,
        scroll:    ScrollOffset,
    },

    // ── プレビューペイン（HTML 丸投げ）──
    PreviewMount  { pane_id: PaneId, html: String },
    PreviewPatch  { pane_id: PaneId, patches: Vec<HtmlPatch> },
    PreviewScroll { pane_id: PaneId, offset_y: f32 },

    // ── コンテンツデータ（内部描画は HTML/CSS に委譲）──
    SetFileTree   { entries: Vec<FileTreeEntry>, expanded: Vec<VfsPath> },
    SetPluginList { plugins: Vec<PluginInfo> },
    SetStatusBar  { left: String, right: String, warning_count: u32 },
    SetWarnings   { warnings: Vec<WarningInfo> },

    // ── IME・オーバーレイ ──
    SetImeCursorArea { rect: Rect },
    ShowDialog       { kind: DialogKind },
    ShowTooltip      { x: f32, y: f32, text: String },
    HideTooltip,
}
```

### 3.6 `AppConfig`（TOML パースなし・no_std）

```rust
pub struct AppConfig {
    pub lint:    LintRules,            // BTreeMap<String, bool>
    pub plugins: Vec<PluginEntrySpec>, // id / path / hooks / config
}
// src-desktop / src-cli の std 層が GlossConfig → AppConfig に変換する
```

### 3.7 `MemoryVfs`

```rust
pub enum VfsNode {
    File { name: String, content: Vec<u8> },
    Dir  { name: String, children: BTreeMap<String, VfsNode> },
}

pub struct MemoryVfs { root: VfsNode }
impl FileSystem for MemoryVfs { /* 全メソッド実装 */ }
impl MemoryVfs {
    pub fn iter_files(&self) -> impl Iterator<Item = (&VfsPath, &[u8])>;
}
```

---

## 4. `src-editor` — テキストエディタコア

### 4.1 GapBuffer

```rust
pub struct GapBuffer {
    buf:       Vec<u8>,   // UTF-8 バイト列
    gap_start: usize,
    gap_end:   usize,
}

impl GapBuffer {
    pub fn insert(&mut self, byte_pos: usize, text: &str);
    pub fn delete(&mut self, byte_range: Range<usize>) -> String;
    pub fn slice(&self, byte_range: Range<usize>) -> &str;
    pub fn as_str(&self) -> String;
    pub fn len_bytes(&self) -> usize;
    pub fn len_chars(&self) -> usize;
    pub fn line_count(&self) -> usize;
    pub fn line_to_byte(&self, line: usize) -> usize;
    pub fn byte_to_line(&self, byte_pos: usize) -> usize;
    pub fn char_visual_width(c: char) -> u32;  // ASCII=1, CJK=2
}
```

### 4.2 Cursor（CJK 対応）

```rust
pub struct Cursor {
    pub byte_pos:           usize,
    pub line:               usize,
    pub col_byte:           usize,
    pub preferred_visual_col: u32,  // ↑↓移動時に保持
}

impl Cursor {
    pub fn move_right(&mut self, buf: &GapBuffer);
    pub fn move_left(&mut self, buf: &GapBuffer);
    pub fn move_down(&mut self, buf: &GapBuffer);
    pub fn move_up(&mut self, buf: &GapBuffer);
    pub fn move_line_start(&mut self, buf: &GapBuffer);
    pub fn move_line_end(&mut self, buf: &GapBuffer);
    pub fn move_word_right(&mut self, buf: &GapBuffer);  // 文字種変化点を境界とする
    pub fn move_word_left(&mut self, buf: &GapBuffer);
    pub fn visual_col(&self, buf: &GapBuffer) -> u32;
}
```

**日本語単語境界：** 形態素解析は行わず「文字種の変化点」（ASCII↔CJK↔ひらがな等）を境界とする。

### 4.3 ImeState（変換中テキスト管理）

```rust
pub struct ImeState {
    pub composing: Option<Preedit>,
}

pub struct Preedit {
    pub text:            String,
    pub cursor:          Option<(usize, usize)>,
    pub insert_byte_pos: usize,  // バッファ内の挿入予定位置
}

impl ImeState {
    pub fn is_composing(&self) -> bool;
    pub fn apply(&mut self, event: ImeEvent, buf: &mut GapBuffer, cursor: &mut Cursor);
}
```

- `Start` → 現在のカーソル位置を記録、バッファは未変更
- `Update` → preedit テキストを保持するだけ（バッファ未変更）
- `Commit` → `insert_byte_pos` にテキストを挿入してカーソルを末尾へ
- `Cancel` → preedit をクリア

### 4.4 UndoHistory

```rust
pub struct UndoHistory { ... }
pub enum EditOp {
    Insert { byte_pos: usize, text: String },
    Delete { byte_pos: usize, deleted: String },
}
```

- `begin_group` / `end_group` でトランザクションをサポート
- IME: `Start` → `begin_group`、`Commit` → `end_group`（変換→確定が1undo単位）

### 4.5 EditorState

```rust
pub struct EditorState {
    pub buffer:    GapBuffer,
    pub cursor:    Cursor,
    pub selection: Option<Selection>,
    pub ime:       ImeState,
    pub undo:      UndoHistory,
    pub scroll:    ScrollOffset,
    pub version:   u64,  // 変更ごとにインクリメント（デバウンス用）
}
```

### 4.6 Highlighter（シンタックスハイライト）

```rust
pub struct Highlighter;
impl Highlighter {
    pub fn highlight_line<'a>(line: &'a str, ctx: HighlightContext) -> Vec<TextSpan<'a>>;
}
pub enum HighlightContext { Normal, InCodeBlock { lang: &'static str }, InMathBlock }
```

状態機械で `.n.md` 固有構文（ルビ・アノ・数式・見出し・コードブロック等）を着色。
正規表現不使用・no_std で完結。

### 4.7 EditorViewModel（表示スナップショット、`src-desktop-types` に定義）

```rust
pub struct EditorViewModel {
    pub doc_id:        DocId,
    pub visible_lines: Vec<EditorLine>,
    pub total_lines:   u32,
    pub cursor:        CursorDisplay,
    pub selection:     Option<SelectionDisplay>,
    pub preedit:       Option<PreeditDraw>,
    pub scroll:        ScrollOffset,
    pub dirty:         bool,
}
```

`src-desktop-core` が `EditorState` から計算して `Model` に格納する。
`src-desktop-layout` の `view()` はこのスナップショットを参照するだけで、バッファには触れない。

---

## 5. `src-desktop-layout` — Elm アーキテクチャ

### 5.1 Model

```rust
pub struct Model {
    pub workspace:      WorkspaceState,
    pub layout:         LayoutState,
    pub file_tree:      FileTreeState,
    pub plugin_manager: PluginManagerState,
    pub status:         StatusState,
    pub config:         AppConfig,
}

pub struct WorkspaceState {
    pub root_path: Option<VfsPath>,
    pub open_docs: BTreeMap<DocId, DocMeta>,
    pub editors:   BTreeMap<DocId, EditorViewModel>,
    pub previews:  BTreeMap<DocId, PreviewState>,
}

pub struct LayoutState {
    pub root:        PanelNode,
    pub window_size: (u32, u32),
    pub focus:       FocusTarget,
}

pub enum PanelNode {
    Split { direction: SplitDirection, ratio: f32, a: Box<PanelNode>, b: Box<PanelNode> },
    Leaf(Pane),
}

pub struct Pane {
    pub id:         PaneId,
    pub kind:       PaneKind,
    pub tabs:       Vec<Tab>,
    pub active_tab: usize,
    pub bounds:     Rect,  // view() が計算して設定
}

pub enum PaneKind { Editor, Preview, FileTree, PluginManager }
```

### 5.2 `update()` — 純粋関数

```rust
pub fn update(model: Model, event: AppEvent) -> (Model, Vec<AppCmd>)
```

**主要ケース：**

| イベント | 動作 |
|--------|------|
| `Key(Ctrl+S)` | `AppCmd::WriteFile` |
| `Key(Ctrl+W)` | dirty なら `AppCmd::ShowSaveFileDialog`、そうでなければタブ閉じ |
| `Key(Ctrl+\)` | ペイン分割 |
| `Key(文字入力)` | `EditorViewModel` 更新 + `AppCmd::ScheduleRender` |
| `Ime(event)` | IME 処理（`AppCore` に委譲） |
| `Resize(w,h)` | `window_size` 更新（bounds 再計算は `view()` 担当） |
| `FileLoaded` | `EditorViewModel` 生成 + `AppCmd::RunRender` |
| `RenderComplete` | `PreviewState` 更新 |
| `PreviewLinkClicked` | 同タブ/新規タブ/外部 URL 処理 |

**Model は値渡し：** `update` は前の `Model` を消費して新しい `Model` を返す。
`src-desktop` 側が `Arc<Mutex<Model>>` で管理する。

### 5.3 `view()` — 純粋関数

```rust
pub fn view(model: &Model) -> Vec<DrawCmd>
```

1. `PanelNode` ツリーを再帰走査し `window_size` + `ratio` から各 `Pane.bounds` を計算
2. 各ペインを描画：
   - `Editor` → `DrawCmd::EditorFrame`
   - `Preview` → `DrawCmd::PreviewMount` / `PreviewPatch`
   - `FileTree` → `DrawCmd::SetFileTree`
   - `PluginManager` → `DrawCmd::SetPluginList`
3. タブバー → `DrawCmd::SetTabBar`
4. ステータスバー → `DrawCmd::SetStatusBar`

`view()` は副作用ゼロ。`AppCmd` は返さない。

---

## 6. `src-desktop-core` — アプリケーションロジック層

### 6.1 `AppCore<Fs, Ph>`

```rust
pub struct AppCore<Fs: FileSystem, Ph: PluginHost> {
    pub fs:          Fs,
    pub plugin_host: Ph,
    pub config:      AppConfig,
    docs:            BTreeMap<DocId, DocumentState>,
    next_doc_id:     u64,
}

pub struct DocumentState {
    pub path:              VfsPath,
    pub editor:            EditorState,
    pub last_rendered_ver: u64,
    pub last_html:         Option<String>,
    pub last_block_hashes: Vec<u64>,
    pub warnings:          Vec<PluginWarning>,
}
```

### 6.2 主要メソッド

```rust
impl<Fs: FileSystem, Ph: PluginHost> AppCore<Fs, Ph> {
    // ドキュメント管理
    pub fn open_file(&mut self, path: &VfsPath)
        -> Result<(DocId, EditorViewModel), FsError>;
    pub fn open_bytes(&mut self, path: VfsPath, content: Vec<u8>)
        -> (DocId, EditorViewModel);
    pub fn close_doc(&mut self, doc_id: DocId);
    pub fn save_doc(&mut self, doc_id: DocId) -> Result<(), FsError>;

    // 編集操作
    pub fn apply_key(&mut self, doc_id: DocId, key: &KeyEvent)
        -> Option<EditorViewModel>;
    pub fn apply_ime(&mut self, doc_id: DocId, ime: ImeEvent)
        -> Option<EditorViewModel>;
    pub fn undo(&mut self, doc_id: DocId) -> Option<EditorViewModel>;
    pub fn redo(&mut self, doc_id: DocId) -> Option<EditorViewModel>;

    // レンダリング
    pub fn render_full(&mut self, doc_id: DocId)
        -> Option<(String, Vec<u64>, Vec<PluginWarning>)>;
    pub fn render_diff(&mut self, doc_id: DocId)
        -> Option<(Vec<HtmlPatch>, Vec<PluginWarning>)>;

    // FS 操作
    pub fn list_dir(&self, path: &VfsPath)
        -> Result<Vec<DirEntry>, FsError>;
    pub fn create_file(&mut self, path: &VfsPath)
        -> Result<(), FsError>;
    pub fn delete_file(&mut self, path: &VfsPath)
        -> Result<(), FsError>;
    pub fn rename_file(&mut self, from: &VfsPath, to: &VfsPath)
        -> Result<(), FsError>;

    // 設定
    pub fn apply_config(&mut self, config: AppConfig);
}
```

### 6.3 レンダリングパイプライン（内部）

```
buffer.as_str()
  → Parser::new_with_source(&text, &path)
  → warnings 収集
  → config.lint でフィルタ
  → plugin_host.run_lint_rule(...)      ← Ph トレイト経由
  → PluginAwareHtmlRenderer::render()   ← src-desktop-core 内で定義
  → html + block_hashes を返す
```

### 6.4 `PluginAwareHtmlRenderer<Ph: PluginHost>`

`src-plugin/renderer.rs` の `GlossPluginHost` 依存版に代わり、`PluginHost` トレイトで汎用化した版を `src-desktop-core` 内に実装。ロジック（CodeBlock 走査・CardLink 処理・fallback replay）は同等。

---

## 7. `src-desktop` — Tauri シェル

### 7.1 トレイト実装

```rust
// NativeFs: std::fs ラッパー
pub struct NativeFs;
impl FileSystem for NativeFs { /* std::fs 使用 */ }

// ExtismHost: src-plugin::GlossPluginHost に委譲
pub struct ExtismHost(GlossPluginHost);
impl PluginHost for ExtismHost { /* 全フック委譲 */ }

// NativeClipboard: arboard クレート使用
pub struct NativeClipboard(arboard::Clipboard);
impl Clipboard for NativeClipboard { ... }

// TauriIme: Tauri IPC 経由で JS compositionイベントを受け取る
pub struct TauriIme { queue: VecDeque<ImeEvent> }
impl ImeSource for TauriIme { fn poll_event(&mut self) -> Option<ImeEvent> { ... } }
```

### 7.2 AppState

```rust
pub struct AppState {
    pub model:     Model,
    pub core:      AppCore<NativeFs, ExtismHost>,
    pub clipboard: NativeClipboard,
    pub ime:       TauriIme,
}
type SharedState = Mutex<AppState>;
```

### 7.3 IME 統合

Canvas ベースのエディタで IME 候補ウィンドウを正しい位置に表示する標準的な手法：

1. 不可視 `<input>` 要素をカーソル位置（`DrawCmd::SetImeCursorArea`）に配置してフォーカスする
2. `compositionstart/update/end` を JS が受け取り Tauri IPC で Rust へ転送
3. `TauriIme.queue` に積まれた `ImeEvent` を次のディスパッチサイクルで処理

### 7.4 フロントエンド構成

```
src-desktop/frontend/
  index.html       ── 最小限シェル（Canvas + Preview div + Chrome HTML）
  renderer.ts      ── DrawCmd ディスパッチャ
  editor-canvas.ts ── EditorFrame → Canvas 2D 精密描画
  preview.ts       ── PreviewMount/Patch → innerHTML
  ime-bridge.ts    ── composition イベント → Tauri IPC
  chrome.ts        ── タブバー・ファイルツリー・ステータスバーの DOM 操作
  style.css        ── web-playground の nm-* スタイルを流用
```

**2つのレンダリング面：**

| ペイン | 方式 | 理由 |
|--------|------|------|
| エディタ | Canvas 2D | カーソル/IME 候補窓の精密な位置制御 |
| プレビュー | `<div>` innerHTML | HTML レンダリングは WebView に丸投げ |
| UI クロム | HTML DOM | CSS でホバー・アニメーション等を高速実装 |

### 7.5 Tauri コマンドハンドラ（イベントループ）

```rust
#[tauri::command]
async fn dispatch(state: tauri::State<'_, SharedState>, window: tauri::Window, event: AppEvent)
-> Result<(), String> {
    let draw_cmds = tokio::task::spawn_blocking({
        let state = state.inner().clone();
        move || {
            let mut s = state.lock().unwrap();
            // 1. IME イベントをキューから処理
            while let Some(ev) = s.ime.poll_event() {
                let (m, cmds) = update(s.model.clone(), AppEvent::Ime(ev));
                s.model = m;
                s.core.execute_cmds(cmds);
            }
            // 2. メインイベント処理
            let (new_model, cmds) = update(s.model.clone(), event);
            s.model = new_model;
            let result_events = s.core.execute_cmds(cmds);
            // 3. AppCmd 結果を再投入
            for ev in result_events {
                let (m, c) = update(s.model.clone(), ev);
                s.model = m;
                s.core.execute_cmds(c);
            }
            // 4. DrawCmd 生成
            view(&s.model)
        }
    }).await.map_err(|e| e.to_string())?;

    window.emit("draw", draw_cmds).map_err(|e| e.to_string())
}
```

---

## 8. `src-desktop-wasm` — WASM シェル（`src-web` 後継）

### 8.1 トレイト実装

```rust
// MemoryVfs は src-desktop-types で定義済み
pub struct NoopPluginHost;
impl PluginHost for NoopPluginHost { /* 全フックが None/空を返す */ }

pub struct WasmClipboard;
impl Clipboard for WasmClipboard { /* navigator.clipboard API */ }

pub struct WasmIme { queue: Rc<RefCell<VecDeque<ImeEvent>>> }
impl ImeSource for WasmIme { /* DOM compositionイベント */ }
```

### 8.2 公開 API

```rust
#[wasm_bindgen]
pub struct GlossApp { model: Model, core: AppCore<MemoryVfs, NoopPluginHost> }

#[wasm_bindgen]
impl GlossApp {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self;
    pub fn dispatch(&mut self, event_json: &str) -> String; // DrawCmd JSON を返す
    pub fn export_zip(&self) -> Vec<u8>;
    pub fn import_zip(&mut self, data: &[u8]) -> Result<(), JsValue>;
}
```

### 8.3 VFS と Zip

`src-desktop-wasm` は std あり（wasm32 ターゲットは std サポート済み）なので `zip` クレートが使用可能。

```rust
pub fn vfs_to_zip(vfs: &MemoryVfs) -> Vec<u8> { /* zip::ZipWriter */ }
pub fn zip_to_vfs(data: &[u8]) -> Result<MemoryVfs, String> { /* zip::ZipArchive */ }
```

### 8.4 IndexedDB 永続化（オプション）

```toml
[features]
default = []
persist = ["rexie"]
```

開発中は `default` のままオフ。必要時に `persist` フィーチャーを有効化。

### 8.5 プレビュー差分更新

`src-core::split_source_blocks()` と `fnv1a()`（既に no_std 実装済み）を使い、変更ブロックのみ `PreviewPatch` で更新することで DOM のフラッシュを抑制する。

---

## 9. `src-cli` の刷新

### 9.1 方針

- `.n.md → HTML` 変換機能は維持する
- 後方互換なしで `src-desktop-core` ベースに刷新する
- `src-plugin/renderer.rs`（旧 `PluginAwareRenderer`）は削除対象とする

### 9.2 新しい CLI パイプライン

```
1. CLI 引数パース（input / output / --config）
2. GlossConfig::from_file() → AppConfig に変換
3. NativeFs + ExtismHost を構築して AppCore::new()
4. AppCore::open_file(input_path)
5. AppCore::render_full(doc_id) → (html, _, warnings)
6. warnings を stderr に出力
7. HTML_HEAD + html + HTML_TAIL をファイルに書き込む
```

---

## 10. テスト戦略

### `src-desktop-layout`：純粋関数テスト（cargo test）

```rust
#[test] fn split_pane_divides_window_evenly() { ... }
#[test] fn tab_close_dirty_doc_emits_save_dialog() { ... }
#[test] fn keyboard_ctrl_s_emits_write_file() { ... }
#[test] fn view_assigns_correct_bounds_to_split_panes() { ... }
```

### `src-desktop-core`：MemoryVfs + NoopPluginHost 注入

```rust
fn make_test_core() -> AppCore<MemoryVfs, NoopPluginHost> {
    let mut vfs = MemoryVfs::new();
    vfs.write(&"/test.n.md".into(), b"# Hello\n\nWorld.").unwrap();
    AppCore::new(vfs, NoopPluginHost, AppConfig::default())
}
#[test] fn open_file_returns_editor_view_model() { ... }
#[test] fn render_full_returns_html_with_heading() { ... }
#[test] fn save_doc_writes_to_vfs() { ... }
#[test] fn lint_config_suppresses_warning() { ... }
```

### `src-editor`：バッファ・IME 単体テスト

```rust
#[test] fn insert_japanese_and_measure_visual_col() { ... }
#[test] fn ime_commit_inserts_text_and_clears_preedit() { ... }
#[test] fn undo_restores_buffer_after_ime_commit() { ... }
#[test] fn word_boundary_detects_cjk_ascii_transition() { ... }
```

### `src-desktop-types`：MemoryVfs 単体テスト

```rust
#[test] fn memory_vfs_create_and_read() { ... }
#[test] fn memory_vfs_rename_updates_path() { ... }
#[test] fn memory_vfs_list_dir_returns_entries() { ... }
```

---

## 11. `src-plugin` 移行方針

| 対象 | 現状 | 最終方針 |
|------|------|---------|
| `GlossPluginHost` | `src-plugin` に存在 | 変更なし。`ExtismHost` としてラップして使用 |
| `PluginAwareRenderer` | `src-plugin/renderer.rs` | **削除**。`src-desktop-core` の汎用版に一本化 |
| `GlossConfig` | `src-plugin/config.rs` | TOML パースはここに残す。`AppConfig` への変換は各シェル層が担当 |
| `src-cli` | `src-plugin` 直接使用 | `src-desktop-core` ベースに刷新（後方互換なし） |
| `src-web` | WASM バインディング | `src-desktop-wasm` に置き換えて廃止 |

---

## 12. 将来の拡張性

- **egui / iced フロントエンド：** `SetLayout`（パネル境界）と `EditorFrame` を解釈するレンダラーを新たに実装するだけで移植可能
- **TUI フロントエンド：** 同様に `DrawCmd` を文字ベースでレンダリングする実装を追加
- **WASM プラグイン（Web 環境）：** `PluginHost` トレイトの WASM 実装として `WebAssembly.instantiate()` ベースのホストを将来実装可能（現フェーズでは `NoopPluginHost`）
- **`src-cli` からのサーバーモード：** `AppCore` をそのまま HTTP サーバーのハンドラとして使用可能（no_std ロジック層を再利用）
