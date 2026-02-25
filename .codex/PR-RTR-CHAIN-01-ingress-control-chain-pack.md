# PR-RTR-CHAIN-01: greentic-control-chain gtpack scaffold + hook offer (post_ingress control v1)
Date: 2026-02-25

## Scope
Unified scope for former PR-01 + PR-02 in this repo:
- Create missing gtpack scaffolding and packaging path.
- Publish canonical hook offer in `pack.cbor`.
- Provide invokable `ingress_control.handle` with Stage 0/1 routing behavior.
- Keep implementation production-shaped: versioned directives, strict asset validation, policy guardrails.

## Canonical discovery contract
Operator selection is canonical by:
- `offer.kind == "hook"`
- `offer.stage == "post_ingress"`
- `offer.contract == "greentic.hook.control.v1"`

`meta.cap_id = "greentic.cap.ingress.control.v1"` is metadata only.
Operator must never select by `cap_id`.

## Pack scaffold
Expected layout:

```text
pack/
  pack.cbor
  assets/
    rules.cbor   (optional)
    policy.cbor  (optional)
```

Build output:
- `.gtpack` zip that contains `pack.cbor` (+ optional CBOR assets when present).

## `pack.cbor` offer (single functional offer)
`offers[]` contains one hook offer:
- `id: "control-chain.post-ingress"`
- `kind: "hook"`
- `stage: "post_ingress"`
- `contract: "greentic.hook.control.v1"`
- `priority: 10`
- `provider.op: "ingress_control.handle"`
- `meta.cap_id: "greentic.cap.ingress.control.v1"` (optional metadata)

No second capability offer entry is emitted.

## Runtime behavior in this unified PR
- Runtime assets are CBOR-only (`rules.cbor`, `policy.cbor`) in production shape.
- `policy.cbor` is optional; if missing:
  - `allow_respond=false`
  - `allow_llm=false`
- `rules.cbor` is optional; if missing:
  - Stage 1 is no-op and handler returns `continue`.
- Stage 0 supports `explicit_path` and validates strict `pack[/flow[/node]]` grammar.
  - Invalid `explicit_path` returns `continue` + diagnostics.
- Stage 1 loads ordered rules and applies first-match semantics:
  - `keyword` / `regex` matching
  - `dispatch`, `respond`, `continue`, `deny` outcomes
- Policy gating:
  - `respond` action downgrades to `continue` when `allow_respond=false` with `diag.policy_blocked="respond"`.
- Invalid assets fail visibly:
  - invalid rules => `deny code=invalid_rules_asset`
  - invalid policy => `deny code=invalid_policy_asset`

## Schemas (v1)

### `assets/rules.cbor`
```cbor
{
  "v": 1,
  "rules": [ RuleV1... ]
}
```

### `assets/policy.cbor`
```cbor
{
  "v": 1,
  "allow_respond": bool,
  "allow_llm": bool
}
```

### `result_cbor` directive envelope
```cbor
{
  "v": 1,
  "action": "dispatch" | "respond" | "continue" | "deny",
  "dispatch": { "pack":"...", "flow":"..."?, "node":"..."? }?,
  "respond":  { "text":"...", "needs_user": bool }?,
  "deny":     { "code":"...", "reason":"...", "details": map? }?,
  "diag": {
    "stage": 0 | 1,
    "matched_rule_id": "..."?,
    "explicit_path_valid": bool?,
    "policy_blocked": "respond"?,
    "allow_llm": bool?,
    "errors": [ { "code":"...", "msg":"..." } ]?
  }?
}
```

## Implementation plan
1. Add pack scaffold and build tooling to produce `.gtpack` zip with `pack.cbor`.
2. Implement canonical single hook offer in `pack.cbor`.
3. Implement Stage 0/1 `ingress_control.handle` runtime with CBOR policy/rules loading and validation.
4. Add packaging test that builds zip, extracts `pack.cbor`, and asserts:
   - `kind=hook`
   - `stage=post_ingress`
   - `contract=greentic.hook.control.v1`
   - `provider.op=ingress_control.handle`
5. Add runtime tests for explicit path validation, missing/invalid assets, deterministic rule ordering, and policy gating.
6. Keep capability id metadata-only language in docs/spec; remove capability-based selection language.

## Compatibility notes
- Requires operator support for hook offers resolved by `(kind, stage, contract)`.
- Works with operators implementing `greentic.hook.control.v1`.

## Acceptance criteria
- Pack scaffold exists and builds a `.gtpack` zip containing `pack.cbor`.
- Hook offer is discoverable by operator using `(kind, stage, contract)`.
- Handler is invokable and returns versioned CBOR directive envelopes.
- Stage 0/1 behavior is covered by tests (including invalid asset and policy-gating cases).
- Packaging test validates required hook-offer fields from `pack.cbor`.
- `cap_id` remains metadata only and is not used for selection.
