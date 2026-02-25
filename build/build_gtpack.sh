#!/usr/bin/env bash
set -euo pipefail

mkdir -p build/dist
cargo run --quiet --bin gtpack_tool -- build --out build/dist/routing-ingress-control-chain.gtpack
