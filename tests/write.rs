use bvh_anim::{
    bvh,
    write::{IndentStyle, WriteOptions},
};

const BVH_STRING: &str = r#"""
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
"""#;

#[ignore] // failing at the moment
#[test]
fn test_write() {
    let bvh = bvh! {
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

    let bvh_string = WriteOptions::new()
        .with_indent(IndentStyle::with_spaces(4))
        .write_to_string(&bvh);

    assert_eq!(bvh_string, BVH_STRING);
}

#[ignore]
#[test]
fn test_load_write_is_identical() {
    const BVH_BYTES: &[u8] = include_bytes!("../data/test_mocapbank.bvh");
    let bvh = bvh_anim::parse(BVH_BYTES).unwrap();
    let bvh_string = WriteOptions::new()
        .with_indent(IndentStyle::with_spaces(2))
        .write_to_string(&bvh);

    assert_eq!(bvh_string, BVH_STRING);
}
