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

use crate::{
    duation_to_fractional_seconds, frames_iter_logic,
    joint::{JointData, JointName, JointPrivateData},
    Bvh, Channel, ChannelType,
};
use bstr::BStr;
use cfile::CFile;
use foreign_types::ForeignType;
use libc::{c_char, c_double, c_float, c_int, c_void, size_t, strlen, uint32_t, uint8_t, FILE};
use mint::Vector3;
use pkg_version::{pkg_version_major, pkg_version_minor, pkg_version_patch};
use static_assertions::{assert_eq_align, assert_eq_size};
use std::{
    alloc::{self, Layout},
    borrow::Borrow,
    convert::TryFrom,
    error::Error,
    ffi::{CStr, CString},
    fmt,
    io::BufReader,
    mem,
    ptr::{self, NonNull},
    slice,
    time::Duration,
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
// @NOTE: These must be in the same order as `crate::ChannelType`.
pub enum bvh_ChannelType {
    /// An `Xposition` channel type.
    X_POSITION,
    /// A `Yposition` channel type.
    Y_POSITION,
    /// A `Zposition` channel type.
    Z_POSITION,
    /// An `Xrotation` channel type.
    X_ROTATION,
    /// A `Yrotation` channel type.
    Y_ROTATION,
    /// A `Zrotation` channel type.
    Z_ROTATION,
}

assert_eq_size!(ChannelType, bvh_ChannelType);
assert_eq_align!(ChannelType, bvh_ChannelType);

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

assert_eq_size!(Channel, bvh_Channel);
assert_eq_align!(Channel, bvh_Channel);

/// A single bvh file.
///
/// # Notes
///
/// Operations on a `bvh_BvhFile` comprised of zeroed memory will behave as
/// if a `nullptr` `bvh_BvhFile` is passed to a function.
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct bvh_BvhFile {
    /// Opaque internals of the `bvh_BvhFile` object. Do not modify.
    pub __bvh_internals: [u8; 80],
}

assert_eq_size!(bvh_BvhFile, Bvh);
assert_eq_align!(bvh_BvhFile, Bvh);

impl bvh_BvhFile {
    const ZERO_BVH: Self = Self { __bvh_internals: [0u8; 80] };

    #[inline]
    fn from_bvh(bvh: Bvh) -> Self {
        unsafe { mem::transmute(bvh) }
    }

    #[inline]
    unsafe fn into_bvh(self) -> Bvh {
        mem::transmute(self)
    }

    #[inline]
    unsafe fn as_bvh_ref<'a>(this: *const Self) -> Option<&'a Bvh> {
        let this = this as *mut _;
        if let Some(nonnull) = NonNull::new(this) {
            Some(&*nonnull.as_ptr())
        } else {
            None
        }
    }

    #[inline]
    unsafe fn as_bvh_mut<'a>(this: *mut Self) -> Option<&'a mut Bvh> {
        if let Some(nonnull) = NonNull::new(this as *mut _) {
            Some(&mut *nonnull.as_ptr())
        } else {
            None
        }
    }

    #[inline]
    const fn zeroed() -> Self {
        Self::ZERO_BVH
    }
}

impl Default for bvh_BvhFile {
    #[inline]
    fn default() -> Self {
        Self::zeroed()
    }
}

/// Get the version of the linked `bvh` library.
///
/// If any parameters are `NULL`, then they will be ignored.
#[no_mangle]
pub unsafe extern "C" fn bvh_get_version(
    major: *mut uint32_t,
    minor: *mut uint32_t,
    patch: *mut uint32_t,
) {
    if !major.is_null() {
        *major = pkg_version_major!();
    }

    if !minor.is_null() {
        *minor = pkg_version_minor!();
    }

    if !patch.is_null() {
        *patch = pkg_version_patch!();
    }
}

/// Read the contents of `bvh_file`, and write the data to `out_bvh`,
/// using the default allocator.
///
/// * On success, this function returns a value greater than `0`, and
///   `out_bvh` will be in a valid state.
///
/// * On failure, this function returns `0`, and `out_bvh` will not
///   be modified.
///
/// This function will not close `bvh_file`.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn bvh_file_read(bvh_file: *mut FILE, out_bvh: *mut bvh_BvhFile) -> c_int {
    let mut cfile = match NonNull::new(bvh_file) {
        Some(f) => BufReader::new(CFile::from_ptr(f.as_ptr())),
        None => return 0,
    };

    let bvh = match Bvh::from_reader(&mut cfile) {
        Ok(bvh) => bvh,
        Err(_) => return 0,
    };

    let src_bvh = &bvh as *const _ as *const bvh_BvhFile;
    ptr::copy_nonoverlapping(src_bvh, out_bvh, 1);

    mem::forget(bvh);
    // Avoid running destructor of `bvh_file`.
    mem::forget(cfile);

    1
}

