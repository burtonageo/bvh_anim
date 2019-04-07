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

#![allow(dead_code)]
#![warn(unused_imports, missing_docs)]

//! A small library for loading and manipulating BioVision motion files.
//!
//! The `Bvh` file format is comprised of two main sections: the 'Heirarchy' section,
//! which defines the joints of the skeleton, and the 'Motion' section, which defines
//! the motion values for each channel.
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
//! ROOT <Root-name>
//! {
//!     OFFSET <Root-offset-x> <Root-offset-y> <Root-offset-z>
//!     CHANNELS <Root-channels ...>
//!     JOINT <Joint-1-name>
//!     {
//!         OFFSET <Joint-1-offset-x> <Joint-1-offset-y> <Joint-1-offset-z>
//!         CHANNELS <Joint-1-channels ...>
//!         {
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
//!         ... More child joints
//!     }
//! }
//! ```
//!
//! More information on this file format can be found [here](here).
//!
//! [here]: https://research.cs.wisc.edu/graphics/Courses/cs-838-1999/Jeff/BVH.html

pub mod errors;
pub mod write;

use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use bstr::{io::BufReadExt, BStr, B};
use lexical::try_parse;
use mint::Vector3;
use smallvec::SmallVec;
use std::{
    fmt,
    io::{self, BufRead, Cursor, Write},
    iter::Iterator,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut, Index, IndexMut, Range},
    str::{self, FromStr},
    time::Duration,
};

use errors::{LoadError, LoadJointsError, LoadMotionError, ParseChannelError};

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
#[derive(Clone, Default, Debug)]
pub struct Bvh {
    /// The list of joints. If the root joint exists, it is always at
    /// index `0`.
    ///
    /// The internal data is wrapped in an `AtomicRefCell` to avoid having to duplicate
    /// large parts of the library to handle the difference between mutable/immutable
    /// parts.
    joints: AtomicRefCell<Vec<JointData>>,
    /// Matrix of animation data.
    clips: AtomicRefCell<Clips>,
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

    /// Loads the `Bvh` from the `reader`.
    pub fn load<R: BufReadExt>(mut reader: R) -> Result<Self, LoadError> {
        let (joints, num_channels) = Bvh::read_joints(reader.by_ref())
            .map(|result| (AtomicRefCell::new(result.0), result.1))?;
        let clips = Clips::read_motion(reader.by_ref(), num_channels).map(AtomicRefCell::new)?;

        Ok(Bvh { joints, clips })
    }

    /// Writes the `Bvh` using the `bvh` file format to the `writer`, with
    /// default settings.
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

