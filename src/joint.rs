use bstr::{BStr, BString, ByteSlice};
use crate::Channel;
use mint::Vector3;
use smallvec::SmallVec;
use std::{
    cmp::{Ordering, PartialEq, PartialOrd},
    ffi::{CStr, CString},
    fmt, mem,
    ops::{Deref, DerefMut},
    str,
};

/// Internal representation of a joint.
#[derive(Clone, Debug, PartialEq)]
pub enum JointData {
    /// Root of the skeletal hierarchy.
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

impl JointData {
    /// Returns `true` if the `Joint` is the root `Joint`, or `false` if it isn't.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bvh_anim::bvh;
    /// let bvh = bvh! {
    ///     HIERARCHY
    ///     ROOT Hips
    ///     {
    ///         OFFSET 0.0 0.0 0.0
    ///         CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation
    ///         End Site {
    ///             OFFSET 0.0 0.0 30.0
    ///         }
    ///     }
    ///     MOTION
    ///     Frames: 0
    ///     Frame Time: 0.0333333
    /// };
    ///
    /// let root = bvh.root_joint().unwrap();
    /// assert!(root.data().is_root());
    /// ```
    #[inline]
    pub fn is_root(&self) -> bool {
        match *self {
            JointData::Root { .. } => true,
            _ => false,
        }
    }

    /// Returns `true` if the `Joint` is a child `Joint`, or `false` if it isn't.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bvh_anim::bvh;
    /// let bvh = bvh! {
    /// #   HIERARCHY
    /// #   ROOT Base
    /// #   {
    /// #       OFFSET 0.0 0.0 0.0
    /// #       CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation
    /// #       JOINT End
    /// #       {
    /// #           OFFSET 0.0 0.0 15.0
    /// #           CHANNELS 3 Zrotation Xrotation Yrotation
    /// #           End Site
    /// #           {
    /// #               OFFSET 0.0 0.0 30.0
    /// #           }
    /// #       }
    /// #   }
    /// #   MOTION
    /// #   Frames: 0
    /// #   Frame Time: 0.0333333
    ///     // bvh hierarchy unspecified ..
    /// };
    ///
    /// for joint in bvh.joints().skip(1) {
    ///     assert!(joint.data().is_child());
    /// }
    /// ```
    #[inline]
    pub fn is_child(&self) -> bool {
        match *self {
            JointData::Child { .. } => true,
            _ => false,
        }
    }

    /// Returns the name of the `JointData`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bvh_anim::bvh;
    /// let bvh = bvh! {
    ///     HIERARCHY
    ///     ROOT Hips
    ///     {
    ///         OFFSET 0.0 0.0 0.0
    ///         CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation
    ///         // ...
    /// #       End Site {
    /// #           OFFSET 0.0 0.0 30.0
    /// #       }
    ///     }
    ///     MOTION
    ///     // ...
    /// #   Frames: 0
    /// #   Frame Time: 0.0333333
    /// };
    ///
    /// let root = bvh.root_joint().unwrap();
    /// assert_eq!(root.data().name(), "Hips");
    /// ```
    #[inline]
    pub fn name(&self) -> &BStr {
        match *self {
            JointData::Root { ref name, .. } | JointData::Child { ref name, .. } => name.as_ref(),
        }
    }

    /// Returns the offset of the `JointData` if it exists, or `None`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bvh_anim::bvh;
    /// let bvh = bvh! {
    ///     HIERARCHY
    ///     ROOT Hips
    ///     {
    ///         OFFSET 1.2 3.4 5.6
    ///         CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation
    ///         // ...
    /// #       End Site {
    /// #           OFFSET 0.0 0.0 30.0
    /// #       }
    ///     }
    ///     MOTION
    ///     // ...
    /// #   Frames: 0
    /// #   Frame Time: 0.0333333
    /// };
    ///
    /// let root = bvh.root_joint().unwrap();
    /// assert_eq!(root.data().offset(), &[1.2, 3.4, 5.6].into());
    /// ```
    #[inline]
    pub fn offset(&self) -> &Vector3<f32> {
        match *self {
            JointData::Child { ref offset, .. } | JointData::Root { ref offset, .. } => offset,
        }
    }

