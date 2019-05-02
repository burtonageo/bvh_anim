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

use bstr::BStr;
use cfile::CFile;
use crate::{
    duation_to_fractional_seconds, fraction_seconds_to_duration, frames_iter_logic,
    joint::{JointPrivateData}, Bvh, Channel, ChannelType, JointData, JointName,
};
use libc::{c_char, c_double, c_float, c_int, c_void, size_t, uint8_t, FILE, strlen};
use mint::Vector3;
use std::{
    alloc::{self, Layout},
    convert::TryFrom,
    error::Error,
    ffi::{CStr, CString},
    fmt,
    io::BufReader,
    mem,
    ptr::{self, NonNull},
    slice,
};

/// Type alias for a function used to allocate memory.
///
/// May return `NULL` to signify allocation failures.
pub type bvh_AllocFunction =
    Option<unsafe extern "C" fn(size: size_t, align: size_t) -> *mut c_void>;

/// Type alias for a function used to free memory.
///
/// Passing a `NULL` pointer value is undefined.
pub type bvh_FreeFunction =
    Option<unsafe extern "C" fn(ptr: *mut c_void, size: size_t, align: size_t)>;

/// A struct which wraps all allocation functions together for convenience.
///
/// This library may make additional transient allocations outside of
/// any specific allocator parameters.
///
/// If the fields are set to `NULL`, then methods will use the rust allocator.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct bvh_AllocCallbacks {
    // Note that the function types must be copied here due to
    // https://github.com/eqrion/cbindgen/issues/326
    /// The `alloc` function.
    pub alloc_cbk: bvh_AllocFunction,
    /// The `free` function.
    pub free_cbk: bvh_FreeFunction,
}

/// The default allocator, which will fall back to the `rust` allocator.
pub const BVH_ALLOCATOR_DEFAULT: bvh_AllocCallbacks = bvh_AllocCallbacks {
    alloc_cbk: None,
    free_cbk: None,
};

#[allow(unused)]
impl bvh_AllocCallbacks {
    /// Create a new `bvh_AllocCallbacks` instance with the given allocation
    /// functions.
    #[inline]
    pub fn new<A, F>(alloc_cbk: A, free_cbk: F) -> Self
    where
        A: Into<bvh_AllocFunction>,
        F: Into<bvh_FreeFunction>,
    {
        bvh_AllocCallbacks {
            alloc_cbk: alloc_cbk.into(),
            free_cbk: free_cbk.into(),
        }
    }

    /// Validates that none of the pointers are valid.
    ///
    /// * If both members are not `null`, returns the struct unchanged.
    /// * If both are `null`, then this returns the rust allocator.
    /// * Otherwise, returns an `err`.
    fn validate(self) -> Result<Self, InvalidAllocator> {
        let bvh_AllocCallbacks {
            alloc_cbk,
            free_cbk,
        } = self;
        match (alloc_cbk, free_cbk) {
            (Some(_), Some(_)) => Ok(self),
            (None, None) => Ok(Default::default()),
            _ => Err(InvalidAllocator { _priv: () }),
        }
    }

    /// Check if the allocator callbacks refer to the rust allocation
    /// callbacks.
    #[inline]
    fn is_rust_allocator(&self) -> bool {
        self == &bvh_AllocCallbacks::default()
    }

    /// Wrapper function to call the alloc callback.
    #[inline]
    unsafe fn alloc<T>(&self) -> *mut c_void {
        self.alloc_n::<T>(1)
    }

    /// Wrapper function to call the alloc callback to allocate enough
    /// memory to hold `n` instances of `T`.
    #[inline]
    unsafe fn alloc_n<T>(&self, n: usize) -> *mut c_void {
        if self.validate().is_err() {
            return ptr::null_mut();
        }

        if let Some(alloc_cbk) = self.alloc_cbk {
            (alloc_cbk)(mem::size_of::<T>(), mem::align_of::<T>())
        } else {
            ptr::null_mut()
        }
    }

