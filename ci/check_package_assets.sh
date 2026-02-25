#!/usr/bin/env bash
set -euo pipefail

if [ "${1:-}" = "" ]; then
  echo "usage: $0 <crate-name>" >&2
  exit 2
fi

crate="$1"
file_list="$(cargo package -p "$crate" --allow-dirty --list)"

assert_list_has() {
  pattern="$1"
  message="$2"
  if ! printf '%s\n' "$file_list" | grep -Eq "$pattern"; then
    echo "package asset check failed for crate '$crate': $message" >&2
    exit 1
  fi
}

assert_list_has '^Cargo.toml$' "missing Cargo.toml in packaged output"
assert_list_has '^README' "missing README in packaged output"
assert_list_has '^LICENSE' "missing LICENSE in packaged output"
assert_list_has '^src/.+\.rs$' "missing Rust source files in packaged output"

if [ -n "${REQUIRED_PACKAGE_ASSET_PATTERNS:-}" ]; then
  old_ifs="$IFS"
  IFS=','
  for pattern in $REQUIRED_PACKAGE_ASSET_PATTERNS; do
    assert_list_has "$pattern" "required runtime asset pattern '$pattern' not found"
  done
  IFS="$old_ifs"
fi

echo "package asset check passed for crate '$crate'"
