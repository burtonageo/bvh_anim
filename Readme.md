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

## License

Copyright © 2019 George Burton

Permission is hereby granted, free of charge, to any person obtaining a copy of this software
and associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial
portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT
LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