    /// Wrapper function to construct an allocation from a `Vec`.
    #[inline]
    unsafe fn copy_vec_to_alloc<T: Copy>(&self, vec: Vec<T>) -> *mut T {
        if self.validate().is_err() {
            return ptr::null_mut();
        }

        if self.is_rust_allocator() {
            Box::into_raw(vec.into_boxed_slice()) as *mut _
        } else {
            let allocation = self.alloc_n::<T>(vec.len()) as *mut T;
            ptr::copy_nonoverlapping(vec.as_ptr(), allocation, vec.len());
            allocation
        }
    }

    /// Wrapper function to create a `Vec` from a raw pointer and a size.
    ///
    /// Takes ownership of the data behind `ptr` - it is a use after free
    /// error to use `ptr` after this.
    #[inline]
    unsafe fn alloc_to_vec<T: Copy>(&self, ptr: *mut T, n: usize) -> Vec<T> {
        if self.validate().is_err() || ptr.is_null() || n == 0 {
            return Vec::new();
        }

        let s = slice::from_raw_parts_mut(ptr, n);

        if self.is_rust_allocator() {
            let boxed = Box::from_raw(s as *mut _);
            Vec::from(boxed)
        } else {
            let mut v = Vec::new();
            v.extend_from_slice(s);
            self.free_n(ptr, n, mem::align_of::<T>());
            v
        }
    }

    #[inline]
    unsafe fn cstring_to_joint_name(&self, cstr: *mut c_char) -> JointName {
        if self.validate().is_err() || cstr.is_null() {
            return Default::default();
        }

        if self.is_rust_allocator() {
            JointName::from(CString::from_raw(cstr))
        } else {
            let len = strlen(cstr) + 1;
            let bytes = self.alloc_to_vec(cstr as *mut u8, len);
            JointName::from(bytes)
        }
    }

    #[inline]
    unsafe fn joint_name_to_cstring(&self, name: &BStr) -> *mut c_char {
        if self.validate().is_err() {
            return ptr::null_mut();
        }

        let name_bytes: &[u8] = name.as_ref();
        if self.is_rust_allocator() {
            CString::new(name_bytes)
                .map(|name| name.into_raw())
                .unwrap_or(ptr::null_mut())
        } else {
            let out_len = name_bytes.len() + 1;
            let allocation = self.alloc_n::<c_char>(out_len);
            ptr::write_bytes(allocation, 0u8, out_len);
            ptr::copy_nonoverlapping(name_bytes.as_ptr(), allocation as *mut _, name.len());
            allocation as *mut _
        }
    }

    /// Wrapper function to call the free callback.
    #[inline]
    unsafe fn free<T>(&self, ptr: *mut T) {
        self.free_n(ptr, 1);
    }

    /// Wrapper function to call the free callback.
    #[inline]
    unsafe fn free_n<T>(&self, ptr: *mut T, n: usize) {
        if let Some(free_cbk) = self.free_cbk {
            (free_cbk)(ptr as *mut _, mem::size_of::<T>() * n, mem::align_of::<T>());
        }
    }
}

/// Create an allocator which uses the `std::alloc` functions.
impl Default for bvh_AllocCallbacks {
    #[inline]
    fn default() -> Self {
        bvh_AllocCallbacks::new(
            Some(rust_alloc as unsafe extern "C" fn(size: size_t, align: size_t) -> *mut c_void),
            Some(rust_free as unsafe extern "C" fn(ptr: *mut c_void, size: size_t, align: size_t)),
        )
    }
}

/// Allocation function which uses the rust allocator.
unsafe extern "C" fn rust_alloc(size: size_t, align: size_t) -> *mut c_void {
    Layout::from_size_align(size, align)
        .map(|layout| alloc::alloc_zeroed(layout))
        .unwrap_or(ptr::null_mut()) as *mut _
}

