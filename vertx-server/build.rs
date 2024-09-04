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

    let push_asset = |s: &mut String, asset: &Asset| {
        s.push_str("::picoserve::response::File::with_content_type_and_headers(\"");
        s.push_str(&asset.mime);
        s.push_str("\",::core::include_bytes!(\"");
        s.push_str(&dist.join(&asset.file).display().to_string());
        s.push_str("\"),&[");
        if asset.gzip {
            s.push_str(r#"("Content-Encoding","gzip")"#);
        }
        s.push_str("])");
    };

    let mut code = String::from("static INDEX: ::picoserve::response::File=");
    push_asset(&mut code, &manifest.index);
    code.push(';');

    code.push_str(
        "#[allow(long_running_const_eval)]static \
         ASSETS:&[(&::core::primitive::str,::picoserve::response::File)]=&[",
    );
    for asset in manifest.assets {
        code.push_str("(\"");
        code.push_str(&asset.route);
        code.push_str("\",");
        push_asset(&mut code, &asset);
        code.push_str("),");
    }
    code.push_str("];\n");

    fs::write(format!("{out_dir}/configurator.rs"), code)
}
