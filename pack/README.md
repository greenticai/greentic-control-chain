# Pack Scaffold

This directory contains the control-chain gtpack scaffold.

- `pack.cbor` is generated/updated by `build/build_gtpack.sh` (or `gtpack_tool build`).
- `assets/rules.cbor` is optional.
- `assets/policy.cbor` is optional. If absent, runtime defaults apply:
  - `allow_respond=false`
  - `allow_llm=false`
