#!/usr/bin/env bash
set -euo pipefail

step() {
  echo
  echo "==> $1"
}

step "cargo fmt --check"
cargo fmt --all -- --check

step "cargo clippy"
cargo clippy --all-targets --all-features -- -D warnings

step "cargo test"
cargo test --all-features

step "cargo build"
cargo build --all-features

step "build gtpack scaffold"
bash build/build_gtpack.sh

step "cargo doc"
cargo doc --no-deps --all-features

step "package + publish dry-run checks"
publishable_crates="$(bash ci/list_publishable_crates.sh)"
if [ -z "$publishable_crates" ]; then
  echo "No publishable crates detected."
  exit 0
fi

for crate in $publishable_crates; do
  echo
  echo "--- crate: $crate ---"
  if [ "${CI:-}" = "true" ]; then
    cargo package --no-verify -p "$crate"
    cargo package -p "$crate"
    cargo publish -p "$crate" --dry-run
  else
    cargo package --no-verify -p "$crate" --allow-dirty
    cargo package -p "$crate" --allow-dirty
    cargo publish -p "$crate" --dry-run --allow-dirty
  fi
  bash ci/check_package_assets.sh "$crate"
done
