use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::process::Command;
use std::{env, fmt, fs, io};

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

    let Ok(target_name) = env::var("VERTX_TARGET") else {
        panic!("VERTX_TARGET must be set");
    };
    println!("cargo::rerun-if-env-changed=VERTX_TARGET");

    if feature("SIMULATOR") {
        assert_eq!(
            target_name, "simulator",
            "Must set VERTX_TARGET=simulator when building the simulator"
        );
    } else if target_name != "test" {
        memory_layout(out_dir, root);
        link_args();
        pins(out_dir, root, &target_name)?;
    }

    if feature("NETWORK") {
        configurator(out_dir, root)?;
    }

    Ok(())
}

fn memory_layout(out_dir: &str, root: &str) {
    let path = feature("CHIP_RP").then_some("src/hal/chip/rp/memory.x");

    if let Some(path) = path {
        fs::copy(format!("{root}/{path}"), format!("{out_dir}/memory.x"))
            .expect("copying memory.x");

        println!("cargo::rustc-link-search={out_dir}");
        println!("cargo::rerun-if-changed={root}/{path}");
    }
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

    let dirty = env::var("VERTX_GIT_DIRTY").map_or_else(
        |_| !git(&["status", "--porcelain"]).is_empty(),
        |env| !env.eq_ignore_ascii_case("false"),
    );
    let mut commit = git_string("VERTX_GIT_COMMIT", &["rev-parse", "--short", "HEAD"]);
    if dirty {
        commit.push_str("-dirty");
    }
    fs::write(format!("{out_dir}/git_commit"), &commit)?;

    let profile = env::var("PROFILE").unwrap();
    let debug = profile != "release";
    fs::write(format!("{out_dir}/is_debug"), debug.to_string())
}

fn pins(out_dir: &str, root: &str, target: &str) -> io::Result<()> {
    #[derive(Debug, Deserialize)]
    struct Target {
        #[expect(unused)]
        chip: String,
        sd: Sd,
        display: Display,
        #[serde(flatten)]
        rest: MiscPins,
    }

    #[derive(Debug, Deserialize)]
    struct Sd {
        #[serde(rename = "type")]
        _type: SdType,
        #[serde(flatten)]
        pins: MiscPins,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum SdType {
        Spi,
    }

    #[derive(Debug, Deserialize)]
    struct Display {
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
    #[serde(transparent)]
    struct MiscPins(HashMap<String, PinSpec>);

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum PinSpec {
        Single(Pin),
        Multiple(Vec<Pin>),
        Nested(MiscPins),
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum Pin {
        Numbered(u8),
        Named(String),
    }

    impl fmt::Display for Pin {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Pin::Numbered(num) => write!(f, "{num}"),
                Pin::Named(name) => f.write_str(name),
            }
        }
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
    target.rest.format(&mut out, gpio, "");
    target.sd.pins.format(&mut out, gpio, "sd");
    target.display.pins.format(&mut out, gpio, "display");
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

    let configurator = format!("{root}/../out/configurator");
    if !fs::exists(&configurator)? {
        panic!("Build vertx-configurator first");
    }
    let configurator = fs::canonicalize(configurator).unwrap();
    println!("cargo::rerun-if-changed={}", configurator.display());

    let assets = fs::read_to_string(configurator.join("assets.json"))?;
    let mut assets: Vec<Asset> = serde_json::from_str(&assets).unwrap();
    assets.sort_unstable_by(|a, b| a.route.cmp(&b.route));

    let out = File::create(format!("{out_dir}/assets.rs"))?;
    let out = &mut BufWriter::new(out);

    let write_asset = |out: &mut BufWriter<File>, asset: &Asset| {
        let (mime_head, mime_parameters) = asset.mime.split_once(';').unwrap_or((&asset.mime, ""));
        let (mime_type, mime_subtype) = mime_head.split_once('/').unwrap();

        let path = configurator.join(&asset.file);
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
