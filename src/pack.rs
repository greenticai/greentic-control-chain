use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;

pub const PACK_ID: &str = "routing-ingress-control-chain";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    pub v: u8,
    pub pack_id: String,
    pub version: String,
    pub description: String,
    pub api_version: u8,
    pub offers: Vec<Offer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offer {
    pub id: String,
    pub kind: String,
    pub stage: String,
    pub contract: String,
    pub priority: u32,
    pub provider: Provider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<OfferMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub op: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferMeta {
    pub cap_id: String,
}

pub fn canonical_manifest(version: &str) -> PackManifest {
    PackManifest {
        v: 1,
        pack_id: PACK_ID.to_string(),
        version: version.to_string(),
        description: "Ingress control chain hook pack for post_ingress routing decisions."
            .to_string(),
        api_version: 1,
        offers: vec![Offer {
            id: "control-chain.post-ingress".to_string(),
            kind: "hook".to_string(),
            stage: "post_ingress".to_string(),
            contract: "greentic.hook.control.v1".to_string(),
            priority: 10,
            provider: Provider {
                op: "ingress_control.handle".to_string(),
            },
            meta: Some(OfferMeta {
                cap_id: "greentic.cap.ingress.control.v1".to_string(),
            }),
        }],
    }
}

pub fn write_pack_cbor(path: &Path, version: &str) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let manifest = canonical_manifest(version);
    let encoded = serde_cbor::to_vec(&manifest)?;
    fs::write(path, encoded)?;
    Ok(())
}

pub fn write_default_policy_cbor(path: &Path) -> Result<(), Box<dyn Error>> {
    #[derive(Serialize)]
    struct Policy {
        v: u8,
        allow_respond: bool,
        allow_llm: bool,
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let policy = Policy {
        v: 1,
        allow_respond: false,
        allow_llm: false,
    };
    fs::write(path, serde_cbor::to_vec(&policy)?)?;
    Ok(())
}

pub fn build_gtpack(
    pack_dir: &Path,
    out_file: &Path,
    crate_version: &str,
    write_default_policy: bool,
) -> Result<(), Box<dyn Error>> {
    let pack_cbor_path = pack_dir.join("pack.cbor");
    write_pack_cbor(&pack_cbor_path, crate_version)?;

    if write_default_policy {
        let policy_path = pack_dir.join("assets").join("policy.cbor");
        write_default_policy_cbor(&policy_path)?;
    }

    if let Some(parent) = out_file.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(out_file)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    add_file_to_zip(&mut zip, &pack_cbor_path, "pack.cbor", options)?;

    let assets_dir = pack_dir.join("assets");
    if assets_dir.exists() {
        for entry in fs::read_dir(&assets_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or("invalid asset file name")?;
                let zip_name = format!("assets/{file_name}");
                add_file_to_zip(&mut zip, &path, &zip_name, options)?;
            }
        }
    }

    zip.finish()?;
    Ok(())
}

fn add_file_to_zip(
    zip: &mut zip::ZipWriter<File>,
    source: &Path,
    zip_name: &str,
    options: SimpleFileOptions,
) -> Result<(), Box<dyn Error>> {
    let mut data = Vec::new();
    File::open(source)?.read_to_end(&mut data)?;
    zip.start_file(zip_name, options)?;
    zip.write_all(&data)?;
    Ok(())
}

pub fn default_gtpack_output_path() -> PathBuf {
    PathBuf::from("build/dist").join(format!("{PACK_ID}.gtpack"))
}
