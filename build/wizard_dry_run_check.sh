#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
greentic_pack_bin="${GREENTIC_PACK_BIN:-greentic-pack}"

if [ -n "${GREENTIC_COMPONENT_BIN:-}" ]; then
  greentic_component_bin="$GREENTIC_COMPONENT_BIN"
elif [ -x "$repo_root/../greentic-component/target/debug/greentic-component" ]; then
  greentic_component_bin="$repo_root/../greentic-component/target/debug/greentic-component"
else
  greentic_component_bin="greentic-component"
fi

tmpdir="$(mktemp -d /tmp/greentic-control-chain-wizard-check-XXXXXX)"
trap 'rm -rf "$tmpdir"' EXIT

pack_answers="$tmpdir/pack.answers.json"
component_answers="$tmpdir/component.answers.json"
component_plan="$tmpdir/component.plan.json"

printf '0\n' | "$greentic_pack_bin" wizard run --dry-run --emit-answers "$pack_answers" >/dev/null
"$greentic_component_bin" wizard --dry-run --plan-out "$component_plan" --emit-answers "$component_answers" >/dev/null

echo "wizard dry-run check passed"
echo "pack answers: $pack_answers"
echo "component answers: $component_answers"
echo "component plan: $component_plan"
