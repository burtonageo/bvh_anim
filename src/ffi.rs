#![allow(nonstandard_style)]

//! The ffi interface to the `bvh_anim` crate. You must enable the `ffi` feature
//! to access this module.
//!
//! # Features
//!
//! The `ffi` module defines a C-compatible interface for the `bvh_anim` crate, as
//! well as the methods: [`Bvh::from_ffi`][`Bvh::from_ffi`] and [`Bvh::into_ffi`]
//! [`Bvh::into_ffi`].
//!
//! [`Bvh::from_ffi`]: struct.Bvh.html#method.from_ffi
//! [`Bvh::into_ffi`]: struct.Bvh.html#method.into_ffi

use cfile::CFile;
use crate::{
    duation_to_fractional_seconds, fraction_seconds_to_duration, frames_iter_logic,
    joint::JointPrivateData, Bvh, Channel, ChannelType, JointData,
};
use libc::{c_char, c_double, c_float, c_int, size_t, uint8_t, FILE};
use mint::Vector3;
use std::{
    convert::TryFrom,
    ffi::{CStr, CString},
    fmt,
    io::BufReader,
    mem,
    ptr::{self, NonNull},
    slice,
};

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

/// A single joint in the `HIERARCHY` section of a `bvh_BvhFile`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct bvh_Joint {
    /// The name of the joint.
    pub joint_name: *mut c_char,
    /// The ordered array of channels of the `bvh_Joint`.
    pub joint_channels: *mut bvh_Channel,
    /// _private_ The capacity of the array of channels of the `bvh_Joint`.
    pub _joint_channels_capacity: size_t,
    /// The length of the `joint_channels` array.
    pub joint_num_channels: size_t,
    /// The index of the parent `bvh_Joint` in the `bvh_BvhFile::bvh_joints`
    /// array to which this joint belongs.
    ///
    /// If this joint doesn't have a parent (because it is the root joint)
    ///  then this will have the value `SIZE_MAX`.
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

impl fmt::Debug for bvh_Joint {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        let joint_name = unsafe { CStr::from_ptr(self.joint_name as *const _) };
        let channels = ptr_to_array(self.joint_channels, self.joint_num_channels);

        let has_end_site = if self.joint_has_end_site == 0 {
            false
        } else {
            true
        };

        let parent_index = if self.joint_parent_index == usize::max_value() {
            #[derive(Debug)]
            struct None;
            &None as &dyn fmt::Debug
        } else {
            &self.joint_parent_index as &dyn fmt::Debug
        };

        fmtr.debug_struct("bvh_Joint")
            .field("joint_name", &joint_name)
            .field("joint_channels", &channels)
            .field("joint_parent_index", &parent_index)
            .field("joint_depth", &self.joint_depth)
            .field("joint_offset", &self.joint_offset)
            .field("joint_end_site", &self.joint_end_site)
            .field("joint_has_end_site", &has_end_site)
            .finish()
    }
}

/// A struct representing a bvh file.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct bvh_BvhFile {
    /// The array of joints of the bvh.
    pub bvh_joints: *mut bvh_Joint,
    /// The length of the array of joints of the bvh.
    pub bvh_num_joints: size_t,
    /// _private_ The capacity of the array of joints of the bvh.
    pub _bvh_joints_capacity: size_t,
    /// The number of frames in the bvh file.
    pub bvh_num_frames: size_t,
    /// The number of channels in the bvh file.
    pub bvh_num_channels: size_t,
    /// The array of motion data in the bvh file. This has a total
    /// size of `bvh_num_frames * bvh_num_channels`.
    pub bvh_motion_data: *mut c_float,
    /// _private_ The capacity of the array of motion data of the bvh.
    pub _bvh_motion_data_capacity: size_t,
    /// The time of each frame of the bvh file in seconds.
    pub bvh_frame_time: c_double,
}

impl fmt::Debug for bvh_BvhFile {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        let joints = ptr_to_array(self.bvh_joints, self.bvh_num_joints);
        let motion_data = ptr_to_array(self.bvh_motion_data, self.bvh_num_frames * self.bvh_num_channels);

        fmtr.debug_struct("bvh_BvhFile")
            .field("bvh_joints", &joints)
            .field("bvh_num_frames", &self.bvh_num_frames)
            .field("bvh_num_channels", &self.bvh_num_channels)
            .field("bvh_motion_data", &motion_data)
            .field("bvh_frame_time", &self.bvh_frame_time)
            .finish()
    }
}

