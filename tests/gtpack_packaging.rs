use greentic_control_chain::pack::{build_gtpack, canonical_manifest};
use serde_cbor::Value;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use zip::ZipArchive;

#[test]
fn canonical_manifest_has_single_hook_offer() {
    let manifest = canonical_manifest(env!("CARGO_PKG_VERSION"));
    assert_eq!(manifest.offers.len(), 1);
    let offer = &manifest.offers[0];
    assert_eq!(offer.kind, "hook");
    assert_eq!(offer.stage, "post_ingress");
    assert_eq!(offer.contract, "greentic.hook.control.v1");
    assert_eq!(offer.provider.op, "ingress_control.handle");
}

#[test]
fn gtpack_contains_pack_cbor_with_required_offer_fields() {
    let temp = unique_tempdir("gtpack_test");
    let pack_dir = temp.join("pack");
    fs::create_dir_all(pack_dir.join("assets")).expect("create pack assets dir");
    let out = temp.join("routing-ingress-control-chain.gtpack");

    build_gtpack(&pack_dir, &out, env!("CARGO_PKG_VERSION"), false).expect("build gtpack");

    let file = fs::File::open(&out).expect("open gtpack zip");
    let mut archive = ZipArchive::new(file).expect("read zip");
    let mut pack_file = archive.by_name("pack.cbor").expect("pack.cbor in zip");
    let mut data = Vec::new();
    pack_file.read_to_end(&mut data).expect("read pack.cbor");

    let decoded: Value = serde_cbor::from_slice(&data).expect("decode pack.cbor");
    let offers = map_get(&decoded, "offers").expect("offers in manifest");
    let offer_list = match offers {
        Value::Array(items) => items,
        _ => panic!("offers is array"),
    };
    assert_eq!(offer_list.len(), 1);
    let offer = &offer_list[0];

    assert_eq!(as_text(map_get(offer, "kind")).as_deref(), Some("hook"));
    assert_eq!(
        as_text(map_get(offer, "stage")).as_deref(),
        Some("post_ingress")
    );
    assert_eq!(
        as_text(map_get(offer, "contract")).as_deref(),
        Some("greentic.hook.control.v1")
    );
    let provider = map_get(offer, "provider").expect("provider in offer");
    assert_eq!(
        as_text(map_get(provider, "op")).as_deref(),
        Some("ingress_control.handle")
    );
}

fn map_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let map = match value {
        Value::Map(map) => map,
        _ => return None,
    };
    map.iter().find_map(|(k, v)| {
        if let Value::Text(k_text) = k
            && k_text == key
        {
            Some(v)
        } else {
            None
        }
    })
}

fn as_text(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::Text(v)) => Some(v.clone()),
        _ => None,
    }
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