    /// Returns the `end_site` position if this `Joint` has an end site, or `None` if
    /// it doesn't.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bvh_anim::bvh;
    /// let bvh = bvh! {
    ///     HIERARCHY
    ///     ROOT Base
    ///     {
    ///         OFFSET 0.0 0.0 0.0
    ///         CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation
    ///         JOINT Tip
    ///         {
    ///             OFFSET 0.0 0.0 5.0
    ///             CHANNELS 3 Zrotation Xrotation Yrotation
    ///             End Site
    ///             {
    ///                 OFFSET 5.4 3.2 1.0
    ///             }
    ///         }
    ///     }
    ///     MOTION
    ///     // ...
    /// #   Frames: 0
    /// #   Frame Time: 0.0333333
    /// };
    /// let mut joints = bvh.joints();
    /// let base = joints.next().unwrap();
    /// assert!(base.data().end_site().is_none());
    ///
    /// let tip = joints.next().unwrap();
    /// assert_eq!(tip.data().end_site(), Some([5.4, 3.2, 1.0].into()).as_ref());
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bvh_anim::{bvh, ChannelType};
    /// let bvh = bvh! {
    ///     HIERARCHY
    ///     ROOT Hips
    ///     {
    ///         OFFSET 0.0 0.0 0.0
    ///         CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation
    ///         // ...
    /// #       End Site {
    /// #           OFFSET 0.0 0.0 30.0
    /// #       }
    ///     }
    ///     MOTION
    ///     // ...
    /// #   Frames: 0
    /// #   Frame Time: 0.0333333
    /// };
    ///
    /// let root = bvh.root_joint().unwrap();
    /// let expected_channels = &[
    ///     ChannelType::PositionX,
    ///     ChannelType::PositionY,
    ///     ChannelType::PositionZ,
    ///     ChannelType::RotationZ,
    ///     ChannelType::RotationX,
    ///     ChannelType::RotationY,
    /// ];
    ///
    /// for (channel, &expected) in root
    ///     .data()
    ///     .channels()
    ///     .iter()
    ///     .map(|c| c.channel_type())
    ///     .zip(expected_channels.iter())
    /// {
    ///     assert_eq!(channel, expected);
    /// }
    /// ```
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
    pub fn index(&self) -> usize {
        self.private_data().map(|d| d.self_index).unwrap_or(0)
    }

    /// Returns the index of the parent `JointData`, or `None` if this `JointData` is the
    /// root joint.
    #[inline]
    pub fn parent_index(&self) -> Option<usize> {
        self.private_data().map(|d| d.parent_index)
    }

    /// Returns a reference to the `JointPrivateData` of the `JointData` if it
    /// exists, or `None`.
    #[inline]
    pub(crate) fn private_data(&self) -> Option<&JointPrivateData> {
        match *self {
            JointData::Child { ref private, .. } => Some(private),
            _ => None,
        }
    }

    /// Returns a mutable reference to the `JointPrivateData` of the `JointData` if it
    /// exists, or `None`.
    #[inline]
    pub(crate) fn private_data_mut(&mut self) -> Option<&mut JointPrivateData> {
        match *self {
            JointData::Child {
                ref mut private, ..
            } => Some(private),
            _ => None,
        }
    }

    /// Get the depth of the `JointData` in the hierarchy.
    #[inline]
    pub(crate) fn depth(&self) -> usize {
        match *self {
            JointData::Child { ref private, .. } => private.depth,
            _ => 0,
        }
    }

    #[inline]
    pub(crate) fn empty_root() -> Self {
        JointData::Root {
            name: Default::default(),
            offset: [0.0, 0.0, 0.0].into(),
            channels: Default::default(),
        }
    }

    #[inline]
    pub(crate) fn empty_child() -> Self {
        JointData::Child {
            name: Default::default(),
            offset: [0.0, 0.0, 0.0].into(),
            channels: Default::default(),
            end_site_offset: Default::default(),
            private: JointPrivateData::empty(),
        }
    }

    #[inline]
    pub(crate) fn set_name<S: Into<JointName>>(&mut self, new_name: S) {
        match *self {
            JointData::Root { ref mut name, .. } => *name = new_name.into(),
            JointData::Child { ref mut name, .. } => *name = new_name.into(),
        }
    }

