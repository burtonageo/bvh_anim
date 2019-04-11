# bvh_anim

A rust library for loading `.bvh` files containing skeletal animation data.

## Basic usage

```rust
use std::fs::File;

let bvh_file = File::open("./path/to/anim.bvh").unwrap();
let bvh = bvh_anim::load(BufReader::new(bvh_file)).unwrap();

for joint in bvh.joints() {
    println!("{:#?}", joint);
}

println!("Frame time: {:?}", bvh.frame_time());

for frame in bvh.frames() {
    println!("{:?}", frame);
}
```

For more information, see the documentation on [docs.rs](https://docs.rs/bvh_anim).
