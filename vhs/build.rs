use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::process::Command;

use cargo_toml::Manifest;

fn main() -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest = format!("{root}/Cargo.toml");

    println!("cargo:rerun-if-changed={manifest}");

    let Manifest { package, .. } = Manifest::from_path(manifest).unwrap();
    let version = package.unwrap().version.unwrap();
    let (major, version) = version.split_once('.').unwrap();
    let (minor, version) = version.split_once('.').unwrap();
    let (patch, suffix) = version.split_once('-').unwrap_or((version, ""));

    let git_branch = git_string(&["branch", "--show-current"]);
    let git_commit = git_string(&["rev-parse", "--short", "HEAD"]);
    let git_dirty = !git(&["status", "--porcelain"]).is_empty();

    let mut out = File::create(format!("{out_dir}/build_info.rs"))?;
    writeln!(&mut out, "Response::BuildInfo {{")?;
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

fn git(args: &[&str]) -> Vec<u8> {
    Command::new("git").args(args).output().unwrap().stdout
}

fn git_string(args: &[&str]) -> String {
    String::from_utf8(git(args)).unwrap().trim().to_owned()
}
