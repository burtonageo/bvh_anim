use bstr::ByteSlice;
use bvh_anim::{
    bvh,
    write::{IndentStyle, LineTerminator, WriteOptions},
};
use pretty_assertions::assert_eq;

#[test]
fn test_write() {
    const BVH_STRING: &[u8] = include_bytes!("../data/test_simple.bvh");

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
        Frame Time: 0.033333333
        0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0
        1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0
    };

    let bvh_string = WriteOptions::new()
        .with_offset_significant_figures(1)
        .with_motion_values_significant_figures(1)
        .with_line_terminator(LineTerminator::native())
        .with_indent(IndentStyle::with_spaces(4))
        .write_to_string(&bvh);

    assert_eq!(bvh_string.trim(), BVH_STRING.trim());
}

#[test]
fn test_load_write_is_identical() {
    const BVH_STRING: &str = include_str!("../data/test_simple.bvh");
    let bvh = bvh_anim::from_str(BVH_STRING).unwrap();
    let bvh_string = WriteOptions::new()
        .with_indent(IndentStyle::with_spaces(4))
        .with_offset_significant_figures(1)
        .with_motion_values_significant_figures(1)
        .with_line_terminator(LineTerminator::native())
        .write_to_string(&bvh);

    assert_eq!(bvh_string, BVH_STRING.as_bytes());
}
