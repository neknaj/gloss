# Gloss Markup Specification (Draft v0.1)

この文書は、Gloss 記法（`[base/reading]` と `{line1/line2/...}`）および `$...$` / `$$...$$` による数式区間を、**パーサーが生成する構文木（AST）**まで含めて定義する仕様書です。  
本仕様は README に記載された「多言語（複数行）」「Gloss の下段でも Ruby を使う」例を **そのまま実現できる**ことを要件として策定します。

---

## 1. 目的

- **Ruby**: ルビ（発音・転写・注釈）を簡潔に書く（例: `[漢字/かんじ]`）。
- **Gloss**: 上段本文（表示の主文字列）に対し、下段に 1 行以上の別表記（英語・転写・別言語など）を添える（例: `{微分係数/derivative}`、`{佛罗伦萨/Firenze/Florence}`）。
- **Math**: `$...$` / `$$...$$` を「数式区間」として扱い、区間内部の `[]` `{}` `/` が Gloss/Ruby と誤解釈されないようにする。

> 注: KaTeX で `$...$` を自動レンダリングしたい場合、利用環境によっては auto-render の delimiters 設定で `$` を追加する必要がある。citeturn0search1turn0search4turn0search6

---

## 2. 用語

- **Input**: UTF-8 のテキスト（1 行でも複数行でもよい）
- **Segment**: Input を分割した最小の意味単位（AST のノード）
- **Ruby Block**: `[...]` の構文要素
- **Gloss Block**: `{...}` の構文要素
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
- **Gloss**
    - `lines: GlossLine[]`
- **Math**
    - `tex: String`
    - `display: bool`（`$$...$$` = true, `$...$` = false）

> 注: Gloss は `lines` により「上段＋下段（多段）」を統一表現する。`lines[0]` が上段本文、`lines[1..]` が下段の注記行である。

### 3.2 InlineSegment 型（Ruby base 内）

- **Plain**
    - `text: String`
- **Math**
    - `tex: String`
    - `display: bool`

### 3.3 GlossLine 型（Gloss の各行）

Gloss の各行は「本文的に表示されるテキスト」なので、Ruby と Math を含められる。

- `segments: GlossInlineSegment[]`

### 3.4 GlossInlineSegment 型（Gloss 行内）

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
- 例: `\{` は「`{`」として扱う（Gloss の開始ではない）
- 例: `\/` は「`/`」として扱う（Gloss の区切りではない）

### 4.3 `$` の扱い

