# 設定ファイル仕様

## ファイルパス

```
~/.config/qql/config.json
```

XDG Base Directory 仕様に従い、`$XDG_CONFIG_HOME` が設定されている場合はそちらを優先する。

## スキーマ

```jsonc
{
  // デフォルトで使用するProvider（1つまたは複数）
  // 出力は常に { "openai": "...", "claude": "..." } の形式になる
  "default_providers": ["openai"],

  "providers": {
    "openai": {
      "api_key": "sk-...",
      "model": "gpt-4o-mini"   // 省略時のデフォルト: "gpt-4o-mini"
    },
    "claude": {
      "api_key": "sk-ant-...",
      "model": "claude-haiku-4-5"  // 省略時のデフォルト: "claude-haiku-4-5"
    },
    "gemini": {
      "api_key": "AIza...",
      "model": "gemini-2.0-flash"  // 省略時のデフォルト: "gemini-2.0-flash"
    }
  }
}
```

## フィールド定義

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `default_providers` | `string[]` | ✓ | デフォルトで使うProvider名の配列。`"openai"` / `"claude"` / `"gemini"` |
| `providers.<name>.api_key` | `string` | ✓ | 各ProviderのAPIキー |
| `providers.<name>.model` | `string` | - | 使用するモデル名。省略時はデフォルト値を使用 |

## 設定例

### 単一Provider（最小構成）

```json
{
  "default_providers": ["claude"],
  "providers": {
    "claude": {
      "api_key": "sk-ant-..."
    }
  }
}
```

### 複数Provider（全部入り）

```json
{
  "default_providers": ["openai", "claude"],
  "providers": {
    "openai": {
      "api_key": "sk-...",
      "model": "gpt-4o-mini"
    },
    "claude": {
      "api_key": "sk-ant-...",
      "model": "claude-haiku-4-5"
    },
    "gemini": {
      "api_key": "AIza...",
      "model": "gemini-2.0-flash"
    }
  }
}
```

この場合の出力形式:

```json
{
  "openai": "LLM is ...",
  "claude": "LLM stands for ..."
}
```

## 履歴ファイル

直前の回答は以下に保存される（`--last` オプション用）:

```
~/.config/qql/history.json
```

```jsonc
{
  "question": "what is LLM?",
  "answer": {
    "openai": "LLM is ...",
    "claude": "LLM stands for ..."
  },
  "providers": ["openai", "claude"],
  "timestamp": "2026-04-03T12:00:00Z"
}
```
