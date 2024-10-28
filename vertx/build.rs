use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::process::Command;

use serde::Deserialize;

fn main() -> io::Result<()> {
    let out_dir = &env::var("OUT_DIR").unwrap();
    let root = &env::var("CARGO_MANIFEST_DIR").unwrap();

    if env::var_os("CARGO_FEATURE_SIMULATOR").is_some() {
        build_info(out_dir, "simulator")
    } else {
        let target_name = env::var("VERTX_TARGET").expect("VERTX_TARGET should be set");
        println!("cargo:rerun-if-env-changed=VERTX_TARGET");

        memory_layout(out_dir, root)?;
        link_args();
        build_info(out_dir, &target_name)?;
        pins(out_dir, root, &target_name)
    }
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
        vec!["-Tlinkall.x", "-Trom_functions.x", "-nostartfiles"]
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

fn build_info(out_dir: &str, target_name: &str) -> io::Result<()> {
    let git = |args: &[_]| Command::new("git").args(args).output().unwrap().stdout;
    let git_string = |args| String::from_utf8(git(args)).unwrap().trim().to_owned();

    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let debug = env::var("PROFILE").unwrap() != "release";

    let git_branch = git_string(&["branch", "--show-current"]);
    let git_commit = git_string(&["rev-parse", "--short", "HEAD"]);
    let git_dirty = !git(&["status", "--porcelain"]).is_empty();

    let out = File::create(format!("{out_dir}/build_info.rs"))?;
    let out = &mut BufWriter::new(out);

    writeln!(out, "Response::BuildInfo {{")?;
    writeln!(out, r#"target: {target_name:?},"#)?;
    writeln!(out, r#"version: {version:?},"#)?;
    writeln!(out, r#"debug: {debug:?},"#)?;
    writeln!(out, r#"git_branch:{git_branch:?},"#)?;
    writeln!(out, r#"git_commit:{git_commit:?},"#)?;
    writeln!(out, r#"git_dirty:{git_dirty:?},"#)?;
    writeln!(out, "}}")?;

    Ok(())
}

fn pins(out_dir: &str, root: &str, target: &str) -> io::Result<()> {
    #[derive(Debug, Deserialize)]
    struct Target {
        pins: Pins,
    }

    #[derive(Debug, Deserialize)]
    #[serde(transparent)]
    struct Pins(HashMap<String, PinSpec>);

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum PinSpec {
        Single(u8),
        Multiple(Vec<u8>),
        Nested(Pins),
    }

    impl Pins {
        fn format(self, output: &mut String, prefix: &str, path: &str) {
            for (key, value) in self.0 {
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
        "pins.gpio"
    } else if feature("CHIP_RP") {
        "PIN_"
    } else {
        return Ok(());
    };

    let path = format!("{root}/../targets/{target}.toml");
    println!("cargo:rerun-if-changed={path}");

    let target = fs::read_to_string(path)?;
    let target: Target = basic_toml::from_str(&target).unwrap();

    let mut out = String::from("macro_rules! pins {\n");
    target.pins.format(&mut out, gpio, "");
    out.push_str("}\n");

    fs::write(format!("{out_dir}/pins.rs"), out)
}

fn feature(feature: &str) -> bool {
    env::var_os(format!("CARGO_FEATURE_{feature}")).is_some()
}