- Ruby/Gloss の字句解析における「特別文字」は §4.1 の 5 文字（＋`\`）である。
- **Math スキャナ**は `$` と `$$` を特別扱いする（§7）。
- `\$` は「リテラル `$`」として扱える（§7.3）。citeturn0search6turn0search2

---

## 5. Ruby Block（`[base/reading]`）

### 5.1 構文（規範）

```ebnf
RubyBlock := '[' RubyBase '/' RubyReading ']'
```

- RubyBlock は **必ず 1 つ以上の `/` を含む**必要がある。
    - `/` が存在しない（例: `[abc]`）場合、その `[` は Ruby 開始とみなさず **Plain** として処理する。
- `base` と `reading` は空文字列でもよい（推奨しない）。

### 5.2 `/` の解釈（規範）

- **最初の 1 回だけ** `/` を区切りとして扱い、以降の `/` は `reading` の一部として扱う。
    - 例: `[a/b/c]` は `base="a"`, `reading="b/c"` と解釈する。

### 5.3 RubyBase（規範）

- RubyBase は **InlineSegment（Plain/Math）**として保持する。
- RubyBase 内の `$...$` / `$$...$$` は **Inline Math** として解釈してよい。
- RubyBase 内では **GlossBlock は解釈しない**（`{...}` は通常文字として扱う）。

### 5.4 RubyReading（規範）

- RubyReading は **生文字列**（String）として保持する。
- RubyReading 内では **Gloss/Ruby/Math の再帰解析を行わない**。

### 5.5 禁止事項（規範）

- RubyBlock の内側で、未エスケープの `[` が出現した場合、その RubyBlock は **不正**とみなし解析に失敗する。
    - 解析失敗時は Ruby として解釈せず、§9 のフォールバック規則に従う。

> 目的: Ruby の入れ子を禁止し、曖昧性と実装複雑性を抑える。

---

## 6. Gloss Block（`{line1/line2/...}`）

Gloss は、「上段本文（line1）」と「下段の別表記行（line2 以降）」を 0 行以上持つ構文。

### 6.1 構文（規範）

```ebnf
GlossBlock := '{' GlossLine ( '/' GlossLine )* '}'
```

- `GlossLine` は 1 行分の内容（GlossLine 型）であり、**Ruby と Math を含んでよい**。
- `/` は **未エスケープ**かつ「Ruby/Math の外側」にあるときのみ、GlossLine の区切りとして働く。

### 6.2 行数（規範）

- `{main}`（`/` を含まない）も GlossBlock として許可する。
- `{a//b}` のように空行を含む場合、空行は `segments=[]`（空）として扱う（推奨しない）。

### 6.3 例（規範的解釈）

- `{Nara/[奈良/なら]}`  
  - `lines[0] = "Nara"`
  - `lines[1] = Ruby("奈良","なら")`
- `{[台湾/たいわん]/[台灣/Táiwān]}`
  - `lines[0]` に Ruby を含みうる
  - `lines[1]` に Ruby を含みうる
- `{佛罗伦萨/Firenze/Florence}`
  - `lines = ["佛罗伦萨", "Firenze", "Florence"]`

---

## 7. Math Segment（`$...$`, `$$...$$`）

### 7.1 構文（規範）

- Inline: `$ TEX $`（display = false）
- Display: `$$ TEX $$`（display = true）

`TEX` は終端区切りが現れるまでの部分文字列とし、内部は **再帰解析しない**。

### 7.2 優先順位（規範）

- `$$...$$` は `$...$` より優先して認識する（`$$` のほうが長い区切りなので先に試す）。
- Math 区間内部の `[]` `{}` `/` は Gloss/Ruby の構文として解釈しない。

### 7.3 エスケープ（規範）

- 開始 `$`（または `$$`）の直前にある連続 `\` の個数が **奇数**のとき、当該 `$` はエスケープされた通常文字として扱う（数式開始ではない）。
    - 例: `\$` はリテラル `$`（数式開始しない）citeturn0search6turn0search2

---

## 8. パース手順（規範）

パーサーは左から右へ走査して Segment を生成する。実装上、次の優先順位を推奨する。

1. **Math の切り出し**（`$$...$$` → `$...$`）
2. **GlossBlock**（`{...}`）
3. **RubyBlock**（`[...]`）
4. それ以外は Plain

### 8.1 GlossLine の解析（規範）

GlossBlock 内部は次の規則で GlossLine を構築する。

- `[` を見たら RubyBlock を試行（成功すれば Ruby を追加）
- `$` / `$$` を見たら MathSegment を試行（成功すれば Math を追加）
- `/` を見たら「未エスケープ」であり Ruby/Math の外側なら次の GlossLine を開始
- `}` で GlossBlock 終了

### 8.2 RubyBase の解析（規範）

RubyBase は **InlineSegment** 列として構築する。

- `$` / `$$` を見たら MathSegment を試行（成功すれば Math を追加）
- それ以外は Plain として蓄積する

---

## 9. エラー処理（フォールバック規則）

パーサーは「失敗した構文を壊さない」方針を採る。

- RubyBlock:
    - `]` が見つからない、`/` がない、未エスケープ `[` が内部にある等の場合は Ruby として解釈しない。
- GlossBlock:
    - `}` が見つからない場合は Gloss として解釈しない。
- Math:
    - 対応する終端 `$`（または `$$`）が見つからない場合は Math として解釈しない。

**フォールバック規則（規範）**  
いずれの構文でも失敗した場合、「開始記号 1 文字」を Plain として出力し、解析を継続する（隣接 Plain は結合してよい）。

---

## 10. HTML レンダリング指針（非規範）

### 10.1 Ruby

HTML の Ruby マークアップの利用を推奨する。citeturn0search3turn0search7turn0search10

- 省略形（簡潔）: `<ruby>base<rt>reading</rt></ruby>`
- 明示形: `<ruby><rb>base</rb><rt>reading</rt></ruby>`

### 10.2 Gloss

Gloss は `lines[0]`（上段）を本文として出し、`lines[1..]` を下段として縦に積む。

- wrapper: `<span class="gloss">...</span>`
- 下段行: `<span class="gloss-alt">...</span>`

> 注: 既存 CSS の都合で `term/term-alt` のクラス名を継続利用してもよい（仕様用語としては gloss を推奨）。

### 10.3 Math（README の方針との整合）

README の「`$` による TeX 数式記法を残す」を満たすため、HTML 生成は次のいずれかを採用してよい。

- **方式A（推奨）**: 入力の delimiters を保持して `"$" + tex + "$"` / `"$$" + tex + "$$"` をそのまま出力する（KaTeX 等の後段処理に任せる）。
- **方式B**: 後段レンダラを使わない場合に限り、`Math` を `<span class="math">...</span>` 等で包み、別途 KaTeX を呼んで置換する（この文書の範囲外）。

---

## 11. テストケース（抜粋）

### 11.1 Ruby

- `[私/わたし]` → Ruby(base="私", reading="わたし")
- `[Text]` → Plain("[Text]")（`/` がないため）
- `[a/b/c]` → Ruby(base="a", reading="b/c")（最初の `/` のみ区切り）

### 11.2 Gloss（多段＋alt 内 Ruby）

- `{Nara/[奈良/なら]}`  
  - `lines=["Nara", Ruby("奈良","なら")]`
- `{[台湾/たいわん]/[台灣/Táiwān]}に行く`  
  - `lines[0]` と `lines[1]` の両方に Ruby が出現しうる
- `{佛罗伦萨/Firenze/Florence}`  
  - `lines=["佛罗伦萨","Firenze","Florence"]`

### 11.3 Math

- `x=$\frac{1}{2}$` → Plain("x="), Math(tex="\frac{1}{2}", display=false)
- `$$a_{[/]}$$` → Math 内の `[/]` は Ruby として解釈しない（保護される）
- `\$40 and $e=mc^2$` → `\$` はリテラル `$`、後半は Math（実装が §7.3 に従う場合）citeturn0search6turn0search2
