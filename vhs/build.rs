use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::process::Command;

use serde::Deserialize;

fn main() -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let target_name = env::var("VHS_TARGET").unwrap();

    println!("cargo:rerun-if-env-changed=VHS_TARGET");

    build_info(&out_dir, &root, &target_name)?;
    pins(&out_dir, &root, &target_name)?;
    web_assets(&out_dir, &root)?;

    Ok(())
}

fn build_info(out_dir: &str, root: &str, target_name: &str) -> io::Result<()> {
    let git = |args: &[_]| Command::new("git").args(args).output().unwrap().stdout;
    let git_string = |args| String::from_utf8(git(args)).unwrap().trim().to_owned();

    let manifest = format!("{root}/Cargo.toml");

    println!("cargo:rerun-if-changed={manifest}");

    let manifest = cargo_toml::Manifest::from_path(manifest).unwrap();
    let version = manifest.package.unwrap().version.unwrap();
    let (major, version) = version.split_once('.').unwrap();
    let (minor, version) = version.split_once('.').unwrap();
    let (patch, suffix) = version.split_once('-').unwrap_or((version, ""));

    let git_branch = git_string(&["branch", "--show-current"]);
    let git_commit = git_string(&["rev-parse", "--short", "HEAD"]);
    let git_dirty = !git(&["status", "--porcelain"]).is_empty();

    let out = File::create(format!("{out_dir}/build_info.rs"))?;
    let mut out = BufWriter::new(out);
    writeln!(&mut out, "response::BuildInfo {{")?;
    writeln!(&mut out, "    target: {target_name:?},")?;
    writeln!(&mut out, "    major: {major},")?;
    writeln!(&mut out, "    minor: {minor},")?;
    writeln!(&mut out, "    patch: {patch},")?;
    writeln!(&mut out, "    suffix: {suffix:?},")?;
    writeln!(&mut out, "    debug: cfg!(debug_assertions),")?;
    writeln!(&mut out, "    git_branch: {git_branch:?},")?;
    writeln!(&mut out, "    git_commit: {git_commit:?},")?;
    writeln!(&mut out, "    git_dirty: {git_dirty:?},")?;
    writeln!(&mut out, "}}")?;

    Ok(())
}

fn pins(out_dir: &str, root: &str, target: &str) -> io::Result<()> {
    #[derive(Debug, Deserialize)]
    struct Target {
        pins: HashMap<String, PinSpec>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum PinSpec {
        Single(u8),
        Multiple(Vec<u8>),
    }

    let path = format!("{root}/targets/{target}.json");
    println!("cargo:rerun-if-changed={path}");
    let target = fs::read_to_string(path)?;
    let target: Target = serde_json::from_str(&target).unwrap();

    let out = format!("{out_dir}/pins.rs");
    let out = File::create(out)?;
    let mut out = BufWriter::new(out);

    writeln!(&mut out, "macro_rules! pins {{")?;
    for (name, spec) in &target.pins {
        let get_pin = |pin| format!("$pins.gpio{pin}");

        match spec {
            PinSpec::Single(pin) => {
                writeln!(&mut out, "($pins:expr, {name}) => {{ {} }};", get_pin(pin))?;
            }
            PinSpec::Multiple(pins) => {
                write!(
                    &mut out,
                    "($pins:expr, {name} $(. $method:ident ())* $(,)?) => {{ ["
                )?;
                for pin in pins {
                    write!(&mut out, "{} $(.$method())*", get_pin(pin))?;
                }
                writeln!(&mut out, "] }};")?;
            }
        }
    }
    writeln!(&mut out, "}}")?;

    writeln!(&mut out, "macro_rules! Pins {{")?;
    for (name, spec) in &target.pins {
        match spec {
            PinSpec::Single(pin) => {
                writeln!(&mut out, "({name}) => {{ {pin} }};")?;
            }
            PinSpec::Multiple(pins) => {
                writeln!(&mut out, "({name} count) => {{ {} }};", pins.len())?;
            }
        }
    }
    writeln!(&mut out, "}}")
}

fn web_assets(out_dir: &str, root: &str) -> io::Result<()> {
    #[derive(Debug, Deserialize)]
    struct Asset {
        route: String,
        file: String,
        mime: String,
        gzip: bool,
    }

    let Ok(web) = fs::canonicalize(format!("{root}/../vhs-web/dist")) else {
        panic!("vhs-web must be built first")
    };
    println!("cargo:rerun-if-changed={}", web.display());

    let assets = fs::read_to_string(web.join("assets.json"))?;
    let assets: Vec<Asset> = serde_json::from_str(&assets).unwrap();

    let out = format!("{out_dir}/router.rs");
    let out = File::create(out)?;
    let mut out = BufWriter::new(out);
    writeln!(&mut out, "macro_rules! router {{")?;
    writeln!(&mut out, "    ($($route:literal => $handler:expr)*) => {{")?;
    writeln!(&mut out, "    ::picoserve::Router::new()")?;
    for Asset {
        route,
        file,
        mime,
        gzip,
    } in assets
    {
        let headers = if gzip {
            r#"("Content-Encoding", "gzip")"#
        } else {
            ""
        };

        let path = web.join(file);

        writeln!(
            &mut out,
            "        .route({route:?}, get(|| \
             ::picoserve::response::fs::File::with_content_type_and_headers({mime:?}, \
             include_bytes!({path:?}), &[{headers}])))"
        )?;
    }
    writeln!(&mut out, "        $( .route($route, $handler) )*")?;
    writeln!(&mut out, "    }};")?;
    writeln!(&mut out, "}}")?;

    Ok(())
}