    pub(crate) fn set_offset(&mut self, new_offset: Vector3<f32>, is_site: bool) {
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

    #[inline]
    pub(crate) fn set_channels(&mut self, new_channels: SmallVec<[Channel; 6]>) {
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

/// A string type for the `Joint` name. A `SmallVec` is used for
/// better data locality.
pub type JointNameInner = SmallVec<[u8; mem::size_of::<String>()]>;

/// Wrapper struct for the `Joint` name type.
#[derive(Clone, Default, Eq, Hash, Ord)]
pub struct JointName(pub JointNameInner);

impl Deref for JointName {
    type Target = JointNameInner;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for JointName {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<B: AsRef<[u8]>> PartialEq<B> for JointName {
    #[inline]
    fn eq(&self, rhs: &B) -> bool {
        AsRef::<BStr>::as_ref(self) == rhs.as_ref()
    }
}

impl<B: AsRef<[u8]>> PartialOrd<B> for JointName {
    #[inline]
    fn partial_cmp(&self, rhs: &B) -> Option<Ordering> {
        AsRef::<BStr>::as_ref(self).partial_cmp(rhs.as_ref())
    }
}

macro_rules! impl_from {
    () => {};
    (;) => {};
    ($t:ty; $($rest:tt)*) => {
        impl From<$t> for JointName {
            #[inline]
            fn from(b: $t) -> Self {
                JointName(b.bytes().collect())
            }
        }
        impl_from!($($rest)*);
    }
}

impl From<CString> for JointName {
    #[inline]
    fn from(s: CString) -> Self {
        From::from(s.into_bytes())
    }
}

impl From<&'_ CStr> for JointName {
    #[inline]
    fn from(s: &'_ CStr) -> Self {
        From::from(s.to_bytes())
    }
}

impl From<Vec<u8>> for JointName {
    #[inline]
    fn from(s: Vec<u8>) -> Self {
        JointName(s.into_iter().collect())
    }
}

impl From<&'_ [u8]> for JointName {
    #[inline]
    fn from(s: &'_ [u8]) -> Self {
        From::from(s.to_vec())
    }
}

impl_from! {
    String; &'_ str;
    BString; &'_ BStr;
}

macro_rules! impl_as_ref {
    ($( $t:ty { ref => $method:path, mut => $mut_method:path } )*) => {
        $(
            impl AsRef<$t> for JointName {
                #[inline]
                fn as_ref(&self) -> &$t {
                    $method(&self.0[..])
                }
            }
            impl AsMut<$t> for JointName {
                #[inline]
                fn as_mut(&mut self) -> &mut $t {
                    $mut_method(&mut self.0[..])
                }
            }
        )*
    }
}

impl_as_ref! {
    BStr { ref => AsRef::<BStr>::as_ref, mut => AsMut::<BStr>::as_mut }
    [u8] { ref => AsRef::<[u8]>::as_ref, mut => AsMut::<[u8]>::as_mut }
}

impl From<JointNameInner> for JointName {
    #[inline]
    fn from(v: JointNameInner) -> Self {
        JointName(v)
    }
}

impl From<JointName> for JointNameInner {
    #[inline]
    fn from(j: JointName) -> Self {
        j.0
    }
}

impl fmt::Debug for JointName {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(AsRef::<BStr>::as_ref(self), fmtr)
    }
}

impl fmt::Display for JointName {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(AsRef::<BStr>::as_ref(self), fmtr)
    }
}

/// Data private to joints.
#[doc(hidden)]
#[derive(Clone, Eq, PartialEq)]
pub struct JointPrivateData {
    /// Index of this `Joint` in the array.
    pub(crate) self_index: usize,
    /// The parent index in the array of `JointPrivateData`s in the `Bvh`.
    pub(crate) parent_index: usize,
    /// Depth of the `Joint`. A depth of `1` signifies a `Joint` attached to
    /// the root.
    pub(crate) depth: usize,
}

impl JointPrivateData {
    #[inline]
    pub(crate) const fn new(self_index: usize, parent_index: usize, depth: usize) -> Self {
        JointPrivateData {
            self_index,
            parent_index,
            depth,
        }
    }

    #[inline]
    pub(crate) const fn empty() -> Self {
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
    pub(crate) joints: &'a [JointData],
    // pub(crate) motion_values: &'a [f32],
    pub(crate) current_joint: usize,
    pub(crate) from_child: Option<usize>,
}

impl fmt::Debug for Joints<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Joints { .. }")
    }
}

impl<'a> Joints<'a> {
    /// Create a `Joints` iterator over all the `joints` in a `Bvh` file.
    pub(crate) fn iter_root(joints: &'a [JointData] //clips: &'a AtomicRefCell<Clips>
    ) -> Self {
        Joints {
            joints,
            // clips,
            current_joint: 0,
            from_child: None,
        }
    }

