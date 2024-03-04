use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fmt};

fn main() -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();

    build_info(&out_dir, &root)?;
    web_assets(&out_dir, &root)?;

    Ok(())
}

fn build_info(out_dir: &str, root: &str) -> io::Result<()> {
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

    let mut out = File::create(format!("{out_dir}/build_info.rs"))?;
    writeln!(&mut out, "response::BuildInfo {{")?;
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

fn web_assets(out_dir: &str, root: &str) -> io::Result<()> {
    let compressed_dir = PathBuf::from(format!("{out_dir}/compressed-web-assets"));

    #[derive(Debug)]
    struct FileInfo {
        path: PathBuf,
        content_type: &'static str,
        gzip: bool,
    }

    impl fmt::Display for FileInfo {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let headers = if self.gzip {
                r#"("Content-Encoding", "gzip")"#
            } else {
                ""
            };

            write!(
                f,
                r#"::picoserve::response::fs::File::with_content_type_and_headers({:?}, include_bytes!({:?}), &[{headers}])"#,
                self.content_type, self.path
            )
        }
    }

    let Ok(web) = fs::canonicalize(format!("{root}/../vhs-web/dist")) else {
        panic!("vhs-web must be built first")
    };
    let web = web.to_str().unwrap();
    println!("cargo:rerun-if-changed={web}");

    let new_file_info = |path: &Path| -> io::Result<FileInfo> {
        let content_type = match path.extension().map(|s| s.to_string_lossy()).as_deref() {
            Some("css") => "text/css",
            Some("html") => "text/html; charset=UTF-8",
            Some("js") => "text/javascript",
            Some("svg") => "image/svg+xml",
            extension => todo!("unknown file extension: `{extension:?}`"),
        };

        let raw_len = {
            let mut data = Vec::new();
            File::open(path)?.read_to_end(&mut data)?;
            data.len()
        };

        let mut compressed = Vec::new();
        let reader = File::open(path)?;
        let reader = BufReader::new(reader);
        let mut reader = flate2::bufread::GzEncoder::new(reader, flate2::Compression::best());
        reader.read_to_end(&mut compressed)?;

        let gzip = compressed.len() < raw_len;

        let path = if gzip {
            let path = compressed_dir.join(path.strip_prefix(web).unwrap());
            fs::create_dir_all(path.parent().unwrap())?;
            fs::write(&path, compressed)?;
            path
        } else {
            path.to_path_buf()
        };

        Ok(FileInfo {
            path,
            content_type,
            gzip,
        })
    };

    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(web).sort_by_file_name() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            continue;
        }

        let path = path.strip_prefix(web).unwrap();
        let path = path.to_string_lossy();
        let path = path.trim_end_matches("index.html");
        let path = path.trim_end_matches(".html");

        files.push((path.to_owned(), new_file_info(entry.path())?));
    }

    let out = format!("{out_dir}/router.rs");
    let mut out = File::create(out)?;
    writeln!(&mut out, "macro_rules! router {{")?;
    writeln!(&mut out, "    ($($route:literal => $handler:expr)*) => {{")?;
    writeln!(&mut out, "    ::picoserve::Router::new()")?;
    for (name, file) in files {
        writeln!(&mut out, "        .route(\"/{name}\", get(|| {file}))")?;
    }
    writeln!(&mut out, "        $( .route($route, $handler) )*")?;
    writeln!(&mut out, "    }};")?;
    writeln!(&mut out, "}}")?;

    Ok(())
}
