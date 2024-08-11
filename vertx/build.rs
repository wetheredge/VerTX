use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::process::Command;

use quote::{format_ident, quote};
use serde::Deserialize;

fn main() -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let target_name = env!("VERTX_TARGET");

    println!("cargo:rerun-if-env-changed=VERTX_TARGET");

    link_args();
    build_info(&out_dir, &root, target_name)?;
    pins(&out_dir, &root, target_name)?;

    Ok(())
}

fn link_args() {
    fn chip(chip: &str) -> bool {
        env::var_os(format!("CARGO_FEATURE_CHIP_{chip}")).is_some()
    }

    let args: &[&str] = if chip("ESP") {
        &["-Tlinkall.x", "-Trom_functions.x", "-nostartfiles"]
    } else {
        &[]
    };

    for arg in args {
        println!("cargo::rustc-link-arg-bins={arg}");
    }
}

// Written as a macro to avoid needing to name the private output type of
// quote!()
macro_rules! out_file {
    ($out:expr, $tokens:expr) => {{
        let parsed = syn::parse2($tokens).unwrap();
        let formatted = prettyplease::unparse(&parsed);
        fs::write($out, formatted)
    }};
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

    let path = format!("{root}/../targets/{target}.toml");
    println!("cargo:rerun-if-changed={path}");

    let target = fs::read_to_string(path)?;
    let target: Target = basic_toml::from_str(&target).unwrap();

    let pins_arms = target.pins.iter().map(|(name, spec)| {
        let name = format_ident!("{name}");
        match spec {
            PinSpec::Single(pin) => {
                let gpio = format_ident!("gpio{pin}");
                quote!( ($pins:expr, #name $(,)?) => { $pins.#gpio }; )
            }
            PinSpec::Multiple(pins) => {
                let gpios = pins.iter().map(|pin| format_ident!("gpio{pin}"));
                quote!( ($pins:expr, #name $(. $method:ident ())* $(,)?) => { [#($pins.#gpios $(.$method())*)*] }; )
            }
        }
    });

    let pins_type_arms = target.pins.iter().map(|(name, spec)| {
        let name = format_ident!("{name}");
        match spec {
            PinSpec::Single(pin) => quote!( (#name) => { #pin }; ),
            PinSpec::Multiple(pins) => {
                let count = pins.len();
                quote!( (#name count) => { #count }; )
            }
        }
    });

    let tokens = quote! {
        macro_rules! pins {
            #(#pins_arms)*
        }

        #[allow(unused)]
        macro_rules! Pins {
            #(#pins_type_arms)*
        }
    };

    out_file!(format!("{out_dir}/pins.rs"), tokens)
}
