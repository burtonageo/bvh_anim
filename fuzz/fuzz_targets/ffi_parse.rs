#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate bvh_anim;

use bvh_anim::ffi::{bvh_parse, bvh_destroy, bvh_BvhFile};

fuzz_target!(|data: &[u8]| {
    let mut bvh = bvh_BvhFile::default();
    let _ = bvh_parse(&mut bvh);
    let _ = bvh_destroy(&mut bvh);
});
