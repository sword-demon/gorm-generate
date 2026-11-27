#!/usr/bin/env bash
# Fmt ratchet gate (design.md §7): a tracked .rs file must be rustfmt-clean
# unless it is still listed in rustfmt-exceptions.txt. The list may only
# shrink — a listed file that has become clean fails the gate, so an entry is
# always removed together with the fix.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
exceptions="rustfmt-exceptions.txt"
status=0

if [ -f "$exceptions" ]; then
  entries="$(grep -v '^[[:space:]]*$' "$exceptions" \
    | grep -v '^[[:space:]]*#' \
    | sed 's/[[:space:]]*$//')"
else
  entries=""
fi

is_exempt() {
  [ -n "$entries" ] && printf '%s\n' "$entries" | grep -qxF -- "$1"
}

clean() {
  rustfmt --edition 2021 --config skip_children=true --check "$1" >/dev/null 2>&1
}

for f in $(git ls-files '*.rs'); do
  if is_exempt "$f"; then
    continue
  fi
  if ! clean "$f"; then
    echo "::error file=$f::not rustfmt-clean; format it or add it to $exceptions"
    rustfmt --edition 2021 --config skip_children=true --check "$f" 2>/dev/null | head -40 || true
    status=1
  fi
done

if [ -n "$entries" ]; then
  while IFS= read -r f; do
    if [ ! -f "$f" ]; then
      echo "::error::$exceptions lists a missing file: $f"
      status=1
    elif clean "$f"; then
      echo "::error file=$f::already rustfmt-clean; remove it from $exceptions"
      status=1
    fi
  done <<< "$entries"
fi

exit "$status"
