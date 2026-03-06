#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
greentic_pack_bin="${GREENTIC_PACK_BIN:-greentic-pack}"

target_dir="${1:-./pack-wizard-control}"
display_name="${2:-Routing ingress control chain}"

mkdir -p "$(dirname "$target_dir")"

cat <<EOF | "$greentic_pack_bin" wizard run --emit-answers "${target_dir}.answers.json" >/dev/null
3

8
1
$target_dir
$display_name
routing.ingress.control.chain
control
1
control-chain.post-ingress
greentic.cap.ingress.control.v1
controller
ingress_control.handle
v1
10
2

0
EOF

echo "created control extension scaffold at: $target_dir"
echo "answers file: ${target_dir}.answers.json"
