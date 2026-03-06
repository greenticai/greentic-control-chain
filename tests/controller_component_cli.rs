use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn controller_wasm() -> PathBuf {
    repo_root().join("pack/components/controller/component.wasm")
}

fn controller_manifest() -> PathBuf {
    repo_root().join("component-src/controller/component.manifest.json")
}

fn control_extension_json() -> PathBuf {
    repo_root().join("pack/extensions/control.json")
}

fn run_component(op: &str, input_json: &str) -> Option<serde_json::Value> {
    let output = match Command::new("greentic-component")
        .arg("test")
        .arg("--wasm")
        .arg(controller_wasm())
        .arg("--manifest")
        .arg(controller_manifest())
        .arg("--op")
        .arg(op)
        .arg("--input-json")
        .arg(input_json)
        .output()
    {
        Ok(output) => output,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            eprintln!("skipping: greentic-component is not available in PATH");
            return None;
        }
        Err(err) => panic!("failed to execute greentic-component test: {err}"),
    };

    assert!(
        output.status.success(),
        "greentic-component test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout must be valid UTF-8 JSON");
    Some(serde_json::from_str(&stdout).expect("test output must be valid JSON"))
}

#[test]
fn component_cli_refund_dispatches() {
    if !controller_wasm().is_file() || !controller_manifest().is_file() {
        eprintln!("skipping: controller wasm or manifest missing");
        return;
    }

    let output = match run_component(
        "ingress_control.handle",
        r#"{"message":{"text":"refund needed"}}"#,
    ) {
        Some(v) => v,
        None => return,
    };

    assert_eq!(output["status"], "ok");
    assert_eq!(output["result"]["action"], "dispatch");
    assert_eq!(output["result"]["dispatch"]["pack"], "commerce-support");
    assert_eq!(output["result"]["dispatch"]["flow"], "flow_refund");
}

#[test]
fn component_cli_invalid_explicit_path_returns_continue() {
    if !controller_wasm().is_file() || !controller_manifest().is_file() {
        eprintln!("skipping: controller wasm or manifest missing");
        return;
    }

    let output = match run_component(
        "ingress_control.handle",
        r#"{"message":{"text":"hello"},"explicit_path":"bad/path/with/too-many"}"#,
    ) {
        Some(v) => v,
        None => return,
    };

    assert_eq!(output["status"], "ok");
    assert_eq!(output["result"]["action"], "continue");
    assert_eq!(output["result"]["diag"]["explicit_path_valid"], false);
}

#[test]
fn control_extension_declares_setup_qa_reference() {
    if !control_extension_json().is_file() {
        eprintln!("skipping: control extension json missing");
        return;
    }

    let raw =
        std::fs::read_to_string(control_extension_json()).expect("read control extension json");
    let control: serde_json::Value =
        serde_json::from_str(&raw).expect("parse control extension json");
    let offer = &control["capabilities_extension"]["offers"][0];
    assert_eq!(offer["cap_id"], "greentic.cap.ingress.control.v1");
    assert_eq!(offer["offer_id"], "control-chain.post-ingress");
    assert_eq!(offer["provider"]["component_ref"], "controller");
    assert_eq!(offer["provider"]["op"], "ingress_control.handle");
    assert_eq!(offer["requires_setup"], true);
    assert_eq!(offer["setup"]["qa_ref"], "qa/control-setup.json");
}

#[test]
fn component_cli_setup_ops_are_usable() {
    if !controller_wasm().is_file() || !controller_manifest().is_file() {
        eprintln!("skipping: controller wasm or manifest missing");
        return;
    }

    let qa_spec = match run_component("qa-spec", r#"{"mode":"setup"}"#) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(qa_spec["status"], "ok");
    assert_eq!(qa_spec["result"]["mode"], "setup");
    assert!(
        qa_spec["result"]["questions"]
            .as_array()
            .map(|qs| !qs.is_empty())
            .unwrap_or(false),
        "qa-spec questions should be non-empty for setup"
    );

    let apply_missing = run_component("apply-answers", r#"{"mode":"setup","answers":{}}"#)
        .expect("apply-answers output");
    assert_eq!(apply_missing["status"], "ok");
    assert_eq!(apply_missing["result"]["ok"], false);
    assert!(
        apply_missing["result"]["errors"]
            .as_array()
            .map(|errs| !errs.is_empty())
            .unwrap_or(false),
        "apply-answers setup validation should emit errors when required answers are missing"
    );

    let apply_valid = run_component(
        "apply-answers",
        r#"{"mode":"setup","answers":{"api_key":"key123","region":"us-east-1","webhook_base_url":"https://example.invalid/hook","enabled":true}}"#,
    )
    .expect("apply-answers output");
    assert_eq!(apply_valid["status"], "ok");
    assert_eq!(apply_valid["result"]["ok"], true);
    assert_eq!(apply_valid["result"]["config"]["api_key"], "key123");
    assert_eq!(apply_valid["result"]["config"]["region"], "us-east-1");
    assert_eq!(
        apply_valid["result"]["config"]["webhook_base_url"],
        "https://example.invalid/hook"
    );
}
