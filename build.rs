//! This build script is only needed for the examples. When just building the library,
//! it still runs but should have no effect.

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Guard against including the linker script when building as a library dependency
    if env::var_os("CARGO_PRIMARY_PACKAGE").and(env::var_os("CARGO_BIN_NAME")).is_some() {
        // Put the linker script somewhere the linker can find it
        let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
        File::create(out.join("memory.x"))
            .unwrap()
            .write_all(include_bytes!("memory.x"))
            .unwrap();
        println!("cargo:rustc-link-search={}", out.display());

        // Only re-run the build script when memory.x is changed,
        // instead of when any part of the source code changes.
        println!("cargo:rerun-if-changed=memory.x");
    }
}
