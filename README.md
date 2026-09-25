# <img src="docs/assets/mosty.svg" width="48" height="48" alt=""> mosty

[![Version](https://img.shields.io/badge/Version-0.1.0-blue)](https://github.com/tamada/mosty/releases/tag/v0.1.0)
[![License-MIT](https://img.shields.io/badge/License-MIT-blue)](https://github.com/tamada/mosty/blob/main/LICENSE)

[![Coverage Status](https://coveralls.io/repos/github/tamada/mosty/badge.svg)](https://coveralls.io/github/tamada/mosty)

[![Docker](https://img.shields.io/badge/Container-quay.io/tama5/mosty:0.1.0-blue?logo=docker)](https://quay.io/repository/tama5/mosty)

もう成績訂正はやだよ。

MOu Seiseki Teisei ha Yadayo!

I hope I never have to correct submitted grades again with this tool.

## 🗣️ Overview

色々な成績処理を出すためには、Excel を使うことが多いと思う。
大量のシート間をまたがって、複雑な数式やデータを関連付けて最終成績を求めることが多いと思う。

そして、成績報告後に成績調査願が提出されることがある。
成績が間違っているのは、学生に申し訳ないのと同様に、調査には報告する以上に気を遣う作業となる。
そして、間違いの原因のほとんどは、Excelの単純な参照ミスで１段ずれていたりすることだったりする。

イレギュラーな行が途中で挟まり、そこを手作業で修正する行為がミスの原因の大部分であったりする。残念なことに、課題とかが提出される学習サイト（Moodle）と履修者情報を管理するサービスが異なっており、これを統合することはまぁ不可能であろう。となると、両者の受講生リストが若干異なることが普通に起こる。すなわち、両者から持ち込んだ Excel データを統合するときにこのようなミスが起こりうるわけである。

潜り受講者にも評価を加えようとすると、途端に破綻する。これを許すかどうかは個人のポリシーにもよるため、潜り受講生を統一的に禁止することはできない。

ということで、成績を求めるための Excel を検証するツールを作り、報告前に間違っていないかを確認しようと狙うわけである。

## 🏃‍♀️ 検証方法

間違いの大半は Excel の各シートをまたがる参照の間違いにある。
シートごとに、学生IDに対して、ある観点での評価結果が並んでいるものと想定する。
例えば、課題の点数や試験の点数などである。
そして、それらを統合して、最終成績を求めるシートがあるものと想定する。

私の場合は、試験の成績、課題の成績、各課題の成績、試験の正答など、一つの Excel ファイルに数多くのシートを並べている。そして、各シートの結果となる成績をまとめて、最終成績を算出するわけである。

どこかのシートでイレギュラーに学生のデータが削除されるとか、挿入されることでズレが生じるわけである。これを防ぐには、シートごとにどの学生の情報が参照されているかを確認し、他の行を参照していないことを確認すると良い。

各シートは、マスターとなる学生名簿をもとに、学生ごとに当該シートの観点での成績が計算されている。
つまり、各シートには、学生IDに基づくデータが各行に並んでいることになる。

シートごとにどの行がどの学生を表しているのかを確認後、その行で、別のシートの別の学生を参照していれば危険だ！というわけである。

ということで、危険な参照が見つけるためのツールを作成しようというものである。

## 🚶 Usage

mosty は 2 段階で検証します。

1. `mosty init` で Excel ファイルを解析し、シートごとの学生証番号列・氏名列・学生の行の範囲を推定して、設定ファイル（`mosty.<Excel ファイル名（拡張子を含む）>.json5`）に書き出します。
2. 書き出された設定ファイルを確認し、推定が誤っていれば修正します。
3. `mosty check` で、設定ファイルに従ってシート間の参照を検証します。

```sh
mosty init grades.xlsx      # mosty.grades.xlsx.json5 を書き出す
mosty check grades.xlsx     # mosty.grades.xlsx.json5 を使って検証する
```

```sh
mosty [OPTIONS] <COMMAND>
COMMANDS
    init     Analyze the Excel files and write the config files (pass 1)
    check    Verify the cross-sheet references with the config files (pass 2) [alias: verify]
OPTIONS
    -l, --level <LEVEL>          Specify the log level [default: warn] [error, warn, info, debug, trace, off]

mosty init [OPTIONS] <EXCEL_FILE...>
OPTIONS
    -p, --id-pattern <REGEX>     Specify the pattern of student ids [default: ^[0-9]{7}$]
    -C, --max-column <N>         Search student ids and names in the columns 0..=N [default: 2]
    -o, --output <FILE>          Specify the destination of the config file ("-" means stdout) [default: mosty.<EXCEL_FILE>.json5]
    -f, --force                  Overwrite the existing config file

mosty check [OPTIONS] <EXCEL_FILE...>
OPTIONS
    -c, --config <FILE>          Specify the config file [default: mosty.<EXCEL_FILE>.json5]
    -p, --id-pattern <REGEX>     Specify the pattern of student ids (overrides the config file)
        --no-init                Do not run init when the config file is missing (exit with an error)
    -o, --output <FILE>          Specify the destination of the results ("-" means stdout) [default: -]
    -F, --format <FORMAT>        Specify the output format [default: default] [default, json, markdown]
```

- 対応形式: `.xlsx`、`.xlsm`
- 終了ステータス: `0` 問題なし、`1` 問題あり、`2` ファイル・設定ファイルのエラー、`3` コマンドライン引数の誤り

### 🐳 コンテナで実行する

カレントディレクトリをコンテナの `/opt` にマウントして実行します。

```sh
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/opt" quay.io/tama5/mosty check grades.xlsx
```

`--user` で自分の uid と gid を指定するのは、mosty が設定ファイルを書き出せるようにするためです。
コンテナ内の mosty は `nonroot`（uid 65532）で動くので、Linux では指定しないとカレントディレクトリに書き込めません。
`mosty init` だけでなく、設定ファイルがないときの `mosty check` も、最初に設定ファイルを書き出します。
macOS の Docker Desktop では、`--user` を省略しても動きます。

詳細な仕様は [.github/assets/spec.md](.github/assets/spec.md) を参照してください。

## 🎉 FAQ

### Q1. AI にチェックしてもらえればいいんじゃないの？

はい。それでも構いません。
ただ、成績は非常にデリケートな個人情報であると思います。
それをAIの学習に用いて良いという組織の判断であるならば良いと思います。

### Q2. クラウドAIじゃなくて、ローカルAIだったら大丈夫でしょう？

はい。ローカルAI**も**活用すれば良いと思います。
何か一つの方法だけでミスを防ぐのではなく、複数の方法で防ぐのがセオリーであると思います。

### Q3. 学生のIDを匿名化すればAIを使っても問題ないんじゃない？

目的は、成績訂正を0にするための事前チェックです。
その事前チェックをかける前にExcelの内容を編集してチェックするのは、余計な手間で本来チェックすべき内容を削除する可能性があることにご留意ください。

### Q4. 私はそんな間違いはしません。

すごいですね。では使わなくて良いのではないでしょうか。
個人的な要望から始まったプロジェクトです。
少なくとも私はそこまで完璧な人間ではないので間違えます。
だからツールの助けを借りてチェックをしようとしています。
