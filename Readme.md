# bvh_anim

[![Latest Version]][crates.io] [![Documentation]][docs.rs] ![License]

A rust library for loading `.bvh` files containing skeletal animation data.

⚠⚠**NOTE**: This library is currently alpha quality software.⚠⚠

## Basic usage

To get started, add the following to your `Cargo.toml`:

```toml
[dependencies]
bvh_anim = "0.4"
```

And then, you can import the library using the `use bvh_anim::*;` statement
in your rust files. A small example is shown below:

```rust
use bvh_anim;
use std::fs::File;
use std::io::BufReader;

let bvh_file = File::open("./path/to/anim.bvh")?;
let bvh = bvh_anim::from_reader(BufReader::new(bvh_file))?;

for joint in bvh.joints() {
    println!("{:#?}", joint);
}

println!("Frame time: {:?}", bvh.frame_time());

for frame in bvh.frames() {
    println!("{:?}", frame);
}

let mut out_file = File::create("./out.bvh");
bvh.write_to(&mut out_file)?;
```

For more information about the bvh file format and using this library,
see the documentation on [docs.rs](https://docs.rs/bvh_anim).

## Features

This crate has a small ffi module which allows you to parse `bvh` files
from `C` code. The `ffi` module can be enabled with the `ffi` feature,
and you can read the docs for it on [`docs.rs`][docs.rs/ffi].

In addition, the `bindings` feature can be enabled to generate the `C`
bindings using `cbindgen`. The bindings header is written to either
`$CARGO_TARGET_DIR` if it is specified, or
`$CARGO_MANIFEST_DIR/target/include/bvh_anim/bvh_anim.h` if it is
not.

## Contributing

This library welcomes open source contributions, including pull requests and bug
reports (including feature requests).

This library aims to be the primary `bvh` parser in the Rust ecosystem, and aims
to correctly parse a wide variety of `bvh` files. If you have a file which does
not parse correctly, please report a bug. Parsing should always return an error on
failure and never panic.

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

[Documentation]: https://docs.rs/bvh_anim/badge.svg
[Latest Version]: https://img.shields.io/crates/v/bvh_anim.svg
[docs.rs]: https://docs.rs/bvh_anim
[crates.io]: https://crates.io/crates/bvh_anim
[License]: https://img.shields.io/crates/l/bvh_anim.svg
<!--
Remember to update this when a new version is published!!!
-->
[docs.rs/ffi]: https://docs.rs/bvh_anim/0.4.0/bvh_anim/ffi/index.html