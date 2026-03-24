# src-desktop 設計仕様

> **日付:** 2026-03-24
> **対象:** Gloss プロジェクトへの Tauri ベース GUI エディタ追加
> **ステータス:** Draft v2（spec review 反映済み）

---

## 1. 概要

Gloss プロジェクトに VSCode 風パネル/タブレイアウトを持つ GUI エディタ `src-desktop` を追加する。
既存の no_std 哲学を継承し、IO を極限まで抽象化することで究極のマルチプラットフォーム設計とする。

**主要方針：**

- I/O（FS・プラグイン・クリップボード・IME）をトレイトで抽象化し、ロジック層をすべて no_std + alloc に保つ
- Tauri は「薄いシェル」として機能し、トレイトの native 実装を提供するだけ
- 同じロジック層を WASM ターゲットでも使用し `src-web` を廃止・統合する
- 日本語 IME 対応のカスタムテキストエディタを実装する
- `src-cli` は機能（`.n.md → HTML` 変換）を維持しつつ `src-desktop-core` ベースに刷新する（後方互換なし）

---

## 2. クレート構成

```
src-core              (no_std + alloc)  パーサー・HtmlRenderer [変更なし]
src-plugin-types      (no_std + alloc)  プラグインフック I/O 型 [serde_json alloc mode に修正]
src-plugin            (std)             Extism ホスト [変更なし]

src-desktop-types     (no_std + alloc)  プロトコル定義層（型・トレイト・MemoryVfs・NoopPluginHost）
src-editor            (no_std + alloc)  テキストエディタコア（GapBuffer・IME・undo）
src-desktop-layout    (no_std + alloc)  Elm アーキテクチャ（Model・update・view）
src-desktop-core      (no_std + alloc)  アプリケーションロジック（AppCore・レンダリング）

src-desktop-native    (std)             NativeFs・GlossPluginHost → PluginHost impl（共有）
src-desktop           (std + Tauri)     Tauri プラットフォームシェル
src-desktop-wasm      (std + wasm32)    WASM シェル（src-web 後継）

src-cli               (std)             CLI 変換ツール [src-desktop-core ベースに刷新]
```

### 依存グラフ

```
src-core ────────────────────────────────────────────────────────┐
src-plugin-types ────────────────────────────────────────────────┤
                                                                  ↓
src-desktop-types ──→ src-editor ──→ src-desktop-core ──────────→ src-desktop-native (std)
                  ──→ src-desktop-layout ──→                  ──→ src-desktop         (std+Tauri)
                                                              ──→ src-desktop-wasm    (wasm32)
                                                              ──→ src-cli             (std)

src-plugin (std, native only)
  ← src-desktop-native のみが依存
  （src-cli / src-desktop は src-desktop-native 経由で間接依存）
```

### no_std 境界

| クレート | no_std | 備考 |
|---------|--------|------|
| `src-core` | ✅ | 変更なし |
| `src-plugin-types` | ✅ | `serde_json = { default-features=false, features=["alloc"] }` |
| `src-desktop-types` | ✅ | MemoryVfs・全トレイト・NoopPluginHost 含む |
| `src-editor` | ✅ | GapBuffer・IME・undo |
| `src-desktop-layout` | ✅ | Elm update/view 純粋関数 |
| `src-desktop-core` | ✅ | トレイト境界で I/O 注入 |
| `src-desktop-native` | ❌ | std + src-plugin 依存 |
| `src-desktop` | ❌ | std + Tauri |
| `src-desktop-wasm` | ❌ | std + wasm32 |
| `src-cli` | ❌ | std |

---

## 3. `src-desktop-types` — プロトコル定義層

### 3.1 基本型

```rust
/// OS パス非依存のパス型（no_std）
pub struct VfsPath(String);
impl VfsPath {
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn join(&self, segment: &str) -> Self;
    pub fn parent(&self) -> Option<Self>;
    pub fn file_name(&self) -> Option<&str>;
}
impl From<&str> for VfsPath { fn from(s: &str) -> Self { Self(s.to_string()) } }

/// ディレクトリエントリ
pub struct DirEntry {
    pub name:   String,
    pub path:   VfsPath,
    pub is_dir: bool,
}

/// FS エラー型（no_std: std::error::Error 非依存）
pub enum FsError {
    NotFound(VfsPath),
    PermissionDenied,
    AlreadyExists(VfsPath),
    Io(String),
}
impl core::fmt::Display for FsError { ... }
```

