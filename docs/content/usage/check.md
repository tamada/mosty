---
title: "参照を検証する（check）"
description: "mosty check による検証の方法、出力形式、終了ステータス。"
date: 2026-09-25
draft: false
weight: 30
---

`mosty check` は、設定ファイルに従って、シートをまたいだ参照が正しい学生の行を指しているかを検証します。
`mosty verify` でも同じように動きます。

---

## 🚀 使い方

```sh
mosty check grades.xlsx
```

`grades.xlsx` と同じディレクトリの `mosty.grades.xlsx.json5` を読み込んで検証します。

```sh
Verify the cross-sheet references with the config files (pass 2)

Usage: mosty check [OPTIONS] <EXCEL_FILEs>...

Arguments:
  <EXCEL_FILEs>...  The target Excel files (.xlsx, .xlsm)

Options:
  -c, --config <FILE>       Specify the config file
  -p, --id-pattern <REGEX>  Specify the pattern of student ids
      --no-init             Exit with an error when the config file is missing, instead of running `init`
  -o, --output <FILE>       Specify the destination of the results (`-` means stdout) [default: -]
  -F, --format <FORMAT>     Specify the output format of the results [default: default] [possible values: default, json, markdown]
  -l, --level <LEVEL>       Specify the log level [default: warn] [possible values: error, warn, info, debug, trace, off]
  -h, --help                Print help (see more with '--help')
```

| オプション | 説明 |
|---|---|
| `-c, --config` | 設定ファイルを指定します。Excel ファイルを 1 つだけ指定したときに使えます。 |
| `-p, --id-pattern` | 学生証番号の正規表現。設定ファイルの `id_pattern` より優先します。 |
| `--no-init` | 設定ファイルがないときに、`mosty init` を実行せずにエラーにします。 |
| `-o, --output` | 検証結果の書き出し先。省略したときと `-` のときは標準出力に出します。 |
| `-F, --format` | 検証結果の出力形式（`default`、`json`、`markdown`）。 |

複数の Excel ファイルを指定すると、それぞれを検証し、結果をまとめて出力します。
一部のファイルが読めなくても、検証できたファイルの結果は出力します。

---

## 🔍 検証のしかた

各シートの学生の行にある数式から、ほかのシートへの参照を取り出し、次のように確かめます。

- **1 つのセル（または 1 行の範囲）への参照**: 参照元の行と参照先の行の学生証番号を比べます。
- **複数の行にまたがる範囲への参照**: 範囲の中に学生の行が 2 行以上あれば、問題として報告します。ある学生の行で、複数の学生のデータを集計しているからです。
  - ただし、`VLOOKUP`、`XLOOKUP`、`INDEX`、`MATCH`、`COUNTIF`、`SUMIF` などの検索関数の範囲は対象外です。学生証番号などのキーで学生を探しているためです。

次の参照は検証しません。

- 同じシートの中の参照（`=SUM(C4:D4)` など）
- 学生のシートではないシート（配点表など）への参照
- 学生の行ではない行（見出し、集計の行など）からの参照
- 学生証番号のセル自身の参照（`=名簿!A2` など）。学生証番号はそこから求めているので、必ず一致するためです。氏名など、同じ行のほかのセルの参照は検証します。
- `INDIRECT`、`OFFSET`、名前付き範囲、ほかのファイルへの参照、3D 参照（`Sheet1:Sheet3!A1`）。参照先を数式から決められないためです。`-l info` で、飛ばした参照を確認できます。

検出される問題の種類と直し方は、[検出される問題](../problems) を参照してください。

---

## 📝 出力形式

### `default`

端末で読むための形式です。

```
grades.xlsx
  最終成績!C4 (1234003 山田 一郎) -> 課題!E7 (1234004 佐藤 次郎): student id mismatch
  最終成績!C5 (1234004 佐藤 次郎) -> 課題!E8 (1234005 鈴木 三子): student id mismatch
  2 problems (4 sheets, 20 references checked)
```

セルの番地は Excel と同じ A1 形式です。

### `json`

ほかのツールで処理するための形式です。

```json
[
  {
    "file": "grades.xlsx",
    "summary": { "sheets": 4, "references": 20, "problems": 1 },
    "problems": [
      {
        "kind": "id_mismatch",
        "source": { "sheet": "最終成績", "cell": "C4", "id": "1234003", "name": "山田 一郎" },
        "target": { "sheet": "課題", "cell": "E7", "id": "1234004", "name": "佐藤 次郎" }
      }
    ]
  }
]
```

### `markdown`

報告書やメモに貼り付けるための形式です。

```markdown
## grades.xlsx

| Kind | Source | Target | Description |
|---|---|---|---|
| id_mismatch | 最終成績!C4 (1234003 山田 一郎) | 課題!E7 (1234004 佐藤 次郎) | student id mismatch |

1 problem (4 sheets, 20 references checked)
```

---

## 🚦 終了ステータス

| 値 | 意味 |
|---|---|
| `0` | 問題なし |
| `1` | 問題が 1 件以上見つかった |
| `2` | ファイルの読み込みエラー、設定ファイルの誤りなど |
| `3` | コマンドラインの引数の誤り |

複数のファイルを指定して結果が異なる場合は、最も大きい値で終了します。
