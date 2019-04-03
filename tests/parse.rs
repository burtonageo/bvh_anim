use bvh_anim::Bvh;
use std::{fs::File, io::BufReader};

#[test]
fn test_load_success() {
    let reader = File::open("./data/test_mocapbank.bvh")
        .map(BufReader::new)
        .unwrap();

    let bvh = Bvh::load(reader).unwrap();
    for joint in bvh.joints() {
        println!("{:#?}", joint);
    }
    println!("Frame time: {:?}", bvh.clips().frame_time());

    for frame in bvh.clips().frames() {
        println!("{:?}", frame);
    }
}