/// Parse `bvh_string` as a bvh file, and write the data to `out_bvh`,
/// using `BVH_ALLOCATOR_DEFAULT`.
///
/// * On success, this function returns a value greater than `0`, and
///   `out_bvh` will be in a valid state.
///
/// * On failure, this function returns `0`, and `out_bvh` will not
///   be modified.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn bvh_file_parse(
    bvh_string: *const c_char,
    out_bvh: *mut bvh_BvhFile,
) -> c_int {
    // @TODO(burtonageo): errors
    if out_bvh.is_null() {
        return 0;
    }

    let bvh_string = CStr::from_ptr(bvh_string);
    let bvh = match Bvh::from_bytes(bvh_string.to_bytes()) {
        Ok(bvh) => bvh,
        Err(_) => {
            return 0;
        }
    };

    let src_bvh = &bvh as *const _ as *const bvh_BvhFile;
    ptr::copy_nonoverlapping(src_bvh, out_bvh, 1);

    mem::forget(bvh);

    1
}

/// Destroy the `bvh_BvhFile`, cleaning up all memory.
///
/// It is a use after free error to read any fields from the `bvh_file`
/// or the `bvh_Joint`s it owned after this function is called on it.
///
/// Returns `0` if `bvh_file` could not be deallocated, otherwise
/// returns a value greater than `0`.
///
/// `bvh_file` will be zeroed if it is successfully destroyed.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn bvh_file_destroy(bvh_file: *mut bvh_BvhFile) -> c_int {
    NonNull::new(bvh_file)
        .map(|mut bvh| {
            let _ = Bvh::from_ffi(bvh.as_mut());
            1
        })
        .unwrap_or(0)
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
#[must_use]
pub unsafe extern "C" fn bvh_file_to_string(
    bvh_file: *const bvh_BvhFile,
    out_buffer: *mut *mut c_char,
) -> c_int {
    /*
    if bvh_file.is_null() {
        return 1;
    }

    let bvh = match Bvh::from_ffi(*bvh_file) {
        Ok(bvh) => bvh,
        Err(_) => return 1,
    };

    let string = bvh.to_bstring();
    *bvh_file = bvh.into_ffi();
    let out_buf_len = string.len() + 1;

    *out_buffer = (out_buffer_allocator(out_buf_len, mem::align_of::<u8>()) as *mut _);
    let out_buffer = *out_buffer;

    if out_buffer.is_null() {
        return 1;
    }

    unsafe {
        ptr::write_bytes(out_buffer, 0u8, out_buf_len);
        ptr::copy_nonoverlapping(string.as_ptr(), out_buffer as *mut _, out_buf_len);
    }
    */

    0
}

/// Compares `bvh_file_0` and `bvh_file_1` to check if each of their `JOINTS`
/// heirarchy and `MOTION` sections have the same value.
///
/// If the values match, returns `1`, otherwise returns `0`.
///
/// If both values are `NULL` or point to the same `bvh_BvhFile`, returns `1`.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn bvh_file_equal(
    bvh_file_0: *const bvh_BvhFile,
    bvh_file_1: *const bvh_BvhFile,
) -> c_int {
    // Quick check for reference equality.
    if ptr::eq(bvh_file_0, bvh_file_1) {
        return 1;
    }

    bvh_BvhFile::as_bvh_ref(bvh_file_0)
        .and_then(|f0| bvh_BvhFile::as_bvh_ref(bvh_file_1).map(|f1| if f0 == f1 { 1 } else { 0 }))
        .unwrap_or(0)
}

