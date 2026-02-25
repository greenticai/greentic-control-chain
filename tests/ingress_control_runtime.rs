use greentic_control_chain::ingress_control::{Action, Directive, handle_with_assets};
use serde::Serialize;
use serde_cbor::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn invalid_explicit_path_returns_continue_with_diag() {
    let temp = unique_tempdir("ingress_test");
    let assets = temp.join("assets");
    fs::create_dir_all(&assets).expect("create assets dir");
    write_policy(&assets, false, false);

    let mut inbound = BTreeMap::new();
    let mut message = BTreeMap::new();
    message.insert(
        Value::Text("text".to_string()),
        Value::Text("hello".to_string()),
    );
    inbound.insert("message".to_string(), Value::Map(message));
    inbound.insert(
        "explicit_path".to_string(),
        Value::Text("bad/path/with/too-many".to_string()),
    );

    let directive = handle_with_assets(&inbound, &assets);
    assert_eq!(directive.action, Action::Continue);
    let diag = directive.diag.expect("diag");
    assert_eq!(diag.explicit_path_valid, Some(false));
}

#[test]
fn missing_rules_asset_returns_continue() {
    let temp = unique_tempdir("ingress_test");
    let assets = temp.join("assets");
    fs::create_dir_all(&assets).expect("create assets dir");
    write_policy(&assets, false, false);

    let inbound = inbound_with_text("refund");
    let directive = handle_with_assets(&inbound, &assets);
    assert_eq!(directive.action, Action::Continue);
}

#[test]
fn invalid_rules_asset_returns_deny() {
    let temp = unique_tempdir("ingress_test");
    let assets = temp.join("assets");
    fs::create_dir_all(&assets).expect("create assets dir");
    write_policy(&assets, false, false);
    fs::write(assets.join("rules.cbor"), b"not-cbor").expect("write invalid rules");

    let inbound = inbound_with_text("refund");
    let directive = handle_with_assets(&inbound, &assets);
    assert_eq!(directive.action, Action::Deny);
    let deny = directive.deny.expect("deny payload");
    assert_eq!(deny.code, "invalid_rules_asset");
}

#[test]
fn invalid_policy_asset_returns_deny() {
    let temp = unique_tempdir("ingress_test");
    let assets = temp.join("assets");
    fs::create_dir_all(&assets).expect("create assets dir");
    fs::write(assets.join("policy.cbor"), b"bad-cbor").expect("write invalid policy");

    let inbound = inbound_with_text("refund");
    let directive = handle_with_assets(&inbound, &assets);
    assert_eq!(directive.action, Action::Deny);
    let deny = directive.deny.expect("deny payload");
    assert_eq!(deny.code, "invalid_policy_asset");
}

#[test]
fn policy_missing_uses_secure_defaults_and_blocks_respond() {
    let temp = unique_tempdir("ingress_test");
    let assets = temp.join("assets");
    fs::create_dir_all(&assets).expect("create assets dir");
    write_rules(
        &assets,
        vec![RuleInput::respond_rule(
            "r1",
            "refund",
            "Need more info",
            true,
        )],
    );

    let inbound = inbound_with_text("refund please");
    let directive = handle_with_assets(&inbound, &assets);
    assert_eq!(directive.action, Action::Continue);
    let diag = directive.diag.expect("diag");
    assert_eq!(diag.policy_blocked.as_deref(), Some("respond"));
    assert!(!diag.allow_llm);
}

#[test]
fn rule_ordering_is_deterministic_first_match_wins() {
    let temp = unique_tempdir("ingress_test");
    let assets = temp.join("assets");
    fs::create_dir_all(&assets).expect("create assets dir");
    write_policy(&assets, true, false);
    write_rules(
        &assets,
        vec![
            RuleInput::dispatch_rule("r1", "refund", "commerce-support/flow_refund"),
            RuleInput::dispatch_rule("r2", "refund", "fallback-pack/flow2"),
        ],
    );

    let inbound = inbound_with_text("refund needed");
    let directive = handle_with_assets(&inbound, &assets);
    assert_eq!(directive.action, Action::Dispatch);
    let dispatch = directive.dispatch.expect("dispatch payload");
    assert_eq!(dispatch.pack, "commerce-support");
    assert_eq!(dispatch.flow.as_deref(), Some("flow_refund"));
}

