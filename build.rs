extern crate bindgen;
extern crate pkg_config;

use std::env;
use std::path::PathBuf;

fn main() {
    let libvips = pkg_config::probe_library("vips").expect("find libvips");
    println!("cargo:rerun-if-changed=wrapper.h");

    let mut builder = bindgen::Builder::default().header("wrapper.h");
    for path in libvips.include_paths {
        builder = builder.clang_arg(format!("-I{}", path.display()));
    }

    let bindings = builder
        .whitelist_function("vips_thumbnail_buffer")
        .whitelist_function("vips_jpegsave_buffer")
        .whitelist_function("vips_error_buffer")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("write bindings!");
}
