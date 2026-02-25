use greentic_control_chain::ingress_control::ingress_control_handle;
use serde_cbor::Value;
use std::collections::BTreeMap;
use std::io::{Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin_bytes = Vec::new();
    std::io::stdin().read_to_end(&mut stdin_bytes)?;

    let inbound: BTreeMap<String, Value> = if stdin_bytes.is_empty() {
        BTreeMap::new()
    } else {
        serde_cbor::from_slice(&stdin_bytes)?
    };

    let output = ingress_control_handle(&inbound)?;
    std::io::stdout().write_all(&output)?;
    Ok(())
}