### 3.2 コアトレイト

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

/// プラグインホスト抽象（src-plugin-types の型を使用）
pub trait PluginHost {
    fn run_code_highlight(&mut self, lang: &str, code: &str, filename: &str)
        -> Option<String>;
    fn run_card_link(&mut self, url: &str)
        -> Option<CardLinkOutput>;
    /// events は呼び出し元（AppCore）が to_plugin_events() で変換済みのもの
    fn run_lint_rule(&mut self, src: &str, md: &str,
        existing: &[PluginWarning], events: &[PluginEvent])
        -> Vec<PluginWarning>;
    fn run_front_matter(&mut self, fields: &[PluginFrontMatterField], src: &str)
        -> Option<String>;
}

pub trait Clipboard {
    fn get_text(&self) -> Option<String>;
    fn set_text(&mut self, text: &str);
}

pub trait ImeSource {
    fn poll_event(&mut self) -> Option<ImeEvent>;
}
```

### 3.3 IME イベント型

```rust
pub enum ImeEvent {
    Start,
    Update { preedit: String, cursor: Option<(usize, usize)> },
    Commit { text: String },
    Cancel,
}
```

### 3.4 `AppConfig`（TOML パースなし・no_std）

```rust
/// LintRules: rule_code → enabled
pub struct LintRules(pub BTreeMap<String, bool>);
impl LintRules {
    pub fn is_enabled(&self, code: &str) -> bool {
        self.0.get(code).copied().unwrap_or(true)
    }
}

/// プラグイン設定エントリ（no_std）
/// config は JSON 文字列として保持する（serde_json::Value の代替）
pub struct PluginEntrySpec {
    pub id:     String,
    pub path:   VfsPath,
    pub hooks:  Vec<String>,
    pub config: String,  // JSON 文字列（シェル層が serde_json::Value → String に変換）
}

pub struct AppConfig {
    pub lint:    LintRules,
    pub plugins: Vec<PluginEntrySpec>,
}

impl Default for AppConfig {
    fn default() -> Self { Self { lint: LintRules(BTreeMap::new()), plugins: vec![] } }
}
```

### 3.5 `AppEvent` — 全入力イベント

```rust
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Ime(ImeEvent),
    Resize { width: u32, height: u32 },
    // FS 完了通知（AppCmd の結果として届く）
    FileLoaded  { path: VfsPath, content: Vec<u8> },
    FileSaved   { path: VfsPath },
    FileError   { path: VfsPath, error: FsError },
    DirLoaded   { path: VfsPath, entries: Vec<DirEntry> },
    // レンダリング完了
    RenderComplete { pane_id: PaneId, html: String, warnings: Vec<PluginWarning> },
    // 設定ロード完了
    ConfigLoaded(AppConfig),
    // クリップボード取得結果
    ClipboardText(String),
    // UI 操作
    TabSelected        { pane_id: PaneId, tab_index: usize },
    PaneSplitRequested { pane_id: PaneId, direction: SplitDirection },
    // プレビュー
    PreviewLinkClicked { pane_id: PaneId, url: String, new_tab: bool },
    PreviewScrolled    { pane_id: PaneId, offset_y: f32 },
    Quit,
}
```

### 3.6 `AppCmd` — サイドエフェクト命令と担当レイヤー

```rust
pub enum AppCmd {
    // ── AppCore が処理するもの ──────────────────────────────
    WriteFile      { path: VfsPath, content: Vec<u8> },  // Fs トレイト経由
    ListDir        { path: VfsPath },                     // Fs トレイト経由
    RunRender      { pane_id: PaneId, doc_id: DocId },   // AppCore 内部で完結
    RunLint        { doc_id: DocId },                     // AppCore 内部で完結
    ScheduleRender { pane_id: PaneId, delay_ms: u32 },   // shell 側でタイマー管理