    /// Returns the root joint if it exists, or `None` if the skeleton is empty.
    #[inline]
    pub fn root_joint(&self) -> Option<Joint<'_>> {
        if self.joints.borrow().is_empty() {
            None
        } else {
            Some(Joint {
                self_index: 0,
                skeleton: &self.joints,
                clips: &self.clips,
            })
        }
    }

    /// Returns an iterator over all the `Joint`s in the `Bvh`.
    #[inline]
    pub fn joints(&self) -> Joints<'_> {
        Joints::iter_root(self)
    }

    /// Returns a mutable iterator over all the joints in the `Bvh`.
    pub fn joints_mut(&mut self) -> JointsMut<'_> {
        JointsMut {
            joints: Joints::iter_root(self),
            _boo: PhantomData,
        }
    }

    /// Returns an immutable reference to the `Clips` data of the `Bvh`.
    #[inline]
    pub fn clips(&self) -> AtomicRef<'_, Clips> {
        self.clips.borrow()
    }

    /// Returns a mutable reference to the `Clips` data of the `Bvh`.
    #[inline]
    pub fn clips_mut(&mut self) -> AtomicRefMut<'_, Clips> {
        self.clips.borrow_mut()
    }

    /// Non-monomorphised logic for parsing the data from a `BufRead`.
    fn read_joints(
        reader: &mut dyn BufReadExt,
    ) -> Result<(Vec<JointData>, usize), LoadJointsError> {
        const HEIRARCHY_KEYWORD: &[u8] = b"HIERARCHY";

        const ROOT_KEYWORD: &[u8] = b"ROOT";
        const JOINT_KEYWORD: &[u8] = b"JOINT";
        const ENDSITE_KEYWORDS: &[&[u8]] = &[b"End", b"Site"];

        const OPEN_BRACE: &[u8] = b"{";
        const CLOSE_BRACE: &[u8] = b"}";

        const OFFSET_KEYWORD: &[u8] = b"OFFSET";
        const CHANNELS_KEYWORD: &[u8] = b"CHANNELS";

        #[derive(Debug, Eq, PartialEq)]
        enum ParseMode {
            NotStarted,
            InHeirarchy,
            Finished,
        }

        #[derive(Eq, PartialEq)]
        enum NextExpectedLine {
            Hierarchy,
            Channels,
            Offset,
            OpeningBrace,
            ClosingBrace,
            JointName,
            RootName,
        }

        let mut joints = vec![];
        let mut curr_mode = ParseMode::NotStarted;
        let mut curr_channel = 0usize;
        let (mut curr_index, mut curr_depth) = (0usize, 0usize);
        let mut next_expected_line = NextExpectedLine::Hierarchy;

        let mut curr_joint = JointData::empty_root();
        let mut in_end_site = false;
        let mut pushed_end_site_joint = false;

        #[inline]
        fn get_parent_index(joints: &[JointData], for_depth: usize) -> usize {
            joints
                .iter()
                .rev()
                .find(|jd| jd.depth() == for_depth.saturating_sub(2))
                .and_then(|jd| jd.private_data().map(|p| p.self_index))
                .unwrap_or(0)
        }

        let lines = reader.byte_lines();
        for (line_num, line) in lines.enumerate() {
            let line = line?;
            let line = line.trim();

            let mut tokens = line.fields_with(|c: char| c.is_ascii_whitespace() || c == ':');

            let first_token = match tokens.next() {
                Some(tok) => tok,
                None => continue,
            };

            if first_token == HEIRARCHY_KEYWORD && curr_mode == ParseMode::NotStarted {
                curr_mode = ParseMode::InHeirarchy;
                next_expected_line = NextExpectedLine::RootName;
                continue;
            }

            if first_token == ROOT_KEYWORD {
                if curr_mode != ParseMode::InHeirarchy
                    || next_expected_line != NextExpectedLine::RootName
                {
                    panic!("Unexpected root: {:?}", curr_mode);
                }

                if let Some(tok) = tokens.next() {
                    curr_joint.set_name(tok.bytes().collect());
                    continue;
                }
            }

            if first_token == OPEN_BRACE {
                curr_depth += 1;
                continue;
            }

            if first_token == CLOSE_BRACE {
                curr_depth -= 1;
                if curr_depth == 0 {
                    // We have closed the brace of the root joint.
                    curr_mode = ParseMode::Finished;
                }

                if in_end_site {
                    if let JointData::Child {
                        ref mut private, ..
                    } = curr_joint
                    {
                        private.self_index = curr_index;
                        private.parent_index = get_parent_index(&joints, curr_depth);
                        private.depth = curr_depth - 1;
                    }

                    let new_joint = mem::replace(&mut curr_joint, JointData::empty_child());
                    joints.push(new_joint);
                    curr_index += 1;
                    in_end_site = false;
                    pushed_end_site_joint = true;
                }
            }

            if first_token == ENDSITE_KEYWORDS[0]
                && tokens.next().map(BStr::as_bytes) == Some(ENDSITE_KEYWORDS[1])
            {
                in_end_site = true;
            }

            if first_token == JOINT_KEYWORD {
                if curr_mode != ParseMode::InHeirarchy {
                    panic!("Unexpected Joint");
                }

                if !pushed_end_site_joint {
                    if let JointData::Child {
                        ref mut private, ..
                    } = curr_joint
                    {
                        private.self_index = curr_index;
                        private.parent_index = get_parent_index(&joints, curr_depth);
                        private.depth = curr_depth - 1;
                    }

                    let new_joint = mem::replace(&mut curr_joint, JointData::empty_child());
                    joints.push(new_joint);

                    curr_index += 1;
                } else {
                    pushed_end_site_joint = false;
                }

                if let Some(name) = tokens.next() {
                    curr_joint.set_name(name.bytes().collect());
                }
            }

            if first_token == OFFSET_KEYWORD {
                if curr_mode != ParseMode::InHeirarchy {
                    return Err(LoadJointsError::UnexpectedOffsetSection { line: line_num });
                }

                let mut offset = Vector3::from([0.0, 0.0, 0.0]);

                macro_rules! parse_axis {
                    ($axis_field:ident, $axis_enum:ident) => {
                        if let Some(tok) = tokens.next() {
                            offset.$axis_field =
                                try_parse(tok).map_err(|e| LoadJointsError::ParseOffsetError {
                                    parse_float_error: e,
                                    axis: Axis::$axis_enum,
                                    line: line_num,
                                })?;
                        } else {
                            return Err(LoadJointsError::MissingOffsetAxis {
                                axis: Axis::$axis_enum,
                                line: line_num,
                            });
                        }
                    };
                }

                parse_axis!(x, X);
                parse_axis!(y, Y);
                parse_axis!(z, Z);

                curr_joint.set_offset(offset, in_end_site);
            }

            if first_token == CHANNELS_KEYWORD {
                if curr_mode != ParseMode::InHeirarchy {
                    return Err(LoadJointsError::UnexpectedChannelsSection { line: line_num });
                }

                let num_channels: usize = tokens
                    .next()
                    .ok_or(LoadJointsError::ParseNumChannelsError {
                        error: None,
                        line: line_num,
                    })
                    .and_then(|tok| match try_parse(tok) {
                        Ok(c) => Ok(c),
                        Err(e) => Err(LoadJointsError::ParseNumChannelsError {
                            error: Some(e),
                            line: line_num,
                        }),
                    })?;

                let mut channels: SmallVec<[Channel; 6]> = Default::default();
                channels.reserve(num_channels);

                while let Some(tok) = tokens.next() {
                    let channel_ty = ChannelType::from_bstr(tok).map_err(|e| {
                        LoadJointsError::ParseChannelError {
                            error: e,
                            line: line_num,
                        }
                    })?;
                    let channel = Channel::new(channel_ty, curr_channel);
                    curr_channel += 1;
                    channels.push(channel);
                }

                curr_joint.set_channels(channels);
            }

            if curr_mode == ParseMode::Finished {
                break;
            }
        }

        if curr_mode != ParseMode::Finished {
            return Err(LoadJointsError::MissingRoot);
        }

        Ok((joints, curr_channel))
    }

    #[allow(unused)]
    fn write_joints(&self, writer: &mut dyn Write) -> Result<(), io::Error> {
        unimplemented!()
    }
}

