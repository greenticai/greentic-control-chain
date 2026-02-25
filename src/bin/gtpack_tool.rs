use greentic_control_chain::pack::{build_gtpack, default_gtpack_output_path};
use std::env;
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        eprintln!(
            "usage: cargo run --bin gtpack_tool -- build [--out <path>] [--with-default-policy]"
        );
        std::process::exit(2);
    };

    match cmd.as_str() {
        "build" => {
            let mut out: Option<PathBuf> = None;
            let mut with_default_policy = false;

            while let Some(arg) = args.next() {
                if arg == "--out" {
                    let Some(path) = args.next() else {
                        return Err("missing value for --out".into());
                    };
                    out = Some(PathBuf::from(path));
                } else if arg == "--with-default-policy" {
                    with_default_policy = true;
                } else {
                    return Err(format!("unknown argument: {arg}").into());
                }
            }

            let out_file = out.unwrap_or_else(default_gtpack_output_path);
            let version = env!("CARGO_PKG_VERSION");
            build_gtpack(
                PathBuf::from("pack").as_path(),
                &out_file,
                version,
                with_default_policy,
            )?;
            println!("{}", out_file.display());
        }
        _ => return Err(format!("unknown command: {cmd}").into()),
    }

    Ok(())
}