/// Deallocation function which uses the rust allocator.
unsafe extern "C" fn rust_free(ptr: *mut c_void, size: size_t, align: size_t) {
    Layout::from_size_align(size, align)
        .map(|layout| alloc::dealloc(ptr as *mut _, layout))
        .expect(&format!("Could not deallocate pointer {:p}", ptr))
}

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
    /// The allocator callbacks used to allocate the data for the `bvh_Joint`.
    pub joint_alloc_callbacks: bvh_AllocCallbacks,
    /// The name of the joint.
    pub joint_name: *mut c_char,
    /// The ordered array of channels of the `bvh_Joint`.
    pub joint_channels: *mut bvh_Channel,
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
    /// The allocator callbacks used to allocate the data for the `bvh_BvhFile`.
    pub bvh_alloc_callbacks: bvh_AllocCallbacks,
    /// The array of joints of the bvh.
    pub bvh_joints: *mut bvh_Joint,
    /// The length of the array of joints of the bvh.
    pub bvh_num_joints: size_t,
    /// The number of frames in the bvh file.
    pub bvh_num_frames: size_t,
    /// The number of channels in the bvh file.
    pub bvh_num_channels: size_t,
    /// The array of motion data in the bvh file. This has a total
    /// size of `bvh_num_frames * bvh_num_channels`.
    pub bvh_motion_data: *mut c_float,
    /// The time of each frame of the bvh file in seconds.
    pub bvh_frame_time: c_double,
}

impl fmt::Debug for bvh_BvhFile {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        let joints = ptr_to_array(self.bvh_joints, self.bvh_num_joints);
        let motion_data = ptr_to_array(
            self.bvh_motion_data,
            self.bvh_num_frames * self.bvh_num_channels,
        );

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
/// * On success, this function returns a value greater than `0`, and
///   `out_bvh` will be in a valid state.
///
/// * On failure, this function returns `0`, and `out_bvh` will not
///   be modified.
///
/// This function will not close `bvh_file`.
#[allow(unused)]
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn bvh_read(
    bvh_file: *mut FILE,
    out_bvh: *mut bvh_BvhFile,
    bvh_alloc_callbacks: bvh_AllocCallbacks,
    joint_alloc_callbacks: bvh_AllocCallbacks,
) -> c_int {
    // @TODO(burtonageo): errors
    let cfile = match NonNull::new(bvh_file) {
        Some(f) => BufReader::new(CFile::borrowed(f)),
        None => return 0,
    };

    let bvh = match Bvh::from_reader(cfile) {
        Ok(bvh) => bvh,
        Err(_) => return 0,
    };

    *out_bvh = bvh.into_ffi();

    1
}

/// Parse `bvh_string` as a bvh file, and write the data to `out_bvh`.
///
/// * On success, this function returns a value greater than `0`, and
///   `out_bvh` will be in a valid state.
///
/// * On failure, this function returns `0`, and `out_bvh` will not
///   be modified.
#[allow(unused)]
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn bvh_parse(
    bvh_string: *const c_char,
    out_bvh: *mut bvh_BvhFile,
    bvh_alloc_callbacks: bvh_AllocCallbacks,
    joint_alloc_callbacks: bvh_AllocCallbacks,
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

    *out_bvh = bvh.into_ffi();

    1
}

/// Destroy the `bvh_BvhFile`, cleaning up all memory.
///
/// It is a use after free error to read any fields from the `bvh_file`
/// or the `bvh_Joint`s it owned after this function is called on it.
///
/// This function will free memory using the `bvh_AllocCallbacks`
/// members in `bvh_BvhFile` and `bvh_Joint`.
///
/// Returns `0` if `bvh_file` could not be deallocated, otherwise
/// returns a value greater than `0`.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn bvh_destroy(bvh_file: *mut bvh_BvhFile) -> c_int {
    if bvh_file.is_null() {
        return 0;
    }

    let bvh_file = &mut *bvh_file;

    if bvh_file.bvh_alloc_callbacks.validate().is_err() {
        return 0;
    }

    let num_joints = bvh_file.bvh_num_joints;
    for i in 0..num_joints {
        let offset = match isize::try_from(i) {
            Ok(i) => i,
            Err(_) => continue,
        };

        let joint = &mut *bvh_file.bvh_joints.offset(offset);
        match joint.free_data() {
            Ok(_) => (),
            Err(_) => return 0,
        }
    }

