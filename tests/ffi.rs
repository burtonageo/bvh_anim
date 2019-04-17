#![cfg(feature = "ffi")]

use bvh_anim::{Bvh, bvh};

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
