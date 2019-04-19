#![cfg(feature = "ffi")]

use bvh_anim::{bvh, Bvh};

#[test]
fn ffi_convert() {
    let bvh = bvh! {
        HIERARCHY
        ROOT Base
        {
            OFFSET 0.0 0.0 0.0
            CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation
            JOINT Middle1
            {
                OFFSET 0.0 0.0 15.0
                CHANNELS 3 Zrotation Xrotation Yrotation
                JOINT Tip1
                {
                    OFFSET 0.0 0.0 30.0
                    CHANNELS 3 Zrotation Xrotation Yrotation
                    End Site
                    {
                        OFFSET 0.0 0.0 45.0
                    }
                }
            }
            JOINT Middle2
            {
                OFFSET 0.0 15.0 0.0
                CHANNELS 3 Zrotation Xrotation Yrotation
                JOINT Tip2
                {
                    OFFSET 0.0 30.0 0.0
                    CHANNELS 3 Zrotation Xrotation Yrotation
                    End Site
                    {
                        OFFSET 0.0 45.0 0.0
                    }
                }
            }
        }

        MOTION
        Frames: 3
        // Time in seconds.
        Frame Time: 0.033333333333
        0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0
        1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0
        2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0
    };

    let bvh_clone = bvh.clone();
    let from_ffi = unsafe {
        let ffi = bvh.into_ffi();
        Bvh::from_ffi(ffi).unwrap()
    };
    assert_eq!(bvh_clone, from_ffi);
}

#[test]
fn ffi_load_from_cfile() {
    use bvh_anim::ffi::{bvh_BvhFile, bvh_read};
    use libc;
    use std::{ffi::CStr, ptr};

    let expected_bvh = bvh! {
        HIERARCHY
        ROOT Base
        {
            OFFSET 0.0 0.0 0.0
            CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation
            JOINT End
            {
                OFFSET 0.0 0.0 15.0
                CHANNELS 3 Zrotation Xrotation Yrotation
                End Site
                {
                    OFFSET 0.0 0.0 30.0
                }
            }
        }
        MOTION
        Frames: 2
        Frame Time: 0.033333333
        0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0
        1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0
    };

    unsafe {
        let file = libc::fopen(
            CStr::from_bytes_with_nul(b"./data/test_simple.bvh\0")
                .unwrap()
                .as_ptr(),
            CStr::from_bytes_with_nul(b"r\0").unwrap().as_ptr(),
        );

        assert_ne!(file, ptr::null_mut());

        let mut bvh = bvh_BvhFile::default();
        let result = bvh_read(file, &mut bvh);
        libc::fclose(file);

        assert_eq!(result, 0);

        let bvh = Bvh::from_ffi(bvh).unwrap();
        assert_eq!(bvh, expected_bvh);
    }
}