/// Read the contents of `bvh_file`, and write the data to `out_bvh`.
///
/// * On success, this function will return `0`, and `out_bvh` will be in a
///   valid state.
///
/// * On failure, this function will return a value greater than `0`,
///   and `out_bvh` will not be modified.
///
/// This function will not close `bvh_file`.
#[no_mangle]
pub unsafe extern "C" fn bvh_read(bvh_file: *mut FILE, out_bvh: *mut bvh_BvhFile) -> c_int {
    // @TODO(burtonageo): errors
    let cfile = match NonNull::new(bvh_file) {
        Some(f) => BufReader::new(CFile::borrowed(f)),
        None => return 1,
    };

    let bvh = match Bvh::from_reader(cfile) {
        Ok(bvh) => bvh,
        Err(_) => return 1,
    };

    *out_bvh = bvh.into_ffi();

    0
}

/// Parse `bvh_string` as a bvh file, and write the data to `out_bvh`.
///
/// * On success, this function returns `0`, and `out_bvh` will be in
///   a valid state.
///
/// * On failure, this function returns a value greater than `0`,
///   and `out_bvh` will not be modified.
#[no_mangle]
pub unsafe extern "C" fn bvh_parse(bvh_string: *const c_char, out_bvh: *mut bvh_BvhFile) -> c_int {
    // @TODO(burtonageo): errors
    if out_bvh.is_null() {
        return 1;
    }

    let bvh_string = CStr::from_ptr(bvh_string);
    let bvh = match Bvh::from_bytes(bvh_string.to_bytes()) {
        Ok(bvh) => bvh,
        Err(_) => {
            return 1;
        }
    };

    *out_bvh = bvh.into_ffi();

    0
}

/// Destroy the `bvh_BvhFile`, cleaning up all memory.
///
/// It is a use after free error to read any fields from the `bvh_file`
/// or the `bvh_Joint`s it owned after this function is called on it.
///
/// This function should only be called on `bvh_BvhFile`s initialised using the
/// `bvh_parse` function, or which have otherwise been created in rust functions
/// using the `Bvh::into_ffi` method. If you have initialised the `bvh_BvhFile`
/// another way, then you will have to destroy it manually.
#[no_mangle]
pub unsafe extern "C" fn bvh_destroy(bvh_file: *mut bvh_BvhFile) {
    if bvh_file.is_null() {
        return;
    }

    let bvh_file = &mut *bvh_file;

    let num_joints = bvh_file.bvh_num_joints;
    for i in 0..num_joints {
        let offset = match isize::try_from(i) {
            Ok(i) => i,
            Err(_) => continue,
        };

        let joint = &mut *bvh_file.bvh_joints.offset(offset);
        let name = CString::from_raw(joint.joint_name);
        let channels = Vec::from_raw_parts(
            joint.joint_channels,
            joint.joint_num_channels,
            joint._joint_channels_capacity,
        );

        drop(name);
        drop(channels);
    }

    let joints = Vec::from_raw_parts(
        bvh_file.bvh_joints,
        num_joints,
        bvh_file._bvh_joints_capacity,
    );

    drop(joints);

    let num_motion_values = bvh_file.bvh_num_channels * bvh_file.bvh_num_frames;
    let data = Vec::from_raw_parts(
        bvh_file.bvh_motion_data,
        num_motion_values,
        bvh_file._bvh_motion_data_capacity,
    );

    drop(data);
}

/// Writes the `bvh_file` to the string `out_buffer`, and the length of
/// the output string (including null terminator) to `out_buffer_len`.
///
/// If `out_buffer` is `NULL`, then it is not written to.
///
/// Returns `0` on success. On failure, this function will return a non-0
/// value, and the contents of  `out_buffer` and `out_buffer_len` will be
/// undefined.
///
/// Generally, it is expected that you will call this function twice when
/// writing to the string: the first time where `out_buffer` is `NULL` so
/// that you can get the length of the string and allocate the buffer to
/// hold it, and then a second time to copy the string into `out_buffer`.
#[allow(unused)]
#[no_mangle]
pub unsafe extern "C" fn bvh_to_string(
    bvh_file: *const bvh_BvhFile,
    out_buffer: *mut c_char,
    out_buffer_len: *mut size_t,
) -> c_int {
    1
}

