// Copyright © 2019 George Burton
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software
// and associated documentation files (the "Software"), to deal in the Software without restriction,
// including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or substantial
// portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT
// LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
// NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
// WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
// SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![warn(unused_imports, missing_docs)]
#![deny(bare_trait_objects)]

//! # About this library
//! 
//! A small library for loading and manipulating BioVision motion files.
//!
//! ## The bvh file format
//! 
//! The `Bvh` file format is comprised of two main sections: the 'Heirarchy' section,
//! which defines the joints of the skeleton, and the 'Motion' section, which defines
//! the motion values for each channel.
//!
//! This project contains some samples in the [`data` directory][`data` directory].
//!
//! ### Heierarchy
//! 
//! The 'Heirarchy' section defines the skeleton as a tree of joints, where there is
//! a single root joint, and a chain of child joints extending out from each joint,
//! terminated by an 'End Site' section.
//!
//! Each joint has:
//!
//! * A list of channels, which are the degrees of freedom in which the joint may move.
//!   Channels are listed in the order in which the transformation should be applied
//!   to the global transform for the root.
//! * An offset, which is the vector distance from the parent joint.
//!
//! ```text
//! HEIRARCHY
//! ROOT <Root-name>
//! {
//!     OFFSET <Root-offset-x> <Root-offset-y> <Root-offset-z>
//!     CHANNELS <Num-root-joint-channels> Xposition Yposition <other-root-channels ...>
//!     JOINT <Joint-1-name>
//!     {
//!         OFFSET <Joint-1-offset-x> <Joint-1-offset-y> <Joint-1-offset-z>
//!         CHANNELS <Num-joint-1-channels> <Joint-1-channels ...>
//!         JOINT <Joint-2-name>
//!         {
//!             OFFSET <Joint-2-offset-x> <Joint-2-offset-y> <Joint-2-offset-z>
//!             CHANNELS <Num-joint-2-channels> <Joint-2-channels ...>
//!             JOINT <Joint-with-end-site>
//!             {
//!                 OFFSET ...
//!                 CHANNELS ...
//!                 End Site
//!                 {
//!                      OFFSET <end-site-offset-x> <end-site-offset-y> <end-site-offset-z>
//!                 }
//!             }
//!             ... More child joints
//!         }
//!         JOINT <Joint-3-name>
//!         {
//!             OFFSET <Joint-3-offset-x> <Joint-3-offset-y> <Joint-3-offset-z>
//!             CHANNELS <Num-joint-3-channels> <Joint-3-channels ...>
//!             ... More child joints
//!         }
//!         ... More child joints
//!     }
//!     ... More child joints
//! }
//! ```
//!
//! Note that the bvh data is defined in terms of a right-handed coordinate system, where
//! the positive y-axis is the up vector.
//! 
//! ### Motion
//! 
//! The `MOTION` section of the bvh file records the number of frames, the frame time, and
//! defines the full range of motions for each channel, frame by frame.
//! 
//! ```text
//! MOTION
//! Frames: <num-frames>
//! Frame Time: <frame-time>
//! <frame-0-channel-0-value> <frame-0-channel-1-value> <frame-0-channel-2-value> ...
//! <frame-1-channel-0-value> <frame-1-channel-1-value> <frame-1-channel-2-value> ...
//! <frame-2-channel-0-value> <frame-2-channel-1-value> <frame-2-channel-2-value> ...
//! ⋮
//! ```
//! 
//! The frame time is recorded in seconds, and tells the animation system how long each frame
//! of the animation should last for. This value is usually around 0.033333333, which is close
//! to 30 frames per second.
//! 
//! The list of motion values is a matrix, where each row represents a frame. Each column of
//! the row represents a transformation around the channel axis - for example a motion value
//! of 130.0 for an `Xposition` channel would correspond to a rotation of 130.0 degrees around
//! the x-axis.
//! 
//! Note that rotations are conventionally in degrees, although it will be up to your application
//! how to interpret each motion's value.
//! 
//! ## Using this library.
//!
//! ### Creating a [`Bvh`][`Bvh`] struct:
//! 
//! There are a few ways to create a [`Bvh`][`Bvh`] struct:
//! 
//! * You can use the [`load`][`load`] function, which will parse a `BufRead` which may contain a bvh
//!   file. The [`parse`][`parse`] function is a convenient wrapper function to parse an in-memory slice
//!   of bytes as a `bvh` file. Note that the file does not need to be strictly utf-8, although it
//!   should be an ascii-compatible encoding. These functions are also available as associated methods on
//!   the `Bvh` type directly as [`Bvh::load`][`Bvh::load`] and [`Bvh::parse`][`Bvh::parse`]
//! 
//! * You can use the [`bvh!`][`bvh!`] macro to construct a [`Bvh`][`Bvh`] instance in your source files
//!   using the same syntax as you would use for a standard bvh file.
//! 
//! * You can use the [`builder`][`builder`] module to dynamically construct a bvh. This is useful
//!   for converting data from other formats into a [`Bvh`][`Bvh`] struct.
//!
//! * You can create an empty [`Bvh`][`Bvh`] using the [`Bvh::new`][`Bvh::new`] or [`Default::default`]
//!   [`Default::default`] methods.
//!
//! ### Other operations:
//! 
//! Once you have a valid [`Bvh`][`Bvh`] struct, there are a number of ways you can inspect and
//! manipulate it:
//! 
//! * The [`Bvh::joints`][`Bvh::joints`] method can be used to iterate through each [`Joint`][`Joint`]
//!   of the [`Bvh`][`Bvh`]. Each [`Joint`][`Joint`] can be inspected through its [`JointData`]
//!   [`JointData`], which can be obtained with the [`Joint::data`][`Joint::data`] method.
//!
//! * The [`Bvh::frames`][`Bvh::frames`] method returns a [`Frames`][`Frames`] iterator over each
//!   frame of the animation. A [`Frame`][`Frame`] can only be indexed by a [`Channel`][`Channel`]
//!   belonging to an associated [`Joint`][`Joint`] of the [`Bvh`][`Bvh`], although you can convert
//!   it into an [`&[`][`slice`][`f32`][`f32`][`]`][`slice`] using the [`Frame::as_slice`][`Frame::as_slice`] method.
//!
//! * You can serialise the [`Bvh`][`Bvh`] into a [`Write`][`Write`] type using the [`Bvh::write_to`]
//!   [`Bvh::write_to`] method. There is also the [`Bvh::to_bstring`][`Bvh::to_bstring`] method, which
//!   converts the [`Bvh`][`Bvh`] into a [`BString`][`BString`]. Various aspects of the formatting
//!   can be customised using the [`WriteOptions`][`WriteOptions`] type, such as the line termination
//!   style, indentation method, and floating point accuracy.
//!
//! ## Examples
//!
//! This library comes with some example applications, which can be viewed on [Github][Github].
//! 
//! ## Other resources
//! 
//! * More information on this file format can be found [here][bvh_html].
//! * A large library of bvh files is freely available from [CMU's motion capture database]
//!   [CMU's motion capture database].
//!
//! [`data` directory]: https://github.com/burtonageo/bvh_anim/tree/master/data
//! [`bvh`]: struct.Bvh.html
//! [`load`]: fn.load.html
//! [`parse`]: fn.parse.html
//! [`Bvh::load`]: struct.Bvh.html#method.load
//! [`Bvh::parse`]:  struct.Bvh.html#method.parse
//! [`bvh!`]: macro.bvh.html
//! [`builder`]: builder/index.html
//! [`Bvh::new`]: struct.Bvh.html#method.new
//! [`Default::default`]: https://doc.rust-lang.org/stable/std/default/trait.Default.html#tymethod.default
//! [`Bvh::joints`]: struct.Bvh.html#method.joints
//! [`Joint`]: struct.Joint.html
//! [`JointData`]: enum.JointData.html
//! [`Joint::data`]: struct.Joint.html#method.data
//! [`Bvh::frames`]: struct.Bvh.html#method.frames
//! [`Frames`]: struct.Frames.html
//! [`Frame`]: struct.Frame.html
//! [`slice`]: https://doc.rust-lang.org/std/primitive.slice.html
//! [`f32`]: https://doc.rust-lang.org/stable/std/primitive.f32.html
//! [`Channel`]: struct.Channel.html
//! [`Frame::as_slice`]: struct.Frame.html#method.as_slice
//! [`Write`]: https://doc.rust-lang.org/stable/std/io/trait.Write.html
//! [`Bvh::write_to`]: struct.Bvh.html#method.write_to
//! [`Bvh::to_bstring`]: struct.Bvh.html#method.to_bstring
//! [`BString`]: https://docs.rs/bstr/0.1.2/bstr/struct.BString.html
//! [`WriteOptions`]: write/struct.WriteOptions.html
//! [Github]: https://github.com/burtonageo/bvh_anim/tree/master/examples
//! [bvh_html]: https://research.cs.wisc.edu/graphics/Courses/cs-838-1999/Jeff/BVH.html
//! [CMU's motion capture database]: https://sites.google.com/a/cgspeed.com/cgspeed/motion-capture/daz-friendly-release

