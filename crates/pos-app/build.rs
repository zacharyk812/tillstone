//! Embeds every business template in the repository's `templates/` folder,
//! so a fork adds a template by adding a file, with no code changes.

use std::{env, fs, path::PathBuf};

fn main() {
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../templates");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let mut out = String::from("&[\n");
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let id = path.file_stem().unwrap().to_string_lossy().into_owned();
        let path = path.canonicalize().unwrap();
        out.push_str(&format!("    ({id:?}, include_str!({:?})),\n", path.display().to_string()));
    }
    out.push(']');
    fs::write(PathBuf::from(env::var("OUT_DIR").unwrap()).join("templates.rs"), out).unwrap();
}
