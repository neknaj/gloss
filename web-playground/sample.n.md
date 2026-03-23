# Gloss Web Playground — サンプル

このエディタでは **Gloss 拡張 Markdown** をリアルタイムでレンダリングできます。

## ルビ記法

日本語の[漢字/かんじ]に読みを付けることができます。
[複数/ふくすう]の[単語/たんご]をまとめて[書/か]くことも[可能/かのう]です。

## Gloss 記法

言語学のグロス標記に使う構文です。

- 単語に英語訳を付ける：{走る/run}
- 多段グロス（複数言語・注釈）：{走る/run/to-run}
- ルビとグロスの組み合わせ：{[微分/びぶん][係数/けいすう]/derivative}

---

## セクションのネスト

`---` を使うと **現在のセクションを閉じ**、直後に水平線を描きます。
`;;;` を使うと**水平線なし**でセクションを閉じます。

### 小見出し（`###`）

この[段落/だんらく]は level-3 のセクション内にあります。

;;;

上の `;;;` で level-3 が閉じられ、level-2 に戻りました。

---

## インライン装飾

- **太字（bold）**
- *斜体（italic）*
- ~~打ち消し線~~
- `インラインコード`
- [リンク](https://example.com)
- 複合：**[漢字/かんじ]の `<ruby>` を*斜体*にする**

---

## リスト

### 順序なし

- 項目 A
- 項目 B：{[形態素/けいたいそ]/morpheme}
- 項目 C

### 番号付き

1. 第一[章/しょう]：はじめに
2. 第二[章/しょう]：[理論/りろん]
3. 第三[章/しょう]：おわりに

---

## テーブル

| [記法/きほう] | 入力例 | 出力 |
|:------|:------|------:|
| Ruby | `[漢/かん]` | [漢/かん] |
| Gloss | `{word/gloss}` | {word/gloss} |
| Bold | `**text**` | **text** |
| Italic | `*text*` | *text* |

---

## コードブロック

```rust
fn main() {
    let text = "# Hello\n[漢字/かんじ]";
    let parser = Parser::new(text);
    let mut out = String::new();
    push_html(&mut out, parser);
    println!("{}", out);
}
```

---

## 数式

### 基本的な数式

インライン数式：$E = mc^2$ — アインシュタインの[質量/しつりょう][エネルギー/えねるぎー][等価/とうか][式/しき]。

ピタゴラスの[定理/ていり]：$a^2 + b^2 = c^2$

二次[方程式/ほうていしき]の[解/かい]：$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$

### 解析学・微積分

{[定積分/ていせきぶん]/definite integral}：

$$\int_{-\infty}^{\infty} e^{-x^2} dx = \sqrt{\pi}$$

{[微分/びぶん]/derivative} の[定義/ていぎ]：

$$f'(x) = \lim_{h \to 0} \frac{f(x+h) - f(x)}{h}$$

{[テイラー展開/てぃらーてんかい]/Taylor expansion}：

$$e^x = \sum_{n=0}^{\infty} \frac{x^n}{n!} = 1 + x + \frac{x^2}{2!} + \frac{x^3}{3!} + \cdots$$

### 線形代数

{[行列/ぎょうれつ]/matrix} の[積/せき]：

$$\begin{pmatrix} a & b \\ c & d \end{pmatrix} \begin{pmatrix} x \\ y \end{pmatrix} = \begin{pmatrix} ax+by \\ cx+dy \end{pmatrix}$$

{[固有値/こゆうち]/eigenvalue} [方程式/ほうていしき]：$Av = \lambda v$

{[行列式/ぎょうれつしき]/determinant}：$\det(A) = ad - bc$

### 確率・統計

{[正規分布/せいきぶんぷ]/normal distribution} の{[確率密度関数/かくりつみつどかんすう]/probability density function}：

$$f(x) = \frac{1}{\sigma\sqrt{2\pi}} \exp\!\left(-\frac{(x-\mu)^2}{2\sigma^2}\right)$$

{[ベイズの定理/べいずのていり]/Bayes' theorem}：

$$P(A|B) = \frac{P(B|A)\,P(A)}{P(B)}$$

### 化学式

{[水/みず]/water}：$\text{H}_2\text{O}$

{[二酸化炭素/にさんかたんそ]/carbon dioxide}：$\text{CO}_2$

{[硫酸/りゅうさん]/sulfuric acid}：$\text{H}_2\text{SO}_4$

[酸化還元反応/さんかかんげんはんのう]：

$$\text{Zn} + \text{H}_2\text{SO}_4 \to \text{ZnSO}_4 + \text{H}_2\uparrow$$

[ゼーベックの法則/ぜーべっくのほうそく]（熱化学）：

$$\Delta G = \Delta H - T\Delta S$$

### 電磁気学・物理

{[マクスウェル方程式/まくすうぇるほうていしき]/Maxwell equations}（積分形）：

$$\oint_{\partial \Omega} E \cdot dA = \frac{Q_{\text{enc}}}{\varepsilon_0}$$

{[シュレーディンガー方程式/しゅれーでぃんがーほうていしき]/Schrödinger equation}：

$$i\hbar\frac{\partial}{\partial t}\Psi = \hat{H}\Psi$$

---

## 引用（Blockquote）

> 言語とは[意味/いみ]を[伝/つた]えるための[道具/どうぐ]である。
> [構造/こうぞう][主義/しゅぎ]的な[観点/かんてん]から見ると、{言語/language}は{記号/sign}の[体系/たいけい]である。

---

## 画像・メディア

![Gloss Parser Logo](https://via.placeholder.com/400x200?text=Gloss+Playground)

---

## バックスラッシュエスケープ

特殊文字のエスケープ：\{ \[ \\ \* \`

明示的な改行 — この行の後で改行します：\n次の行

---

## [設計/せっけい]の[考/かんが]え[方/かた]

### なぜ[拡張/かくちょう] Markdown か

標準 Markdown には以下の[機能/きのう]が[不足/ふそく]しています：

1. 日本語の[振/ふ]り[仮名/がな]（ルビ）
2. 言語学的グロス標記
3. [上記/じょうき]を[含/ふく]むセクション[階層/かいそう]の[明示的/めいじてき]な[制御/せいぎょ]

### AST [駆動/くどう]パーサー

```
Parser::new(text)
  → parse_blocks() // ブロック要素
    → parse_inline() // インライン要素
      → Event stream
        → push_html() // HTML 生成
```
