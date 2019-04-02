#![allow(dead_code, unused)]
#![warn(unused_imports)]

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

mod errors;
pub mod write;

use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use mint::Vector3;
use smallstring::SmallString;
use smallvec::SmallVec;
use std::{
    fmt,
    io::{self, BufRead, Write},
    iter::Iterator,
    marker::PhantomData,
    mem,
    ops::Deref,
    str::{self, FromStr},
    time::Duration,
};

pub use errors::{LoadError, ParseChannelError};

/// Loads the `Bvh` from the `reader`.
#[inline]
pub fn load<R: BufRead>(data: R) -> Result<Bvh, LoadError> {
    Bvh::load(data)
}

/// A complete `bvh` file.
#[derive(Clone, Default, Debug)]
pub struct Bvh {
    /// The list of bones. If the root bone exists, it is always at
    /// index `0`.
    ///
    /// The internal data is wrapped in an `AtomicRefCell` to avoid having to duplicate
    /// large parts of the library to handle the difference between mutable/immutable
    /// parts.
    bones: AtomicRefCell<Vec<BoneData>>,
    /// Matrix of animation data.
    clips: AtomicRefCell<Clips>,
}

impl Bvh {
    /// Create an empty `Bvh`.
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// Loads the `Bvh` from the `reader`.
    pub fn load<R: BufRead>(mut reader: R) -> Result<Self, LoadError> {
        let bones = Bvh::read_bones(reader.by_ref()).map(AtomicRefCell::new)?;
        let clips = Clips::read_motion(reader.by_ref()).map(AtomicRefCell::new)?;

        Ok(Bvh { bones, clips })
    }

    /// Writes the `Bvh` using the `bvh` file format to the `writer`, with
    /// default settings.
    #[inline]
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        write::WriteOptions::default().write(self, writer)
    }

    /// Returns the root bone if it exists, or `None` if the skeleton is empty.
    #[inline]
    pub fn root_bone(&self) -> Option<Bone<'_>> {
        if self.bones.borrow().is_empty() {
            None
        } else {
            Some(Bone {
                self_index: 0,
                skeleton: &self.bones,
                clips: &self.clips,
            })
        }
    }

    /// Returns an iterator over all the bones in the `Bvh`.
    #[inline]
    pub fn bones(&self) -> Bones<'_> {
        Bones::iter_root(self)
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
    fn read_bones(reader: &mut dyn BufRead) -> Result<Vec<BoneData>, LoadError> {
        const HEIRARCHY_KEYWORD: &str = "HIERARCHY";

        const ROOT_KEYWORD: &str = "ROOT";
        const JOINT_KEYWORD: &str = "JOINT";
        const ENDSITE_KEYWORDS: &[&str] = &["End", "Site"];

        const OPEN_BRACE: &str = "{";
        const CLOSE_BRACE: &str = "}";

        const OFFSET_KEYWORD: &str = "OFFSET";
        const CHANNELS_KEYWORD: &str = "CHANNELS";

        #[derive(Debug, Eq, PartialEq)]
        enum ParseMode {
            NotStarted,
            InHeirarchy,
            Finished,
        }

        #[derive(Eq, PartialEq)]
        enum NextExpectedLine {
            Channels,
            Offset,
            OpeningBrace,
            ClosingBrace,
            JointsName,
            RootName,
        }

        let mut bones = vec![];
        let mut curr_mode = ParseMode::NotStarted;
        let mut curr_channel = 0usize;
        let (mut curr_index, mut curr_depth) = (0usize, 0usize);

        let mut curr_bone = BoneData::empty_root();
        let mut in_end_site = true;

        let lines = reader.lines();
        for (line_num, line) in lines.enumerate() {
            let line = line?;
            let line = line.trim();

            let mut tokens = line.split(|c| c == ' ' || c == '\t' || c == ':');

            let first_token = match tokens.next() {
                Some(tok) => tok,
                None => continue,
            };

            if first_token == HEIRARCHY_KEYWORD && curr_mode == ParseMode::NotStarted {
                println!("Found heirarchy");
                curr_mode = ParseMode::InHeirarchy;
                continue;
            }

            if first_token == ROOT_KEYWORD {
                if curr_mode != ParseMode::InHeirarchy {
                    panic!("Unexpected root: {:?}", curr_mode);
                }

                if let Some(tok) = tokens.next() {
                    curr_bone.set_name(From::from(tok));
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
                    in_end_site = false;
                }
                continue;
            }

            if first_token == ENDSITE_KEYWORDS[0] && tokens.next() == Some(ENDSITE_KEYWORDS[1]) {
                in_end_site = true;
                continue;
            }

            if first_token == JOINT_KEYWORD {
                if curr_mode != ParseMode::InHeirarchy {
                    panic!("Unexpected Joint");
                }

                if let BoneData::Joint {
                    ref mut private, ..
                } = curr_bone
                {
                    private.self_index = curr_index;
                    private.parent_index = curr_depth - 1;
                    private.depth = curr_depth;
                }

                let new_joint = mem::replace(&mut curr_bone, BoneData::empty_joint());
                bones.push(new_joint);
                curr_index + 1;

                if let Some(name) = tokens.next() {
                    curr_bone.set_name(From::from(name));
                }
            }

            if first_token == OFFSET_KEYWORD {
                if curr_mode != ParseMode::InHeirarchy {
                    return Err(LoadError::UnexpectedOffsetSection { line: line_num });
                }

                let mut offset = Vector3::from_slice(&[0.0, 0.0, 0.0]);

                macro_rules! parse_axis {
                    ($axis_field:ident, $axis_enum:ident) => {
                        if let Some(tok) = tokens.next() {
                            offset.$axis_field =
                                str::parse(tok).map_err(|e| LoadError::ParseOffsetError {
                                    parse_float_error: e,
                                    axis: Axis::$axis_enum,
                                    line: line_num,
                                })?;
                        } else {
                            return Err(LoadError::MissingOffsetAxis {
                                axis: Axis::$axis_enum,
                                line: line_num,
                            });
                        }
                    };
                }

                parse_axis!(x, X);
                parse_axis!(y, Y);
                parse_axis!(z, Z);

                curr_bone.set_offset(offset, in_end_site);
            }

            if first_token == CHANNELS_KEYWORD {
                if curr_mode != ParseMode::InHeirarchy {
                    return Err(LoadError::UnexpectedChannelsSection { line: line_num });
                }

                let num_channels: usize = if let Some(tok) = tokens.next() {
                    str::parse(tok).unwrap()
                } else {
                    panic!("Num channels not found!");
                };

                let mut channels: SmallVec<[Channel; 6]> = Default::default();
                channels.reserve(num_channels);

                while let Some(tok) = tokens.next() {
                    let channel_ty = str::parse(tok).map_err(|e| LoadError::ParseChannelError {
                        error: e,
                        line: line_num,
                    })?;
                    let channel = Channel::new(channel_ty, curr_channel);
                    curr_channel += 1;
                    channels.push(channel);
                }
                curr_bone.set_channels(channels);
            }

            if curr_mode == ParseMode::Finished {
                break;
            }
        }

        if curr_mode != ParseMode::Finished {
            return Err(LoadError::MissingRoot);
        }

        Ok(bones)
    }

    fn write_bones(&self, writer: &mut dyn Write) -> Result<(), io::Error> {
        unimplemented!()
    }
}