#[macro_use]
mod macros;

pub mod errors;
pub mod write;

mod joint;
mod parse;

use bstr::{
    io::{BufReadExt, ByteLines},
    BStr, BString, B,
};
use mint::Vector3;
use num_traits::{one, zero, One, Zero};
use std::{
    fmt,
    io::{self, Cursor, Write},
    iter::Enumerate,
    mem,
    ops::{Index, IndexMut, Range},
    str::{self, FromStr},
    time::Duration,
};

pub use joint::{Joint, JointData, JointMut, JointName, Joints, JointsMut};
#[doc(hidden)]
pub use macros::BvhLiteralBuilder;

use errors::{LoadError, ParseChannelError, SetMotionError};

struct CachedEnumerate<I> {
    iter: Enumerate<I>,
    last_enumerator: Option<usize>,
}

impl<I> CachedEnumerate<I> {
    #[inline]
    fn new(iter: Enumerate<I>) -> Self {
        CachedEnumerate {
            iter,
            last_enumerator: None,
        }
    }

    #[inline]
    fn last_enumerator(&self) -> Option<usize> {
        self.last_enumerator
    }
}

impl<I: Iterator> Iterator for CachedEnumerate<I> {
    type Item = <Enumerate<I> as Iterator>::Item;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (curr, item) = self.iter.next()?;
        self.last_enumerator = Some(curr);
        Some((curr, item))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

type EnumeratedLines<'a> = CachedEnumerate<ByteLines<&'a mut dyn BufReadExt>>;

impl EnumeratedLines<'_> {
    pub fn next_non_empty_line(&mut self) -> Option<<Self as Iterator>::Item> {
        let mut next = self.next();
        loop {
            match next {
                None => return None,
                Some((idx, result)) => {
                    let string = match result {
                        Ok(s) => s,
                        Err(e) => return Some((idx, Err(e))),
                    };
                    if string.trim().is_empty() {
                        next = self.next()
                    } else {
                        return Some((idx, Ok(string)));
                    }
                }
            }
        }
    }
}

