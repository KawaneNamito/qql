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
