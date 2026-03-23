# Gloss Markup Specification (Draft v0.1)

この文書は、Gloss の5つの拡張構文（**Ruby**・**Anno**・**Nest**・**Math**・**Lint**）を、**パーサーが生成する構文木（AST）**まで含めて定義する仕様書です。
本仕様は README に記載された「多言語（複数行）」「Anno の下段でも Ruby を使う」例を **そのまま実現できる**ことを要件として策定します。

---

## 1. 目的

- **Ruby**: ルビ（発音・転写・注釈）を簡潔に書く（例: `[漢字/かんじ]`）。
- **Anno**: 上段本文（表示の主文字列）に対し、下段に 1 行以上の別表記（英語・転写・別言語など）を添える（例: `{微分係数/derivative}`、`{佛罗伦萨/Firenze/Florence}`）。
- **Nest**: `---` / `;;;` でセクション階層を明示的に閉じる。見出しレベルが `<section class="nm-sec level-N">` のネスト深さに対応する。
- **Math**: `$...$` / `$$...$$` を「数式区間」として扱い、区間内部の `[]` `{}` `/` が Anno/Ruby と誤解釈されないようにする。
- **Lint**: 構文ミスや非推奨パターンを `Parser::warnings` に収集して報告する。

> 注: KaTeX などの auto-render を使う場合、利用環境によっては `$...$` を delimiters に追加設定する必要がある。

---

## 2. 用語

- **Input**: UTF-8 のテキスト（1 行でも複数行でもよい）
- **Segment**: Input を分割した最小の意味単位（AST のノード）
- **Ruby Block**: `[...]` の構文要素
- **Anno Block**: `{...}` の構文要素
- **Math Segment**: `$...$` または `$$...$$` による数式区間

---

## 3. AST（構文木）モデル

### 3.1 Segment 型（Top-level）

パーサーは Input を次の Segment 列として返す。

- **Plain**
    - `text: String`
- **Ruby**
    - `base: InlineSegment[]`
    - `reading: String`
- **Anno**
    - `lines: AnnoLine[]`
- **Math**
    - `tex: String`
    - `display: bool`（`$$...$$` = true, `$...$` = false）

> 注: Anno は `lines` により「上段＋下段（多段）」を統一表現する。`lines[0]` が上段本文、`lines[1..]` が下段の注記行である。

### 3.2 InlineSegment 型（Ruby base 内）

- **Plain**
    - `text: String`
- **Math**
    - `tex: String`
    - `display: bool`

### 3.3 AnnoLine 型（Anno の各行）

Anno の各行は「本文的に表示されるテキスト」なので、Ruby と Math を含められる。

- `segments: AnnoInlineSegment[]`

### 3.4 AnnoInlineSegment 型（Anno 行内）

- **Plain**
- **Ruby**
- **Math**

---

## 4. 字句（トークン）とエスケープ

### 4.1 構文用の特別文字

構文文字は次の 5 つ:

- `[` `]` `/` `{` `}`

### 4.2 バックスラッシュによるエスケープ（規範）

