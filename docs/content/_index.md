---
title: "mosty"
description: "もう成績訂正はやだよ。成績処理用の Excel ファイルで、シートをまたいだ参照のずれを検出します。"
date: 2026-09-25
draft: false
outputs:
  - html
---

{{< keywordList markdownify="true" >}}
[![Version](https://img.shields.io/badge/Version-0.1.0-blue)](https://github.com/tamada/mosty/releases/tag/v0.1.0)
[![License-MIT](https://img.shields.io/badge/License-MIT-blue)](https://github.com/tamada/mosty/blob/main/LICENSE)
[![Coverage Status](https://coveralls.io/repos/github/tamada/mosty/badge.svg)](https://coveralls.io/github/tamada/mosty)
[![Docker](https://img.shields.io/badge/Container-quay.io/tama5/mosty:0.1.0-blue?logo=docker)](https://quay.io/repository/tama5/mosty)
{{< /keywordList >}}

[ [🗣️ 概要](#-概要) | [🔍 何を検出するか](#-何を検出するか) | [🚶 検証の流れ](#-検証の流れ) | [🧭 ナビゲーション](#-ナビゲーション) ]

**mosty**（**MO**u **S**eiseki **T**eisei ha **Y**adayo、もう成績訂正はやだよ）は、成績処理用の Excel ファイルを検証するツールです。
シートをまたいだセルの参照が、別の学生の行を指していないか（いわゆる「1 行ずれ」）を、成績を報告する前に機械的に確かめます。

---

## 🗣️ 概要

成績を求めるときには、課題の点数、試験の点数、出席などをシートごとに並べ、最終成績のシートでそれらを参照して計算することがよくあります。
成績報告の後に成績調査願が出され、調べてみると原因は Excel の単純な参照ミスで 1 行ずれていた、ということは珍しくありません。

学習支援システム（Moodle など）と履修者を管理するシステムの受講生リストは、少しずつ異なるのが普通です。
両方から持ち込んだデータを統合するとき、途中にイレギュラーな行を挿入・削除して手作業で直すと、参照がずれる原因になります。

mosty は、各シートの行がどの学生を表しているかを学生証番号から判断し、ある学生の行が別の学生の行を参照していれば警告します。

---

## 🔍 何を検出するか

```
最終成績シート                           課題シート
行 5: 1234004 | 山田 | =課題!C7   ──→   行 7: 1234005 | 佐藤 | 10
      ^^^^^^^                                  ^^^^^^^
      参照元の学生                              参照先の学生    → 一致しない: 警告
```

- 参照元の行と参照先の行で、学生証番号が一致しない。
- ある学生の行から、複数の学生の行を含む範囲を参照している（`=SUM(課題!E4:E8)` など）。
- 同じシートの中で、学生証番号が重複している。
- 設定ファイルを作った後に、シートや学生が追加・削除された。

詳しくは [検出される問題](/usage/problems) を参照してください。

---

## 🚶 検証の流れ

mosty は 2 段階で検証します。

1. **`mosty init`**: Excel ファイルを解析し、シートごとに学生証番号の列、氏名の列、学生の行の範囲を推定して、設定ファイルに書き出します。
2. **確認**: 書き出された設定ファイルを確認し、推定が誤っていれば直します。
3. **`mosty check`**: 設定ファイルに従って、シートをまたいだ参照を検証します。

```sh
mosty init grades.xlsx      # mosty.grades.xlsx.json5 を書き出す
mosty check grades.xlsx     # 設定ファイルに従って検証する
```

---

## 🧭 ナビゲーション

- インストール方法は [⚓️ Install](/install) を参照してください。
- 使い方は [🏃 Usage](/usage) を参照してください。
- 名前の由来やよくある質問は [About](/about) を参照してください。
