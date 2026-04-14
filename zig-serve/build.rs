//! Build script: stage `../web/dist/` into `$OUT_DIR/web-dist/` so
//! that `rust-embed` can find it at a location that's inside the
//! cargo build's `OUT_DIR`. Using `OUT_DIR` (instead of a relative
//! path pointing outside the crate) means the embed still works
//! after `cargo package` extracts the crate to
//! `target/package/zig-serve-<ver>/`, where `../web/dist/` no longer
//! resolves to anything.
//!
//! If the source folder doesn't exist or is empty, we still create
//! an empty target directory so the `#[derive(RustEmbed)]`
//! folder-exists check passes. Consumers will then get an empty
//! asset bundle and the runtime handler falls back to a helpful
//! "web bundle not built" message.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    let out_dir = PathBuf::from(env("OUT_DIR"));
    let dest = out_dir.join("web-dist");
    fs::create_dir_all(&dest).expect("failed to create web-dist in OUT_DIR");

    let manifest_dir = PathBuf::from(env("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("..").join("web").join("dist");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", src.display());

    if src.is_dir() {
        copy_dir(&src, &dest).expect("failed to copy web/dist into OUT_DIR");
    }
}

fn env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{key} not set"))
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            fs::create_dir_all(&dest)?;
            copy_dir(&path, &dest)?;
        } else {
            fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}
