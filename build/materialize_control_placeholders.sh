#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pack_dir="$repo_root/pack"
component_ref="controller"
component_dir="$pack_dir/components/$component_ref"
component_src_dir="$repo_root/component-src/$component_ref"
legacy_component_src_dir="$pack_dir/components/${component_ref}-src"
qa_dir="$pack_dir/qa"
require_real_wasm="${GREENTIC_REQUIRE_REAL_WASM:-1}"

if [ -n "${GREENTIC_COMPONENT_BIN:-}" ]; then
  greentic_component_bin="$GREENTIC_COMPONENT_BIN"
elif [ -x "$repo_root/../greentic-component/target/debug/greentic-component" ]; then
  greentic_component_bin="$repo_root/../greentic-component/target/debug/greentic-component"
else
  greentic_component_bin="greentic-component"
fi

if [ -n "${GREENTIC_QA_BIN:-}" ]; then
  greentic_qa_bin="$GREENTIC_QA_BIN"
elif [ -x "$repo_root/../greentic-qa/target/debug/greentic-qa" ]; then
  greentic_qa_bin="$repo_root/../greentic-qa/target/debug/greentic-qa"
else
  greentic_qa_bin="greentic-qa"
fi

tmpdir="$(mktemp -d /tmp/greentic-control-chain-materialize-XXXXXX)"
trap 'rm -rf "$tmpdir"' EXIT
component_scaffold_dir="$tmpdir/$component_ref"

mkdir -p "$pack_dir/components"
mkdir -p "$repo_root/component-src"

# Keep pack component discovery deterministic: source scaffolds live outside pack/components.
if [ -d "$legacy_component_src_dir" ]; then
  rm -rf "$legacy_component_src_dir"
fi

component_new_exit=0
if [ ! -f "$component_src_dir/component.manifest.json" ] || [ "${GREENTIC_FORCE_SCAFFOLD:-0}" = "1" ]; then
  set +e
  "$greentic_component_bin" new \
    --name "$component_ref" \
    --path "$component_scaffold_dir" \
    --non-interactive \
    --no-git
  component_new_exit=$?
  set -e

  if [ -d "$component_scaffold_dir" ]; then
    mkdir -p "$component_src_dir"
    if command -v rsync >/dev/null 2>&1; then
      rsync -a --delete --exclude target/ "$component_scaffold_dir"/ "$component_src_dir"/
    else
      find "$component_src_dir" -mindepth 1 -maxdepth 1 ! -name target -exec rm -rf {} +
      cp -R "$component_scaffold_dir"/. "$component_src_dir"/
    fi
  else
    echo "warning: component scaffold directory was not created."
  fi
else
  echo "component source scaffold already exists at $component_src_dir; skipping re-scaffold."
fi

if [ ! -f "$component_dir/component.wasm" ]; then
  # Keep pack buildable until a real wasm is produced from the scaffolded project.
  printf '\x00\x61\x73\x6d\x01\x00\x00\x00' > "$component_dir/component.wasm"
  echo "warning: component.wasm missing in pack component; wrote wasm header placeholder."
fi

if [ "$component_new_exit" -ne 0 ]; then
  echo "warning: greentic-component new returned non-zero (likely dependency/network check)."
fi

if command -v cargo >/dev/null 2>&1 && [ -d "$component_src_dir" ] && cargo component --version >/dev/null 2>&1; then
  set +e
  (
    cd "$component_src_dir"
    cargo component build --release --target wasm32-wasip2 >/dev/null
  )
  build_exit=$?
  set -e
  if [ "$build_exit" -eq 0 ] && [ -f "$component_src_dir/target/wasm32-wasip2/release/controller.wasm" ]; then
    cp "$component_src_dir/target/wasm32-wasip2/release/controller.wasm" "$component_dir/component.wasm"
    echo "updated pack component wasm from controller-src build output."
  else
    echo "warning: controller-src wasm build did not produce a usable artifact; keeping current pack wasm."
  fi
else
  echo "warning: cargo component not available; skipping controller-src wasm build."
fi

mkdir -p "$qa_dir"
"$greentic_qa_bin" generate \
  --input "$repo_root/build/wizard/control-qa.input.json" \
  --out "$tmpdir" \
  --force >/dev/null

cp "$tmpdir/control-setup/forms/control-setup.form.json" "$qa_dir/control-setup.json"
echo "materialized control QA at $qa_dir/control-setup.json"
echo "component source scaffold refreshed at $component_src_dir"

if [ "$require_real_wasm" = "1" ]; then
  if [ ! -f "$component_dir/component.wasm" ]; then
    echo "error: missing $component_dir/component.wasm; real wasm required."
    exit 1
  fi

  wasm_size="$(wc -c < "$component_dir/component.wasm" | tr -d '[:space:]')"
  if [ "$wasm_size" -le 8 ]; then
    echo "error: controller wasm appears to be placeholder-only (${wasm_size} bytes)."
    echo "error: real wasm required. Ensure cargo component build succeeds."
    exit 1
  fi
fi