/// Get the array of channels at `frame_num` from `bvh_file`.
///
/// If `frame_num` > `bvh_file::bvh_num_frames`, then this
/// will return `NULL`.
///
/// Indexing the returned array with a value greater than
/// `bvh_file::bvh_num_channels` is an out of bounds index.
#[no_mangle]
pub unsafe extern "C" fn bvh_get_frame(
    bvh_file: *mut bvh_BvhFile,
    frame_num: size_t,
) -> *mut c_float {
    if bvh_file.is_null() {
        return ptr::null_mut();
    }

    let bvh_BvhFile {
        ref bvh_num_frames,
        ref bvh_num_channels,
        ref bvh_motion_data,
        ..
    } = *bvh_file;

    if frame_num >= *bvh_num_frames {
        return ptr::null_mut();
    }

    frames_iter_logic(*bvh_num_channels, *bvh_num_frames, frame_num)
        .and_then(|range| isize::try_from(range.start).ok())
        .map(|i| bvh_motion_data.offset(i))
        .unwrap_or(ptr::null_mut())
}

impl From<Vector3<f32>> for bvh_Offset {
    #[inline]
    fn from(v: Vector3<f32>) -> Self {
        bvh_Offset {
            offset_x: v.x,
            offset_y: v.y,
            offset_z: v.z,
        }
    }
}

impl From<bvh_Offset> for Vector3<f32> {
    #[inline]
    fn from(offset: bvh_Offset) -> Self {
        let crate::ffi::bvh_Offset {
            offset_x,
            offset_y,
            offset_z,
        } = offset;
        [offset_x, offset_y, offset_z].into()
    }
}

impl From<ChannelType> for bvh_ChannelType {
    #[inline]
    fn from(channel_ty: ChannelType) -> Self {
        match channel_ty {
            ChannelType::RotationX => bvh_ChannelType::X_ROTATION,
            ChannelType::RotationY => bvh_ChannelType::Y_ROTATION,
            ChannelType::RotationZ => bvh_ChannelType::Z_ROTATION,
            ChannelType::PositionX => bvh_ChannelType::X_POSITION,
            ChannelType::PositionY => bvh_ChannelType::Y_POSITION,
            ChannelType::PositionZ => bvh_ChannelType::Z_POSITION,
        }
    }
}