    // ── シェル層（src-desktop / src-cli）が処理するもの ─────
    ReadFile       { path: VfsPath },     // 非同期 I/O → FileLoaded で返る
    LoadConfig     { path: VfsPath },     // 非同期 I/O → ConfigLoaded で返る
    CopyToClipboard { text: String },     // Clipboard トレイト（AppCore 非保有）
    PasteRequest,                         // → ClipboardText で返る
    SetImeCursorArea { rect: Rect },      // OS/WebView ウィンドウハンドル必要
    ShowOpenFileDialog,                   // OS ネイティブダイアログ
    ShowSaveFileDialog { suggested: Option<VfsPath> },
    OpenUrl        { url: String },       // OS ブラウザ起動
    Quit,
}
```

**`AppCore::execute_cmds()` の担当範囲：**

`AppCore` は `WriteFile`・`ListDir`・`RunRender`・`RunLint` のみを処理し、残りは呼び出し元（シェル層）が処理する。

```rust
// AppCore が処理して即時 AppEvent を返すもの
impl<Fs, Ph> AppCore<Fs, Ph> {
    pub fn execute_cmds(&mut self, cmds: Vec<AppCmd>) -> (Vec<AppEvent>, Vec<AppCmd>)
    //  ↑ AppCore が処理した結果イベント    ↑ シェル層に渡す未処理コマンド
}
```

### 3.7 `DrawCmd` — レンダリング命令

パネル/タブの境界ボックスは Rust 側で計算し、JS 側で `position: absolute` として配置する。
パネル内コンテンツの詳細描画は HTML/CSS に委譲する。エディタペインのみ Canvas 精密描画。

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

    // ── コンテンツデータ（HTML/CSS に委譲）──
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

pub struct PanelLayout  { pub pane_id: PaneId, pub bounds: Rect, pub kind: PaneKind, pub visible: bool }
pub struct DividerLayout { pub bounds: Rect, pub direction: SplitDirection, pub draggable: bool }
```

### 3.8 `MemoryVfs` と `NoopPluginHost`

```rust
pub enum VfsNode {
    File { name: String, content: Vec<u8> },
    Dir  { name: String, children: BTreeMap<String, VfsNode> },
}
pub struct MemoryVfs { root: VfsNode }
impl FileSystem for MemoryVfs { /* 全メソッド実装 */ }
impl MemoryVfs {
    pub fn iter_files(&self) -> impl Iterator<Item = (&str, &[u8])>;
}

/// テスト・WASM プレイグラウンドでプラグインを無効化するためのスタブ
/// src-desktop-types に配置し、テストから直接使用できる
#[cfg(any(test, feature = "test-utils"))]
pub struct NoopPluginHost;
#[cfg(any(test, feature = "test-utils"))]
impl PluginHost for NoopPluginHost {
    fn run_code_highlight(&mut self, ..) -> Option<String> { None }
    fn run_card_link(&mut self, ..)      -> Option<CardLinkOutput> { None }
    fn run_lint_rule(&mut self, ..)      -> Vec<PluginWarning> { vec![] }
    fn run_front_matter(&mut self, ..)   -> Option<String> { None }
}
// src-desktop-wasm では feature フラグなしで使用するため、
// wasm32 ターゲット時は cfg(test) なしで公開する
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

    /// ギャップを移動して連続した &str を返す（内部でギャップを範囲外に移動する）
    pub fn slice(&mut self, byte_range: Range<usize>) -> &str;

    pub fn as_str(&self) -> String;
    pub fn len_bytes(&self) -> usize;
    pub fn len_chars(&self) -> usize;
    pub fn line_count(&self) -> usize;
    pub fn line_to_byte(&self, line: usize) -> usize;
    pub fn byte_to_line(&self, byte_pos: usize) -> usize;
    pub fn char_visual_width(c: char) -> u32;  // ASCII=1, CJK=2
}
```

**注：** `slice()` はギャップを指定範囲外に移動することで連続した `&str` を返す。このためシグネチャは `&mut self`。読み取り専用に見えても変更が発生する点に注意。

### 4.2 Cursor（CJK 対応）

