# PR-RTR-CHAIN-01: Wizard-Managed Control Extension Pack (Capability-First)
Date: 2026-03-06

## Scope
- Replace hand-rolled pack artifacts with wizard-managed `pack.yaml` + extension files.
- Build `dist/routing-ingress-control-chain.gtpack` from wizard replay + materialization.
- Keep capability-first control offer canonical (`greentic.ext.capabilities.v1`).
- Provide a real invokable controller wasm and setup QA payload.
- Add runtime/integration tests for component ops and setup path.

## Current Canonical Contract
Control capability is published in:
- `pack/extensions/control.json`
- `pack/pack.yaml` under `extensions.greentic.ext.capabilities.v1.inline`

Offer fields:
- `cap_id: greentic.cap.ingress.control.v1`
- `offer_id: control-chain.post-ingress`
- `provider.component_ref: controller`
- `provider.op: ingress_control.handle`
- `requires_setup: true`
- `setup.qa_ref: qa/control-setup.json`
- `version: v1`

## Build Flow
`bash build/build_gtpack.sh` now does:
1. `greentic-pack wizard apply --answers build/wizard/control-pack.answers.json`
2. `bash build/materialize_control_placeholders.sh`
3. `greentic-pack components --in ./pack` (deterministic component sync)
4. `greentic-pack doctor --in ./pack`
5. `greentic-pack build --in ./pack`
6. copy `pack/dist/*.gtpack` to `dist/routing-ingress-control-chain.gtpack`

## Materialization Behavior
- Component source scaffold is created with `greentic-component new` at:
  - `component-src/controller`
- Built wasm is copied into canonical pack component path:
  - `pack/components/controller/component.wasm`
- Setup QA is generated with `greentic-qa generate` into:
  - `pack/qa/control-setup.json`
- Legacy `pack/components/controller-src` is removed to avoid duplicate component discovery.

## Tests
- Unit/runtime tests:
  - `tests/ingress_control_runtime.rs`
- CLI integration tests invoking built wasm via `greentic-component test`:
  - `tests/controller_component_cli.rs`
  - validates:
    - `ingress_control.handle` refund dispatch
    - invalid `explicit_path` fallback behavior
    - extension setup wiring (`requires_setup` + `qa_ref`)
    - `qa-spec` and `apply-answers` setup lifecycle behavior

## Acceptance Criteria
- `dist/routing-ingress-control-chain.gtpack` is produced via wizard-first build path.
- Pack publishes canonical capability offer for control (`greentic.ext.capabilities.v1`).
- Controller wasm is real and invokable for `ingress_control.handle`.
- Setup QA artifact exists and is referenced by offer setup metadata.
- `cargo test --all-features` and `bash ci/local_check.sh` pass.