impl fmt::Display for Bvh {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = write::WriteOptions::default().write_to_string(self);
        fmt::Display::fmt(&s, f)
    }
}

/// A string type for the `Joint` name. A `SmallVec` is used for
/// better data locality.
pub type JointName = SmallVec<[u8; 15]>;

/// Internal representation of a joint.
#[derive(Clone)]
pub enum JointData {
    /// Root of the skeletal heirarchy.
    Root {
        /// Name of the root `Joint`.
        name: JointName,
        /// Positional offset of this `Joint` relative to the parent.
        offset: Vector3<f32>,
        /// The channels applicable to this `Joint`.
        channels: SmallVec<[Channel; 6]>,
    },
    /// A child joint in the skeleton.
    Child {
        /// Name of the `Joint`.
        name: JointName,
        /// Positional offset of this `Joint` relative to the parent.
        offset: Vector3<f32>,
        /// The channels applicable to this `Joint`.
        channels: SmallVec<[Channel; 3]>,
        /// End site offset.
        end_site_offset: Option<Vector3<f32>>,
        /// Private data.
        #[doc(hidden)]
        private: JointPrivateData,
    },
}

impl fmt::Debug for JointData {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            JointData::Root { .. } => fmtr
                .debug_struct("Root")
                .field("name", &self.name())
                .field("offset", &self.offset())
                .field("channels", &self.channels())
                .finish(),
            JointData::Child { ref private, .. } => fmtr
                .debug_struct("Child")
                .field("name", &self.name())
                .field("offset", &self.offset())
                .field("channels", &self.channels())
                .field("end_site_offset", &self.end_site())
                .field("private", &private)
                .finish(),
        }
    }
}

