#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate bvh_anim;

fuzz_target!(|data: &[u8]| {
    let _ = bvh_anim::from_bytes(data);
});
