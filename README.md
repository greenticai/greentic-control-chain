# greentic-control-chain

`greentic-control-chain` is a Rust crate for Greentic control-chain functionality.

## Hook Pack Scaffold

The repo uses wizard-managed pack sources under `pack/` and a builder:

```bash
bash build/build_gtpack.sh
```

This produces:
- `dist/routing-ingress-control-chain.gtpack`

The build flow is:
1. `greentic-pack wizard apply --answers build/wizard/control-pack.answers.json`
2. `bash build/materialize_control_placeholders.sh`
3. `greentic-pack doctor --in ./pack`
4. `greentic-pack build --in ./pack`
5. copy generated `pack/dist/*.gtpack` to `dist/routing-ingress-control-chain.gtpack`

`materialize_control_placeholders.sh` does two things:
- scaffolds real component source with `greentic-component new` into `component-src/controller`
- generates a real QA form with `greentic-qa generate` into `pack/qa/control-setup.json`

Note: in offline environments, `greentic-component new` may fail its internal `cargo check`; the script keeps pack buildable and prints a warning.
By default, packaging now requires a real controller wasm (`GREENTIC_REQUIRE_REAL_WASM=1`).
Set `GREENTIC_REQUIRE_REAL_WASM=0` only if you intentionally want to allow placeholder wasm during local iteration.

### Runtime behavior (current)

`ingress_control.handle` returns a versioned CBOR directive envelope (`v=1`) and currently implements:
- Stage 0: `explicit_path` validation and dispatch for `pack[/flow[/node]]`.
- Stage 1: optional `assets/rules.cbor` deterministic first-match routing.
- Optional `assets/policy.cbor` enforcement:
  - if missing, defaults are `allow_respond=false`, `allow_llm=false`
  - `respond` is downgraded to `continue` when policy disallows it.
- Invalid policy/rules assets return `deny` with explicit error codes.

## CI and Releases

### Local developer checks

Use the single local entrypoint:

```bash
bash ci/local_check.sh
```

This runs:

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all-features`
4. `cargo build --all-features`
5. `cargo doc --no-deps --all-features`
6. Packaging checks per publishable crate:
   - `cargo package --no-verify -p <crate>`
   - `cargo package -p <crate>`
   - `cargo publish -p <crate> --dry-run`
   - packaged asset verification
7. Wizard dry-run smoke checks:
   - `greentic-pack wizard run --dry-run --emit-answers ...`
   - `greentic-component wizard --dry-run --plan-out ... --emit-answers ...`

### Wizard-first workflow

- Run wizard dry-run checks in a temporary directory:

```bash
bash build/wizard_dry_run_check.sh
```

- Scaffold a control extension pack via the wizard menu (`3 -> 8`):

```bash
bash build/wizard_create_control_pack.sh ./pack
```

- Scaffold a component via wizard defaults:

```bash
greentic-component wizard --emit-answers ./component.answers.json
```

### Pack invoke/build helpers

- Build gtpack:

```bash
bash build/build_gtpack.sh
```

Artifact path:
- `dist/routing-ingress-control-chain.gtpack`

- Invoke minimal handler (reads CBOR map from stdin, writes CBOR directive to stdout):

```bash
cargo run --bin ingress_control_handle
```

### GitHub Actions

- `.github/workflows/ci.yml`
  - Runs on pull requests, branch pushes (`main`/`master`), and `workflow_dispatch`.
  - Splits validation into parallel jobs: `lint`, `test`.
  - Runs `publish` only on pushes to `master`, and only after `lint` and `test` succeed.
  - `publish` reads version from `Cargo.toml`, publishes crates to crates.io, and publishes `.gtpack` bundles as OCI artifacts to GHCR.

### How to cut a release

1. Bump `version` in `Cargo.toml`.
2. Commit and push to `master`.

```bash
git push origin master
```

### Required repository secrets

- `CARGO_REGISTRY_TOKEN` for crates.io publishing.
- `GHCR_TOKEN` for GHCR publishing (optional if `GITHUB_TOKEN` has package write permission).

## Wizard Binary

Build scripts use `greentic-pack` from PATH by default.
Override only when needed:
- `GREENTIC_PACK_BIN=<path-to-greentic-pack>`