```rust
pub struct Cursor {
    pub byte_pos:             usize,
    pub line:                 usize,
    pub col_byte:             usize,
    pub preferred_visual_col: u32,  // ↑↓移動時に保持
}

impl Cursor {
    pub fn move_right(&mut self, buf: &mut GapBuffer);
    pub fn move_left(&mut self, buf: &mut GapBuffer);
    pub fn move_down(&mut self, buf: &mut GapBuffer);
    pub fn move_up(&mut self, buf: &mut GapBuffer);
    pub fn move_line_start(&mut self, buf: &mut GapBuffer);
    pub fn move_line_end(&mut self, buf: &mut GapBuffer);
    pub fn move_word_right(&mut self, buf: &mut GapBuffer);  // 文字種変化点を境界
    pub fn move_word_left(&mut self, buf: &mut GapBuffer);
    pub fn visual_col(&self, buf: &mut GapBuffer) -> u32;
    pub fn sync_line_col(&mut self, buf: &mut GapBuffer);
}
```

**日本語単語境界：** 形態素解析は行わず「文字種の変化点」（ASCII↔CJK↔ひらがな等）を境界とする。

### 4.3 ImeState（変換中テキスト管理）

```rust
pub struct ImeState { pub composing: Option<Preedit> }
pub struct Preedit {
    pub text:            String,
    pub cursor:          Option<(usize, usize)>,
    pub insert_byte_pos: usize,
}

impl ImeState {
    pub fn is_composing(&self) -> bool;
    pub fn apply(&mut self, event: ImeEvent, buf: &mut GapBuffer, cursor: &mut Cursor);
}
```

| ImeEvent | 動作 |
|---------|------|
| `Start` | カーソル位置を `insert_byte_pos` として記録。バッファ未変更 |
| `Update` | `preedit.text` を更新するだけ。バッファ未変更 |
| `Commit` | `insert_byte_pos` にテキストを挿入。カーソルを末尾へ |
| `Cancel` | preedit をクリア |

### 4.4 UndoHistory

```rust
pub struct UndoHistory {
    undo_stack: Vec<UndoGroup>,
    redo_stack: Vec<UndoGroup>,
    group_open: bool,
    pending:    Vec<EditOp>,
}
pub struct UndoGroup { ops: Vec<EditOp>, cursor_before: usize, cursor_after: usize }
pub enum EditOp {
    Insert { byte_pos: usize, text: String },
    Delete { byte_pos: usize, deleted: String },
}
```

IME の undo 単位：`ImeEvent::Start` → `begin_group()`、`ImeEvent::Commit` → `end_group()`。

### 4.5 EditorState

```rust
pub struct EditorState {
    pub buffer:    GapBuffer,
    pub cursor:    Cursor,
    pub selection: Option<Selection>,
    pub ime:       ImeState,
    pub undo:      UndoHistory,
    pub scroll:    ScrollOffset,
    pub version:   u64,
}
```

### 4.6 Highlighter（シンタックスハイライト）

```rust
pub struct Highlighter;
impl Highlighter {
    /// 1行分のスパン列を返す（no_std・状態機械実装）
    pub fn highlight_line<'a>(line: &'a str, ctx: HighlightContext) -> Vec<TextSpan<'a>>;
}
pub enum HighlightContext { Normal, InCodeBlock { lang: &'static str }, InMathBlock }
```

ルビ・アノ・数式・見出し・コードブロック等を正規表現なし（状態機械）で着色。no_std で完結。

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

`src-desktop-core` が `EditorState` から計算して `Model` に格納。`view()` はこれを参照するだけ。

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

pub struct PreviewState {
    pub html:          String,
    pub block_hashes:  Vec<u64>,
    pub scroll_offset: f32,
    pub warnings:      Vec<PluginWarning>,
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
| `Ime(event)` | IME 処理 |
| `Resize(w,h)` | `window_size` 更新（bounds 再計算は `view()` 担当） |
| `FileLoaded` | `EditorViewModel` 生成 + `AppCmd::RunRender` |
| `RenderComplete` | `PreviewState` 更新（差分ハッシュ比較） |
| `PreviewLinkClicked { new_tab: false }` | 同ペインで URL ロード（内部リンクはスクロール） |
| `PreviewLinkClicked { new_tab: true }` | 新規ペインを開いて URL ロード（外部 URL は `AppCmd::OpenUrl`） |
| `PreviewScrolled` | editor と連動スクロール（`AppCmd::ScheduleRender` なし） |
| `ConfigLoaded` | `model.config` 更新 |

**Model は値渡し：** `update` は前の `Model` を消費して新しい `Model` を返す。

### 5.3 `view()` — 純粋関数

```rust
pub fn view(model: &Model) -> Vec<DrawCmd>
```

1. `PanelNode` ツリーを再帰走査し `window_size` + `ratio` から各 `Pane.bounds` を計算
2. 各ペインを描画（種類に応じた `DrawCmd` を発行）
3. タブバー・ステータスバーを発行

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
    pub fm_config:         Option<AppConfig>,  // フロントマター上書き設定
}
```

### 6.2 主要メソッド

```rust
impl<Fs: FileSystem, Ph: PluginHost> AppCore<Fs, Ph> {

