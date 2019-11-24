#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::{c_void, CStr};
use std::ptr;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

const NULL: *const c_void = ptr::null_mut();

pub(crate) fn thumbnail(mut img: Vec<u8>) -> Result<Vec<u8>, Error> {
    let src = img.as_mut_ptr();
    let len = img.len();
    let mut vips_img: *mut VipsImage = ptr::null_mut();
    if unsafe { vips_thumbnail_buffer(src as *mut c_void, len, &mut vips_img, 100, NULL) } != 0 {
        return Err(Error::new("vips_thumbnail_buffer failed"));
    };

    let mut len: usize = 0;
    let mut buf: *mut c_void = ptr::null_mut();
    if unsafe { vips_jpegsave_buffer(vips_img, &mut buf, &mut len, NULL) } != 0 {
        return Err(Error::new("vips_jpegsave_buffer failed"));
    };

    Ok(unsafe { Vec::from_raw_parts(buf as *mut u8, len, len) })
}

#[derive(Debug)]
pub(crate) struct Error(String);

impl Error {
    fn new(details: &str) -> Self {
        Self(format!("{}:{}", details, unsafe {
            CStr::from_ptr(vips_error_buffer()).to_string_lossy()
        }))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/*#[test]
fn test_thumbnail() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let img = std::fs::read(root.join("tests/data/img.png")).expect("read img");
    let img_thumb = std::fs::read(root.join("tests/data/img_thumb.jpeg")).expect("read img_thumb");

    let res = thumbnail(img).expect("make thumbnail");
    assert_eq!(res, img_thumb);
}*/