#[test]
fn policy_disallows_respond_downgrades_to_continue() {
    let temp = unique_tempdir("ingress_test");
    let assets = temp.join("assets");
    fs::create_dir_all(&assets).expect("create assets dir");
    write_policy(&assets, false, false);
    write_rules(
        &assets,
        vec![RuleInput::respond_rule(
            "r1",
            "refund",
            "Need confirmation",
            true,
        )],
    );

    let inbound = inbound_with_text("refund needed");
    let directive = handle_with_assets(&inbound, &assets);
    assert_eq!(directive.action, Action::Continue);
    let diag = directive.diag.expect("diag");
    assert_eq!(diag.policy_blocked.as_deref(), Some("respond"));
}

#[test]
fn unsupported_rule_action_is_invalid_rules_asset() {
    let temp = unique_tempdir("ingress_test");
    let assets = temp.join("assets");
    fs::create_dir_all(&assets).expect("create assets dir");
    write_policy(&assets, true, false);
    write_rules(
        &assets,
        vec![RuleInput {
            id: "bad-action".to_string(),
            when: RuleWhenInput {
                keyword: Some("refund".to_string()),
                regex: None,
                case_sensitive: None,
            },
            then: RuleThenInput {
                action: "llm".to_string(),
                target: None,
                text: None,
                needs_user: None,
                deny: None,
            },
        }],
    );

    let inbound = inbound_with_text("refund");
    let directive = handle_with_assets(&inbound, &assets);
    assert_eq!(directive.action, Action::Deny);
    let deny = directive.deny.expect("deny payload");
    assert_eq!(deny.code, "invalid_rules_asset");
    assert_eq!(
        deny_detail_text(&deny, "rule_id").as_deref(),
        Some("bad-action")
    );
}

#[test]
fn invalid_regex_rule_returns_deny_with_rule_id() {
    let temp = unique_tempdir("ingress_test");
    let assets = temp.join("assets");
    fs::create_dir_all(&assets).expect("create assets dir");
    write_policy(&assets, true, false);
    write_rules(
        &assets,
        vec![RuleInput {
            id: "bad-regex".to_string(),
            when: RuleWhenInput {
                keyword: None,
                regex: Some("(".to_string()),
                case_sensitive: None,
            },
            then: RuleThenInput {
                action: "dispatch".to_string(),
                target: Some("commerce-support/flow_refund".to_string()),
                text: None,
                needs_user: None,
                deny: None,
            },
        }],
    );

    let inbound = inbound_with_text("refund");
    let directive = handle_with_assets(&inbound, &assets);
    assert_eq!(directive.action, Action::Deny);
    let deny = directive.deny.expect("deny payload");
    assert_eq!(deny.code, "invalid_rules_asset");
    assert_eq!(
        deny_detail_text(&deny, "rule_id").as_deref(),
        Some("bad-regex")
    );
}

#[test]
fn invalid_dispatch_target_rule_returns_deny_with_rule_id() {
    let temp = unique_tempdir("ingress_test");
    let assets = temp.join("assets");
    fs::create_dir_all(&assets).expect("create assets dir");
    write_policy(&assets, true, false);
    write_rules(
        &assets,
        vec![RuleInput::dispatch_rule(
            "bad-target",
            "refund",
            "BadPack/flow_refund",
        )],
    );

    let inbound = inbound_with_text("refund");
    let directive = handle_with_assets(&inbound, &assets);
    assert_eq!(directive.action, Action::Deny);
    let deny = directive.deny.expect("deny payload");
    assert_eq!(deny.code, "invalid_rules_asset");
    assert_eq!(
        deny_detail_text(&deny, "rule_id").as_deref(),
        Some("bad-target")
    );
}