impl JointData {
    /// Returns the name of the `JointData`.
    #[inline]
    pub fn name(&self) -> &BStr {
        match *self {
            JointData::Root { ref name, .. } | JointData::Child { ref name, .. } => B(&name[..]),
        }
    }

    /// Returns the offset of the `JointData` if it exists, or `None`.
    #[inline]
    pub fn offset(&self) -> &Vector3<f32> {
        match *self {
            JointData::Child { ref offset, .. } | JointData::Root { ref offset, .. } => offset,
        }
    }

    /// Returns the `end_site_offset` if this `Joint` has an end site, or `None` if
    /// it doesn't.
    #[inline]
    pub fn end_site(&self) -> Option<&Vector3<f32>> {
        match *self {
            JointData::Child {
                ref end_site_offset,
                ..
            } => end_site_offset.as_ref(),
            _ => None,
        }
    }

    /// Returns `true` if the `Joint` has an `end_site_offset`, or `false` if it doesn't.
    #[inline]
    pub fn has_end_site(&self) -> bool {
        self.end_site().is_some()
    }

    /// Returns the ordered array of `Channel`s of this `JointData`.
    #[inline]
    pub fn channels(&self) -> &[Channel] {
        match *self {
            JointData::Child { ref channels, .. } => &channels[..],
            JointData::Root { ref channels, .. } => &channels[..],
        }
    }

    /// Returns a mutable reference to ordered array of `Channel`s of this `JointData`.
    #[inline]
    pub fn channels_mut(&mut self) -> &mut [Channel] {
        match *self {
            JointData::Child {
                ref mut channels, ..
            } => &mut channels[..],
            JointData::Root {
                ref mut channels, ..
            } => &mut channels[..],
        }
    }

    /// Returns the total number of channels applicable to this `JointData`.
    #[inline]
    pub fn num_channels(&self) -> usize {
        self.channels().len()
    }

    /// Return the index of this `Joint` in the array.
    #[inline]
    fn index(&self) -> usize {
        self.private_data().map(|d| d.self_index).unwrap_or(0)
    }

    /// Returns the index of the parent `JointData`, or `None` if this `JointData` is the
    /// root joint.
    #[inline]
    fn parent_index(&self) -> Option<usize> {
        self.private_data().map(|d| d.parent_index)
    }

    /// Returns a reference to the `JointPrivateData` of the `JointData` if it
    /// exists, or `None`.
    #[inline]
    fn private_data(&self) -> Option<&JointPrivateData> {
        match *self {
            JointData::Child { ref private, .. } => Some(private),
            _ => None,
        }
    }

    /// Get the depth of the `JointData` in the heirarchy.
    #[inline]
    fn depth(&self) -> usize {
        match *self {
            JointData::Child { ref private, .. } => private.depth,
            _ => 0,
        }
    }

    fn empty_root() -> Self {
        JointData::Root {
            name: Default::default(),
            offset: Vector3::from_slice(&[0.0, 0.0, 0.0]),
            channels: Default::default(),
        }
    }

