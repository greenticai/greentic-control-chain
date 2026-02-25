# greentic-control-chain

`greentic-control-chain` is a Rust crate for Greentic control-chain functionality.

## Hook Pack Scaffold

The repo includes minimal gtpack scaffolding under `pack/` and a builder:

```bash
bash build/build_gtpack.sh
```

This produces:
- `build/dist/routing-ingress-control-chain.gtpack`

The generated `pack.cbor` publishes a single canonical hook offer:
- `kind=hook`
- `stage=post_ingress`
- `contract=greentic.hook.control.v1`
- `provider.op=ingress_control.handle`

`meta.cap_id=greentic.cap.ingress.control.v1` is metadata-only (cataloging); operator selection must use `(kind, stage, contract)`.

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

### Pack invoke/build helpers

- Build gtpack:

```bash
bash build/build_gtpack.sh
```

- Invoke minimal handler (reads CBOR map from stdin, writes CBOR directive to stdout):

```bash
cargo run --bin ingress_control_handle
```

### GitHub Actions

- `.github/workflows/ci.yml`
  - Runs on pull requests, branch pushes (`main`/`master`), tag pushes (`v*`), and `workflow_dispatch`.
  - Splits validation into parallel jobs: `lint`, `test`.
  - Runs `publish` only on tag pushes (`v*`) and only after `lint` and `test` succeed.
  - `publish` verifies tag/version match, publishes crates to crates.io, and publishes `.gtpack` bundles as OCI artifacts to GHCR.

### How to cut a release

1. Bump `version` in `Cargo.toml`.
2. Commit and push to your main branch.
3. Create and push a matching tag:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

The publish workflow fails if the tag does not match the crate version.

### Required repository secrets

- `CARGO_REGISTRY_TOKEN` for crates.io publishing.
- `GHCR_TOKEN` for GHCR publishing (optional if `GITHUB_TOKEN` has package write permission).