    // ── ドキュメント管理 ──
    pub fn open_file(&mut self, path: &VfsPath)
        -> Result<(DocId, EditorViewModel), FsError>;
    pub fn open_bytes(&mut self, path: VfsPath, content: Vec<u8>)
        -> (DocId, EditorViewModel);
    pub fn close_doc(&mut self, doc_id: DocId);
    pub fn save_doc(&mut self, doc_id: DocId) -> Result<(), FsError>;

    // ── 編集操作 ──
    pub fn apply_key(&mut self, doc_id: DocId, key: &KeyEvent)
        -> Option<EditorViewModel>;
    pub fn apply_ime(&mut self, doc_id: DocId, ime: ImeEvent)
        -> Option<EditorViewModel>;
    pub fn undo(&mut self, doc_id: DocId) -> Option<EditorViewModel>;
    pub fn redo(&mut self, doc_id: DocId) -> Option<EditorViewModel>;

    // ── フロントマター設定上書き ──
    /// パース済みフロントマターフィールドからドキュメントスコープの設定を適用する
    pub fn apply_front_matter(&mut self, doc_id: DocId, fields: &[FrontMatterField]);

    // ── レンダリング ──
    pub fn render_full(&mut self, doc_id: DocId)
        -> Option<(String, Vec<u64>, Vec<PluginWarning>)>;
    pub fn render_diff(&mut self, doc_id: DocId)
        -> Option<(Vec<HtmlPatch>, Vec<PluginWarning>)>;

    // ── FS 操作 ──
    pub fn list_dir(&self, path: &VfsPath)  -> Result<Vec<DirEntry>, FsError>;
    pub fn create_file(&mut self, path: &VfsPath) -> Result<(), FsError>;
    pub fn delete_file(&mut self, path: &VfsPath) -> Result<(), FsError>;
    pub fn rename_file(&mut self, from: &VfsPath, to: &VfsPath) -> Result<(), FsError>;

    // ── 設定 ──
    pub fn apply_config(&mut self, config: AppConfig);

    // ── AppCmd 実行（シェル層から呼ぶ）──
    /// AppCore が担当できる AppCmd を処理し、結果イベントとシェル向け未処理コマンドを返す
    pub fn execute_cmds(&mut self, cmds: Vec<AppCmd>)
        -> (Vec<AppEvent>, Vec<AppCmd>);
    //      ↑ 結果イベント       ↑ シェルが処理すべき残りコマンド
}
```

**`execute_cmds` の担当分類：**

| AppCmd | 担当 | 備考 |
|--------|------|------|
| `WriteFile` | AppCore | `Fs::write()` 経由 |
| `ListDir` | AppCore | `Fs::list_dir()` 経由 → `DirLoaded` イベント |
| `RunRender` | AppCore | `render_full/diff()` 呼び出し → `RenderComplete` イベント |
| `RunLint` | AppCore | lint のみ実行 |
| `ReadFile` | Shell | 非同期 I/O → `FileLoaded` |
| `LoadConfig` | Shell | 非同期 I/O → `ConfigLoaded` |
| `ScheduleRender` | Shell | タイマー管理 |
| `CopyToClipboard` | Shell | `Clipboard` トレイト |
| `PasteRequest` | Shell | → `ClipboardText` |
| `SetImeCursorArea` | Shell | OS ウィンドウ API |
| `ShowOpenFileDialog` | Shell | OS ダイアログ |
| `ShowSaveFileDialog` | Shell | OS ダイアログ |
| `OpenUrl` | Shell | OS ブラウザ |
| `Quit` | Shell | アプリ終了 |

### 6.3 レンダリングパイプライン（内部）

```
buffer.as_str()
  → Parser::new_with_source(&text, &path)
  → warnings 収集
  → effective_config.lint でフィルタ  ← fm_config があれば優先
  → to_plugin_events(&events)          ← src-plugin-types の変換関数
  → plugin_host.run_lint_rule(src, md, &filtered_warnings, &plugin_events)
  → PluginAwareHtmlRenderer::render()  ← 6.4 参照
  → html + fnv1a block_hashes を返す