    fn empty_child() -> Self {
        JointData::Child {
            name: Default::default(),
            offset: Vector3::from_slice(&[0.0, 0.0, 0.0]),
            channels: Default::default(),
            end_site_offset: Default::default(),
            private: JointPrivateData::empty(),
        }
    }

    fn set_name(&mut self, new_name: JointName) {
        match *self {
            JointData::Root { ref mut name, .. } => *name = new_name,
            JointData::Child { ref mut name, .. } => *name = new_name,
        }
    }

    fn set_offset(&mut self, new_offset: Vector3<f32>, is_site: bool) {
        match *self {
            JointData::Root { ref mut offset, .. } => *offset = new_offset,
            JointData::Child {
                ref mut offset,
                ref mut end_site_offset,
                ..
            } => {
                if is_site {
                    *end_site_offset = Some(new_offset);
                } else {
                    *offset = new_offset;
                }
            }
        }
    }

    fn set_channels(&mut self, new_channels: SmallVec<[Channel; 6]>) {
        match *self {
            JointData::Root {
                ref mut channels, ..
            } => *channels = new_channels,
            JointData::Child {
                ref mut channels, ..
            } => *channels = new_channels.iter().map(|c| *c).collect(),
        }
    }
}

/// Data private to joints.
#[doc(hidden)]
#[derive(Clone)]
pub struct JointPrivateData {
    /// Index of this `Joint` in the array.
    self_index: usize,
    /// The parent index in the array of `JointPrivateData`s in the `Bvh`.
    parent_index: usize,
    /// Depth of the `Joint`. A depth of `1` signifies a `Joint` attached to
    /// the root.
    depth: usize,
}

impl JointPrivateData {
    #[inline]
    fn new(self_index: usize, parent_index: usize, depth: usize) -> Self {
        JointPrivateData {
            self_index,
            parent_index,
            depth,
        }
    }

    #[inline]
    fn empty() -> Self {
        JointPrivateData::new(0, 0, 0)
    }

    #[inline]
    fn new_default() -> Self {
        Self::new(0, 0, 0)
    }
}

impl fmt::Debug for JointPrivateData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JointPrivateData { .. }")
    }
}

/// An iterator over the `Joint`s of a `Bvh` skeleton.
pub struct Joints<'a> {
    bvh: &'a Bvh,
    current_joint: usize,
    joint_depth: Option<usize>,
}

impl fmt::Debug for Joints<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Joints { .. }")
    }
}

impl<'a> Joints<'a> {
    fn iter_root(bvh: &'a Bvh) -> Self {
        Joints {
            bvh,
            current_joint: 0,
            joint_depth: None,
        }
    }

    #[allow(unused)]
    fn iter_children(joint: &Joint<'a>) -> Self {
        unimplemented!()
    }

    /// Finds the `Joint` named `joint_name`, or `None` if it doesn't exist.
    #[inline]
    pub fn find_by_name(&mut self, joint_name: &str) -> Option<Joint<'a>> {
        self.find(|b| b.data().name() == joint_name)
    }
}

impl<'a> Iterator for Joints<'a> {
    type Item = Joint<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_joint >= self.bvh.joints.borrow().len() {
            return None;
        }

        let joint = Some(Joint {
            self_index: self.current_joint,
            skeleton: &self.bvh.joints,
            clips: &self.bvh.clips,
        });

        self.current_joint += 1;

        joint
    }
}

/// A mutable iterator over the `Joint`s of a `Bvh` skeleton.
pub struct JointsMut<'a> {
    joints: Joints<'a>,
    _boo: PhantomData<&'a mut Bvh>,
}

impl<'a> Iterator for JointsMut<'a> {
    type Item = JointMut<'a>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.joints.next().map(JointMut::from_joint)
    }
}

impl fmt::Debug for JointsMut<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JointsMut { .. }")
    }
}