/// Loads the `Bvh` from the `reader`.
#[inline]
pub fn load<R: BufReadExt>(data: R) -> Result<Bvh, LoadError> {
    Bvh::load(data)
}

/// Parse a sequence of bytes as if it were an in-memory `Bvh` file.
#[inline]
pub fn parse<B: AsRef<[u8]>>(bytes: B) -> Result<Bvh, LoadError> {
    Bvh::parse(bytes)
}

/// A complete `bvh` file.
///
/// You can also create a `Bvh` using the [`bvh!` macro][`bvh!`].
///
/// [`bvh!`]: macro.bvh.html
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Bvh {
    /// The list of joints. If the root joint exists, it is always at
    /// index `0`.
    joints: Vec<JointData>,
    /// The motion values of the `Frame`.
    motion_values: Vec<f32>,
    /// The number of frames in the bvh.
    num_frames: usize,
    /// The number of `Channel`s in the bvh.
    num_channels: usize,
    /// The total time it takes to play one frame.
    frame_time: Duration,
}

impl Bvh {
    /// Create an empty `Bvh`.
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// Parse a sequence of bytes as if it were an in-memory `Bvh` file.
    pub fn parse<B: AsRef<[u8]>>(bytes: B) -> Result<Self, LoadError> {
        Bvh::load(Cursor::new(bytes))
    }

