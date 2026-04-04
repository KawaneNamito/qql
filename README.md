# qql

Quick Question for LLM

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/built_with-Rust-orange?logo=rust)
![Providers](https://img.shields.io/badge/providers-OpenAI%20%7C%20Claude%20%7C%20Gemini-blueviolet)

Japanese README: [README.ja.md](./README.ja.md)

`qql` is a small CLI tool for sending short questions to LLMs from the terminal.

Inspired by [llm](https://github.com/simonw/llm). I wanted a simpler, dependency-free alternative that outputs structured JSON and stays close to the metal.

![demo](./docs/demo.gif)

## Features

- Ask questions with OpenAI, Claude, or Gemini
- Configure one or more default providers
- Always emit provider-keyed JSON, even for a single provider
- Replay the last answer with `--last`
- Create the config file interactively with `qql init`

## Installation

```sh
cargo build --release
```

Then run the generated binary:

```sh
./target/release/qql init
```

`qql init` walks you through:

- Selecting providers to configure
- Pasting API keys
- Choosing from available models fetched at runtime
- Entering a custom model when needed
- Confirming overwrite if a config file already exists

Model lists are fetched at runtime using the API key you enter. If fetching fails, `qql` falls back to a built-in model list.

## Usage

Ask using the default provider set:

```sh
qql "what is LLM?"
```

Ask with explicit providers:

```sh
qql -p claude "what is LLM?"
qql -p openai -p gemini "what is LLM?"
```

Compose the question in your editor:

```sh
qql --editor
qql --editor "draft prompt"
```

Replay the most recent answer:

```sh
qql --last
```

`--editor` opens `$VISUAL`, then `$EDITOR`, and falls back to `vi` if neither is set. If you pass a positional argument with `--editor`, it is used as the initial draft text.

## Output

Output is always JSON, keyed by provider name.

```json
{
  "openai": "LLM is ...",
  "claude": "LLM stands for ..."
}
```

If you want prettier terminal output, wrap `qql` with a `qq` shell function:

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

Add it to `~/.zshrc` or `~/.bashrc`, reload your shell, and use:

```sh
qq "what is LLM?"
qq -p openai -p claude "what is LLM?"
qq --editor
qq --last
```

`glow` runs only once, so multi-provider responses can be compared in one continuous scrollable view.

## Config

Config file: `~/.config/qql/config.json`  
History file: `~/.config/qql/history.json`

If `$XDG_CONFIG_HOME` is set, `qql` follows the XDG Base Directory spec.

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

## Supported Providers

| Provider | Value |
|---|---|
| OpenAI | `openai` |
| Claude (Anthropic) | `claude` |
| Gemini (Google) | `gemini` |

## Development

```sh
cargo run -- init
cargo run -- "what is LLM?"
cargo run -- --last
```
