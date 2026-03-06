#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
greentic_pack_bin="${GREENTIC_PACK_BIN:-greentic-pack}"

answers_file="$repo_root/build/wizard/control-pack.answers.json"

# Reset component placeholder directory so replay output stays deterministic.
rm -rf "$repo_root/pack/components/controller"
"$greentic_pack_bin" wizard apply --answers "$answers_file"
# Clean unresolved template-path artifacts emitted by some wizard catalog versions.
find "$repo_root/pack" -name '*{{*' -print0 | xargs -0 rm -rf 2>/dev/null || true

if [ "${GREENTIC_SKIP_MATERIALIZE:-0}" != "1" ]; then
  bash "$repo_root/build/materialize_control_placeholders.sh"
fi

# Ensure pack.yaml component list is derived from pack/components only.
"$greentic_pack_bin" components --in "$repo_root/pack"

"$greentic_pack_bin" doctor --in "$repo_root/pack"
"$greentic_pack_bin" build --in "$repo_root/pack"

mkdir -p "$repo_root/dist"
pack_gtpack="$(ls "$repo_root"/pack/dist/*.gtpack | head -n1)"
cp "$pack_gtpack" "$repo_root/dist/routing-ingress-control-chain.gtpack"