    /// Loads the `Bvh` from the `reader`.F
    pub fn load<R: BufReadExt>(mut reader: R) -> Result<Self, LoadError> {
        let reader: &mut dyn BufReadExt = reader.by_ref();
        let mut lines = CachedEnumerate::new(reader.byte_lines().enumerate());

        let mut bvh = Bvh::default();

        bvh.read_joints(&mut lines)?;
        bvh.read_motion(&mut lines)?;

        Ok(bvh)
    }

    /// Writes the `Bvh` using the `bvh` file format to the `writer`, with
    /// the default formatting options.
    ///
    /// # Notes
    ///
    /// To customise the formatting, see the [`WriteOptions`][`WriteOptions`] type.
    ///
    /// [`WriteOptions`]: write/struct.WriteOptions.html
    #[inline]
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        write::WriteOptions::default().write(self, writer)
    }

    /// Writes the `Bvh` using the `bvh` file format into a `BString` with
    /// the default formatting options.
    ///
    /// # Notes
    ///
    /// To customise the formatting, see the [`WriteOptions`][`WriteOptions`] type.
    ///
    /// [`WriteOptions`]: write/struct.WriteOptions.html
    #[inline]
    pub fn to_bstring(&self) -> BString {
        write::WriteOptions::default().write_to_string(self)
    }

    /// Returns the root joint if it exists, or `None` if the skeleton is empty.
    #[inline]
    pub fn root_joint(&self) -> Option<Joint<'_>> {
        if self.joints.is_empty() {
            None
        } else {
            Some(Joint {
                index: 0,
                joints: &self.joints[..],
            })
        }
    }

    /// Returns an iterator over all the `Joint`s in the `Bvh`.
    #[inline]
    pub fn joints(&self) -> Joints<'_> {
        Joints::iter_root(&self.joints[..])
    }

    /// Returns a mutable iterator over all the joints in the `Bvh`.
    pub fn joints_mut(&mut self) -> JointsMut<'_> {
        JointsMut::iter_root(&mut self.joints[..])
    }

    /// Returns a `Frames` iterator over the frames of the bvh.
    #[inline]
    pub fn frames(&self) -> Frames<'_> {
        Frames {
            motion_values: &self.motion_values[..],
            num_channels: self.num_channels,
            num_frames: self.num_frames,
            curr_frame: 0,
        }
    }

    /// Returns a mutable iterator over the frames of the bvh.
    #[inline]
    pub fn frames_mut(&mut self) -> FramesMut<'_> {
        FramesMut {
            motion_values: &mut self.motion_values[..],
            num_channels: self.num_channels,
            num_frames: self.num_frames,
            curr_frame: 0,
        }
    }

    /// Gets the motion value at `frame` and `Channel`.
    ///
    /// # Panics
    ///
    /// This method will panic if `frame` is greater than `self.num_frames()`.
    #[inline]
    pub fn get_motion(&self, frame: usize, channel: &Channel) -> f32 {
        *self.frames().nth(frame).unwrap().index(channel)
    }

    /// Returns the motion value at `frame` and `channel` if they are in bounds,
    /// `None` otherwise.
    #[inline]
    pub fn try_get_motion(&self, frame: usize, channel: &Channel) -> Option<f32> {
        self.frames()
            .nth(frame)
            .and_then(|f| f.get(channel))
            .map(|m| *m)
    }

    /// Updates the `motion` value at `frame` and `channel` to `new_motion`.
    ///
    /// # Panics
    ///
    /// This method will panic if `frame` is greater than `self.num_frames()`.
    #[inline]
    pub fn set_motion(&mut self, frame: usize, channel: &Channel, new_motion: f32) {
        self.try_set_motion(frame, channel, new_motion).unwrap();
    }

    /// Updates the `motion` value at `frame` and `channel` to `new_motion`.
    ///
    /// # Notes
    ///
    /// Returns `Ok(())` if the `motion` value was successfully set, and `Err(())` if
    /// the operation was out of bounds.
    #[inline]
    pub fn try_set_motion<'a>(
        &mut self,
        frame: usize,
        channel: &'a Channel,
        new_motion: f32,
    ) -> Result<(), SetMotionError<'a>> {
        let m = self
            .frames_mut()
            .nth(frame)
            .ok_or(SetMotionError::BadFrame(frame))
            .and_then(|f| {
                f.get_mut(channel)
                    .ok_or(SetMotionError::BadChannel(channel))
            })?;

            *m = new_motion;
            Ok(())
    }

    /// Get the number of frames in the `Bvh`.
    #[inline]
    pub const fn num_frames(&self) -> usize {
        self.num_frames
    }

    /// Get the number of channels in the `Bvh`.
    #[inline]
    pub const fn num_channels(&self) -> usize {
        self.num_channels
    }

    /// Get the duration each frame should play for in the `Bvh`.
    #[inline]
    pub const fn frame_time(&self) -> &Duration {
        &self.frame_time
    }

    /// Set the duration each frame should play for in the `Bvh` to `new_frame_time`.
    #[inline]
    pub fn set_frame_time(&mut self, new_frame_time: Duration) {
        self.frame_time = new_frame_time;
    }
}

