---
title: "⚓️ Install"
description: "mosty のインストール方法（Homebrew、ビルド済みのバイナリ、ソースからのビルド、コンテナ）。"
date: 2026-09-25
draft: false
---

**mosty** は、次のいずれかの方法でインストールできます。
対応している OS は macOS と Linux です。

---

## 🍺 Homebrew

macOS と Linux では、Homebrew でインストールできます。

```sh
brew install tamada/tap/mosty
```

---

## 📦 ビルド済みのバイナリ

[リリースページ](https://github.com/tamada/mosty/releases) から、環境に合ったファイルをダウンロードして展開してください。

| OS | CPU | ファイル名 |
|---|---|---|
| macOS | Apple シリコン | `mosty-<バージョン>_arm64_darwin.tar.gz` |
| macOS | Intel | `mosty-<バージョン>_amd64_darwin.tar.gz` |
| Linux | arm64 | `mosty-<バージョン>_arm64_linux.tar.gz` |
| Linux | x86_64 | `mosty-<バージョン>_amd64_linux.tar.gz` |

展開したディレクトリの `mosty` を、`PATH` の通ったディレクトリに置いてください。

---

## 🛠️ ソースからビルドする

[Rust](https://www.rust-lang.org/tools/install)（1.88 以降）が必要です。

```sh
git clone https://github.com/tamada/mosty.git
cd mosty
cargo install --path .
```

`~/.cargo/bin` に `mosty` がインストールされます。

---

## 🐳 コンテナ

インストールせずに、コンテナで実行することもできます。
カレントディレクトリをコンテナの `/opt` にマウントしてください。

```sh
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/opt" quay.io/tama5/mosty check grades.xlsx
```

`--user` で自分の uid と gid を指定するのは、mosty が設定ファイルを書き出せるようにするためです。
コンテナ内の mosty は `nonroot`（uid 65532）で動くので、Linux では指定しないとカレントディレクトリに書き込めません。
`mosty init` だけでなく、設定ファイルがないときの `mosty check` も、最初に設定ファイルを書き出します。
macOS の Docker Desktop では、`--user` を省略しても動きます。

---

## 🚀 インストールの確認

```sh
mosty --version
mosty --help
```
