use bvh_anim;
use pretty_assertions::assert_eq;
use std::{fs::File, io::BufReader};

#[test]
fn load_success() {
    let reader = File::open("./data/test_mocapbank.bvh")
        .map(BufReader::new)
        .unwrap();

    let _bvh = bvh_anim::from_reader(reader).unwrap();
}

#[test]
fn string_parse_small() {
    const BVH_BYTES: &[u8] = include_bytes!("../data/test_simple.bvh");
    let bvh = bvh_anim::from_bytes(BVH_BYTES).unwrap();

    let bvh_from_macro = bvh_anim::bvh! {
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
        Frame Time: 0.033333333333
        0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0
        1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0
    };

    assert_eq!(bvh, bvh_from_macro);
}

#[test]
#[ignore] // @TODO(burtonageo): Turn this on when the nom parser lands
fn nonstandard_formatting() {
    const BVH_STRING: &[u8] = br#"
    HIERARCHY ROOT Base {
        OFFSET 0.0 0.0
        0.0
        CHANNELS 6
        Xposition
        Yposition
        Zposition
        Zrotation
        Xrotation
        Yrotation
        Joint End {
            Offset
            0.0
            15.0
            0.0
            CHANNELS 3 Zrotation
            Xrotation Yrotation
            End Site { Offset 0.0 0.0 30.0 }
        }
        Joint
        End2
            { Offset 15.0 0.0 0.0
            Channels 3 Zrotation Xrotation
            Yrotation
            End
            Site
            {
                Offset
                30.0
                0.0
                0.0
            }

    }}

    MOTION Frames : 2
    Frame



    Time
    :
    0.0333333333333333333

    0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0
    "#;
    let bvh = bvh_anim::from_bytes(BVH_STRING).unwrap();

    let bvh_from_macro = bvh_anim::bvh! {
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
            JOINT End2
            {
                OFFSET 15.0 0.0 0.0
                CHANNELS 3 Zrotation Xrotation Yrotation
                End Site
                {
                    OFFSET 30.0 0.0 0.0
                }
            }
        }

        MOTION
        Frames: 2
        Frame Time: 0.033333333333
        0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0
        1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0
    };

    assert_eq!(bvh, bvh_from_macro);
}