/// A view of a joint which provides access to various relevant data.
pub struct Joint<'a> {
    /// Index of the `Joint` in the skeleton.
    self_index: usize,
    /// Skeleton which the joint is part of.
    skeleton: &'a AtomicRefCell<Vec<JointData>>,
    /// Motion clip data relevant to the skeleton.
    clips: &'a AtomicRefCell<Clips>,
}

impl Joint<'_> {
    /// Return the parent `Joint` if it exists, or `None` if it doesn't.
    #[inline]
    pub fn parent(&self) -> Option<Joint<'_>> {
        self.data().parent_index().map(|idx| Joint {
            self_index: idx,
            skeleton: self.skeleton,
            clips: self.clips,
        })
    }

    /// Returns an iterator over the children of `self`.
    #[inline]
    pub fn children(&self) -> Joints<'_> {
        Joints::iter_children(self.clone())
    }

    /// Access a read-only view of the internal data of the `Joint`.
    #[inline]
    pub fn data(&self) -> AtomicRef<JointData> {
        AtomicRef::map(self.skeleton.borrow(), |skel| &skel[self.self_index])
    }
}

impl fmt::Debug for Joint<'_> {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmtr.debug_struct("Joint")
            .field("index", &self.self_index)
            .field("data", &self.data())
            .finish()
    }
}

/// A view of a joint which provides mutable access.
pub struct JointMut<'a> {
    joint: Joint<'a>,
    _boo: PhantomData<&'a mut ()>,
}

impl<'a> JointMut<'a> {
    /// Mutable access to the internal data of the `JointMut`.
    #[inline]
    pub fn data_mut(&mut self) -> AtomicRefMut<JointData> {
        AtomicRefMut::map(self.skeleton.borrow_mut(), |skel| {
            &mut skel[self.self_index]
        })
    }

    /// Construct a `JointMut` from a `Joint`.
    #[inline]
    fn from_joint(joint: Joint<'a>) -> Self {
        JointMut {
            joint,
            _boo: PhantomData,
        }
    }
}

impl<'a> Deref for JointMut<'a> {
    type Target = Joint<'a>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.joint
    }
}