impl fmt::Display for Bvh {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = write::WriteOptions::default().write_to_string(self);
        f.write_str(&s)
    }
}

/// A string type for the bone name. A `SmallString` is used for
/// better data locality.
pub type BoneName = SmallString<[u8; 24]>;

/// Internal representation of a bone.
#[derive(Clone, Debug)]
pub enum BoneData {
    /// Root of the skeletal heirarchy.
    Root {
        /// Name of the root bone.
        name: BoneName,
        /// Positional offset of this bone relative to the parent.
        offset: Vector3<f32>,
        /// The channels applicable to this `Joint`.
        channels: SmallVec<[Channel; 6]>,
    },
    /// A joint in the skeleton.
    Joint {
        /// Name of the joint bone.
        name: BoneName,
        /// Positional offset of this bone relative to the parent.
        offset: Vector3<f32>,
        /// The channels applicable to this `Joint`.
        channels: SmallVec<[Channel; 3]>,
        /// End site offset
        end_site_offset: Option<Vector3<f32>>,
        /// Private data.
        #[doc(hidden)]
        private: JointPrivateData,
    },
}

impl BoneData {
    fn empty_root() -> Self {
        BoneData::Root {
            name: Default::default(),
            offset: Vector3::from_slice(&[0.0, 0.0, 0.0]),
            channels: Default::default(),
        }
    }

    fn empty_joint() -> Self {
        BoneData::Joint {
            name: Default::default(),
            offset: Vector3::from_slice(&[0.0, 0.0, 0.0]),
            channels: Default::default(),
            end_site_offset: Default::default(),
            private: JointPrivateData::empty(),
        }
    }

