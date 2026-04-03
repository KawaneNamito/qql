# qql

Quick Question for LLM

CLI から短い質問をすばやく LLM に投げるためのツールです。

## できること

- OpenAI / Claude / Gemini を使って質問できる
- デフォルト provider を 1 つまたは複数設定できる
- 単一 provider / 複数 provider のどちらでも JSON 形式で出力できる
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
- 既存設定ファイルがある場合の上書き確認

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

出力は常に JSON 形式です。単一 provider の場合も provider 名を key にしたオブジェクトで返します。

```json
{
  "openai": "LLM is ...",
  "claude": "LLM stands for ..."
}
```

単一 provider の例:

```json
{
  "claude": "LLM stands for Large Language Model."
}
```

JSON 出力を見やすく表示したい場合は、`qql` をラップする `qq` 関数を用意しておくと便利です。

```sh
qq() {
  local output markdown
  output="$(qql "$@")" || return $?

  if printf '%s' "$output" | jq -e 'type == "object"' >/dev/null 2>&1; then
    markdown="$(
      printf '%s' "$output" \
        | jq -r '
            to_entries
            | map("# \(.key)\n\n\(.value)")
            | join("\n\n---\n\n")
          '
    )"
  else
    markdown="$output"
  fi

  printf '%s\n' "$markdown" | glow -p
}
```

この関数を `~/.zshrc` や `~/.bashrc` に追加してシェルを読み直すと、次のように使えます。

```sh
qq "what is LLM?"
qq -p openai -p claude "what is LLM?"
qq --last
```

複数 provider の場合でも `glow` は 1 回だけ起動するため、1 つの画面で連続スクロールしながら比較できます。

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