impl fmt::Display for Bvh {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.to_bstring(), f)
    }
}

/// A `Channel` composed of a `ChannelType` and an index into the
/// corresponding motion data.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Channel {
    /// The type of the `Channel`.
    channel_type: ChannelType,
    /// The index into the `Frame` which corresponds to this `Channel`.
    motion_index: usize,
}

impl Channel {
    #[inline]
    const fn new(channel_type: ChannelType, motion_index: usize) -> Self {
        Channel {
            channel_type,
            motion_index,
        }
    }

    /// Returns the `ChannelType` to which this `Channel` corresponds.
    #[inline]
    pub const fn channel_type(&self) -> ChannelType {
        self.channel_type
    }

    /// Returns the index of the motion value to which this `Channel` corresponds.
    #[inline]
    pub const fn motion_index(&self) -> usize {
        self.motion_index
    }
}

/// The available degrees of freedom along which a `Joint` may be manipulated.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChannelType {
    /// Can be rotated along the `x` axis.
    RotationX,
    /// Can be rotated along the `y` axis.
    RotationY,
    /// Can be rotated along the `z` axis.
    RotationZ,
    /// Can be translated along the `x` axis.
    PositionX,
    /// Can be translated along the `y` axis.
    PositionY,
    /// Can be translated along the `z` axis.
    PositionZ,
}

impl ChannelType {
    /// Attempt to parse a bvh channel string into a `ChannelType`.
    /// Returns `Err` if the string cannot be parsed.
    #[inline]
    pub fn from_bstr(s: &BStr) -> Result<Self, ParseChannelError> {
        match s.as_bytes() {
            b"Xrotation" => Ok(ChannelType::RotationX),
            b"Yrotation" => Ok(ChannelType::RotationY),
            b"Zrotation" => Ok(ChannelType::RotationZ),

            b"Xposition" => Ok(ChannelType::PositionX),
            b"Yposition" => Ok(ChannelType::PositionY),
            b"Zposition" => Ok(ChannelType::PositionZ),

            _ => Err(ParseChannelError::from(s)),
        }
    }

