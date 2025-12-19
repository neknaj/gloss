# Gloss Markup Specification (Draft v0.1)

この文書は、Gloss 記法（`[base/reading]` と `{main/alt...}`）および `$...$` / `$$...$$` の数式区間を、**パーサーが生成する構文木（AST）**まで含めて厳密に定義する仕様書です。  
本仕様は、現状の参照実装（JS パーサー）に合わせた **互換仕様** を主にしつつ、「多言語（複数 alt 行）」など Readme に書かれている将来拡張を **拡張仕様** として明示します。

---

## 1. 目的

- **Ruby**: ルビ（発音・転写・注釈）を簡潔に書く（例: `[漢字/かんじ]`）。
- **Gloss**: 用語等の本文（上段）に対して、下段に別言語（英語など）を添える（例: `{微分係数/derivative}`）。
- **Math**: `$...$` / `$$...$$` を「数式区間」として扱い、区間内部の `[]` `{}` `/` が Gloss/Ruby と誤解釈されないようにする。

---

## 2. 用語

- **Input**: UTF-8 のテキスト（1 行でも複数行でもよい）
- **Segment**: Input を分割した最小の意味単位（AST のノード）
- **Ruby Block**: `[...]` の構文要素
- **Gloss Block**: `{...}` の構文要素
- **Math Segment**: `$...$` または `$$...$$` の区間

---

## 3. AST（構文木）モデル

### 3.1 Segment 型

本仕様のパーサーは、Input を次の Segment 列として返す。

- **Plain**
    - `text: String`
- **Annotated**（Ruby）
    - `base: InlineSegment[]`
    - `reading: String`
- **Gloss**（互換仕様では “Term” と同等）
    - `children: GlossChildSegment[]`
    - `alts: String[]`（互換仕様 v0.1 では最大 1 要素。拡張仕様で複数可）
- **Math**
    - `tex: String`
    - `display: bool`（`$$...$$` = true, `$...$` = false）

### 3.2 InlineSegment 型（Ruby base 内など）

- **Plain**
- **Math**

### 3.3 GlossChildSegment 型（Gloss の上段本文）

- **Plain**
- **Annotated**（Ruby）
- **Math**

---

## 4. 字句（トークン）とエスケープ

### 4.1 構文用の特別文字

構文文字は次の 5 つ:

- `[` `]` `/` `{` `}`

### 4.2 バックスラッシュによるエスケープ

