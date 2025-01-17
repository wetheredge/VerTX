use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::process::Command;
use std::{env, fs, io};

use serde::Deserialize;

fn main() -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();

    #[derive(Debug, Deserialize)]
    struct Manifest {
        index: Asset,
        assets: Vec<Asset>,
    }

    #[derive(Debug, Deserialize)]
    struct Asset {
        route: String,
        file: String,
        mime: String,
        gzip: bool,
    }

    let configurator = fs::canonicalize(format!("{root}/../vertx-configurator")).unwrap();
    if env::var_os("VERTX_SERVER_SKIP_CONFIGURATOR_BUILD").is_none() {
        let configurator_build = Command::new("task")
            .arg("build")
            .current_dir(&configurator)
            .status()
            .unwrap();
        assert!(configurator_build.success(), "configurator failed to build");
    }
    let dist = configurator.join("dist");
    println!("cargo:rerun-if-changed={}", dist.display());

    let assets = fs::read_to_string(dist.join("manifest.json"))?;
    let mut manifest: Manifest = serde_json::from_str(&assets).unwrap();
    manifest
        .assets
        .sort_unstable_by(|a, b| a.route.cmp(&b.route));

    let write_asset = |out: &mut BufWriter<File>, asset: &Asset| {
        let (mime_head, mime_parameters) = asset.mime.split_once(';').unwrap_or((&asset.mime, ""));
        let (mime_type, mime_subtype) = mime_head.split_once("/").unwrap();

        let gzipped = asset.gzip;
        let path = dist.join(&asset.file).display().to_string();

        write!(out, "File {{ ")?;
        write!(
            out,
            "mime: Mime::new({mime_type:?}, {mime_subtype:?}, {mime_parameters:?}), "
        )?;
        write!(out, "gzipped: {gzipped:?}, ")?;
        write!(out, "content: ::core::include_bytes!({path:?})")?;
        write!(out, " }}")
    };

    let out = File::create(format!("{out_dir}/index.rs"))?;
    let out = &mut BufWriter::new(out);
    write_asset(out, &manifest.index)?;

    let out = File::create(format!("{out_dir}/assets.rs"))?;
    let out = &mut BufWriter::new(out);
    writeln!(out, "&[")?;
    for asset in manifest.assets {
        write!(out, "({:?}, ", asset.route)?;
        write_asset(out, &asset)?;
        writeln!(out, "),")?;
    }
    writeln!(out, "]")
}
