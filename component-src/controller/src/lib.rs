#[cfg(target_arch = "wasm32")]
use std::collections::BTreeMap;

#[cfg(target_arch = "wasm32")]
use greentic_interfaces_guest::component_v0_6::node;
#[cfg(target_arch = "wasm32")]
use greentic_types::cbor::canonical;
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::common::schema_ir::{AdditionalProperties, SchemaIr};
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::component::v0_6_0::{ComponentInfo, I18nText};

// i18n: runtime lookup + embedded CBOR bundle helpers.
pub mod i18n;
pub mod i18n_bundle;
// qa: mode normalization, QA spec generation, apply-answers validation.
pub mod qa;

const COMPONENT_NAME: &str = "controller";
const COMPONENT_ORG: &str = "ai.greentic";
const COMPONENT_VERSION: &str = "0.1.0";

#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = ".greentic.wasi")]
static WASI_TARGET_MARKER: [u8; 13] = *b"wasm32-wasip2";

#[cfg(target_arch = "wasm32")]
struct Component;

#[cfg(target_arch = "wasm32")]
impl node::Guest for Component {
    // Component metadata advertised to host/operator tooling.
    // Extend here when you add more operations or capability declarations.
    fn describe() -> node::ComponentDescriptor {
        let input_schema_cbor = input_schema_cbor();
        let output_schema_cbor = output_schema_cbor();
        node::ComponentDescriptor {
            name: COMPONENT_NAME.to_string(),
            version: COMPONENT_VERSION.to_string(),
            summary: Some(format!("Greentic component {COMPONENT_NAME}")),
            capabilities: Vec::new(),
            ops: vec![
                node::Op {
                    name: "ingress_control.handle".to_string(),
                    summary: Some("Evaluate ingress control and emit directive".to_string()),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(input_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(output_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    examples: Vec::new(),
                },
                node::Op {
                    name: "qa-spec".to_string(),
                    summary: Some("Return QA spec (CBOR) for a requested mode".to_string()),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(input_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(output_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    examples: Vec::new(),
                },
                node::Op {
                    name: "apply-answers".to_string(),
                    summary: Some("Apply QA answers and optionally return config override".to_string()),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(input_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(output_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    examples: Vec::new(),
                },
                node::Op {
                    name: "i18n-keys".to_string(),
                    summary: Some("Return i18n keys referenced by QA/setup".to_string()),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(input_schema_cbor),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(output_schema_cbor),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    examples: Vec::new(),
                },
            ],
            schemas: Vec::new(),
            setup: None,
        }
    }

    // Single ABI entrypoint. Keep this dispatcher model intact.
    // Extend behavior by adding/adjusting operation branches in `run_component_cbor`.
    fn invoke(
        operation: String,
        envelope: node::InvocationEnvelope,
    ) -> Result<node::InvocationResult, node::NodeError> {
        let output = run_component_cbor(&operation, envelope.payload_cbor);
        Ok(node::InvocationResult {
            ok: true,
            output_cbor: output,
            output_metadata_cbor: None,
        })
    }
}

#[cfg(target_arch = "wasm32")]
greentic_interfaces_guest::export_component_v060!(Component);

// Host-side metadata helper used by tooling/tests outside wasm runtime.
pub fn describe_payload() -> String {
    serde_json::json!({
        "component": {
            "name": COMPONENT_NAME,
            "org": COMPONENT_ORG,
            "version": COMPONENT_VERSION,
            "world": "greentic:component/component@0.6.0",
            "schemas": {
                "component": "schemas/component.schema.json",
                "input": "schemas/io/input.schema.json",
                "output": "schemas/io/output.schema.json"
            }
        }
    })
    .to_string()
}

pub fn ingress_control_handle(value: &serde_json::Value) -> serde_json::Value {
    if let Some(path) = value.get("explicit_path").and_then(|v| v.as_str()) {
        return match parse_dispatch_target(path) {
            Ok((pack, flow, node)) => serde_json::json!({
                "v": 1,
                "action": "dispatch",
                "dispatch": { "pack": pack, "flow": flow, "node": node },
                "diag": { "stage": 0, "explicit_path_valid": true, "allow_llm": false }
            }),
            Err(err) => serde_json::json!({
                "v": 1,
                "action": "continue",
                "diag": {
                    "stage": 0,
                    "explicit_path_valid": false,
                    "allow_llm": false,
                    "errors": [{ "code": "invalid_explicit_path", "msg": err }]
                }
            }),
        };
    }

    let text = value
        .get("message")
        .and_then(|m| m.get("text"))
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
        .to_lowercase();

    if text.contains("refund") {
        return serde_json::json!({
            "v": 1,
            "action": "dispatch",
            "dispatch": { "pack": "commerce-support", "flow": "flow_refund" },
            "diag": { "stage": 1, "matched_rule_id": "keyword.refund", "allow_llm": false }
        });
    }

    serde_json::json!({
        "v": 1,
        "action": "continue",
        "diag": { "stage": 1, "allow_llm": false }
    })
}

fn parse_dispatch_target(target: &str) -> Result<(String, Option<String>, Option<String>), String> {
    if target.contains(char::is_whitespace)
        || target.contains("..")
        || target.contains('?')
        || target.contains('#')
    {
        return Err("target contains forbidden characters".to_string());
    }
    let parts: Vec<&str> = target.split('/').collect();
    if parts.is_empty() || parts.len() > 3 {
        return Err("target must have 1 to 3 segments".to_string());
    }
    for part in &parts {
        if part.is_empty() {
            return Err("target contains empty segment".to_string());
        }
        if !valid_segment(part) {
            return Err(format!("invalid segment '{part}'"));
        }
    }
    Ok((
        parts[0].to_string(),
        parts.get(1).map(|s| (*s).to_string()),
        parts.get(2).map(|s| (*s).to_string()),
    ))
}

fn valid_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    if segment.len() > 64 {
        return false;
    }
    chars.all(|ch| {
        ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '.' || ch == '_' || ch == '-'
    })
}

#[cfg(target_arch = "wasm32")]
fn encode_cbor<T: serde::Serialize>(value: &T) -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(value).expect("encode cbor")
}

#[cfg(target_arch = "wasm32")]
// Accept canonical CBOR first, then fall back to JSON for local debugging.
fn parse_payload(input: &[u8]) -> serde_json::Value {
    if let Ok(value) = canonical::from_cbor(input) {
        return value;
    }
    serde_json::from_slice(input).unwrap_or_else(|_| serde_json::json!({}))
}

#[cfg(target_arch = "wasm32")]
// Keep ingress compatibility: default/setup/install -> setup, update/upgrade -> update.
fn normalized_mode(payload: &serde_json::Value) -> qa::NormalizedMode {
    let mode = payload
        .get("mode")
        .and_then(|v| v.as_str())
        .or_else(|| payload.get("operation").and_then(|v| v.as_str()))
        .unwrap_or("setup");
    qa::normalize_mode(mode).unwrap_or(qa::NormalizedMode::Setup)
}

#[cfg(target_arch = "wasm32")]
// Minimal schema for generic operation input.
// Extend these schemas when you harden operation contracts.
fn input_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([(
            "input".to_string(),
            SchemaIr::String {
                min_len: Some(0),
                max_len: None,
                regex: None,
                format: None,
            },
        )]),
        required: vec!["input".to_string()],
        additional: AdditionalProperties::Allow,
    }
}

#[cfg(target_arch = "wasm32")]
fn output_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([(
            "message".to_string(),
            SchemaIr::String {
                min_len: Some(0),
                max_len: None,
                regex: None,
                format: None,
            },
        )]),
        required: vec!["message".to_string()],
        additional: AdditionalProperties::Allow,
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
fn config_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::new(),
        required: Vec::new(),
        additional: AdditionalProperties::Forbid,
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
fn component_info() -> ComponentInfo {
    ComponentInfo {
        id: format!("{COMPONENT_ORG}.{COMPONENT_NAME}"),
        version: COMPONENT_VERSION.to_string(),
        role: "tool".to_string(),
        display_name: Some(I18nText::new(
            "component.display_name",
            Some(COMPONENT_NAME.to_string()),
        )),
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
fn component_info_cbor() -> Vec<u8> {
    encode_cbor(&component_info())
}

#[cfg(target_arch = "wasm32")]
fn input_schema_cbor() -> Vec<u8> {
    encode_cbor(&input_schema())
}

#[cfg(target_arch = "wasm32")]
fn output_schema_cbor() -> Vec<u8> {
    encode_cbor(&output_schema())
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
fn config_schema_cbor() -> Vec<u8> {
    encode_cbor(&config_schema())
}

#[cfg(target_arch = "wasm32")]
// Central operation dispatcher.
// This is the primary extension point for new operations.
fn run_component_cbor(operation: &str, input: Vec<u8>) -> Vec<u8> {
    let value = parse_payload(&input);
    let output = match operation {
        "qa-spec" => {
            let mode = normalized_mode(&value);
            qa::qa_spec_json(mode)
        }
        "apply-answers" => {
            let mode = normalized_mode(&value);
            qa::apply_answers(mode, &value)
        }
        "i18n-keys" => serde_json::Value::Array(
            qa::i18n_keys()
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        ),
        "ingress_control.handle" => ingress_control_handle(&value),
        _ => ingress_control_handle(&value),
    };

    encode_cbor(&output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_payload_is_json() {
        let payload = describe_payload();
        let json: serde_json::Value = serde_json::from_str(&payload).expect("valid json");
        assert_eq!(json["component"]["name"], "controller");
    }

    #[test]
    fn explicit_path_dispatches() {
        let input = serde_json::json!({ "explicit_path": "commerce-support/flow_refund" });
        let out = ingress_control_handle(&input);
        assert_eq!(out["action"], "dispatch");
        assert_eq!(out["dispatch"]["pack"], "commerce-support");
    }

    #[test]
    fn keyword_dispatches_refund_flow() {
        let input = serde_json::json!({ "message": { "text": "refund needed" } });
        let out = ingress_control_handle(&input);
        assert_eq!(out["action"], "dispatch");
        assert_eq!(out["dispatch"]["flow"], "flow_refund");
    }
}
