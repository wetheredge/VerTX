use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::process::Command;
use std::{env, fs, io};

use serde::Deserialize;

fn main() -> io::Result<()> {
    let out_dir = &env::var("OUT_DIR").unwrap();
    let root = &env::var("CARGO_MANIFEST_DIR").unwrap();

    let config = format!("{root}/../vertx-config/out/config.rs");
    println!("cargo::rerun-if-changed={config}");
    fs::copy(config, format!("{out_dir}/config.rs"))?;

    fs::write(
        format!("{out_dir}/qr_url"),
        env!("CARGO_PKG_HOMEPAGE").to_ascii_uppercase(),
    )?;

    build_info(out_dir)?;

    if !feature("SIMULATOR") {
        let target_name = env::var("VERTX_TARGET").expect("VERTX_TARGET should be set");
        println!("cargo::rerun-if-env-changed=VERTX_TARGET");

        memory_layout(out_dir, root)?;
        link_args();
        pins(out_dir, root, &target_name)?;
    }

    if feature("NETWORK") {
        configurator(out_dir, root)?;
    }

    Ok(())
}

fn memory_layout(out_dir: &str, root: &str) -> io::Result<()> {
    let path = feature("CHIP_RP").then_some("src/hal/rp/memory.x");

    if let Some(path) = path {
        fs::copy(format!("{root}/{path}"), format!("{out_dir}/memory.x"))?;

        println!("cargo::rustc-link-search={out_dir}");
        println!("cargo::rerun-if-changed={root}/{path}");
    }

    Ok(())
}

fn link_args() {
    let mut args = if feature("CHIP_ESP") {
        vec!["-Tlinkall.x", "-nostartfiles"]
    } else if feature("CHIP_RP") {
        vec!["--nmagic", "-Tlink.x", "-Tlink-rp.x"]
    } else {
        vec![]
    };

    if feature("DEFMT") {
        args.push("-Tdefmt.x");
    }

    for arg in args {
        println!("cargo::rustc-link-arg-bins={arg}");
    }
}

fn build_info(out_dir: &str) -> io::Result<()> {
    let git = |args: &[_]| Command::new("git").args(args).output().unwrap().stdout;
    let git_string = |env, args| {
        println!("cargo::rerun-if-env-changed={env}");
        env::var(env).unwrap_or_else(|_| String::from_utf8(git(args)).unwrap().trim().to_owned())
    };

    let branch = git_string("VERTX_GIT_BRANCH", &["branch", "--show-current"]);
    fs::write(format!("{out_dir}/git_branch"), branch)?;

    let commit = git_string("VERTX_GIT_COMMIT", &["rev-parse", "--short", "HEAD"]);
    fs::write(format!("{out_dir}/git_commit"), commit)?;

    println!("cargo::rerun-if-env-changed=VERTX_GIT_DIRTY");
    let dirty = env::var("VERTX_GIT_DIRTY").map_or_else(
        |_| !git(&["status", "--porcelain"]).is_empty(),
        |env| !env.eq_ignore_ascii_case("false"),
    );
    fs::write(format!("{out_dir}/git_dirty"), dirty.to_string())?;

    let profile = env::var("PROFILE").unwrap();
    let debug = profile != "release";
    fs::write(format!("{out_dir}/is_debug"), debug.to_string())
}