/// Duplicates the data of `bvh_file` into `clone`.
///
/// If the `bvh_file` cannot be cloned, this method will return `0`, otherwise
/// it will return `1`.
#[allow(unused)]
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn bvh_file_duplicate(
    bvh_file: *const bvh_BvhFile,
    clone: *mut bvh_BvhFile,
) -> c_int {
    if clone.is_null() {
        0
    } else {
        if let Some(bvh_file) = bvh_BvhFile::as_bvh_ref(bvh_file) {
            let cloned = bvh_file.clone();
            let clone_src_ptr = (&cloned) as *const _ as *const bvh_BvhFile;
            ptr::copy_nonoverlapping(clone_src_ptr, clone, 1);
            mem::forget(cloned);
            1
        } else {
            0
        }
    }
}

/// Get the array of channels at `frame_num` from `bvh_file`.
///
/// If `frame_num` > `bvh_file::bvh_num_frames`, then this
/// will return `NULL`.
///
/// Indexing the returned array with a value greater than
/// `bvh_file::bvh_num_channels` is an out of bounds error.
#[no_mangle]
pub unsafe extern "C" fn bvh_file_get_frame(
    bvh_file: *const bvh_BvhFile,
    frame_num: size_t,
) -> *const c_float {
    bvh_BvhFile::as_bvh_ref(bvh_file)
        .and_then(|bvh| {
            frames_iter_logic(bvh.num_channels, bvh.num_frames, frame_num)
                .map(|range| bvh.motion_values[range].as_ptr())
        })
        .unwrap_or(ptr::null())
}

/// Get the mutable array of channels at `frame_num` from `bvh_file`.
///
/// If `frame_num` > `bvh_file::bvh_num_frames`, then this
/// will return `NULL`.
///
/// Indexing the returned array with a value greater than
/// `bvh_file::bvh_num_channels` is an out of bounds error.
#[no_mangle]
pub unsafe extern "C" fn bvh_file_get_frame_mut(
    bvh_file: *mut bvh_BvhFile,
    frame_num: size_t,
) -> *mut c_float {
    bvh_BvhFile::as_bvh_mut(bvh_file)
        .and_then(|bvh| {
            frames_iter_logic(bvh.num_channels, bvh.num_frames, frame_num)
                .map(|range| bvh.motion_values[range].as_mut_ptr())
        })
        .unwrap_or(ptr::null_mut())
}

/// Get the total number of frames in the `bvh_BvhFile`.
///
/// If `bvh_file` is `NULL`, this function returns `0`.
#[no_mangle]
pub unsafe extern "C" fn bvh_file_get_num_frames(bvh_file: *const bvh_BvhFile) -> size_t {
    bvh_BvhFile::as_bvh_ref(bvh_file)
        .map(|bvh| bvh.num_frames)
        .unwrap_or(0)
}

/// Get the total number of channels in the `bvh_BvhFile`.
///
/// If `bvh_file` is `NULL`, this function returns `0`.
#[no_mangle]
pub unsafe extern "C" fn bvh_file_get_num_channels(bvh_file: *const bvh_BvhFile) -> size_t {
    bvh_BvhFile::as_bvh_ref(bvh_file)
        .map(|bvh| bvh.num_channels)
        .unwrap_or(0)
}

/// Get the frame time of the `bvh_BvhFile`. The frame time
/// is given in seconds.
///
/// If `bvh_file` is `NULL`, this function returns `0.0`.
#[no_mangle]
pub unsafe extern "C" fn bvh_file_get_frame_time(bvh_file: *const bvh_BvhFile) -> c_double {
    bvh_BvhFile::as_bvh_ref(bvh_file)
        .map(|bvh| bvh.frame_time.as_secs_f64())
        .unwrap_or(0.0)
}

/// Set the frame time of the `bvh_BvhFile` to `new_frame_time`. The frame time
/// is given in seconds.
///
/// If `bvh_file` is `NULL`, this function does nothing.
#[no_mangle]
pub unsafe extern "C" fn bvh_file_set_frame_time(
    bvh_file: *mut bvh_BvhFile,
    new_frame_time: c_double,
) {
    if let Some(bvh) = bvh_BvhFile::as_bvh_mut(bvh_file) {
        bvh.frame_time = Duration::from_secs_f64(new_frame_time)
    }
}