    /// Returns `true` if this channel corresponds to a rotational
    /// transform, otherwise `false`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bvh_anim::ChannelType;
    /// let channel_type = ChannelType::RotationX;
    /// assert!(channel_type.is_rotation());
    /// ```
    #[inline]
    pub fn is_rotation(&self) -> bool {
        match *self {
            ChannelType::RotationX | ChannelType::RotationY | ChannelType::RotationZ => true,
            _ => false,
        }
    }

    /// Returns `true` if this channel corresponds to a positional
    /// transform, otherwise `false`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bvh_anim::ChannelType;
    /// let channel_type = ChannelType::PositionZ;
    /// assert!(channel_type.is_position());
    /// ```
    #[inline]
    pub fn is_position(&self) -> bool {
        !self.is_rotation()
    }

    /// Get the `Axis` about which this `Channel` transforms.
    ///
    /// # Example
    ///
    /// ```
    /// # use bvh_anim::{Axis, ChannelType};
    /// let channel_type = ChannelType::PositionX;
    /// assert_eq!(channel_type.axis(), Axis::X);
    /// ```
    #[inline]
    pub fn axis(&self) -> Axis {
        match *self {
            ChannelType::RotationX | ChannelType::PositionX => Axis::X,
            ChannelType::RotationY | ChannelType::PositionY => Axis::Y,
            ChannelType::RotationZ | ChannelType::PositionZ => Axis::Z,
        }
    }

    /// Returns the `Vector3` of the channel axis.
    #[inline]
    // @TODO: remove `Clone` bound when
    // https://github.com/kvark/mint/commit/8c6c501e442152e776a17322dff10e723bf0eeda
    // is published
    pub fn axis_vector<T: Clone + One + Zero>(&self) -> Vector3<T> {
        self.axis().vector::<T>()
    }

    /// Returns the string representation of the `ChannelType`.
    #[inline]
    pub fn as_str(&self) -> &'static str {
        match *self {
            ChannelType::RotationX => "Xrotation",
            ChannelType::RotationY => "Yrotation",
            ChannelType::RotationZ => "Zrotation",

            ChannelType::PositionX => "Xposition",
            ChannelType::PositionY => "Yposition",
            ChannelType::PositionZ => "Zposition",
}
    }

    /// Returns the string representation of the `ChannelType`.
    #[inline]
    pub fn as_bstr(&self) -> &'static BStr {
        B(self.as_str())
    }
}

impl FromStr for ChannelType {
    type Err = ParseChannelError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ChannelType::from_bstr(From::from(s))
    }
}

impl fmt::Display for ChannelType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An enum which represents an axis along a direction in 3D space.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Axis {
    /// `x` axis.
    X,
    /// `y` axis.
    Y,
    /// `z` axis.
    Z,
}

impl Axis {
    /// Returns the `Vector3` which represents the axis.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bvh_anim::Axis;
    ///
    /// assert_eq!(Axis::X.vector(), [1.0, 0.0, 0.0].into());
    /// assert_eq!(Axis::Y.vector(), [0.0, 1.0, 0.0].into());
    /// assert_eq!(Axis::Z.vector(), [0.0, 0.0, 1.0].into());
    /// ```
    #[inline]
    // @TODO: remove `Clone` bound when
    // https://github.com/kvark/mint/commit/8c6c501e442152e776a17322dff10e723bf0eeda
    // is published
    pub fn vector<T: Clone + One + Zero>(&self) -> Vector3<T> {
        let (_1, _0) = (one, zero);
        match *self {
            Axis::X => [_1(), _0(), _0()].into(),
            Axis::Y => [_0(), _1(), _0()].into(),
            Axis::Z => [_0(), _0(), _1()].into(),
        }
    }
}

impl fmt::Display for Axis {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        };
        f.write_str(s)
    }
}

/// An iterator over the frames of a `Bvh`.
#[derive(Debug)]
pub struct Frames<'a> {
    motion_values: &'a [f32],
    num_channels: usize,
    num_frames: usize,
    curr_frame: usize,
}