impl fmt::Debug for JointMut<'_> {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmtr.debug_struct("JointMut")
            .field("index", &self.self_index)
            .field("data", &self.data())
            .finish()
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
    fn new(channel_type: ChannelType, motion_index: usize) -> Self {
        Channel {
            channel_type,
            motion_index,
        }
    }

    /// Returns the `ChannelType` to which this `Channel` corresponds.
    #[inline]
    pub fn channel_type(&self) -> ChannelType {
        self.channel_type
    }

    /// Returns the index of the motion value to which this `Channel` corresponds.
    #[inline]
    pub fn motion_index(&self) -> usize {
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
    pub fn axis_vector(&self) -> Vector3<f32> {
        self.axis().vector()
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
        let s = match *self {
            ChannelType::RotationX => "Xrotation",
            ChannelType::RotationY => "Yrotation",
            ChannelType::RotationZ => "Zrotation",

            ChannelType::PositionX => "Xposition",
            ChannelType::PositionY => "Yposition",
            ChannelType::PositionZ => "Zposition",
        };

        f.write_str(s)
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
    #[inline]
    pub fn vector(&self) -> Vector3<f32> {
        match *self {
            Axis::X => Vector3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Axis::Y => Vector3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            Axis::Z => Vector3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
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

/// A struct type which holds information about the animation frames of the `Bvh`.
#[derive(Clone, Default)]
pub struct Clips {
    /// The motion values of the `Frame`.
    data: Vec<f32>,
    /// The number of frames in the bvh.
    num_frames: usize,
    /// The number of `Channel`s in the bvh.
    num_channels: usize,
    /// The total time it takes to play one frame.
    frame_time: Duration,
}

impl Clips {
    fn read_motion(reader: &mut dyn BufRead, num_channels: usize) -> Result<Self, LoadMotionError> {
        fn fraction_seconds_to_nanoseconds(x: f64) -> u64 {
            const NSEC_FACTOR: f64 = 1000_000_000.0;
            (x * NSEC_FACTOR) as u64
        }

        const MOTION_KEYWORD: &[u8] = b"MOTION";
        const FRAMES_KEYWORD: &[u8] = b"Frames";
        const FRAME_TIME_KEYWORDS: &[&[u8]] = &[b"Frame", b"Time:"];

        let mut out_clips = Clips::default();
        out_clips.num_channels = num_channels;
        let mut lines = reader.byte_lines();

        lines
            .next()
            .ok_or(LoadMotionError::MissingMotionSection)
            .and_then(|line| {
                let line = line?;
                let line = line.trim();
                if line == MOTION_KEYWORD {
                    Ok(())
                } else {
                    Err(LoadMotionError::MissingMotionSection)
                }
            })?;

        out_clips.num_frames = lines
            .next()
            .ok_or(LoadMotionError::MissingNumFrames { parse_error: None })
            .and_then(|line| {
                let line = line?;
                let line = line.trim();
                let mut tokens = line.fields_with(|c: char| c.is_ascii_whitespace() || c == ':');

                if tokens.next().map(BStr::as_bytes) != Some(FRAMES_KEYWORD) {
                    return Err(LoadMotionError::MissingNumFrames { parse_error: None });
                }

                let parse_num_frames = |token: Option<&BStr>| {
                    if let Some(num_frames) = token {
                        try_parse::<usize, _>(num_frames)
                            .map_err(|e| LoadMotionError::MissingNumFrames {
                                parse_error: Some(e),
                            })
                            .map_err(Into::into)
                    } else {
                        Err(LoadMotionError::MissingNumFrames { parse_error: None })
                    }
                };

                match tokens.next() {
                    Some(tok) if tok == B(":") => parse_num_frames(tokens.next()),
                    Some(tok) => parse_num_frames(Some(tok)),
                    None => Err(LoadMotionError::MissingNumFrames { parse_error: None }),
                }
            })?;

        out_clips.frame_time = lines
            .next()
            .ok_or(LoadMotionError::MissingFrameTime { parse_error: None })
            .and_then(|line| {
                let line = line?;
                let mut tokens = line.fields();

                let frame_time_kw = tokens.next();
                if frame_time_kw.map(BStr::as_bytes) == FRAME_TIME_KEYWORDS.get(0).map(|b| *b) {
                    // do nothing
                } else {
                    return Err(LoadMotionError::MissingFrameTime { parse_error: None });
                }

                let frame_time_kw = tokens.next();
                if frame_time_kw.map(BStr::as_bytes) == FRAME_TIME_KEYWORDS.get(1).map(|b| *b) {
                    // do nothing
                } else {
                    return Err(LoadMotionError::MissingFrameTime { parse_error: None });
                }

                let parse_frame_time = |token: Option<&BStr>| {
                    if let Some(frame_time) = token {
                        let frame_time_secs = try_parse::<f64, _>(frame_time).map_err(|e| {
                            LoadMotionError::MissingFrameTime {
                                parse_error: Some(e),
                            }
                        })?;
                        Ok(Duration::from_nanos(fraction_seconds_to_nanoseconds(
                            frame_time_secs,
                        )))
                    } else {
                        Err(LoadMotionError::MissingFrameTime { parse_error: None })
                    }
                };

                match tokens.next() {
                    Some(tok) if tok == B(":") => parse_frame_time(tokens.next()),
                    Some(tok) => parse_frame_time(Some(tok)),
                    None => Err(LoadMotionError::MissingNumFrames { parse_error: None }),
                }
            })?;

        let expected_total_motion_values = out_clips.num_channels * out_clips.num_frames;

        out_clips.data.reserve(expected_total_motion_values);

        for line in lines {
            let line = line?;
            let tokens = line.fields();
            for token in tokens {
                let motion: f32 = try_parse(token)
                    .map_err(|e| LoadMotionError::ParseMotionSection { parse_error: e })?;
                out_clips.data.push(motion);
            }
        }

        if out_clips.data.len() != out_clips.num_channels * out_clips.num_frames {
            return Err(LoadMotionError::MotionCountMismatch {
                actual_total_motion_values: out_clips.data.len(),
                expected_total_motion_values,
                expected_num_frames: out_clips.num_frames,
                expected_num_clips: out_clips.num_channels,
            });
        }

        Ok(out_clips)
    }

    /// Returns a `Frames` iterator over the frames of the bvh.
    #[inline]
    pub fn frames(&self) -> Frames<'_> {
        Frames {
            clips: self,
            curr_frame: 0,
        }
    }

    /// Returns a mutable iterator over the frames of the bvh.
    #[inline]
    pub fn frames_mut(&mut self) -> FramesMut<'_> {
        FramesMut {
            clips: self,
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
    pub fn try_set_motion(
        &mut self,
        frame: usize,
        channel: &Channel,
        new_motion: f32,
    ) -> Result<(), ()> {
        if let Some(m) = self
            .frames_mut()
            .nth(frame)
            .and_then(|f| f.get_mut(channel))
        {
            *m = new_motion;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Get the number of frames in the `Bvh`.
    #[inline]
    pub fn num_frames(&self) -> usize {
        self.num_frames
    }

    /// Get the number of channels in the `Bvh`.
    #[inline]
    pub fn num_channels(&self) -> usize {
        self.num_channels
    }

    /// Get the duration each frame should play for in the `Bvh`.
    #[inline]
    pub fn frame_time(&self) -> &Duration {
        &self.frame_time
    }

    /// Set the duration each frame should play for in the `Bvh` to `new_frame_time`.
    #[inline]
    pub fn set_frame_time(&mut self, new_frame_time: Duration) {
        self.frame_time = new_frame_time;
    }
}

impl fmt::Debug for Clips {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for frame in self.frames() {
            fmt::Debug::fmt(&frame, f)?;
        }
        Ok(())
    }
}

/// An iterator over the frames of a `Bvh`.
#[derive(Debug)]
pub struct Frames<'a> {
    clips: &'a Clips,
    curr_frame: usize,
}

impl<'a> Iterator for Frames<'a> {
    type Item = &'a Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let range = frames_iter_logic(&self.clips, &mut self.curr_frame)?;
        Some(From::from(&self.clips.data[range]))
    }
}

/// A mutable iterator over the frames of a `Bvh`.
#[derive(Debug)]
pub struct FramesMut<'a> {
    clips: &'a mut Clips,
    curr_frame: usize,
}

impl<'a> Iterator for FramesMut<'a> {
    type Item = &'a mut Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let range = frames_iter_logic(&self.clips, &mut self.curr_frame)?;
        unsafe {
            // Cast the anonymous lifetime to the 'a lifetime to avoid E0495.
            Some(mem::transmute::<&mut Frame, &'a mut Frame>(From::from(
                &mut self.clips.data[range],
            )))
        }
    }
}

#[inline(always)]
fn frames_iter_logic(clips: &Clips, curr_frame: &mut usize) -> Option<Range<usize>> {
    let nchans = clips.num_channels;
    let nframes = clips.num_frames;

    if nframes == 0 || *curr_frame >= nframes {
        return None;
    }

    let start = *curr_frame * nchans;
    let end = start + nchans;

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

impl<'a> From<&'a [f32]> for &'a Frame {
    #[inline]
    fn from(frame_motions: &'a [f32]) -> Self {
        unsafe { &*(frame_motions as *const [f32] as *const Frame) }
    }
}

impl<'a> From<&'a mut [f32]> for &'a mut Frame {
    #[inline]
    fn from(frame_motions: &'a mut [f32]) -> Self {
        unsafe { &mut *(frame_motions as *mut [f32] as *mut Frame) }
    }
}

impl Frame {
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

impl Deref for Frame {
    type Target = [f32];
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Frame {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