/// Get the array of `bvh_BvhJoint`s in the heirarchy of this
/// `bvh_BvhFile`.
///
/// Use the `bvh_file_get_num_joints` function to get the size
/// of the returned array.
///
/// If `bvh_file` is `NULL`, then this method will return `NULL`.
#[no_mangle]
pub unsafe extern "C" fn bvh_file_get_joints(bvh_file: *const bvh_BvhFile) -> *const bvh_Joint {
    bvh_BvhFile::as_bvh_ref(bvh_file)
        .map(|bvh| bvh.joints.as_ptr() as *const _)
        .unwrap_or(ptr::null())
}

/// Get the mutable array of `bvh_BvhJoint`s in the heirarchy of this
/// `bvh_BvhFile`.
///
/// Use the `bvh_file_get_num_joints` function to get the size
/// of the returned array.
///
/// If `bvh_file` is `NULL`, then this method will return `NULL`.
#[no_mangle]
pub unsafe extern "C" fn bvh_file_get_joints_mut(bvh_file: *mut bvh_BvhFile) -> *mut bvh_Joint {
    bvh_BvhFile::as_bvh_mut(bvh_file)
        .map(|bvh| bvh.joints.as_mut_ptr() as *mut _)
        .unwrap_or(ptr::null_mut())
}

/// Get the number of joints in the `bvh_BvhFile`.
#[no_mangle]
pub unsafe extern "C" fn bvh_file_get_num_joints(bvh_file: *const bvh_BvhFile) -> size_t {
    bvh_BvhFile::as_bvh_ref(bvh_file)
        .map(|b| b.joints.len())
        .unwrap_or(0)
}

/// Add a child `bvh_Joint` to the current `bvh_Joint`.
#[no_mangle]
pub unsafe extern "C" fn bvh_file_add_joint_child(
    bvh_file: *mut bvh_BvhFile,
    joint: *const bvh_Joint,
    child_name: *const c_char,
    child_offset: bvh_Offset,
    child_has_end_site: c_int,
    child_end_site: bvh_Offset,
    child_channels: *const bvh_Channel,
) -> c_int {
    todo!()
}

/// Write the `bvh_file` out to `out_file`.
///
/// If this function succeeds, it will return `1`, otherwise it will
/// return `0`.
///
/// If `format_options` is `NULL`, then the following default options will be
/// used:
///
/// * `format_options_line_terminator`: `LINE_TERMINATOR_UNIX`
/// * `format_options_indent_char`: `INDENT_CHAR_SPACES`
/// * `format_options_num_indent_levels`: `4`
/// * `format_options_offset_significant_figures`: `9`
/// * `format_options_frame_time_significant_figures`: `9`
/// * `format_options_motion_values_significant_figures`: `9`
#[no_mangle]
pub unsafe extern "C" fn bvh_file_write(
    bvh_file: *const bvh_BvhFile,
    out_file: *mut FILE,
    format_options: *const bvh_FormatOptions,
) -> c_int {
    let _ = format_options;
    if out_file.is_null() {
        return 0;
    }

    if let Some(bvh) = bvh_BvhFile::as_bvh_ref(bvh_file) {
        let mut out_file = CFile::from_ptr(out_file);
        if let Err(_) = bvh.write_to(&mut out_file) {
            return 0;
        }
        1
    } else {
        0
    }
}

/// A single joint in the `HIERARCHY` section of a `bvh_BvhFile`.
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct bvh_Joint {
    /// Opaque internals of the `bvh_Joint` object. Do not modify.
    pub __joint_internals: [u8; 168],
}

assert_eq_size!(bvh_Joint, JointData);
assert_eq_align!(bvh_Joint, JointData);

impl bvh_Joint {
    #[inline]
    fn from_joint_data(joint_data: JointData) -> Self {
        unsafe { mem::transmute(joint_data) }
    }

    #[inline]
    unsafe fn into_joint_data(self) -> JointData {
        mem::transmute(self)
    }

    #[inline]
    unsafe fn as_joint_data_ref<'a>(this: *const Self) -> Option<&'a JointData> {
        let this = this as *mut _;
        if let Some(nonnull) = NonNull::new(this) {
            Some(&*nonnull.as_ptr())
        } else {
            None
        }
    }

    #[inline]
    unsafe fn as_joint_data_mut<'a>(this: *mut Self) -> Option<&'a mut JointData> {
        if let Some(nonnull) = NonNull::new(this as *mut _) {
            Some(&mut *nonnull.as_ptr())
        } else {
            None
        }
    }

