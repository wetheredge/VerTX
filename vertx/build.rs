use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::process::Command;
use std::{env, fs, io};

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chip {
    Esp,
    Rp,
    Stm32,
}

impl Chip {
    fn get() -> Self {
        if feature("CHIP_ESP") {
            Self::Esp
        } else if feature("CHIP_RP") {
            Self::Rp
        } else if feature("CHIP_STM32") {
            Self::Stm32
        } else {
            unreachable!("unknown chip")
        }
    }
}

fn main() -> io::Result<()> {
    let out_dir = &env::var("OUT_DIR").unwrap();
    let root = &env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo::rustc-check-cfg=cfg(peripheral, values(any()))");

    let config = format!("{root}/../vertx-config/out/config.rs");
    println!("cargo::rerun-if-changed={config}");
    fs::copy(config, format!("{out_dir}/config.rs"))?;

    fs::write(
        format!("{out_dir}/qr_url"),
        env!("CARGO_PKG_HOMEPAGE").to_ascii_uppercase(),
    )?;

    build_info(out_dir)?;

    if !feature("SIMULATOR") {
        let chip = Chip::get();
        memory_layout(out_dir, root, chip);
        link_args(chip);
    }

    if feature("NETWORK") {
        configurator(out_dir, root)?;
    }

    Ok(())
}

fn memory_layout(out_dir: &str, root: &str, chip: Chip) {
    let path = (chip == Chip::Rp).then_some("src/hal/chip/rp/memory.x");

    if let Some(path) = path {
        fs::copy(format!("{root}/{path}"), format!("{out_dir}/memory.x"))
            .expect("copying memory.x");

        println!("cargo::rustc-link-search={out_dir}");
        println!("cargo::rerun-if-changed={root}/{path}");
    }
}

fn link_args(chip: Chip) {
    let mut args = match chip {
        Chip::Esp => vec!["-Tlinkall.x", "-nostartfiles"],
        Chip::Rp => vec!["--nmagic", "-Tlink.x", "-Tlink-rp.x"],
        Chip::Stm32 => vec!["--nmagic", "-Tlink.x"],
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
