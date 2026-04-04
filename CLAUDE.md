# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## プロジェクト概要

**qql (Quick Question for LLM)** — CLIから簡単な質問をLLMに投げるツール。

```sh
qql "what is LLM?"
qql "質問" -p claude          # Providerを指定
qql -e "下書き"               # エディタで質問を編集して送信 (--editor / -e)
qql --stdin                   # stdinから質問を読み込む（パイプ対応、`-` も可）
qql --last                    # 直前の回答を再出力（API呼び出しなし）
qql init                      # インタラクティブな初期設定
```

## コマンド

```sh
cargo build                   # ビルド
cargo run -- "質問"           # 実行
cargo test                    # テスト全件
cargo test <test_name>        # 単一テスト
cargo clippy                  # Lint
```

## アーキテクチャ

処理フロー:

```
CLI引数 (cli.rs)
  → app::run() (app.rs)
    → qql init → init.rs（インタラクティブ設定生成）
    → --last   → history.rs（ファイルから再出力）
    → 通常質問 → resolve_question()（editor/stdin/引数から質問取得）
               → config.rs（設定読み込み）
               → provider.rs（Providerディスパッチ）
                 ├── openai.rs / claude.rs / gemini.rs（API呼び出し）
               → history.rs（結果保存）
```

### 各モジュールの役割

- **`app.rs`** — メインロジック。`run()`関数がエントリポイント。依存をすべてtraitで受け取りテスト容易。`Clock`・`QuestionEditor`・`QuestionStdin` traitもここで定義。
- **`cli.rs`** — `clap`によるCLI引数定義。`Cli`構造体と`Command::Init`サブコマンド。
- **`config.rs`** — `~/.config/qql/config.json`の読み書き。`Config`・`ProviderKind`・`AppPaths`を定義。`XDG_CONFIG_HOME`に対応。
- **`provider.rs`** — `Provider` traitと`ProviderFactory` trait。複数Provider時は`std::thread`で並列呼び出し、結果を`BTreeMap<String, String>`に集約。
- **`openai.rs` / `claude.rs` / `gemini.rs`** — 各APIの`Provider` trait実装。`ureq`で同期HTTP（tokio不要）。
- **`history.rs`** — 直前の回答を`~/.config/qql/history.json`に保存・読み出し。`AnswerPayload = BTreeMap<String, String>`。
- **`init.rs`** — `qql init`のインタラクティブUI。`InitUi` trait（`DialoguerInitUi`）と`ModelCatalog` trait（`RealModelCatalog`）を使用。初期化時にAPIを叩いてモデル一覧を取得し、失敗時はハードコードされたプリセットにフォールバック。
- **`main.rs`** — `SystemClock`・`DialoguerQuestionEditor`・`RealQuestionStdin`など実装体を定義し、`app::run()`に注入。
- **`lib.rs`** — 全モジュールを`pub`で公開。`tests/app.rs`がクレートとして参照するために必要。

### 出力形式

- 単一Provider: `{ "openai": "..." }`（JSON）
- 複数Provider: `{ "claude": "...", "openai": "..." }`（JSON、キーはアルファベット順）

### 設定ファイル形式

```json
{
  "default_providers": ["openai"],
  "providers": {
    "openai": { "api_key": "sk-...", "model": "gpt-4o-mini" },
    "claude": { "api_key": "sk-ant-...", "model": "claude-haiku-4-5" }
  }
}
```

### テスト戦略

- `tests/app.rs`が統合テストの本体。`InitUi`・`ProviderFactory`・`Clock`・`ModelCatalog`・`QuestionEditor`・`QuestionStdin`のtraitをモック実装して`app::run()`を直接呼び出す。
- `app.rs`内にもユニットテストあり（エラーフォーマット系）。
- `tempfile`クレートで一時ディレクトリを使用。