    #[inline]
    const fn zeroed() -> Self {
        Self {
            __joint_internals: [0; 168],
        }
    }
}

impl Default for bvh_Joint {
    #[inline]
    fn default() -> Self {
        Self::zeroed()
    }
}

/// Get the array of `bvh_Channel`s from this `bvh_Joint`.
///
/// Use the `bvh_joint_get_num_channels` function to get the size
/// of the returned array.
///
/// If `joint` is `NULL`, this method will return `NULL`.
#[no_mangle]
pub unsafe extern "C" fn bvh_joint_get_channels(joint: *const bvh_Joint) -> *const bvh_Channel {
    bvh_Joint::as_joint_data_ref(joint)
        .map(|jdata| jdata.channels().as_ptr() as *const _)
        .unwrap_or(ptr::null())
}

/// Get the number of channels in the `bvh_Joint`.
///
/// If `joint` is `NULL`, returns `0`.
#[no_mangle]
pub unsafe extern "C" fn bvh_joint_get_num_channels(joint: *const bvh_Joint) -> size_t {
    bvh_Joint::as_joint_data_ref(joint)
        .map(|jdata| jdata.channels().len())
        .unwrap_or(0)
}

/// Get the end site of this `bvh_Joint`.
///
/// If `joint` does not have an end site, or if `joint` is `NULL`, this function
/// will return a zero vector.
///
/// To check if `joint` has an end site, use the `bvh_joint_has_end_site` function.
#[no_mangle]
pub unsafe extern "C" fn bvh_joint_get_end_site(joint: *const bvh_Joint) -> bvh_Offset {
    bvh_Joint::as_joint_data_ref(joint)
        .and_then(|jdata| jdata.end_site())
        .map(Into::into)
        .unwrap_or_default()
}

/// If this joint has an end site, returns `1`, otherwise returns `0`.
///
/// If `joint` is `NULL`, returns `0`.
#[no_mangle]
pub unsafe extern "C" fn bvh_joint_has_end_site(joint: *const bvh_Joint) -> c_int {
    bvh_Joint::as_joint_data_ref(joint)
        .map(|jdata| if jdata.end_site().is_some() { 1 } else { 0 })
        .unwrap_or(0)
}

/// Get the name of the joint as a byte-encoded null-terminated string.
///
/// If `joint` is `NULL`, returns `NULL`.
#[no_mangle]
pub unsafe extern "C" fn bvh_joint_get_name(joint: *const bvh_Joint) -> *const c_char {
    bvh_Joint::as_joint_data_ref(joint)
        .map(|joint| joint.name().as_ptr() as *const _)
        .unwrap_or(ptr::null())
}

/// Set the name of the `bvh_Joint` to `new_name`.
///
/// If `joint` is `NULL`, this function is a no-op.
/// If `new_name` is `NULL`, then the name will be set to the empty string.
#[no_mangle]
pub unsafe extern "C" fn bvh_joint_set_name(joint: *mut bvh_Joint, new_name: *const c_char) {
    if let Some(jdata) = bvh_Joint::as_joint_data_mut(joint) {
        let name = CStr::from_ptr(new_name);
        jdata.set_name(name);
    }
}

/// Get the offset of the `bvh_Joint`.
///
/// If `joint` is `NULL`, returns a zero vector.
#[no_mangle]
pub unsafe extern "C" fn bvh_joint_get_offset(joint: *const bvh_Joint) -> bvh_Offset {
    bvh_Joint::as_joint_data_ref(joint)
        .map(|jdata| jdata.offset().into())
        .unwrap_or_default()
}

/// Set the offset of the `bvh_Joint` to `new_offset`.
///
/// If `joint` is `NULL`, this function is a no-op.
#[no_mangle]
pub unsafe extern "C" fn bvh_joint_set_offset(joint: *mut bvh_Joint, new_offset: bvh_Offset) {
    if let Some(jdata) = bvh_Joint::as_joint_data_mut(joint) {
        jdata.set_offset(new_offset, false);
    }
}

