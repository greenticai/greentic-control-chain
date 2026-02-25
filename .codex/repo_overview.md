# Repository Overview

## 1. High-Level Purpose
This repository hosts the `greentic-control-chain` Rust crate plus a production-shaped gtpack scaffold for ingress control hook publication. It now includes hook-manifest packaging (`pack.cbor` in `.gtpack`) and an invokable handler boundary with Stage 0/1 directive decisions in versioned CBOR.

The stack is Rust (edition 2024), shell scripts for local checks/build, and GitHub Actions workflows for CI and release publishing. Runtime now includes Stage 0/1 control-chain logic (explicit-path + rules/policy CBOR handling), with validation and policy-gating behavior.

## 2. Main Components and Functionality
- **Path:** `Cargo.toml`
- **Role:** Crate/package manifest for the repository.
- **Key functionality:**
  - Defines package metadata required for publishing (`license`, `repository`, `description`, `readme`, `include`).
  - Builds a single binary from `src/main.rs`.
- **Key dependencies / integration points:**
  - Used by local and GitHub CI checks for formatting, linting, tests, build, docs, and packaging dry-runs.

- **Path:** `src/main.rs`
- **Role:** Binary entrypoint.
- **Key functionality:**
  - Prints `Hello, world!`.
- **Key dependencies / integration points:**
  - No internal module graph yet.

- **Path:** `src/ingress_control.rs` and `src/bin/ingress_control_handle.rs`
- **Role:** Ingress control operation contract boundary and Stage 0/1 engine.
- **Key functionality:**
  - Loads optional `assets/policy.cbor` and `assets/rules.cbor` (CBOR-only runtime).
  - Enforces policy defaults when missing (`allow_respond=false`, `allow_llm=false`) and deny-on-invalid-asset behavior.
  - Handles Stage 0 explicit path routing with strict grammar validation.
  - Handles Stage 1 deterministic rule evaluation (keyword/regex, first-match wins).
  - Includes rule-level diagnostics in invalid rules denies (`deny.details.rule_id`) when a specific rule fails validation.
  - Emits versioned directive envelopes (`dispatch`, `respond`, `continue`, `deny`) with diagnostics.
  - CLI binary reads CBOR map from stdin and writes CBOR directive to stdout.
- **Key dependencies / integration points:**
  - Uses `serde_cbor` for runtime serialization and `regex` for rule regex matching.
  - Provider op published in manifest as `ingress_control.handle`.

- **Path:** `src/pack.rs`, `src/bin/gtpack_tool.rs`, `build/build_gtpack.sh`, `pack/`
- **Role:** Gtpack scaffold + build pipeline.
- **Key functionality:**
  - Defines canonical pack manifest with a single hook offer:
    - `kind=hook`, `stage=post_ingress`, `contract=greentic.hook.control.v1`
    - `provider.op=ingress_control.handle`
    - `meta.cap_id=greentic.cap.ingress.control.v1` (metadata only)
  - Generates `pack/pack.cbor` and builds `build/dist/routing-ingress-control-chain.gtpack`.
  - Includes optional asset directory (`pack/assets/`) and optional default policy CBOR generator.
- **Key dependencies / integration points:**
  - Uses `serde_cbor` to encode `pack.cbor`.
  - Uses `zip` crate to package `.gtpack`.

- **Path:** `tests/gtpack_packaging.rs`
- **Role:** Packaging contract verification tests.
- **Key functionality:**
  - Asserts canonical manifest has one hook offer with required fields.
  - Builds `.gtpack`, decodes `pack.cbor`, and verifies offer fields (`kind`, `stage`, `contract`, `provider.op`).
- **Key dependencies / integration points:**
  - Exercises `pack` builder APIs directly.

- **Path:** `tests/ingress_control_runtime.rs`
- **Role:** Stage 0/1 runtime behavior and error-policy tests.
- **Key functionality:**
  - Covers explicit-path valid/invalid handling.
  - Covers missing/invalid rules and policy assets.
  - Covers invalid regex and invalid dispatch target rule validation failures.
  - Covers deterministic first-match rule ordering.
  - Covers policy-gated respond downgrade to continue.
- **Key dependencies / integration points:**
  - Exercises `handle_with_assets(...)` directly using test CBOR fixtures.

- **Path:** `ci/local_check.sh`
- **Role:** Single developer CI entrypoint.
- **Key functionality:**
  - Runs `fmt`, `clippy`, `test`, `build`, `doc`.
  - Builds gtpack scaffold via `build/build_gtpack.sh`.
  - Detects publishable crates and runs `cargo package` and `cargo publish --dry-run`.
  - Invokes package asset validation helper.
- **Key dependencies / integration points:**
  - Uses `ci/list_publishable_crates.sh` and `ci/check_package_assets.sh`.

- **Path:** `.github/workflows/ci.yml`
- **Role:** Pull request and push validation pipeline.
- **Key functionality:**
  - Parallel jobs for linting, tests, and package/publish dry-run checks.
  - Uses Rust toolchain setup and Cargo caching.
- **Key dependencies / integration points:**
  - Reuses local CI expectations from scripts under `ci/`.

- **Path:** `.github/workflows/publish.yml`
- **Role:** Release/publish automation.
- **Key functionality:**
  - Validates tag/version match (`v<version>` from `Cargo.toml`).
  - Runs local checks before publish stages.
  - Publishes crates (with retry) and optionally publishes `.gtpack` files to GHCR as OCI artifacts.
- **Key dependencies / integration points:**
  - Requires `CARGO_REGISTRY_TOKEN`; uses `GHCR_TOKEN` or `GITHUB_TOKEN`.

## 3. Work In Progress, TODOs, and Stubs
- **Location:** `src/ingress_control.rs` (`ingress_control_handle`)
- **Status:** partial
- **Short description:** Stage 0/1 logic exists, but broader future chain behavior (external router chaining and expanded action semantics) is not yet implemented.

- **Location:** Repository-wide search
- **Status:** no explicit TODO markers found
- **Short description:** No `TODO`, `FIXME`, `XXX`, `HACK`, `unimplemented!`, or `todo!` markers were detected.

## 4. Broken, Failing, or Conflicting Areas
- **Location:** `ci/local_check.sh` publish dry-run step in network-restricted environments
- **Evidence:** `cargo publish --dry-run` requires crates.io index access and fails without DNS/network.
- **Likely cause / nature of issue:** Environment constraint, not logic error.

- **Location:** Full roadmap beyond Stage 0/1
- **Evidence:** Current implementation does not include external router invocation or advanced multi-step decision chaining.
- **Likely cause / nature of issue:** Out of scope for current unified PR, which focuses on deterministic local middleware behavior.

## 5. Notes for Future Work
- Add optional integration path for future router-chain/LLM steps guarded by `allow_llm`.
