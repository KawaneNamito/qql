# アーキテクチャ設計

## ディレクトリ構造

```
qql/
├── src/
│   ├── main.rs        # エントリポイント・CLI引数パース・処理フロー
│   ├── config.rs      # 設定ファイルの読み込み・書き込み・パース
│   ├── provider.rs    # Provider trait と呼び出しディスパッチ
│   ├── openai.rs      # OpenAI API 実装
│   ├── claude.rs      # Claude (Anthropic) API 実装
│   ├── gemini.rs      # Gemini API 実装
│   └── history.rs     # 直前の回答の保存・読み出し
├── docs/
│   ├── architecture.md
│   └── config-schema.md
├── Cargo.toml
└── README.ja.md
```

## 各モジュールの役割

### `main.rs`
- CLIの引数・オプションを `clap` でパース
- 設定ファイルを読み込む
- `--last` オプション時は `history` から再出力して終了
- Providerを決定し `provider.rs` 経由で問い合わせ
- 結果を標準出力に書き出し、`history` に保存

```
qql "質問"           # デフォルトProviderに問い合わせ
qql "質問" -p claude # 指定Providerに問い合わせ
qql --last           # 直前の回答を再出力（API呼び出しなし）
```

### `config.rs`
- `~/.config/qql/config.json` を読み書きする
- `Config` 構造体にデシリアライズ
- ファイルが存在しない場合のデフォルト値を提供

### `provider.rs`
- `Provider` trait を定義（メソッド: `ask(&self, question: &str) -> Result<String>`）
- `default_providers` の設定から呼び出すProviderを選択
- 複数Provider時は `std::thread` で並列呼び出し、結果を `serde_json::Map` に集約して JSON 出力

### `openai.rs` / `claude.rs` / `gemini.rs`
- 各APIのリクエスト/レスポンス構造体
- `Provider` trait の実装
- `ureq` で同期HTTPリクエスト（軽量・シンプル）

### `history.rs`
- 直前の回答を `~/.config/qql/history.json` に保存
- `--last` オプション時に読み出す

## 主要な依存クレート（軽量構成）

```toml
[dependencies]
clap    = { version = "4", features = ["derive"] }
serde   = { version = "1", features = ["derive"] }
serde_json = "1"
ureq    = { version = "2", features = ["json"] }   # 同期HTTPクライアント（tokio不要）
dirs    = "5"                                        # ホームディレクトリ解決
```

> 複数Providerの並列呼び出しは `std::thread` で実装する。Provider数が最大3と少数・固定なので
> スレッドのオーバーヘッドは無視できる。`ureq` は同期クライアントなのでスレッド内でそのまま使える。
> `tokio` + `reqwest` は不要。

## データフロー

```
CLI引数
  │
  ▼
config.rs ──→ Config { default_providers, providers: { api_key, model } }
  │
  ▼
main.rs ──→ --last? ──yes──→ history.rs ──→ 標準出力
              │
              no
              │
              ▼
          provider.rs
          ├── openai.rs  ──→ OpenAI API
          ├── claude.rs  ──→ Claude API
          └── gemini.rs  ──→ Gemini API
              │
              ▼
          結果集約（単一: 文字列 / 複数: JSON）
              │
              ▼
          標準出力 + history.rs に保存
```