impl Frames<'_> {
    /// Returns the number of `Frame`s left to iterate over.
    #[inline]
    pub const fn len(&self) -> usize {
        self.num_frames - self.curr_frame
    }

    /// Returns `true` if the number of `Frame`s left to iterate over is `0`.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a> Iterator for Frames<'a> {
    type Item = &'a Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let range = frames_iter_logic(self.num_channels, self.num_frames, &mut self.curr_frame)?;
        Some(Frame::from_slice(&self.motion_values[range]))
    }
}

/// A mutable iterator over the frames of a `Bvh`.
#[derive(Debug)]
pub struct FramesMut<'a> {
    motion_values: &'a mut [f32],
    num_channels: usize,
    num_frames: usize,
    curr_frame: usize,
}

impl FramesMut<'_> {
    /// Returns the number of `Frame`s left to iterate over.
    #[inline]
    pub const fn len(&self) -> usize {
        self.num_frames - self.curr_frame
    }

    /// Returns `true` if the number of `Frame`s left to iterate over is `0`.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a> Iterator for FramesMut<'a> {
    type Item = &'a mut Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let range = frames_iter_logic(self.num_channels, self.num_frames, &mut self.curr_frame)?;
        unsafe {
            // Cast the anonymous lifetime to the 'a lifetime to avoid E0495.
            // @TODO: is this safe?
            Some(mem::transmute::<&mut Frame, &'a mut Frame>(
                Frame::from_mut_slice(&mut self.motion_values[range]),
            ))
        }
    }
}

#[inline(always)]
fn frames_iter_logic(
    num_channels: usize,
    num_frames: usize,
    curr_frame: &mut usize,
) -> Option<Range<usize>> {
    if num_frames == 0 || *curr_frame >= num_frames {
        return None;
    }

    let start = *curr_frame * num_channels;
    let end = start + num_channels;

    *curr_frame += 1;

    Some(Range { start, end })
}

/// A wrapper for a slice of motion values, so that they can be indexed by `Channel`.
#[derive(PartialEq)]
pub struct Frame([f32]);

impl fmt::Debug for Frame {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, fmtr)
    }
}

impl Frame {
    #[inline]
    fn from_slice<'a>(frame_motions: &'a [f32]) -> &'a Frame {
        unsafe { &*(frame_motions as *const [f32] as *const Frame) }
    }

    #[inline]
    fn from_mut_slice<'a>(frame_motions: &'a mut [f32]) -> &'a mut Frame {
        unsafe { &mut *(frame_motions as *mut [f32] as *mut Frame) }
    }

    /// Returns the number of motion values in the `Frame`.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the `Frame` has a length of 0.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a reference to the motion element corresponding to `Channel`, or `None`
    /// if out of bounds.
    #[inline]
    pub fn get(&self, channel: &Channel) -> Option<&f32> {
        self.0.get(channel.motion_index)
    }

    /// Returns a mutable reference to the motion element corresponding to `Channel`,
    /// or `None` if out of bounds.
    #[inline]
    pub fn get_mut(&mut self, channel: &Channel) -> Option<&mut f32> {
        self.0.get_mut(channel.motion_index)
    }

    /// Get the `Frame` as a slice of `f32` values.
    pub fn as_slice(&self) -> &[f32] {
        &self.0[..]
    }

    /// Get the `Frame` as a mutable slice of `f32` values.
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.0[..]
    }
}

impl Index<&Channel> for Frame {
    type Output = f32;
    #[inline]
    fn index(&self, channel: &Channel) -> &Self::Output {
        self.0.index(channel.motion_index)
    }
}

impl IndexMut<&Channel> for Frame {
    fn index_mut(&mut self, channel: &Channel) -> &mut Self::Output {
        self.0.index_mut(channel.motion_index)
    }
}

    const NSEC_FACTOR: f64 = 1000_000_000.0;

#[inline]
fn fraction_seconds_to_duration(x: f64) -> Duration {
    Duration::from_nanos((x * NSEC_FACTOR) as u64)
}

#[inline]
fn duation_to_fractional_seconds(duration: &Duration) -> f64 {
    duration.subsec_nanos() as f64 / NSEC_FACTOR
}
