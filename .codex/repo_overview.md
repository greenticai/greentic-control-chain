# Repository Overview

## 1. Purpose
`greentic-control-chain` provides:
- a Rust crate with ingress-control directive logic (`src/ingress_control.rs`)
- a wizard-managed control extension pack under `pack/`
- build scripts that produce `dist/routing-ingress-control-chain.gtpack`

The pack now uses capability-extension metadata (`greentic.ext.capabilities.v1`), not legacy
`pack.cbor` hook-offer authoring.

## 2. Main Areas
- `src/ingress_control.rs`
  - Stage 0/1 directive logic (`dispatch`/`respond`/`continue`/`deny`)
  - policy/rules asset parsing and validation
- `pack/`
  - generated pack files (`pack.yaml`, `extensions/control.json`, `qa/control-setup.json`)
  - packaged component artifact at `pack/components/controller/component.wasm`
- `component-src/controller/`
  - source scaffold for the controller component
  - implements `ingress_control.handle`, `qa-spec`, `apply-answers`, `i18n-keys`
- `build/build_gtpack.sh`
  - replays wizard answers, materializes placeholders, syncs components, runs doctor/build
- `build/materialize_control_placeholders.sh`
  - scaffolds component source if missing
  - builds real wasm via `cargo component`
  - generates control setup QA via `greentic-qa`
- `tests/controller_component_cli.rs`
  - end-to-end `greentic-component test` checks against built wasm
- `tests/ingress_control_runtime.rs`
  - runtime/unit tests for deterministic control behavior

## 3. Contract Snapshot
Canonical control offer is in `pack/extensions/control.json` and mirrored in `pack/pack.yaml`:
- `cap_id: greentic.cap.ingress.control.v1`
- `offer_id: control-chain.post-ingress`
- `provider.component_ref: controller`
- `provider.op: ingress_control.handle`
- `requires_setup: true`
- `setup.qa_ref: qa/control-setup.json`

## 4. Validation Entry Points
- `bash build/build_gtpack.sh`
- `bash ci/local_check.sh`

Both are currently green in this checkout.