```

`src-desktop-core` は `src-plugin-types` に依存（`to_plugin_events` と `PluginEvent` のため）。

### 6.4 `PluginAwareHtmlRenderer<'a, Ph: PluginHost>`

`src-plugin/renderer.rs` の削除に伴い、`PluginHost` トレイトで汎用化した版を `src-desktop-core` 内に実装する。

```rust
pub struct PluginAwareHtmlRenderer<'a, Ph: PluginHost> {
    host: &'a mut Ph,
}
impl<'a, Ph: PluginHost> PluginAwareHtmlRenderer<'a, Ph> {
    pub fn render(&mut self, events: &[Event<'_>], out: &mut String, source: &str, markdown: &str);
}
```

**実装範囲：**
- `CodeBlock` イベントのインターセプト（`host.run_code_highlight()`）
- `CardLink` イベントのインターセプト（`host.run_card_link()`）
- `FrontMatter` イベントのインターセプト（`host.run_front_matter()`）
- 未処理イベントは `src-core::HtmlRenderer::feed()` にフォールバック
- `render_card_output()` をこのクレート内に再実装（`src-plugin/renderer.rs` が削除されるため）

---

## 7. `src-desktop-native` — 共有ネイティブ実装

`src-desktop` と `src-cli` が共通して使う native 実装を提供する小さなクレート。

```rust
// NativeFs: std::fs ラッパー
pub struct NativeFs;
impl FileSystem for NativeFs { /* std::fs 使用 */ }

// src-plugin::GlossPluginHost への PluginHost トレイト実装
// （src-plugin-types の型をそのまま使うのでラッパー不要）
impl PluginHost for src_plugin::host::GlossPluginHost {
    fn run_code_highlight(&mut self, lang, code, filename) -> Option<String> {
        self.run_code_highlight(lang, code, filename)
    }
    // ...
}

// TOML 設定ロード
pub fn load_app_config(path: &str) -> AppConfig {
    let gc = src_plugin::config::GlossConfig::from_file(path);
    AppConfig {
        lint: LintRules(gc.lint.rules.into_iter().collect()),
        plugins: gc.plugins.into_iter().map(|p| PluginEntrySpec {
            id:     p.id,
            path:   VfsPath::from(p.path.as_str()),
            hooks:  p.hooks,
            config: serde_json::to_string(&p.config).unwrap_or_default(),
        }).collect(),
    }
}
```

---

## 8. `src-desktop` — Tauri シェル

### 8.1 追加トレイト実装

```rust
// NativeClipboard: arboard クレート使用
pub struct NativeClipboard(arboard::Clipboard);
impl Clipboard for NativeClipboard { ... }

// TauriIme: JS compositionイベント → ImeEvent キュー
pub struct TauriIme { queue: VecDeque<ImeEvent> }
impl ImeSource for TauriIme { fn poll_event(&mut self) -> Option<ImeEvent> { ... } }
```

### 8.2 AppState

```rust
pub struct AppState {
    pub model:     Model,
    pub core:      AppCore<NativeFs, GlossPluginHost>,
    pub clipboard: NativeClipboard,
    pub ime:       TauriIme,
}
type SharedState = Mutex<AppState>;
```

### 8.3 IME 統合

Canvas ベースのエディタで IME 候補ウィンドウを正しい位置に表示する手法：

1. `DrawCmd::SetImeCursorArea { rect }` を受け取った JS が、不可視 `<input>` 要素をカーソル位置に移動してフォーカス
2. `compositionstart/update/end` イベントを JS が受け取り、`invoke("push_ime_event", ...)` で Rust へ転送
3. `TauriIme.queue` に積まれた `ImeEvent` を次のディスパッチサイクルで処理

### 8.4 フロントエンド構成

