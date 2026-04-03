# qql

Quick Question for LLM

## 概要

CLI上で簡単な質問をLLMに投げるツール。

## 例

```sh
qql init
qql "what is LLM?"
```

`qql init` を実行すると、`~/.config/qql/config.json` に設定ファイルの雛形を作成する。

## 仕様

- API Providerは OpenAI, Claude, Geminiから選ぶことができる
- デフォルトのAPI Providerを設定できる（複数選択もできる）
  - 複数選択した場合、JSON形式で`{ <provider_name>: "message"}`の形式で出力される
- オプションから直前の回答を再出力できる（API呼び出しはされない）
- 設定はJSON形式（API Keyも含む）
- `qql init` で設定ファイルの雛形を生成できる
