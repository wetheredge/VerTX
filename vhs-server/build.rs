use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fmt, fs};

const DIRECTORY: &str = "::picoserve::response::fs::Directory";
const FILE: &str = "::picoserve::response::fs::File";

#[derive(Debug, Default)]
struct Directory {
    sub_directories: HashMap<String, Directory>,
    files: Vec<(String, File)>,
}

impl fmt::Display for Directory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{DIRECTORY} {{ files: &[")?;

        for (name, file) in &self.files {
            write!(f, "({name:?}, {file}),")?;
        }

        write!(f, "], sub_directories: &[")?;

        for (name, dir) in &self.sub_directories {
            write!(f, "({name:?}, {dir}),")?;
        }

        write!(f, "] }}")
    }
}

#[derive(Debug)]
struct File {
    path: PathBuf,
    content_type: &'static str,
}

impl File {
    fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let content_type = match path.extension().map(|s| s.to_string_lossy()).as_deref() {
            Some("html") => "text/html; charset=UTF-8",
            Some("js") => "text/javascript; charset=UTF-8",
            Some("svg") => "image/svg+xml; charset=UTF-8",
            _ => todo!("unknown content type"),
        };

        Self {
            path: path.to_path_buf(),
            content_type,
        }
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{FILE}::with_content_type({:?}, include_bytes!({:?}))"#,
            self.content_type, self.path
        )
    }
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();

    let web = format!("{root}/../vhs-web/dist");
    println!("cargo:rerun-if-changed={web}");
    // let assets = format!("{web}/assets");

    // let mut tree = Directory::default();
    //
    // for entry in WalkDir::new(&assets).sort_by_file_name() {
    //     let entry = entry.unwrap();
    //
    //     if entry.path().is_dir() {
    //         continue;
    //     }
    //
    //     let relative = entry.path().strip_prefix(&assets).unwrap();
    //
    //     let mut dir = &mut tree;
    //     if let Some(parent) = relative.parent() {
    //         for component in parent.components() {
    //             let component: &Path = component.as_ref();
    //
    //             dir = dir
    //                 .sub_directories
    //                 .entry(component.to_string_lossy().into_owned())
    //                 .or_insert_with(Directory::default);
    //         }
    //     }
    //
    //     let name = if relative.extension() == Some(OsStr::new("html")) {
    //         relative.file_stem()
    //     } else {
    //         relative.file_name()
    //     };
    //
    //     dir.files.push((
    //         name.unwrap().to_string_lossy().into_owned(),
    //         File::new(entry.path()),
    //     ));
    // }

    let index = File::new(format!("{web}/index.html"));
    // let favicon = File::new(format!("{web}/favicon.svg"));

    let out = dbg!(format!("{out_dir}/web_assets.rs"));
    let mut out = fs::File::create(out).unwrap();
    // writeln!(&mut out, "pub const ASSETS: {DIRECTORY} = {tree};").unwrap();
    writeln!(&mut out, "pub const INDEX: {FILE} = {index};").unwrap();
    // writeln!(&mut out, "pub const FAVICON: {FILE} = {favicon};").unwrap();
}