    bvh_file.bvh_alloc_callbacks.free_n(bvh_file.bvh_joints, num_joints);

    let num_motion_values = bvh_file.bvh_num_channels * bvh_file.bvh_num_frames;
    bvh_file.bvh_alloc_callbacks.free_n(bvh_file.bvh_motion_data, num_motion_values);

    1
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
pub unsafe extern "C" fn bvh_to_string(
    bvh_file: *const bvh_BvhFile,
    out_buffer_allocator: bvh_AllocFunction,
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

/// Duplicates the data of `bvh_file` into `clone`.
#[allow(unused)]
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn bvh_duplicate(
    bvh_file: *const bvh_BvhFile,
    clone: *mut bvh_BvhFile,
) -> c_int {
    1
}

/// Get the array of channels at `frame_num` from `bvh_file`.
///
/// If `frame_num` > `bvh_file::bvh_num_frames`, then this
/// will return `NULL`.
///
/// Indexing the returned array with a value greater than
/// `bvh_file::bvh_num_channels` is an out of bounds error.
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
        // @TODO(burtonageo): Check custom allocators
        let joints = if bvh.bvh_num_joints == 0 {
            Vec::new()
        } else {
            let mut out_joints = Vec::with_capacity(bvh.bvh_num_joints);
            {
                let ffi_root = &*bvh.bvh_joints.offset(0);

                let channels = Box::from_raw(slice::from_raw_parts_mut(
                    ffi_root.joint_channels,
                    ffi_root.joint_num_channels,
                ));

                let root = JointData::Root {
                    name: CString::from_raw(ffi_root.joint_name).into(),
                    offset: ffi_root.joint_offset.into(),
                    channels: Vec::from(channels).into_iter().map(Into::into).collect(),
                };

                out_joints.push(root);
            }

            for i in 1..bvh.bvh_num_joints {
                let signed_i = match isize::try_from(i) {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                let ffi_joint = &*bvh.bvh_joints.offset(signed_i);

                let channels = Box::from_raw(slice::from_raw_parts_mut(
                    ffi_joint.joint_channels,
                    ffi_joint.joint_num_channels,
                ));

                let joint = JointData::Child {
                    name: CString::from_raw(ffi_joint.joint_name).into(),
                    offset: ffi_joint.joint_offset.into(),
                    channels: Vec::from(channels).into_iter().map(Into::into).collect(),
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

        let motion_values = Box::from_raw(slice::from_raw_parts_mut(
            bvh.bvh_motion_data,
            bvh.bvh_num_channels * bvh.bvh_num_frames,
        ));

        let out_bvh = Bvh {
            joints,
            motion_values: motion_values.into(),
            num_channels: bvh.bvh_num_channels,
            num_frames: bvh.bvh_num_frames,
            frame_time: fraction_seconds_to_duration(bvh.bvh_frame_time),
        };

        Ok(out_bvh)
    }

    /// Converts the `Bvh` into a `ffi::bvh_BvhFile`, using the default
    /// `bvh_AllocCallback`s.
    ///
    /// # Notes
    ///
    /// This method is only present if the `ffi` feature is enabled.
    pub fn into_ffi(self) -> bvh_BvhFile {
        self.into_ffi_with_allocator(Default::default(), Default::default()).unwrap()
    }

    /// Converts the `Bvh` into a `ffi::bvh_BvhFile`, using the given allocator callbacks
    /// to allocate memory.
    ///
    /// # Notes
    ///
    /// This method is only present if the `ffi` feature is enabled.
    ///
    /// If both allocators are the default allocators, this method will use the rust
    /// allocator, and will move the pointers over without copying them.
    pub fn into_ffi_with_allocator(
        mut self,
        bvh_allocator: bvh_AllocCallbacks,
        joints_allocator: bvh_AllocCallbacks,
    ) -> Result<bvh_BvhFile, ()> {
        let (bvh_allocator, joints_allocator) =
            match (bvh_allocator.validate(), joints_allocator.validate()) {
                (Ok(a0), Ok(a1)) => (a0, a1),
                _ => return Err(()),
            };

        let mut out_bvh = bvh_BvhFile::default();
        out_bvh.bvh_num_joints = self.joints.len();
        out_bvh.bvh_alloc_callbacks = bvh_allocator;

        let joints = mem::replace(&mut self.joints, Vec::new());

        let out_joints = joints
            .into_iter()
            .map(|joint| {
                let channels = joint
                    .channels()
                    .iter()
                    .map(|&c| c.into())
                    .collect::<Vec<bvh_Channel>>();

                bvh_Joint {
                    joint_name: unsafe {
                        joints_allocator.joint_name_to_cstring(joint.name())
                    },
                    joint_num_channels: channels.len(),
                    joint_channels: unsafe {
                        joints_allocator.copy_vec_to_alloc(channels)
                    },
                    joint_parent_index: joint.parent_index().unwrap_or(usize::max_value()),
                    joint_depth: joint.depth(),
                    joint_offset: (*joint.offset()).into(),
                    joint_end_site: joint.end_site().map(|&e| e.into()).unwrap_or_default(),
                    joint_has_end_site: if joint.has_end_site() { 1 } else { 0 },
                    joint_alloc_callbacks: joints_allocator,
                }
            })
            .collect::<Vec<_>>();

        out_bvh.bvh_joints = unsafe {
            bvh_allocator.copy_vec_to_alloc(out_joints)
        };

        out_bvh.bvh_frame_time = duation_to_fractional_seconds(self.frame_time());
        out_bvh.bvh_motion_data = unsafe {
            bvh_allocator.copy_vec_to_alloc(self.motion_values)
        };
        out_bvh.bvh_num_channels = self.num_channels;
        out_bvh.bvh_num_frames = self.num_frames;

        Ok(out_bvh)
    }
}

/// An error type returned from [`bvh_AllocCallbacks::validate`]
/// [`bvh_AllocCallbacks::validate`].
///
/// [`bvh_AllocCallbacks::validate`]: ffi/struct.bvh_AllocCallbacks.html#method.validate
#[derive(Debug)]
pub struct InvalidAllocator {
    _priv: (),
}

impl fmt::Display for InvalidAllocator {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmtr.write_str(self.description())
    }
}

impl Error for InvalidAllocator {
    #[inline]
    fn description(&self) -> &'static str {
        "The allocator is invalid"
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
            bvh_num_frames: Default::default(),
            bvh_num_channels: Default::default(),
            bvh_motion_data: ptr::null_mut(),
            bvh_frame_time: Default::default(),
            bvh_alloc_callbacks: Default::default(),
        }
    }
}

impl Default for bvh_Joint {
    #[inline]
    fn default() -> Self {
        bvh_Joint {
            joint_name: ptr::null_mut(),
            joint_channels: ptr::null_mut(),
            joint_num_channels: 0,
            joint_parent_index: 0,
            joint_depth: 0,
            joint_offset: Default::default(),
            joint_end_site: Default::default(),
            joint_has_end_site: 0,
            joint_alloc_callbacks: Default::default(),
        }
    }
}

impl bvh_Joint {
    #[inline]
    fn free_data(&mut self) -> Result<(), InvalidAllocator> {
        self.joint_alloc_callbacks.validate()?;
        unsafe {
            self.joint_alloc_callbacks.free_n(
                self.joint_name,
                strlen(self.joint_name) + 1);

            self.joint_alloc_callbacks.free_n(
                self.joint_channels,
                self.joint_num_channels);
        }
        Ok(())
    }
}

#[inline]
fn ptr_to_array<'a, T>(data: *mut T, size: libc::size_t) -> &'a [T] {
    if data.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(data as *const _, size) }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bvh_anim,
        ffi::{
            bvh_AllocCallbacks, bvh_BvhFile, bvh_ChannelType, bvh_Offset, bvh_destroy,
            bvh_get_frame, bvh_parse,
        },
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
            let result = bvh_parse(
                bvh_c_str.as_ptr(),
                &mut bvh_ffi,
                Default::default(),
                Default::default(),
            );
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