```
src-desktop/frontend/
  index.html       ── 最小限シェル
  renderer.ts      ── DrawCmd ディスパッチャ
  editor-canvas.ts ── EditorFrame → Canvas 2D 精密描画（CJK フォント対応）
  preview.ts       ── PreviewMount/Patch → innerHTML（KaTeX 含む）
  ime-bridge.ts    ── composition イベント → Tauri IPC
  chrome.ts        ── タブバー・ファイルツリー・ステータスバーの DOM 操作
  style.css        ── web-playground の nm-* スタイルを流用
```

### 8.5 Tauri コマンドハンドラ（イベントループ）

```rust
#[tauri::command]
async fn dispatch(
    state: tauri::State<'_, SharedState>,
    window: tauri::Window,
    event: AppEvent,
) -> Result<(), String> {
    let draw_cmds = tokio::task::spawn_blocking({
        let state = state.inner().clone();
        move || {
            let mut s = state.lock().unwrap();

            // 1. IME イベントをキューから先行処理
            while let Some(ev) = s.ime.poll_event() {
                let (m, cmds) = update(s.model.clone(), AppEvent::Ime(ev));
                s.model = m;
                let (events, shell_cmds) = s.core.execute_cmds(cmds);
                dispatch_shell_cmds(&mut s, shell_cmds, &window);
                for ev in events {
                    let (m, _) = update(s.model.clone(), ev);
                    s.model = m;
                }
            }

            // 2. メインイベント処理（最大 8 ラウンドでドレイン）
            let mut pending = vec![event];
            for _ in 0..8 {
                if pending.is_empty() { break; }
                let mut next = Vec::new();
                for ev in pending.drain(..) {
                    let (m, cmds) = update(s.model.clone(), ev);
                    s.model = m;
                    let (events, shell_cmds) = s.core.execute_cmds(cmds);
                    dispatch_shell_cmds(&mut s, shell_cmds, &window);
                    next.extend(events);
                }
                pending = next;
            }

            // 3. DrawCmd 生成
            view(&s.model)
        }
    }).await.map_err(|e| e.to_string())?;

    window.emit("draw", draw_cmds).map_err(|e| e.to_string())
}

fn dispatch_shell_cmds(s: &mut AppState, cmds: Vec<AppCmd>, window: &tauri::Window) {
    for cmd in cmds {
        match cmd {
            AppCmd::CopyToClipboard { text } => s.clipboard.set_text(&text),
            AppCmd::SetImeCursorArea { rect } => { window.emit("ime-cursor-area", rect).ok(); }
            AppCmd::OpenUrl { url } => { tauri::api::shell::open(&window.shell_scope(), url, None).ok(); }
            AppCmd::Quit => { window.close().ok(); }
            // ReadFile / LoadConfig / ScheduleRender / ShowDialog は
            // 非同期タスクとして別途 spawn され、完了時に AppEvent を再投入する
            _ => {}
        }
    }
}
```

---

## 9. `src-desktop-wasm` — WASM シェル（`src-web` 後継）

### 9.1 トレイト実装

```rust
// MemoryVfs: src-desktop-types から（no_std 実装をそのまま使用）
// NoopPluginHost: src-desktop-types の test-utils または wasm32 向け公開版

pub struct WasmClipboard;
impl Clipboard for WasmClipboard { /* navigator.clipboard API */ }

pub struct WasmIme { queue: Rc<RefCell<VecDeque<ImeEvent>>> }
impl ImeSource for WasmIme { /* DOM compositionイベント */ }
```

### 9.2 公開 API

```rust
#[wasm_bindgen]
pub struct GlossApp { model: Model, core: AppCore<MemoryVfs, NoopPluginHost> }

#[wasm_bindgen]
impl GlossApp {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self;
    pub fn dispatch(&mut self, event_json: &str) -> String;  // DrawCmd JSON を返す
    pub fn export_zip(&self) -> Vec<u8>;
    pub fn import_zip(&mut self, data: &[u8]) -> Result<(), JsValue>;
}
```

### 9.3 VFS と Zip

```rust
pub fn vfs_to_zip(vfs: &MemoryVfs) -> Vec<u8>            { /* zip::ZipWriter */ }
pub fn zip_to_vfs(data: &[u8]) -> Result<MemoryVfs, String> { /* zip::ZipArchive */ }
```

### 9.4 IndexedDB 永続化（オプション）

```toml
[features]
default = []
persist = ["rexie"]
```

開発中は `default` のままオフ。

### 9.5 プレビュー差分更新