    fn set_name(&mut self, new_name: BoneName) {
        match *self {
            BoneData::Root { ref mut name, .. } => *name = new_name,
            BoneData::Joint { ref mut name, .. } => *name = new_name,
        }
    }

    fn set_offset(&mut self, new_offset: Vector3<f32>, is_site: bool) {
        match *self {
            BoneData::Root { ref mut offset, .. } => *offset = new_offset,
            BoneData::Joint {
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
            BoneData::Root {
                ref mut channels, ..
            } => *channels = new_channels,
            BoneData::Joint {
                ref mut channels, ..
            } => *channels = new_channels.iter().map(|c| *c).collect(),
        }
    }
}

/// Data private to joints.
#[doc(hidden)]
#[derive(Clone)]
pub struct JointPrivateData {
    /// Index of this bone in the array.
    self_index: usize,
    /// The parent index in the array of bones in the `Bvh`.
    parent_index: usize,
    /// Depth of the bone. A depth of `0` signifies a bone attached to
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

impl BoneData {
    /// Returns the name of the bone if it exists, or `None`.
    #[inline]
    pub fn name(&self) -> &str {
        match *self {
            BoneData::Root { ref name, .. } | BoneData::Joint { ref name, .. } => &*name,
        }
    }

    /// Returns the offset of the bone if it exists, or `None`.
    #[inline]
    pub fn offset(&self) -> &Vector3<f32> {
        match *self {
            BoneData::Joint { ref offset, .. } | BoneData::Root { ref offset, .. } => offset,
        }
    }

    /// Returns the ordered array of `Channel`s of this `Joint`.
    #[inline]
    pub fn channels(&self) -> &[Channel] {
        match *self {
            BoneData::Joint { ref channels, .. } => &channels[..],
            BoneData::Root { ref channels, .. } => &channels[..],
        }
    }

    /// Returns a mutable reference to ordered array of `Channel`s of this `Joint`.
    #[inline]
    pub fn channels_mut(&mut self) -> &mut [Channel] {
        match *self {
            BoneData::Joint { ref mut channels, .. } => &mut channels[..],
            BoneData::Root { ref mut channels, .. } => &mut channels[..],
        }
    }

    /// Returns the total number of channels applicable to this bone.
    #[inline]
    pub fn num_channels(&self) -> usize {
        self.channels().len()
    }

    /// Return the index of this bone in the array.
    #[inline]
    fn index(&self) -> usize {
        self.private_data().map(|d| d.self_index).unwrap_or(0)
    }

    /// Returns the index of the parent bone, or `None` if this bone is the
    /// root bone.
    #[inline]
    fn parent_index(&self) -> Option<usize> {
        self.private_data().map(|d| d.parent_index)
    }

    /// Returns a reference to the `JointPrivateData` of the `Bone` if it
    /// exists, or `None`.
    #[inline]
    fn private_data(&self) -> Option<&JointPrivateData> {
        match *self {
            BoneData::Joint { ref private, .. } => Some(private),
            _ => None,
        }
    }
}

/// An iterator over the bones of a `Bvh` skeleton.
pub struct Bones<'a> {
    bvh: &'a Bvh,
    starting_bone_idx: usize,
}

impl fmt::Debug for Bones<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Bones { .. }")
    }
}

impl<'a> Bones<'a> {
    fn iter_root(bvh: &'a Bvh) -> Self {
        Bones {
            bvh,
            starting_bone_idx: 0,
        }
    }

    fn iter_children(bone: &Bone<'a>) -> Self {
        unimplemented!()
    }

    #[inline]
    pub fn find_by_name(&mut self, bone_name: &str) -> Option<Bone<'a>> {
        self.find(|b| b.data().name() == bone_name)
    }
}

impl<'a> Iterator for Bones<'a> {
    type Item = Bone<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub struct BonesMut<'a> {
    bvh: &'a mut Bvh,
    starting_bone_idx: usize,
}

impl<'a> Iterator for BonesMut<'a> {
    type Item = BoneMut<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

impl fmt::Debug for BonesMut<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BonesMut { .. }")
    }
}

/// A view of a bone which provides access to various relevant data.
pub struct Bone<'a> {
    /// Index of the bone in the skeleton.
    self_index: usize,
    /// Skeleton which the bone is part of.
    skeleton: &'a AtomicRefCell<Vec<BoneData>>,
    /// Motion clip data relevant to the skeleton.
    clips: &'a AtomicRefCell<Clips>,
}

