use bvh_anim::Bvh;
use std::{fs::File, io::BufReader};

#[test]
fn test_load_success() {
    let reader = File::open("./data/test_mocapbank.bvh")
        .map(BufReader::new)
        .unwrap();

    let bvh = Bvh::load(reader).unwrap();
    println!("{:?}", bvh);
}
