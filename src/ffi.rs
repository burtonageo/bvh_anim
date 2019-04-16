#![allow(nonstandard_style)]
#![allow(unused, missing_docs)]

//! The ffi interface to the `bvh_anim` crate.

use crate::{Channel, ChannelType};
use std::ffi::CString;
use mint::Vector3;
use libc::{c_char, c_double, c_float, c_int, size_t, uint8_t};
use std::ptr;

/// A type representing an `OFFSET` position.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct bvh_Offset {
    /// The x-component of the `OFFSET`.
    pub offset_x: c_float,
    /// The y-component of the `OFFSET`.
    pub offset_y: c_float,
    /// The z-component of the `OFFSET`.
    pub offset_z: c_float,
}

/// A channel type representing a degree of freedom along which a
/// joint may move.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum bvh_ChannelType {
    /// An `Xposition` channel type.
    X_POSITION,
    /// An `Yposition` channel type.
    Y_POSITION,
    /// An `Zposition` channel type.
    Z_POSITION,
    /// An `Xrotation` channel type.
    X_ROTATION,
    /// An `Yrotation` channel type.
    Y_ROTATION,
    /// An `Zrotation` channel type.
    Z_ROTATION,
}

/// A channel composed of a `bvh_ChannelType` and an index into the
/// `bvh_BvhFile::bvh_motion_data` array to which it corresponds.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct bvh_Channel {
    /// The type of the channel.
    pub channel_type: bvh_ChannelType,
    /// The index into the motion data array.
    pub channel_index: size_t,
}

pub const BVH_JOINT_PARENT_INDEX_NONE: size_t = usize::max_value();

/// A single joint in the `HIERARCHY` section of a `bvh_BvhFile`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct bvh_Joint {
    /// The name of the joint.
    pub joint_name: *mut c_char,
    /// The ordered array of channels of the `bvh_Joint`.
    pub joint_channels: *mut bvh_Channel,
    /// The length of the `joint_channels` array.
    pub joint_num_channels: size_t,
    /// The index of the parent `bvh_Joint` in the `bvh_BvhFile::bvh_joints`
    /// array to which this joint belongs. If this joint doesn't have a
    /// parent (because it is the root joint), then this will have the
    /// value `BVH_JOINT_PARENT_INDEX_NONE`.
    pub joint_parent_index: size_t,
    /// The depth of the joint from the root joint. The root joint always
    /// has a depth of `0`.
    pub joint_depth: size_t,
    /// The offset of the `Joint`.
    pub joint_offset: bvh_Offset,
    /// The end site of the `Joint`. Should not be used if
    /// `bvh_Joint::joint_has_end_site` is `0`.
    pub joint_end_site: bvh_Offset,
    /// Boolean condition as to whether this joint has an end site.
    /// If it does not, then this value is `0`, Otherwise it will
    /// be greater than `0`.
    pub joint_has_end_site: uint8_t,
}

/// A struct representing a parsed bvh file.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct bvh_BvhFile {
    /// The array of joints of the bvh.
    pub bvh_joints: *mut bvh_Joint,
    /// The length of the array of joints from the bvh.
    pub bvh_num_joints: size_t,
    /// The number of frames in the bvh file.
    pub bvh_num_frames: size_t,
    /// The number of channels in the bvh file.
    pub bvh_num_channels: size_t,
    /// The array of motion data in the bvh file. This has a total
    /// size of `bvh_num_frames * bvh_num_channels`.
    pub bvh_motion_data: *mut c_float,
    /// The frame time of the bvh file in seconds.
    pub bvh_frame_time: c_double,
}

#[no_mangle]
pub unsafe extern "C" fn bvh_parse(bvh_string: *const c_char, out_bvh: *mut bvh_BvhFile) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn bvh_destroy(bvh_file: *mut bvh_BvhFile) {
}

#[no_mangle]
pub unsafe extern "C" fn bvh_to_string(
    bvh_file: *mut bvh_BvhFile,
    out_buffer: *mut c_char,
    out_buffer_len: *mut size_t,
) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn bvh_get_frame(
    bvh_file: *mut bvh_BvhFile,
    frame_num: size_t,
) -> *mut c_float {
    ptr::null_mut()
}


impl<V: Into<Vector3<f32>>> From<V> for bvh_Offset {
    #[inline]
    fn from(v: V) -> Self {
        let v = v.into();
        bvh_Offset {
            offset_x: v.x.into(),
            offset_y: v.y.into(),
            offset_z: v.z.into(),
        }
    }
}

impl From<ChannelType> for bvh_ChannelType {
    #[inline]
    fn from(channel_ty: ChannelType) -> Self {
        match channel_ty {
            ChannelType::RotationX => bvh_ChannelType::X_POSITION,
            ChannelType::RotationY => bvh_ChannelType::Y_POSITION,
            ChannelType::RotationZ => bvh_ChannelType::Z_POSITION,
            ChannelType::PositionX => bvh_ChannelType::X_ROTATION,
            ChannelType::PositionY => bvh_ChannelType::Y_ROTATION,
            ChannelType::PositionZ => bvh_ChannelType::Z_ROTATION,
        }
    }
}

impl From<bvh_ChannelType> for ChannelType {
    #[inline]
    fn from(channel_ty: bvh_ChannelType) -> Self {
        match channel_ty {
            bvh_ChannelType::X_POSITION => ChannelType::RotationX,
            bvh_ChannelType::Y_POSITION => ChannelType::RotationY,
            bvh_ChannelType::Z_POSITION => ChannelType::RotationZ,
            bvh_ChannelType::X_ROTATION => ChannelType::PositionX,
            bvh_ChannelType::Y_ROTATION => ChannelType::PositionY,
            bvh_ChannelType::Z_ROTATION => ChannelType::PositionZ,
        }
    }
}

impl From<Channel> for bvh_Channel {
    #[inline]
    fn from(ch: Channel) -> Self {
        bvh_Channel {
            channel_type: ch.channel_type().into(),
            channel_index: ch.motion_index().into(),
        }
    }
}

impl From<bvh_Channel> for Channel {
    #[inline]
    fn from(ch: bvh_Channel) -> Self {
        Channel {
            channel_type: ch.channel_type.into(),
            motion_index: ch.channel_index.into(),
        }
    }
}