impl Bone<'_> {
    /// Return the parent `Bone` if it exists, or `None` if it doesn't.
    #[inline]
    pub fn parent(&self) -> Option<Bone<'_>> {
        self.data().parent_index().map(|idx| Bone {
            self_index: idx,
            skeleton: self.skeleton,
            clips: self.clips,
        })
    }

    /// Returns an iterator over the children of `self`.
    #[inline]
    pub fn children(&self) -> Bones<'_> {
        Bones::iter_children(self.clone())
    }

    /// Access a read-only view of the internal data of the bone.
    #[inline]
    pub fn data(&self) -> AtomicRef<BoneData> {
        AtomicRef::map(self.skeleton.borrow(), |skel| &skel[self.self_index])
    }
}

impl fmt::Debug for Bone<'_> {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmtr.debug_struct("Bone")
            .field("index", &self.self_index)
            .field("data", &self.data())
            .finish()
    }
}

/// A view of a bone which provides mutable access.
pub struct BoneMut<'a> {
    bone: Bone<'a>,
    _boo: PhantomData<&'a mut ()>,
}

impl<'a> BoneMut<'a> {
    /// Mutable access to the internal data of the bone.
    #[inline]
    pub fn data_mut(&mut self) -> AtomicRefMut<BoneData> {
        AtomicRefMut::map(self.skeleton.borrow_mut(), |skel| {
            &mut skel[self.self_index]
        })
    }

    /// Construct a `BoneMut` from a `Bone`.
    #[inline]
    fn from_bone(bone: Bone<'a>) -> Self {
        BoneMut {
            bone,
            _boo: PhantomData,
        }
    }
}

impl<'a> Deref for BoneMut<'a> {
    type Target = Bone<'a>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.bone
    }
}

impl fmt::Debug for BoneMut<'_> {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmtr.debug_struct("BoneMut")
            .field("index", &self.self_index)
            .field("data", &self.data())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Channel {
    channel_type: ChannelType,
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

/// The available degrees of freedom along which a bone may be manipulated.
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
    #[inline]
    pub fn is_rotation(&self) -> bool {
        match *self {
            ChannelType::RotationX => true,
            ChannelType::RotationY => true,
            ChannelType::RotationZ => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_position(&self) -> bool {
        !self.is_rotation()
    }

    /// Get the `Axis` about which this `Channel` transforms.
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

///
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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

impl FromStr for ChannelType {
    type Err = ParseChannelError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Xrotation" => Ok(ChannelType::RotationX),
            "Yrotation" => Ok(ChannelType::RotationY),
            "Zrotation" => Ok(ChannelType::RotationZ),

            "Xposition" => Ok(ChannelType::PositionX),
            "Yposition" => Ok(ChannelType::PositionY),
            "Zposition" => Ok(ChannelType::PositionZ),

            _ => Err(ParseChannelError(From::from(s))),
        }
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

#[derive(Clone, Default)]
pub struct Clips {
    data: Vec<f32>,
    width: usize,
    frame_time: Duration,
}

impl Clips {
    fn read_motion(reader: &mut dyn BufRead) -> Result<Self, LoadError> {
        const MOTION_KEYWORD: &str = "MOTION";
        const FRAMES_KEYWORD: &str = "Frames";
        const FRAME_TIME_KEYWORDS: &[&str] = &[&"Frame", &"Time"];
        Ok(Default::default())
    }

    #[inline]
    pub fn new(width: usize, frame_time: Duration) -> Self {
        Clips {
            data: vec![],
            width,
            frame_time,
        }
    }

    pub fn insert_anim_clip(&mut self, clip: &[f32]) -> Result<(), ()> {
        Err(())
    }

    #[inline]
    pub fn rows(&self) -> RowsIter<'_> {
        RowsIter {
            mat: self,
            curr_row: 0,
        }
    }

    #[inline]
    pub fn frame_time(&self) -> &Duration {
        &self.frame_time
    }

    #[inline]
    pub fn set_frame_time(&mut self, new_frame_time: Duration) {
        self.frame_time = new_frame_time;
    }
}

impl fmt::Debug for Clips {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in self.rows() {
            fmt::Debug::fmt(&row, f)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct RowsIter<'a> {
    mat: &'a Clips,
    curr_row: usize,
}

impl<'a> Iterator for RowsIter<'a> {
    type Item = &'a [f32];

    fn next(&mut self) -> Option<Self::Item> {
        let w = self.mat.width;

        let row_idx_start = self.curr_row * w;
        let row_idx_end = row_idx_start + w;

        self.curr_row += 1;

        if row_idx_end < self.mat.data.len() {
            Some(&self.mat.data[row_idx_start..row_idx_end])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