`\`（バックスラッシュ）に続く文字が「特別文字」または `\` のとき、直後 1 文字は **構文文字ではなく通常文字**として扱う。

- 例: `\[` は「`[`」として扱う（Ruby の開始ではない）
- 例: `\{` は「`{`」として扱う（Anno の開始ではない）
- 例: `\/` は「`/`」として扱う（Anno/Ruby の区切りではない）

### 4.3 エスケープの“除去”（規範）

- **Plain.text / Ruby.reading / Anno の Plain**: エスケープは **除去して格納**する  
  例: 入力 `\[` → AST では `"["`
- **Math.tex**: delimiters（`$` / `$$`）は除去するが、`tex` 本体は **入力のまま格納**する（`\` を含む TeX を保持する）

### 4.4 `$` の扱い

- Ruby/Anno の字句解析における「特別文字」は §4.1 の 5 文字（＋`\`）である。
- **Math スキャナ**は `$` と `$$` を特別扱いする（§7）。
- `\$` は「リテラル `$`」として扱える（§7.3）。

---

## 5. Ruby Block（`[base/reading]`）

### 5.1 構文（規範）

```ebnf
RubyBlock := '[' RubyBase '/' RubyReading ']'
```

- RubyBlock は **必ず 1 つ以上の `/` を含む**必要がある。
    - `/` が存在しない（例: `[abc]`）場合、その `[` は Ruby 開始とみなさず **Plain** として処理する。
- `base` と `reading` は空文字列でもよい（推奨しない）。

### 5.2 区切り `/` と閉じ `]`（規範）

- base と reading の区切りは、RubyBlock 内で最初に現れる **未エスケープ `/`** とする。
- RubyBlock の終端は、reading 内で最初に現れる **未エスケープ `]`** とする。
- `/` や `]` を通常文字として含めたい場合は `\/`、`\]` のようにエスケープする。

> 注: RubyBase では Math を解釈できるため、Math 内の `/` は区切りとして数えない（§8.2）。

### 5.3 RubyBase（規範）

- RubyBase は **InlineSegment（Plain/Math）**として保持する。
- RubyBase 内の `$...$` / `$$...$$` は **Inline Math** として解釈してよい。
- RubyBase 内では **AnnoBlock は解釈しない**（`{...}` は通常文字として扱う）。

### 5.4 RubyReading（規範）

- RubyReading は **生文字列**（String）として保持する（§4.3 のエスケープ除去は適用する）。
- RubyReading 内では **Anno/Ruby/Math の再帰解析を行わない**。

### 5.5 禁止事項（規範）

- RubyBlock の内側で、未エスケープの `[` が出現した場合、その RubyBlock は **不正**とみなし解析に失敗する。
    - 解析失敗時は Ruby として解釈せず、§9 のフォールバック規則に従う。

> 目的: Ruby の入れ子を禁止し、曖昧性と実装複雑性を抑える。

---

## 6. Anno Block（`{line1/line2/...}` (anno)）

Anno は、「上段本文（line1）」と「下段の別表記行（line2 以降）」を 0 行以上持つ構文。

### 6.1 構文（規範）

```ebnf
AnnoBlock := '{' AnnoLine ( '/' AnnoLine )* '}'
```

- `AnnoLine` は 1 行分の内容（AnnoLine 型）であり、**Ruby と Math を含んでよい**。
- `/` は **未エスケープ**かつ「Ruby/Math の外側」にあるときのみ、AnnoLine の区切りとして働く。
- AnnoBlock の終端は、AnnoBlock 内で最初に現れる **未エスケープ `}`** とする（Ruby/Math の外側）。

### 6.2 行数（規範）

- `{main}`（`/` を含まない）も AnnoBlock として許可する。
- `{a//b}` のように空行を含む場合、空行は `segments=[]`（空）として扱う（推奨しない）。

---

## 7. Nest（`---` / `;;;`）

Nest は見出し（`#`〜`######`）によって自動的に開かれる `<section>` 要素を、**明示的に**閉じるための構文である。

### 7.1 構文（規範）

| 記号 | 動作 |
|------|------|
| `---`（段落として現れる水平線） | 現在のセクションを閉じ、`<hr/>` を描画する |
| `;;;`（単独行） | 現在のセクションを閉じる（HR なし） |

- `---` は Markdown の水平線（thematic break）として現れたときのみ Nest として扱う。インライン文字列としての `---` は通常テキスト。
- `;;;` は行の先頭から行末まで `;;;` のみの行を Nest クローズとして扱う。

### 7.2 セクション対応（規範）

- 見出し `# H1` は `<section class="nm-sec level-1">` を開く。
- 見出し `## H2` は `<section class="nm-sec level-2">` を開く（以下同様）。
- `---` / `;;;` は**現在開いている最も深いセクション**を 1 つ閉じる。
- ドキュメント終端では、開いているすべてのセクションが自動的に閉じられる。

### 7.3 出力例（非規範）

入力:
```markdown
# H1
## H2
内容

---

## H2-2
### H3

;;;
```

出力:
```html
<section class="nm-sec level-1">
<h1>H1</h1>
<section class="nm-sec level-2">
<h2>H2</h2>
<p>内容</p>
<hr/>
</section>
<section class="nm-sec level-2">
<h2>H2-2</h2>
<section class="nm-sec level-3">
<h3>H3</h3>
</section>
</section>
</section>
```

---

## 8. Math Segment（`$...$`, `$$...$$`）

### 8.1 構文（規範）

- Inline: `$ TEX $`（display = false）
- Display: `$$ TEX $$`（display = true）

`TEX` は終端区切りが現れるまでの部分文字列とし、内部は **再帰解析しない**。

### 8.2 優先順位（規範）

- `$$...$$` は `$...$` より優先して認識する（`$$` のほうが長い区切りなので先に試す）。
- Math 区間内部の `[]` `{}` `/` は Anno/Ruby の構文として解釈しない。

### 8.3 エスケープ（規範）

- **開始 delimiter**（`$` または `$$`）は、直前の連続 `\` の個数が **奇数**のとき「エスケープされた通常文字」として扱い、Math を開始しない。
- **終端 delimiter** も同様に、直前の連続 `\` の個数が奇数のとき終端として扱わない。
- Math は「開始した delimiter と同じ delimiter」でのみ閉じる（`$` で開始したら `$` で閉じる、`$$` で開始したら `$$` で閉じる）。

> 推奨: 終端探索では `\` を見たら次の 1 文字をスキップしてよい（TeX のエスケープとして扱う）。

---

## 9. パース手順（規範）

パーサーは左から右へ走査して Segment を生成する。実装上、次の優先順位を推奨する。

1. **Math の切り出し**（`$$...$$` → `$...$`）
2. **AnnoBlock**（`{...}`）
3. **RubyBlock**（`[...]`）
4. それ以外は Plain

### 9.1 AnnoLine の解析（規範）

AnnoBlock 内部は次の規則で AnnoLine を構築する。

- `[` を見たら RubyBlock を試行（成功すれば Ruby を追加）
- `$` / `$$` を見たら MathSegment を試行（成功すれば Math を追加）
- `/` を見たら「未エスケープ」であり Ruby/Math の外側なら次の AnnoLine を開始
- `}`（未エスケープ）で AnnoBlock 終了

### 9.2 RubyBase の解析（規範）

RubyBase は **InlineSegment** 列として構築する。

- `$` / `$$` を見たら MathSegment を試行（成功すれば Math を追加）
- `/`（未エスケープ）を見たら RubyBase を終了し、RubyReading の解析へ移る
- それ以外は Plain として蓄積する

---

## 10. Lint / エラー処理

### 10.1 フォールバック規則（規範）

パーサーは「失敗した構文を壊さない」方針を採る。

- RubyBlock:
    - `]` が見つからない、未エスケープ `/` がない、未エスケープ `[` が内部にある等の場合は Ruby として解釈しない。
- AnnoBlock:
    - `}` が見つからない場合は Anno として解釈しない。
- Math:
    - 対応する終端 `$`（または `$$`）が見つからない場合は Math として解釈しない。

いずれの構文でも失敗した場合、「開始記号 1 文字」を Plain として出力し、解析を継続する（隣接 Plain は結合してよい）。

### 10.2 Lint 警告（規範）

パーサーは `warnings: Vec<String>` に警告を収集する。CLI は黄色で stderr に出力し、Web は警告ボックスに表示する。

| 警告 | 条件 |
|------|------|
| Possibly malformed ruby syntax | `[` に対応 `]` がない、またはネストが壊れている |
| Possibly malformed anno syntax | `{` に対応 `}` がない |
| Unclosed `$` / `$$` | 数式が閉じられていない |
| Anno looks like Ruby | `{漢字/かな}` — Anno のつもりが Ruby 記法に見える |
| Katakana base + hiragana reading | `[インド/いんど]` — カタカナはすでに表音文字 |
| Kanji without ruby | ルビのない漢字がテキストに含まれる |
| Undefined footnote ref | `[^id]` の定義がない |
| Unused footnote def | `[^id]: …` が参照されていない |
| Card link: non-HTTP URL | `@[card](ftp://…)` など |
| Card link: unknown type | `@[embed](…)` など未対応タイプ |

---

## 11. HTML レンダリング指針（非規範）

### 11.1 Ruby

HTML の Ruby マークアップの利用を推奨する。

- 省略形（簡潔）: `<ruby>base<rt>reading</rt></ruby>`
- 明示形: `<ruby><rb>base</rb><rt>reading</rt></ruby>`
- 互換性のため、必要に応じて `<rp>` を用いたフォールバック括弧表示を追加してよい。

### 11.2 Anno

Anno は `lines[0]`（上段）を本文として出し、`lines[1..]` を下段として縦に積む。

- wrapper: `<ruby class="nm-anno"><rb>…</rb><rt>…</rt></ruby>`
- 下段注記: `<span class="nm-anno-note">…</span>`（`<rt>` 内に配置）

### 11.3 Nest

セクション開閉の HTML 出力は §7.3 の例を参照。

### 11.4 Math（README の方針との整合）

README の「`$` による TeX 数式記法を残す」を満たすため、HTML 生成は次のいずれかを採用してよい。

- **方式A（推奨）**: 入力の delimiters を保持して `"$" + tex + "$"` / `"$$" + tex + "$$"` をそのまま出力する（KaTeX 等の後段処理に任せる）。
- **方式B**: 後段レンダラを使わない場合に限り、`Math` を `<span class="math">...</span>` 等で包み、別途 KaTeX を呼んで置換する（この文書の範囲外）。

---

## 12. テストケース（抜粋）

### 12.1 Ruby

- `[私/わたし]` → Ruby(base="私", reading="わたし")
- `[Text]` → Plain("[Text]")（`/` がないため）
- `[a/b/c]` → Ruby(base="a", reading="b/c")（最初の未エスケープ `/` のみ区切り）
- `[AC\/DC/えーしーでぃーしー]` → base=`"AC/DC"`

### 12.2 Anno（多段＋注釈内 Ruby）

- `{Nara/[奈良/なら]}`
  - `lines=["Nara", Ruby("奈良","なら")]`
- `{[台湾/たいわん]/[台灣/Táiwān]}に行く`
  - `lines[0]` と `lines[1]` の両方に Ruby が出現しうる
- `{佛罗伦萨/Firenze/Florence}`
  - `lines=["佛罗伦萨","Firenze","Florence"]`

### 12.3 Math

- `x=$\frac{1}{2}$` → Plain("x="), Math(tex="\frac{1}{2}", display=false)
- `$$a_{[/]}$$` → Math 内の `[/]` は Ruby として解釈しない（保護される）
- `\$40 and $e=mc^2$` → 先頭 `\$` はリテラル `$`、後半は Math
- `$\$100$` → Math.tex は `\$100`（終端 `$` と混同しない）