`src-core::split_source_blocks()` と `fnv1a()`（既に no_std 実装済み）で変更ブロックを検出し、`PreviewPatch` で差分更新。

---

## 10. `src-cli` の刷新

### 10.1 方針

- `.n.md → HTML` 変換機能は維持する
- 後方互換なしで `src-desktop-core` ベースに刷新する
- `AppCore<NativeFs, GlossPluginHost>` を使用（`src-desktop-native` 経由）

### 10.2 新しいパイプライン

```
1. CLI 引数パース（input / output / --config）
2. load_app_config("gloss.toml") で AppConfig を構築
3. AppCore::new(NativeFs, GlossPluginHost::new(&config.plugins), config)
4. AppCore::open_file(input_path)
5. AppCore::apply_front_matter(doc_id, &fm_fields)  ← フロントマター上書き
6. AppCore::render_full(doc_id) → (html, _, warnings)
7. warnings を config.lint でフィルタして stderr に出力
8. HTML_HEAD + html + HTML_TAIL をファイルに書き込む
```

---

## 11. `src-web` の廃止

| ステップ | 内容 |
|--------|------|
| 1 | `src-desktop-wasm` でプレイグラウンドが動作することを確認 |
| 2 | `Cargo.toml` workspace members から `src-web` を削除 |
| 3 | `web-playground/index.html` の `<link data-trunk rel="rust" href="../src-web/Cargo.toml">` を `src-desktop-wasm` 参照に変更 |
| 4 | `web-playground/src/main.ts` を `GlossApp` API に合わせて書き直す |
| 5 | `src-web/` ディレクトリを削除 |
| 6 | CI / ビルドスクリプトを更新 |

---

## 12. `src-plugin` 移行方針

| 対象 | 最終方針 |
|------|---------|
| `GlossPluginHost` | 変更なし。`src-desktop-native` で `PluginHost` トレイトを実装 |
| `PluginAwareRenderer` | **削除**。`src-desktop-core` の汎用版に一本化 |
| `GlossConfig` / `from_file()` | `src-plugin` に残す。`src-desktop-native::load_app_config()` が変換 |
| `src-cli` | `src-desktop-core` ベースに刷新（後方互換なし） |
| `src-web` | `src-desktop-wasm` に置き換えて廃止（セクション 11 参照） |

---

## 13. テスト戦略

### `src-desktop-layout`：純粋関数テスト

```rust
#[test] fn split_pane_divides_window_evenly() { ... }
#[test] fn tab_close_dirty_doc_emits_save_dialog() { ... }
#[test] fn keyboard_ctrl_s_emits_write_file() { ... }
#[test] fn view_assigns_correct_bounds_to_split_panes() { ... }
#[test] fn preview_scrolled_syncs_editor_scroll_position() { ... }
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
#[test] fn execute_cmds_handles_run_render() { ... }
#[test] fn execute_cmds_returns_shell_cmds_for_read_file() { ... }
#[test] fn front_matter_override_disables_lint_rule() { ... }
```

### `src-editor`：バッファ・IME 単体テスト

```rust
#[test] fn insert_japanese_and_measure_visual_col() { ... }
#[test] fn ime_commit_inserts_text_and_clears_preedit() { ... }
#[test] fn undo_restores_buffer_after_ime_commit() { ... }
#[test] fn word_boundary_detects_cjk_ascii_transition() { ... }
#[test] fn slice_crosses_gap_returns_correct_str() { ... }
```

### `src-desktop-types`：MemoryVfs・型テスト

```rust
#[test] fn memory_vfs_create_and_read() { ... }
#[test] fn memory_vfs_rename_updates_path() { ... }
#[test] fn vfs_path_join_and_parent() { ... }
```

---

## 14. 将来の拡張性

- **egui / iced フロントエンド：** `SetLayout` と `EditorFrame` を解釈するレンダラーを新たに実装するだけで移植可能
- **TUI フロントエンド：** 同様に `DrawCmd` を文字ベースでレンダリングする実装を追加
- **WASM プラグイン（Web 環境）：** `PluginHost` トレイトの `WebAssembly.instantiate()` ベース実装を `src-desktop-wasm` に将来追加可能
- **HTTP サーバーモード：** `AppCore` は no_std・同期なので HTTP ハンドラからそのまま呼び出し可能