#[test]
fn valid_explicit_path_dispatches() {
    let temp = unique_tempdir("ingress_test");
    let assets = temp.join("assets");
    fs::create_dir_all(&assets).expect("create assets dir");
    write_policy(&assets, false, false);

    let mut inbound = inbound_with_text("ignored");
    inbound.insert(
        "explicit_path".to_string(),
        Value::Text("commerce-support/flow_refund".to_string()),
    );

    let directive = handle_with_assets(&inbound, &assets);
    assert_eq!(directive.action, Action::Dispatch);
    let dispatch = directive.dispatch.expect("dispatch");
    assert_eq!(dispatch.pack, "commerce-support");
    assert_eq!(dispatch.flow.as_deref(), Some("flow_refund"));
}

#[derive(Serialize)]
struct PolicyV1 {
    v: u8,
    allow_respond: bool,
    allow_llm: bool,
}

#[derive(Serialize)]
struct RulesV1 {
    v: u8,
    rules: Vec<RuleInput>,
}

#[derive(Serialize)]
struct RuleInput {
    id: String,
    when: RuleWhenInput,
    then: RuleThenInput,
}

#[derive(Serialize)]
struct RuleWhenInput {
    keyword: Option<String>,
    regex: Option<String>,
    case_sensitive: Option<bool>,
}

#[derive(Serialize)]
struct RuleThenInput {
    action: String,
    target: Option<String>,
    text: Option<String>,
    needs_user: Option<bool>,
    deny: Option<RuleDenyInput>,
}

#[derive(Serialize)]
struct RuleDenyInput {
    code: String,
    reason: String,
    details: Option<BTreeMap<String, Value>>,
}

impl RuleInput {
    fn dispatch_rule(id: &str, keyword: &str, target: &str) -> Self {
        Self {
            id: id.to_string(),
            when: RuleWhenInput {
                keyword: Some(keyword.to_string()),
                regex: None,
                case_sensitive: None,
            },
            then: RuleThenInput {
                action: "dispatch".to_string(),
                target: Some(target.to_string()),
                text: None,
                needs_user: None,
                deny: None,
            },
        }
    }

    fn respond_rule(id: &str, keyword: &str, text: &str, needs_user: bool) -> Self {
        Self {
            id: id.to_string(),
            when: RuleWhenInput {
                keyword: Some(keyword.to_string()),
                regex: None,
                case_sensitive: None,
            },
            then: RuleThenInput {
                action: "respond".to_string(),
                target: None,
                text: Some(text.to_string()),
                needs_user: Some(needs_user),
                deny: None,
            },
        }
    }
}

fn inbound_with_text(text: &str) -> BTreeMap<String, Value> {
    let mut inbound = BTreeMap::new();
    let mut message = BTreeMap::new();
    message.insert(
        Value::Text("text".to_string()),
        Value::Text(text.to_string()),
    );
    inbound.insert("message".to_string(), Value::Map(message));
    inbound
}

fn write_policy(assets: &Path, allow_respond: bool, allow_llm: bool) {
    let data = serde_cbor::to_vec(&PolicyV1 {
        v: 1,
        allow_respond,
        allow_llm,
    })
    .expect("serialize policy");
    fs::write(assets.join("policy.cbor"), data).expect("write policy");
}

fn write_rules(assets: &Path, rules: Vec<RuleInput>) {
    let data = serde_cbor::to_vec(&RulesV1 { v: 1, rules }).expect("serialize rules");
    fs::write(assets.join("rules.cbor"), data).expect("write rules");
}

fn unique_tempdir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}_{nanos}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

#[allow(dead_code)]
fn _decode(bytes: &[u8]) -> Directive {
    serde_cbor::from_slice(bytes).expect("decode directive")
}

fn deny_detail_text(
    deny: &greentic_control_chain::ingress_control::DenyPayload,
    key: &str,
) -> Option<String> {
    let details = deny.details.as_ref()?;
    let v = details.get(key)?;
    match v {
        Value::Text(s) => Some(s.clone()),
        _ => None,
    }
}
