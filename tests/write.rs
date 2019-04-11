use bvh_anim::bvh;

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
Frames: 1
Frame Time: 0.033333333333
0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0
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
        Frames: 1
        Frame Time: 0.033333333333
        0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0
    };

    let bvh_string = bvh.to_string();
    assert_eq!(bvh_string, BVH_STRING);
}