`\`（バックスラッシュ）に続く文字が「特別文字」または `\` のとき、直後 1 文字は **構文文字ではなく通常文字**として扱う。

- 例: `\[` は「`[`」として扱う（Ruby の開始ではない）
- 例: `\{` は「`{`」として扱う（Gloss の開始ではない）
- 例: `\/` は「`/`」として扱う（区切りではない）

> 注: 互換仕様（参照 JS 実装）では、構文エスケープ対象は上記 5 文字＋`\` であり、`$` は字句段階では特別扱いしない。

---

## 5. Ruby Block（`[base/reading]`）

### 5.1 構文

```ebnf
RubyBlock := '[' RubyBase '/' RubyReading ']'
```

- `base`（RubyBase）と `reading`（RubyReading）は空文字列でもよい（推奨しない）。
- RubyBlock は、**必ず 1 つ以上の `/` を含む**必要がある。
    - `/` が存在しない（例: `[abc]`）場合、その `[` は Ruby 開始とみなさず **Plain** として処理する。

### 5.2 RubyBase

- RubyBase は、**InlineSegment（Plain/Math）**として保持する。
- RubyBase 内の `$...$` / `$$...$$` は **Inline Math** として解釈してよい（参照実装も対応）。

### 5.3 RubyReading

- RubyReading は **生文字列**（String）として保持する。
- RubyReading 内では、Gloss/Ruby/Math の再帰解析を行わない（互換仕様）。

### 5.4 禁止事項（互換仕様）

- RubyBlock の内側で、未エスケープの `[` が出現した場合、その RubyBlock は **不正**とみなし解析に失敗する。
    - 解析失敗時は「Ruby として解釈せず Plain として扱う」（フォールバック）。

---

## 6. Gloss Block（`{main/alt...}`）

Gloss は、「上段本文（main）」と「下段の別表記（alt）」を 1 つ以上持つ構文。

### 6.1 構文（拡張仕様）

```ebnf
GlossBlock := '{' GlossMain ( '/' GlossAlt )* '}'
```

- `GlossMain` は上段本文（表示の主文字列）。
- `GlossAlt` は下段の別表記（英語・転写・別言語など）。

### 6.2 互換仕様 v0.1（参照 JS 実装と同じ挙動）

- 最初の `/` までが `GlossMain`。
- 最初の `/` 以降、閉じ `}` までが **1 本の文字列**として `alts[0]` に格納される。
    - `{A/b/β}` は `main="A"`, `alts=["b/β"]` になる（複数行には分割しない）。
- `GlossMain` は `RubyBlock` を 1 階層だけ含めることができる（`{Nara/[奈良/なら]}` のような利用を想定）。
- `GlossMain` 内の `$...$` / `$$...$$` は Inline Math として解釈してよい。
- `GlossAlt` 側は **再帰解析しない**（Plain 文字列）。

### 6.3 拡張仕様（将来互換の推奨）

- `/` 区切りを **すべて分割**し、`alts = [alt1, alt2, ...]` として保持する。
- レンダラーは alt を縦に積む（例: 日本語 → 英語 → ギリシャ文字）。

---

## 7. Math Segment（`$...$`, `$$...$$`）

### 7.1 構文

- Inline: `$ TEX $`（display = false）
- Display: `$$ TEX $$`（display = true）

### 7.2 解釈ルール（互換仕様）

- Math は「最外層」を先に分割するため、**Math 区間内部の `[]` `{}` `/` は Gloss/Ruby として解釈しない**。
- ただし `[` `{` の **内側**（Ruby/Gloss 内）に書かれた `$...$` は、RubyBase や GlossMain の Inline Math として解釈してよい。

### 7.3 エスケープ（推奨）

- `\$` は数式開始・終了の `$` として扱わず、通常文字 `$` として扱えることが望ましい。
    - 参照 JS 実装の Inline Math 分割は `\` の個数（奇偶）でエスケープ判定を行う。

---

## 8. パース手順（規範）

参照実装は概ね次の 2 段構えで処理する:

1. **Top-level Math 分割**  
   入力を走査し、`[` `]` `{` `}` の深さを追跡しつつ、深さ 0 のときだけ `$...$` / `$$...$$` を Math として切り出す。
2. **Plain 部分の Ruby/Gloss 分割**  
   Plain 部分を字句解析（特別文字とエスケープを反映）し、左から順に
   - `{` が来たら GlossBlock を試行
   - `[` が来たら RubyBlock を試行
   - 成功したら該当 Segment を 1 つ出力しカーソルを進める
   - 失敗したら 1 文字分を Plain として出力（隣接 Plain は結合してよい）

---

## 9. エラー処理（フォールバック規則）

パーサーは「失敗した構文を壊さない」方針で、次を推奨する:

- RubyBlock:
    - `]` が見つからない、`/` がない、未エスケープ `[` が内部にある等の場合は Ruby として解釈しない。
- GlossBlock:
    - `}` が見つからない場合は Gloss として解釈しない。
- いずれも「開始記号 1 文字」を Plain として出力して解析を継続する。

---

## 10. HTML レンダリング指針（非規範）

Web レンダラーは、HTML の Ruby マークアップを使用することを推奨する。

- Ruby: `<ruby><rb>base</rb><rt>reading</rt></ruby>`
- Gloss: 上段本文を `<ruby>` で組み、下段を `<span class="term-alt">...</span>` のように別要素で表現する、など。

---

## 11. 互換性メモ（参照実装との対応）

参照 JS 実装（`ruby-parser.js`）の主要な一致点:

- 構文文字のエスケープ（`[ ] / { }` と `\`）
- RubyBlock の `[` 内ネスト禁止と `/` 必須
- GlossBlock は `GlossMain` に Ruby を含められる
- Math 分割で `[]` `{}` 深さを追跡し、数式内部で Gloss/Ruby を誤爆させない

Readme に書かれている「複数言語（`{B/b/β}`）」は、v0.1 互換実装では `alts` を 1 本の文字列として保持するため、**正式対応は拡張仕様（将来版）**とする。

---

## 12. テストケース（抜粋）

### 12.1 Ruby

- `[私/わたし]` → Annotated(base="私", reading="わたし")
- `[Text]` → Plain("[Text]")（`/` がないため）

### 12.2 Gloss

- `{[微分/びぶん][係数/けいすう]/derivative}`
- `{Nara/[奈良/なら]}`（GlossMain に Ruby）

### 12.3 Math

- `x=$\frac{1}{2}$` → Plain("x="), Math(tex="\frac{1}{2}", display=false)
- `$$a_{[/]}$$` → Math 内の `[/]` は Ruby として解釈しない（保護される）
