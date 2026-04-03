# qql

Quick Question for LLM

CLI から短い質問をすばやく LLM に投げるためのツールです。

## できること

- OpenAI / Claude / Gemini を使って質問できる
- デフォルト provider を 1 つまたは複数設定できる
- 複数 provider に同時に問い合わせて JSON で比較できる
- 直前の回答を `--last` で再表示できる
- `qql init` で対話式に設定ファイルを作成できる

## ビルド

```sh
cargo build --release
```

生成されるバイナリ:

```sh
./target/release/qql
```

## クイックスタート

初回は設定を作成します。

```sh
./target/release/qql init
```

`qql init` では以下を対話式に設定します。

- 利用する provider の選択
- API key の貼り付け
- 利用可能モデル一覧からの選択
- 一覧にない場合の custom model 入力

モデル一覧は入力した API key を使って実行時に取得します。取得に失敗した場合は、内蔵の候補一覧にフォールバックします。

設定後はそのまま質問できます。

```sh
./target/release/qql "what is LLM?"
```

## 使い方

デフォルト provider に問い合わせる:

```sh
qql "what is LLM?"
```

provider を指定して問い合わせる:

```sh
qql -p claude "what is LLM?"
qql -p openai -p gemini "what is LLM?"
```

直前の回答を再表示する:

```sh
qql --last
```

## 出力

単一 provider の場合は文字列をそのまま出力します。

```txt
LLM stands for Large Language Model.
```

複数 provider の場合は JSON を出力します。

```json
{
  "openai": "LLM is ...",
  "claude": "LLM stands for ..."
}
```

## 設定ファイル

設定ファイル:

```txt
~/.config/qql/config.json
```

`XDG Base Directory` に従い、`$XDG_CONFIG_HOME` が設定されている場合はそちらを優先します。

履歴ファイル:

```txt
~/.config/qql/history.json
```

## 設定例

```json
{
  "default_providers": ["openai", "claude"],
  "providers": {
    "openai": {
      "api_key": "sk-...",
      "model": "gpt-5-mini"
    },
    "claude": {
      "api_key": "sk-ant-...",
      "model": "claude-sonnet-4-20250514"
    }
  }
}
```

## provider

現在サポートしている provider:

- `openai`
- `claude`
- `gemini`

## 開発時の実行例

```sh
cargo run -- init
cargo run -- "what is LLM?"
cargo run -- --last
```