fn pins(out_dir: &str, root: &str, target: &str) -> io::Result<()> {
    #[derive(Debug, Deserialize)]
    struct Target {
        pins: Pins,
    }

    #[derive(Debug, Deserialize)]
    struct Pins {
        display: DisplayPins,

        #[serde(flatten)]
        rest: MiscPins,
    }

    #[derive(Debug, Deserialize)]
    #[serde(transparent)]
    struct MiscPins(HashMap<String, PinSpec>);

    #[derive(Debug, Deserialize)]
    struct DisplayPins {
        #[serde(rename = "type")]
        _type: DisplayType,
        #[serde(flatten)]
        pins: MiscPins,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum DisplayType {
        Ssd1306,
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum PinSpec {
        Single(u8),
        Multiple(Vec<u8>),
        Nested(MiscPins),
    }

    impl MiscPins {
        fn format(self, output: &mut String, prefix: &str, path: &str) {
            for (key, value) in self.0 {
                let key = key.replace('-', "_");
                let key = if path.is_empty() {
                    key
                } else {
                    format!("{path}.{key}")
                };
                match value {
                    PinSpec::Single(pin) => {
                        output.push_str("    ($p:expr, ");
                        output.push_str(&key);
                        output.push_str(") => { $p.");
                        output.push_str(prefix);
                        output.push_str(&pin.to_string());
                        output.push_str(" };\n");
                    }
                    PinSpec::Multiple(pins) => {
                        output.push_str("    ($p:expr, ");
                        output.push_str(&key);
                        output.push_str(" $(.$method:ident())*) => { &[");
                        for pin in pins {
                            output.push_str("$p.");
                            output.push_str(prefix);
                            output.push_str(&pin.to_string());
                            output.push_str(" $(.$method())*,");
                        }
                        output.push_str("] };\n");
                    }
                    PinSpec::Nested(inner) => inner.format(output, prefix, &key),
                }
            }
        }
    }

    let gpio = if feature("CHIP_ESP") {
        "GPIO"
    } else if feature("CHIP_RP") {
        "PIN_"
    } else {
        return Ok(());
    };

    let path = format!("{root}/../targets/{target}.toml");
    println!("cargo::rerun-if-changed={path}");

    let target = fs::read_to_string(path)?;
    let target: Target = basic_toml::from_str(&target).unwrap();

    let mut out = String::from("macro_rules! pins {\n");
    target.pins.rest.format(&mut out, gpio, "");
    target.pins.display.pins.format(&mut out, gpio, "display");
    out.push_str("}\n");

    fs::write(format!("{out_dir}/pins.rs"), out)
}

fn configurator(out_dir: &str, root: &str) -> io::Result<()> {
    #[derive(Debug, Deserialize)]
    struct Asset {
        route: String,
        file: String,
        mime: String,
        gzip: bool,
    }

    let configurator = fs::canonicalize(format!("{root}/../vertx-configurator")).unwrap();
    if env::var_os("VERTX_SKIP_CONFIGURATOR_BUILD").is_none() {
        let configurator_build = Command::new("task")
            .arg("build")
            .current_dir(&configurator)
            .status()
            .unwrap();
        assert!(configurator_build.success(), "configurator failed to build");
    }
    let dist = &configurator.join("dist");
    println!("cargo::rerun-if-changed={}", dist.display());

    let assets = fs::read_to_string(dist.join("assets.json"))?;
    let mut assets: Vec<Asset> = serde_json::from_str(&assets).unwrap();
    assets.sort_unstable_by(|a, b| a.route.cmp(&b.route));

    let out = File::create(format!("{out_dir}/assets.rs"))?;
    let out = &mut BufWriter::new(out);

    let write_asset = |out: &mut BufWriter<File>, asset: &Asset| {
        let (mime_head, mime_parameters) = asset.mime.split_once(';').unwrap_or((&asset.mime, ""));
        let (mime_type, mime_subtype) = mime_head.split_once('/').unwrap();

        let path = dist.join(&asset.file);
        let path = path.display();

        write!(out, "Asset {{ ")?;
        write!(
            out,
            "mime: Mime::new({mime_type:?}, {mime_subtype:?}, {mime_parameters:?}), ",
        )?;
        write!(out, "gzipped: {:?}, ", asset.gzip)?;
        write!(out, "content: ::core::include_bytes!(\"{path}\")")?;
        write!(out, " }}")
    };

    writeln!(out, "&[")?;
    for asset in &assets {
        write!(out, "    ({:?}, ", asset.route)?;
        write_asset(out, asset)?;
        writeln!(out, "),")?;
    }
    writeln!(out, "]")
}

fn feature(feature: &str) -> bool {
    env::var_os(format!("CARGO_FEATURE_{feature}")).is_some()
}
