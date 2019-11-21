extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=vips");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rustc-link-search=native=/usr/include/vips/");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        //.whitelist_function("vips_thumbnail_buffer")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("write bindings!");
}
