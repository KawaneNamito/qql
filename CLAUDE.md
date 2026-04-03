# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## プロジェクト概要

**qql (Quick Question for LLM)** — CLIから簡単な質問をLLMに投げるツール。

```sh
qql "what is LLM?"
```

## 仕様

- API Provider: OpenAI / Claude / Gemini から選択可能
- デフォルトProviderを設定できる（複数選択も可）
  - 複数選択時はJSON形式 `{ <provider_name>: "message" }` で出力
- オプションで直前の回答を再出力できる（API呼び出しなし）
- 設定はJSON形式（APIキーを含む）

## 環境変数

| 環境変数 | 説明 |
| --- | --- |
| （README.ja.mdに未記載） | |

## 実装時の注意

- APIキーは設定ファイル（JSON）で管理する。環境変数との優先順位はREADME/仕様を確認。
- 複数Provider指定時の出力形式: `{ "openai": "...", "claude": "..." }`
- 「直前の回答の再出力」機能はキャッシュ/ログからAPI呼び出しなしで返す。