/// Get the depth of this `bvh_Joint` from the root in the heirarchy. The root itself has
/// a depth of `0`, and each descendent of the current joint has a depth 1 greater than
/// the current joint.
///
/// If `joint` is `NULL`, returns `0`.
#[no_mangle]
pub unsafe extern "C" fn bvh_joint_get_depth(joint: *const bvh_Joint) -> size_t {
    bvh_Joint::as_joint_data_ref(joint)
        .map(|jdata| jdata.depth())
        .unwrap_or(0)
}

impl<V: Borrow<Vector3<f32>>> From<V> for bvh_Offset {
    #[inline]
    fn from(v: V) -> Self {
        let v = v.borrow();
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
    /// Construct a `Bvh` from a `ffi::bvh_BvhFile`. `bvh_file` will
    /// be left in a zeroed state after the conversion.
    ///
    /// # Notes
    ///
    /// This method is only present if the `ffi` feature is enabled.
    ///
    /// # Safety
    ///
    /// This method will take ownership of memory which was owned
    /// by `bvh`, which may cause corruption if there are still
    /// references to `bvh_file`'s data.
    #[inline]
    pub unsafe fn from_ffi(bvh_file: &mut bvh_BvhFile) -> Self {
        let bvh = unsafe { mem::transmute(*bvh_file) };
        *bvh_file = Default::default();
        bvh
    }

    /// Converts the `Bvh` into a `ffi::bvh_BvhFile`.
    ///
    /// # Notes
    ///
    /// This method is only present if the `ffi` feature is enabled.
    #[inline]
    pub fn into_ffi(self) -> bvh_BvhFile {
        let bvh_file = unsafe { mem::transmute(self) };

        bvh_file
    }
}

/// Which character type to use for whitespace indentation.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum bvh_IndentChar {
    /// Use a tab ('\t') for each level of indentation.
    INDENT_CHAR_TAB,
    /// Use a space (' ') for each level of indentation.
    INDENT_CHAR_SPACES,
}

/// Which style of line terminator to use when serializing a `bvh_BvhFile`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum bvh_LineTerminator {
    /// Use UNIX style line endings ('\n').
    LINE_TERMINATOR_UNIX,
    /// Use Windows style line endings ('\r\n').
    LINE_TERMINATOR_WINDOWS,
}

/// Options for formatting a bvh file when serializing it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct bvh_FormatOptions {
    /// Which style of line terminator to use. See the definition of
    /// `bvh_LineTerminator` for more info.
    pub format_options_line_terminator: bvh_LineTerminator,
    /// Which whitespace character to use for indentation. See the definition of
    /// `bvh_IndentChar` for more info.
    pub format_options_indent_char: bvh_IndentChar,
    /// How many whitespace characters to use per level of indentation.
    pub format_options_num_indent_levels: size_t,
    /// How many significant figures to use for each `OFFSET` value in the
    /// `JOINTS` heirarchy.
    ///
    /// If this value is equal to or greater than `SIZE_MAX`, then the minimum
    /// precision required will be used.
    pub format_options_offset_significant_figures: size_t,
    /// How many significant figures to use when writing the frame time.
    ///
    /// If this value is equal to or greater than `SIZE_MAX`, then the minimum
    /// precision required will be used.
    pub format_options_frame_time_significant_figures: size_t,
    /// How many significant figures to use when writing out each value in the
    /// `MOTION` section.
    ///
    /// If this value is equal to or greater than `SIZE_MAX`, then the minimum
    /// precision required will be used.
    pub format_options_motion_values_significant_figures: size_t,
}

/*
#[cfg(test)]
mod tests {
    use crate::ffi::{
        bvh_BvhFile, bvh_ChannelType, bvh_Offset, bvh_destroy, bvh_get_frame,
        bvh_file_parse,
    };
    use libc::strcmp;
    use std::ffi::CStr;

    fn check_ffi_bvh(mut bvh_ffi: bvh_BvhFile) {
        assert_eq!(bvh_get_num_joints(&bvh_ffi), 2);

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
            let result = bvh_destroy(&mut bvh_ffi);
            assert_ne!(result, 0);
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
            assert_ne!(result, 0);
        }

        check_ffi_bvh(bvh_ffi);

        unsafe {
            let result = bvh_destroy(&mut bvh_ffi);
            assert_ne!(result, 0);
        }
    }

    #[test]
    fn default_alloc_is_rust_allocator() {
        let alloc = bvh_AllocCallbacks::default();
        assert!(alloc.is_rust_allocator());
    }
}
*/
