---
title: "🏃 Usage"
description: "mosty の使い方。Excel ファイルの前提、init と check、検出される問題、設定ファイル。"
date: 2026-09-25
draft: false
---

**mosty** は、`init` と `check` の 2 つのサブコマンドで検証します。

---

## 🚶 検証の流れ

```sh
mosty init grades.xlsx      # 1. シートの構造を推定して、設定ファイルを書き出す
                            # 2. mosty.grades.xlsx.json5 を確認し、誤りがあれば直す
mosty check grades.xlsx     # 3. 設定ファイルに従って、シートをまたいだ参照を検証する
```

Excel ファイルを直しながら何度も検証するときは、`mosty check` だけを繰り返し実行します。
シートや学生を追加・削除したときは、`mosty init -f` で設定ファイルを作り直してください。

設定ファイルがないまま `mosty check` を実行すると、`mosty init` を自動で実行してから検証します。
このときも、書き出された設定ファイルを後で確認してください。

---

## 🚀 ヘルプ

```sh
Detects misaligned cross-sheet references in grade workbooks

Usage: mosty [OPTIONS] <COMMAND>

Commands:
  init   Analyze the Excel files and write the config files (pass 1)
  check  Verify the cross-sheet references with the config files (pass 2) [alias: verify]
  help   Print this message or the help of the given subcommand(s)

Options:
  -l, --level <LEVEL>  Specify the log level [default: warn] [possible values: error, warn, info, debug, trace, off]
  -h, --help           Print help (see more with '--help')
  -V, --version        Print version

Run `mosty init` to write the config files, review them, then run `mosty check`.
```

`-h` は短い説明を、`--help` は詳しい説明を表示します。

`-l, --level` は、処理中の警告や情報（標準エラー出力）の量を指定します。
検証結果（標準出力）は、`-l off` にしても必ず出力されます。

---

## 📚 詳しい説明

1. **[Excel ファイルの前提](workbook)**  
   mosty が想定しているシートの構造と、学生証番号の書き方。

2. **[設定ファイルを作る（`init`）](init)**  
   シートの構造の推定方法と、書き出される設定ファイルの確認のしかた。

3. **[参照を検証する（`check`）](check)**  
   検証の方法、出力形式、終了ステータス。

4. **[検出される問題](problems)**  
   検出される問題の種類と、直し方。

5. **[設定ファイル](config)**  
   設定ファイルの書式と、各項目の意味。
