use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Dispatch,
    Respond,
    Continue,
    Deny,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Directive {
    pub v: u8,
    pub action: Action,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispatch: Option<DispatchPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub respond: Option<RespondPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<DenyPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diag: Option<Diag>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DispatchPayload {
    pub pack: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RespondPayload {
    pub text: String,
    pub needs_user: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DenyPayload {
    pub code: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Diag {
    pub stage: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_rule_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit_path_valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_blocked: Option<String>,
    pub allow_llm: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<DiagError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiagError {
    pub code: String,
    pub msg: String,
}

#[derive(Debug, Deserialize)]
struct PolicyV1 {
    v: u8,
    allow_respond: bool,
    allow_llm: bool,
}

#[derive(Debug, Deserialize)]
struct RulesV1 {
    v: u8,
    rules: Vec<RuleV1>,
}

#[derive(Debug, Deserialize)]
struct RuleV1 {
    id: String,
    when: RuleWhenV1,
    then: RuleThenV1,
}

#[derive(Debug, Deserialize)]
struct RuleWhenV1 {
    keyword: Option<String>,
    regex: Option<String>,
    case_sensitive: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RuleThenV1 {
    action: String,
    target: Option<String>,
    text: Option<String>,
    needs_user: Option<bool>,
    deny: Option<RuleDenyV1>,
}

#[derive(Debug, Deserialize)]
struct RuleDenyV1 {
    code: String,
    reason: String,
    details: Option<BTreeMap<String, Value>>,
}

enum CompiledRuleKind {
    Keyword {
        needle: String,
        case_sensitive: bool,
    },
    Regex {
        re: Regex,
    },
}

enum CompiledOutcome {
    Dispatch(DispatchPayload),
    Respond { text: String, needs_user: bool },
    Continue,
    Deny(DenyPayload),
}

struct CompiledRule {
    id: String,
    kind: CompiledRuleKind,
    outcome: CompiledOutcome,
}

#[derive(Clone, Copy)]
struct Policy {
    allow_respond: bool,
    allow_llm: bool,
}

pub fn ingress_control_handle(
    inbound: &BTreeMap<String, Value>,
) -> Result<Vec<u8>, serde_cbor::Error> {
    let directive = handle_with_assets(inbound, Path::new("pack/assets"));
    serde_cbor::to_vec(&directive)
}

pub fn handle_with_assets(inbound: &BTreeMap<String, Value>, assets_dir: &Path) -> Directive {
    let mut diag = Diag {
        stage: 0,
        allow_llm: false,
        ..Diag::default()
    };

    let policy = match load_policy(assets_dir) {
        Ok(policy) => policy,
        Err(AssetError::Missing) => Policy {
            allow_respond: false,
            allow_llm: false,
        },
        Err(AssetError::Invalid { msg, .. }) => {
            diag.errors.push(DiagError {
                code: "invalid_policy_asset".to_string(),
                msg: msg.clone(),
            });
            let mut details = BTreeMap::new();
            details.insert("error".to_string(), Value::Text(msg));
            return deny_directive(
                diag,
                "invalid_policy_asset",
                "policy.cbor failed validation",
                Some(details),
            );
        }
    };
    diag.allow_llm = policy.allow_llm;

    if let Some(explicit_path) = read_optional_text(inbound, "explicit_path") {
        match parse_dispatch_target(&explicit_path) {
            Ok(dispatch) => {
                diag.stage = 0;
                diag.explicit_path_valid = Some(true);
                return dispatch_directive(dispatch, diag);
            }
            Err(e) => {
                diag.stage = 0;
                diag.explicit_path_valid = Some(false);
                diag.errors.push(DiagError {
                    code: "invalid_explicit_path".to_string(),
                    msg: e,
                });
            }
        }
    }

    let text = read_message_text(inbound);
    if text.is_none() {
        return continue_directive(diag);
    }
    let text = text.unwrap_or_default();

    let rules = match load_rules(assets_dir) {
        Ok(Some(rules)) => rules,
        Ok(None) => return continue_directive(diag),
        Err(AssetError::Missing) => return continue_directive(diag),
        Err(AssetError::Invalid { msg, rule_id }) => {
            diag.errors.push(DiagError {
                code: "invalid_rules_asset".to_string(),
                msg: msg.clone(),
            });
            let mut details = BTreeMap::new();
            details.insert("error".to_string(), Value::Text(msg));
            if let Some(rule_id) = rule_id {
                details.insert("rule_id".to_string(), Value::Text(rule_id));
            }
            return deny_directive(
                diag,
                "invalid_rules_asset",
                "rules.cbor failed validation",
                Some(details),
            );
        }
    };

    for rule in rules {
        if !rule_matches(&rule.kind, &text) {
            continue;
        }

        diag.stage = 1;
        diag.matched_rule_id = Some(rule.id.clone());
        return match rule.outcome {
            CompiledOutcome::Dispatch(dispatch) => dispatch_directive(dispatch, diag),
            CompiledOutcome::Continue => continue_directive(diag),
            CompiledOutcome::Deny(deny) => Directive {
                v: 1,
                action: Action::Deny,
                dispatch: None,
                respond: None,
                deny: Some(deny),
                diag: Some(diag),
            },
            CompiledOutcome::Respond { text, needs_user } => {
                if policy.allow_respond {
                    Directive {
                        v: 1,
                        action: Action::Respond,
                        dispatch: None,
                        respond: Some(RespondPayload { text, needs_user }),
                        deny: None,
                        diag: Some(diag),
                    }
                } else {
                    let mut blocked_diag = diag;
                    blocked_diag.policy_blocked = Some("respond".to_string());
                    continue_directive(blocked_diag)
                }
            }
        };
    }

    continue_directive(diag)
}

fn continue_directive(diag: Diag) -> Directive {
    Directive {
        v: 1,
        action: Action::Continue,
        dispatch: None,
        respond: None,
        deny: None,
        diag: Some(diag),
    }
}

fn dispatch_directive(dispatch: DispatchPayload, diag: Diag) -> Directive {
    Directive {
        v: 1,
        action: Action::Dispatch,
        dispatch: Some(dispatch),
        respond: None,
        deny: None,
        diag: Some(diag),
    }
}

fn deny_directive(
    diag: Diag,
    code: &str,
    reason: &str,
    details: Option<BTreeMap<String, Value>>,
) -> Directive {
    Directive {
        v: 1,
        action: Action::Deny,
        dispatch: None,
        respond: None,
        deny: Some(DenyPayload {
            code: code.to_string(),
            reason: reason.to_string(),
            details,
        }),
        diag: Some(diag),
    }
}

fn read_message_text(inbound: &BTreeMap<String, Value>) -> Option<String> {
    let msg = inbound.get("message")?;
    let map = match msg {
        Value::Map(map) => map,
        _ => return None,
    };
    for (k, v) in map {
        if let Value::Text(key) = k
            && key == "text"
            && let Value::Text(text) = v
            && !text.trim().is_empty()
        {
            return Some(text.clone());
        }
    }
    None
}

fn read_optional_text(inbound: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    inbound.get(key).and_then(|v| match v {
        Value::Text(s) if !s.trim().is_empty() => Some(s.clone()),
        _ => None,
    })
}

fn load_policy(assets_dir: &Path) -> Result<Policy, AssetError> {
    let path = assets_dir.join("policy.cbor");
    if !path.exists() {
        return Err(AssetError::Missing);
    }
    let bytes = fs::read(path).map_err(|e| AssetError::invalid(e.to_string()))?;
    let policy: PolicyV1 =
        serde_cbor::from_slice(&bytes).map_err(|e| AssetError::invalid(e.to_string()))?;
    if policy.v != 1 {
        return Err(AssetError::invalid("policy version must be v=1"));
    }
    Ok(Policy {
        allow_respond: policy.allow_respond,
        allow_llm: policy.allow_llm,
    })
}

fn load_rules(assets_dir: &Path) -> Result<Option<Vec<CompiledRule>>, AssetError> {
    let path = assets_dir.join("rules.cbor");
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|e| AssetError::invalid(e.to_string()))?;
    let rules: RulesV1 =
        serde_cbor::from_slice(&bytes).map_err(|e| AssetError::invalid(e.to_string()))?;
    if rules.v != 1 {
        return Err(AssetError::invalid("rules version must be v=1"));
    }

    let mut compiled = Vec::new();
    for rule in rules.rules {
        compiled.push(compile_rule(rule)?);
    }
    Ok(Some(compiled))
}

fn compile_rule(rule: RuleV1) -> Result<CompiledRule, AssetError> {
    if rule.id.trim().is_empty() {
        return Err(AssetError::invalid("rule id must not be empty"));
    }
    let case_sensitive = rule.when.case_sensitive.unwrap_or(false);
    let kind = match (&rule.when.keyword, &rule.when.regex) {
        (Some(keyword), None) => CompiledRuleKind::Keyword {
            needle: keyword.clone(),
            case_sensitive,
        },
        (None, Some(regex_src)) => {
            let pat = if case_sensitive {
                regex_src.clone()
            } else {
                format!("(?i:{regex_src})")
            };
            let re = Regex::new(&pat).map_err(|e| {
                AssetError::invalid_rule(
                    &rule.id,
                    format!("invalid regex for rule '{}': {}", rule.id, e),
                )
            })?;
            CompiledRuleKind::Regex { re }
        }
        _ => {
            return Err(AssetError::invalid_rule(
                &rule.id,
                format!(
                    "rule '{}' must set exactly one of when.keyword or when.regex",
                    rule.id
                ),
            ));
        }
    };

    let outcome = match rule.then.action.as_str() {
        "dispatch" => {
            let target = rule.then.target.ok_or_else(|| {
                AssetError::invalid_rule(
                    &rule.id,
                    format!("rule '{}' dispatch missing target", rule.id),
                )
            })?;
            let dispatch = parse_dispatch_target(&target).map_err(|e| {
                AssetError::invalid_rule(
                    &rule.id,
                    format!("rule '{}' target invalid: {}", rule.id, e),
                )
            })?;
            CompiledOutcome::Dispatch(dispatch)
        }
        "respond" => {
            let text = rule.then.text.ok_or_else(|| {
                AssetError::invalid_rule(
                    &rule.id,
                    format!("rule '{}' respond missing text", rule.id),
                )
            })?;
            CompiledOutcome::Respond {
                text,
                needs_user: rule.then.needs_user.unwrap_or(false),
            }
        }
        "continue" => CompiledOutcome::Continue,
        "deny" => {
            let deny = rule.then.deny.ok_or_else(|| {
                AssetError::invalid_rule(
                    &rule.id,
                    format!("rule '{}' deny missing deny object", rule.id),
                )
            })?;
            CompiledOutcome::Deny(DenyPayload {
                code: deny.code,
                reason: deny.reason,
                details: deny.details,
            })
        }
        other => {
            return Err(AssetError::invalid_rule(
                &rule.id,
                format!("rule '{}' has unsupported action '{}'", rule.id, other),
            ));
        }
    };

    Ok(CompiledRule {
        id: rule.id,
        kind,
        outcome,
    })
}

fn parse_dispatch_target(target: &str) -> Result<DispatchPayload, String> {
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
    Ok(DispatchPayload {
        pack: parts[0].to_string(),
        flow: parts.get(1).map(|s| (*s).to_string()),
        node: parts.get(2).map(|s| (*s).to_string()),
    })
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

fn rule_matches(kind: &CompiledRuleKind, text: &str) -> bool {
    match kind {
        CompiledRuleKind::Keyword {
            needle,
            case_sensitive,
        } => {
            if *case_sensitive {
                text.contains(needle)
            } else {
                text.to_lowercase().contains(&needle.to_lowercase())
            }
        }
        CompiledRuleKind::Regex { re } => re.is_match(text),
    }
}

enum AssetError {
    Missing,
    Invalid {
        msg: String,
        rule_id: Option<String>,
    },
}

impl AssetError {
    fn invalid(msg: impl Into<String>) -> Self {
        Self::Invalid {
            msg: msg.into(),
            rule_id: None,
        }
    }

    fn invalid_rule(rule_id: &str, msg: impl Into<String>) -> Self {
        Self::Invalid {
            msg: msg.into(),
            rule_id: Some(rule_id.to_string()),
        }
    }
}
