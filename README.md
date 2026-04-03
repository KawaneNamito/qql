# qql

Quick Question for LLM

Japanese README: [README.ja.md](./README.ja.md)

`qql` is a small CLI tool for sending short questions to LLMs from the terminal.

## Features

- Ask questions with OpenAI, Claude, or Gemini
- Configure one or more default providers
- Always emit provider-keyed JSON, even for a single provider
- Replay the last answer with `--last`
- Create the config file interactively with `qql init`

## Build

```sh
cargo build --release
```

Generated binary:

```sh
./target/release/qql
```

## Quick Start

Create your config on first run:

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

After that, you can ask a question directly:

```sh
./target/release/qql "what is LLM?"
```

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

Replay the most recent answer:

```sh
qql --last
```

## Output

Output is always JSON. Even a single-provider response is returned as an object keyed by provider name.

```json
{
  "openai": "LLM is ...",
  "claude": "LLM stands for ..."
}
```

Single-provider example:

```json
{
  "claude": "LLM stands for Large Language Model."
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
qq --last
```

`glow` runs only once, so multi-provider responses can be compared in one continuous scrollable view.

## Config Files

Config file:

```txt
~/.config/qql/config.json
```

If `$XDG_CONFIG_HOME` is set, `qql` follows the XDG Base Directory spec and uses that instead.

History file:

```txt
~/.config/qql/history.json
```

## Example Config

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

- `openai`
- `claude`
- `gemini`

## Development

```sh
cargo run -- init
cargo run -- "what is LLM?"
cargo run -- --last
```
