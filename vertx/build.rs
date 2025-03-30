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

    if !feature("SIMULATOR") {
        let target_name = env::var("VERTX_TARGET").expect("VERTX_TARGET should be set");
        println!("cargo::rerun-if-env-changed=VERTX_TARGET");

        memory_layout(out_dir, root);
        link_args();
        target_macro(out_dir, root, &target_name)?;
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

fn target_macro(out_dir: &str, root: &str, target: &str) -> io::Result<()> {
    #[derive(Debug, Deserialize)]
    struct Target {
        #[expect(unused)]
        chip: String,
        leds: Leds,
        sd: Sd,
        spi: Option<Spi>,
        display: Display,
        #[serde(flatten)]
        rest: Tree,
    }

    #[derive(Debug, Deserialize)]
    struct Leds {
        timer: Option<String>,
        dma: Option<String>,
        #[serde(flatten)]
        pins: Tree,
    }

    #[derive(Debug, Deserialize)]
    struct Sd {
        #[serde(rename = "type")]
        _type: SdType,
        #[serde(flatten)]
        pins: Tree,
    }

    #[derive(Debug, Deserialize)]
    struct Spi {
        peripheral: Option<String>,
        #[serde(flatten)]
        pins: Tree,
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
        pins: Tree,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum DisplayType {
        Ssd1306,
    }

    #[derive(Debug, Deserialize, Default)]
    #[serde(transparent)]
    struct Tree(HashMap<String, Node>);

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum Node {
        Single(Spec),
        Multiple(Vec<Spec>),
        Tree(Tree),
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum Arm {
        Pin(Spec),
        PinArray(Vec<Spec>),
        Peripheral(String),
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum Spec {
        Number(u8),
        Name(String),
    }

    impl fmt::Display for Spec {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Spec::Number(num) => write!(f, "{num}"),
                Spec::Name(name) => f.write_str(name),
            }
        }
    }

    let path = format!("{root}/../targets/{target}.toml");
    println!("cargo::rerun-if-changed={path}");

    let target = fs::read_to_string(path)?;
    let target: Target = basic_toml::from_str(&target).unwrap();

    let (spi, spi_pins) = target
        .spi
        .map(|spi| (spi.peripheral, spi.pins))
        .unwrap_or_default();

    let mut arms = Vec::new();
    let peripherals = [
        ("leds.timer", target.leds.timer),
        ("leds.dma", target.leds.dma),
        ("spi", spi),
    ];
    for (path, peripheral) in peripherals {
        if let Some(peripheral) = peripheral {
            arms.push((path.to_owned(), Arm::Peripheral(peripheral)));
        }
    }

    fn pins(tree: Tree, prefix: &str) -> impl Iterator<Item = (String, Node)> {
        tree.0.into_iter().map(move |(mut key, spec)| {
            if !prefix.is_empty() {
                key = format!("{prefix}.{key}");
            }
            (key, spec)
        })
    }
    let mut stack = pins(target.leds.pins, "leds")
        .chain(pins(target.sd.pins, "sd"))
        .chain(pins(spi_pins, "spi"))
        .chain(pins(target.display.pins, "display"))
        .chain(pins(target.rest, ""))
        .collect::<Vec<_>>();
    while let Some((key, spec)) = stack.pop() {
        match spec {
            Node::Single(pin) => arms.push((key, Arm::Pin(pin))),
            Node::Multiple(pin) => arms.push((key, Arm::PinArray(pin))),
            Node::Tree(inner) => {
                let iter = pins(inner, &key);
                stack.extend(iter);
            }
        }
    }

    let gpio = if feature("CHIP_ESP") {
        "GPIO"
    } else if feature("CHIP_RP") {
        "PIN_"
    } else {
        unreachable!("unknown chip");
    };

    let out = File::create(format!("{out_dir}/target_macro.rs"))?;
    let out = &mut BufWriter::new(out);

    writeln!(out, "macro_rules! target {{")?;
    for (key, arm) in arms {
        match arm {
            Arm::Pin(spec) => writeln!(out, "    ($p:expr, {key}) => {{ $p.{gpio}{spec} }};")?,
            Arm::PinArray(specs) => {
                writeln!(out, "    ($p:expr, {key} $(.$method:ident())*) => {{ &[")?;
                for spec in specs {
                    writeln!(out, "        $p.{gpio}{spec} $(.$method())*,")?;
                }
                writeln!(out, "    ] }};")?;
            }
            Arm::Peripheral(name) => writeln!(out, "    ($p:expr, {key}) => {{ $p.{name} }};")?,
        }
    }
    writeln!(out, "}}")
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