impl From<bvh_ChannelType> for ChannelType {
    #[inline]
    fn from(channel_ty: bvh_ChannelType) -> Self {
        match channel_ty {
            bvh_ChannelType::X_POSITION => ChannelType::PositionX,
            bvh_ChannelType::Y_POSITION => ChannelType::PositionY,
            bvh_ChannelType::Z_POSITION => ChannelType::PositionZ,
            bvh_ChannelType::X_ROTATION => ChannelType::RotationX,
            bvh_ChannelType::Y_ROTATION => ChannelType::RotationY,
            bvh_ChannelType::Z_ROTATION => ChannelType::RotationZ,
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

impl Bvh {
    /// Construct a `Bvh` from a `ffi::bvh_BvhFile`.
    ///
    /// # Notes
    ///
    /// This method is only present if the `ffi` feature is enabled.
    ///
    /// # Safety
    ///
    /// This operation is unsafe because `bvh` may point to memory not
    /// allocated by the rust allocator, which may cause memory errors.
    ///
    /// In addition, this method will take ownership of memory which was
    /// owned by `bvh`, which may cause corruption if there are still
    /// references to `bvh`'s data.
    pub unsafe fn from_ffi(bvh: bvh_BvhFile) -> Result<Self, ()> {
        // @TODO(burtonageo): Massive error checking/consistency checking required here
        let joints = if bvh.bvh_num_joints == 0 {
            if bvh._bvh_joints_capacity > 0 {
                let joints = Vec::from_raw_parts(bvh.bvh_joints, 0, bvh._bvh_joints_capacity);
                drop(joints);
            }
            Vec::new()
        } else {
            let mut out_joints = Vec::with_capacity(bvh.bvh_num_joints);
            {
                let ffi_root = &*bvh.bvh_joints.offset(0);

                let channels = Vec::from_raw_parts(
                    ffi_root.joint_channels,
                    ffi_root.joint_num_channels,
                    ffi_root._joint_channels_capacity,
                );

                let root = JointData::Root {
                    name: CString::from_raw(ffi_root.joint_name).into(),
                    offset: ffi_root.joint_offset.into(),
                    channels: channels.into_iter().map(Into::into).collect(),
                };

                out_joints.push(root);
            }

            for i in 1..bvh.bvh_num_joints {
                let signed_i = match isize::try_from(i) {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                let ffi_joint = &*bvh.bvh_joints.offset(signed_i);

                let channels = Vec::from_raw_parts(
                    ffi_joint.joint_channels,
                    ffi_joint.joint_num_channels,
                    ffi_joint._joint_channels_capacity,
                );

                let joint = JointData::Child {
                    name: CString::from_raw(ffi_joint.joint_name).into(),
                    offset: ffi_joint.joint_offset.into(),
                    channels: channels.into_iter().map(Into::into).collect(),
                    end_site_offset: if ffi_joint.joint_has_end_site == 1 {
                        Some(ffi_joint.joint_end_site.into())
                    } else {
                        None
                    },
                    private: JointPrivateData {
                        self_index: i,
                        parent_index: ffi_joint.joint_parent_index,
                        depth: ffi_joint.joint_depth,
                    },
                };

                out_joints.push(joint);
            }

            out_joints
        };

        let out_bvh = Bvh {
            joints,
            motion_values: Vec::from_raw_parts(
                bvh.bvh_motion_data,
                bvh.bvh_num_channels * bvh.bvh_num_frames,
                bvh._bvh_motion_data_capacity,
            ),
            num_channels: bvh.bvh_num_channels,
            num_frames: bvh.bvh_num_frames,
            frame_time: fraction_seconds_to_duration(bvh.bvh_frame_time),
        };

        Ok(out_bvh)
    }

    /// Converts the `Bvh` into a `ffi::bvh_BvhFile`.
    ///
    /// # Notes
    ///
    /// This method is only present if the `ffi` feature is enabled.
    pub fn into_ffi(mut self) -> bvh_BvhFile {
        let mut out_bvh = bvh_BvhFile::default();
        out_bvh.bvh_num_joints = self.joints.len();

        let joints = mem::replace(&mut self.joints, Vec::new());

        let mut out_bvh_joints_vec = Vec::new();
        out_bvh_joints_vec.reserve_exact(self.joints.len());

        for joint in joints {
            let mut channels = joint
                .channels()
                .iter()
                .map(|&c| c.into())
                .collect::<Vec<_>>();

            let bvh_joint = bvh_Joint {
                joint_name: CString::new(joint.name().as_ref())
                    .map(|name| name.into_raw())
                    .unwrap_or(ptr::null_mut()),
                joint_num_channels: channels.len(),
                _joint_channels_capacity: channels.capacity(),
                joint_channels: channels.as_mut_ptr(),
                joint_parent_index: joint.parent_index().unwrap_or(usize::max_value()),
                joint_depth: joint.depth(),
                joint_offset: (*joint.offset()).into(),
                joint_end_site: joint.end_site().map(|&e| e.into()).unwrap_or_default(),
                joint_has_end_site: if joint.has_end_site() { 1 } else { 0 },
            };

            mem::forget(channels);

            out_bvh_joints_vec.push(bvh_joint);
        }

        out_bvh.bvh_joints = out_bvh_joints_vec.as_mut_ptr();
        out_bvh._bvh_joints_capacity = out_bvh_joints_vec.capacity();

        mem::forget(out_bvh_joints_vec);

        out_bvh.bvh_frame_time = duation_to_fractional_seconds(self.frame_time());
        out_bvh.bvh_motion_data = self.motion_values.as_mut_ptr();
        out_bvh._bvh_motion_data_capacity = self.motion_values.capacity();
        out_bvh.bvh_num_channels = self.num_channels;
        out_bvh.bvh_num_frames = self.num_frames;

        mem::forget(self);

        out_bvh
    }
}

impl From<Bvh> for bvh_BvhFile {
    #[inline]
    fn from(bvh: Bvh) -> Self {
        bvh.into_ffi()
    }
}

impl Default for bvh_BvhFile {
    #[inline]
    fn default() -> Self {
        bvh_BvhFile {
            bvh_joints: ptr::null_mut(),
            bvh_num_joints: Default::default(),
            _bvh_joints_capacity: Default::default(),
            bvh_num_frames: Default::default(),
            bvh_num_channels: Default::default(),
            bvh_motion_data: ptr::null_mut(),
            _bvh_motion_data_capacity: Default::default(),
            bvh_frame_time: Default::default(),
        }
    }
}

impl Default for bvh_Joint {
    #[inline]
    fn default() -> Self {
        bvh_Joint {
            joint_name: ptr::null_mut(),
            joint_channels: ptr::null_mut(),
            _joint_channels_capacity: Default::default(),
            joint_num_channels: 0,
            joint_parent_index: 0,
            joint_depth: 0,
            joint_offset: Default::default(),
            joint_end_site: Default::default(),
            joint_has_end_site: 0,
        }
    }
}

#[inline]
fn ptr_to_array<'a, T>(data: *mut T, size: libc::size_t) -> &'a [T] {
    if data.is_null() {
        &[]
    } else {
        unsafe {
            slice::from_raw_parts(data as *const _, size)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bvh_anim,
        ffi::{bvh_BvhFile, bvh_ChannelType, bvh_Offset, bvh_destroy, bvh_get_frame, bvh_parse},
    };
    use libc::strcmp;
    use std::ffi::CStr;

    fn check_ffi_bvh(mut bvh_ffi: bvh_BvhFile) {
        assert_eq!(bvh_ffi.bvh_num_joints, 2);

        unsafe {
            let root = *bvh_ffi.bvh_joints.offset(0);

            let expected_name = CStr::from_bytes_with_nul(b"Base\0").unwrap();
            assert_eq!(strcmp(root.joint_name, expected_name.as_ptr()), 0);

            assert_eq!(root.joint_num_channels, 6);

            let expected_channels = [
                bvh_ChannelType::X_POSITION,
                bvh_ChannelType::Y_POSITION,
                bvh_ChannelType::Z_POSITION,
                bvh_ChannelType::Z_ROTATION,
                bvh_ChannelType::X_ROTATION,
                bvh_ChannelType::Y_ROTATION,
            ];

            for i in 0..root.joint_num_channels {
                let channel = *root.joint_channels.offset(i as isize);
                assert_eq!(channel.channel_index, i);
                assert_eq!(channel.channel_type, expected_channels[i]);
            }

            assert_eq!(root.joint_offset, Default::default());
            assert_eq!(root.joint_parent_index, usize::max_value());
            assert_eq!(root.joint_has_end_site, 0);
        }

        unsafe {
            let end = *bvh_ffi.bvh_joints.offset(1);

            let expected_name = CStr::from_bytes_with_nul(b"End\0").unwrap();
            assert_eq!(strcmp(end.joint_name, expected_name.as_ptr()), 0);

            assert_eq!(end.joint_num_channels, 3);

            let expected_channels = [
                bvh_ChannelType::Z_ROTATION,
                bvh_ChannelType::X_ROTATION,
                bvh_ChannelType::Y_ROTATION,
            ];

            for i in 0..end.joint_num_channels {
                let channel = *end.joint_channels.offset(i as isize);
                assert_eq!(channel.channel_index, i + 6);
                assert_eq!(channel.channel_type, expected_channels[i]);
            }

            let expected_offset = bvh_Offset {
                offset_z: 15.0,
                ..Default::default()
            };

            assert_eq!(end.joint_offset, expected_offset);
            assert_eq!(end.joint_parent_index, 0);

            let expected_end_site = bvh_Offset {
                offset_z: 30.0,
                ..Default::default()
            };

            assert_eq!(end.joint_has_end_site, 1);
            assert_eq!(end.joint_end_site, expected_end_site);
        }

        assert_eq!(bvh_ffi.bvh_frame_time, 0.033333333);
        for i in 0..bvh_ffi.bvh_num_frames {
            let frame = unsafe { bvh_get_frame(&mut bvh_ffi, i) };
            for j in 0..bvh_ffi.bvh_num_channels {
                let channel = unsafe { *frame.offset(j as isize) };
                assert_eq!(channel, i as f32);
            }
        }
    }

    #[test]
    fn into_ffi() {
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
            Frames: 5
            Frame Time: 0.033333333333
            0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0
            1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0
            2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0
            3.0 3.0 3.0 3.0 3.0 3.0 3.0 3.0 3.0
            4.0 4.0 4.0 4.0 4.0 4.0 4.0 4.0 4.0
        };

        let mut bvh_ffi = bvh.into_ffi();

        check_ffi_bvh(bvh_ffi);

        unsafe {
            bvh_destroy(&mut bvh_ffi);
        }
    }

    #[test]
    fn ffi_parse() {
        const BVH_BYTES: &[u8] = b"
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
            Frames: 5
            Frame Time: 0.033333333333
            0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0
            1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0
            2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0 2.0
            3.0 3.0 3.0 3.0 3.0 3.0 3.0 3.0 3.0
            4.0 4.0 4.0 4.0 4.0 4.0 4.0 4.0 4.0
        \0";

        let bvh_c_str = CStr::from_bytes_with_nul(BVH_BYTES).unwrap();
        let mut bvh_ffi = bvh_BvhFile::default();

        unsafe {
            let result = bvh_parse(bvh_c_str.as_ptr(), &mut bvh_ffi);
            assert_eq!(result, 0);
        }

        check_ffi_bvh(bvh_ffi);

        unsafe {
            bvh_destroy(&mut bvh_ffi);
        }
    }
}