    /// Create a `Joints` iterator over all the child joints of `joint`.
    ///
    /// # Notes
    ///
    /// This function only iterates over the direct children of `joint`. If you
    /// need to iterate through to the end sites of all children, you will
    /// need to continually call `iter_children` on each `Joint` in the iterator.
    pub(crate) fn iter_children(joint: &Joint<'a>) -> Self {
        let first_child = joint
            .joints
            .iter()
            .find(|jd| {
                if let Some(p) = jd.private_data() {
                    p.parent_index == joint.index
                } else {
                    false
                }
            })
            .map(JointData::index)
            .unwrap();

        Joints {
            joints: joint.joints,
            // clips: joint.clips,
            current_joint: joint.data().index(),
            from_child: Some(first_child),
        }
    }

    /// Finds the `Joint` named `joint_name`, or `None` if it doesn't exist.
    #[inline]
    pub fn find_by_name(&mut self, joint_name: &str) -> Option<Joint<'a>> {
        self.find(|b| b.data().name() == joint_name)
    }

    #[allow(unused)]
    pub(crate) fn nth_child(joint: &Joint<'a>, child: usize) -> Option<usize> {
        Joints::iter_children(joint)
            .nth(child)
            .map(|joint| joint.index)
    }
}

impl<'a> Iterator for Joints<'a> {
    type Item = Joint<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_joint >= self.joints.len() {
            return None;
        }

        let joint = Some(Joint {
            index: self.current_joint,
            joints: self.joints,
        });

        if self.from_child.is_none() {
            self.current_joint += 1;
        } else {
            unimplemented!()
        }

        joint
    }
}

/// A mutable iterator over the `Joint`s of a `Bvh` skeleton.
#[allow(unused)]
pub struct JointsMut<'a> {
    pub(crate) joints: &'a mut [JointData],
    pub(crate) current_joint: usize,
    pub(crate) from_child: Option<usize>,
}

impl<'a> JointsMut<'a> {
    pub(crate) fn iter_root(joints: &'a mut [JointData] //clips: &'a AtomicRefCell<Clips>
    ) -> Self {
        JointsMut {
            joints,
            // clips,
            current_joint: 0,
            from_child: None,
        }
    }
}

impl<'a> Iterator for JointsMut<'a> {
    type Item = JointMut<'a>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

impl fmt::Debug for JointsMut<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JointsMut { .. }")
    }
}

/*

impl<'a> Iterator for JointsMut<'a> {
    type Item = JointMut<'a>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.joints.next().map(JointMut::from_joint)
    }
}

*/

/// A view of a joint which provides access to various relevant data.
pub struct Joint<'a> {
    /// Index of the `Joint` in the skeleton.
    pub(crate) index: usize,
    /// `Joints` array which the joint is part of.
    pub(crate) joints: &'a [JointData],
}

impl Joint<'_> {
    /// Return the parent `Joint` if it exists, or `None` if it doesn't.
    #[inline]
    pub fn parent(&self) -> Option<Joint<'_>> {
        self.data().parent_index().map(|idx| Joint {
            index: idx,
            joints: self.joints,
        })
    }

    /// Returns an iterator over the children of `self`.
    #[inline]
    pub fn children(&self) -> Joints<'_> {
        Joints::iter_children(&self)
    }

    /// Access a read-only view of the internal data of the `Joint`.
    #[inline]
    pub fn data(&self) -> &JointData {
        &self.joints[self.index]
    }
}

impl fmt::Debug for Joint<'_> {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmtr.debug_struct("Joint")
            .field("index", &self.index)
            .field("data", &self.data())
            .finish()
    }
}

/// A view of a joint which provides mutable access.
pub struct JointMut<'a> {
    /// Index of the `Joint` in the skeleton.
    pub(crate) index: usize,
    /// `Joints` array which the joint is part of.
    pub(crate) joints: &'a mut [JointData],
}

impl<'a> JointMut<'a> {
    /*
    /// Return the parent `Joint` if it exists, or `None` if it doesn't.
    #[inline]
    pub fn parent(&self) -> Option<Joint<'_>> {
        self.data().parent_index().map(|idx| Joint {
            self_index: idx,
            skeleton: self.skeleton,
            clips: self.clips,
        })
    }
    
    pub fn 
    
    /// Returns an iterator over the children of `self`.
    #[inline]
    pub fn children(&self) -> Joints<'_> {
        Joints::iter_children(&self)
    }
    
    /// Access a read-only view of the internal data of the `Joint`.
    #[inline]
    pub fn data(&self) -> &JointData {
        &self.joints[self_index]
    }
    */
    /// Mutable access to the internal data of the `JointMut`.
    #[inline]
    pub fn data_mut(&mut self) -> &mut JointData {
        &mut self.joints[self.index]
    }
}
