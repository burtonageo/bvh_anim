use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use bstr::BStr;
use crate::{Bvh, Channel, Clips};
use mint::Vector3;
use smallvec::SmallVec;
use std::{
    cmp::{Ordering, PartialEq, PartialOrd},
    fmt,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    str,
};

/// Internal representation of a joint.
#[derive(Clone, Debug)]
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

impl JointData {
    /// Returns the name of the `JointData`.
    #[inline]
    pub fn name(&self) -> &BStr {
        match *self {
            JointData::Root { ref name, .. } | JointData::Child { ref name, .. } => name.as_ref(),
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
    pub(crate) fn index(&self) -> usize {
        self.private_data().map(|d| d.self_index).unwrap_or(0)
    }

    /// Returns the index of the parent `JointData`, or `None` if this `JointData` is the
    /// root joint.
    #[inline]
    pub(crate) fn parent_index(&self) -> Option<usize> {
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

    /// Get the depth of the `JointData` in the heirarchy.
    #[inline]
    pub(crate) fn depth(&self) -> usize {
        match *self {
            JointData::Child { ref private, .. } => private.depth,
            _ => 0,
        }
    }

    pub(crate) fn empty_root() -> Self {
        JointData::Root {
            name: Default::default(),
            offset: Vector3::from_slice(&[0.0, 0.0, 0.0]),
            channels: Default::default(),
        }
    }

    pub(crate) fn empty_child() -> Self {
        JointData::Child {
            name: Default::default(),
            offset: Vector3::from_slice(&[0.0, 0.0, 0.0]),
            channels: Default::default(),
            end_site_offset: Default::default(),
            private: JointPrivateData::empty(),
        }
    }

    pub(crate) fn set_name(&mut self, new_name: JointName) {
        match *self {
            JointData::Root { ref mut name, .. } => *name = new_name,
            JointData::Child { ref mut name, .. } => *name = new_name,
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

macro_rules! impl_as_ref {
    ($t:ty { ref => $method:path, mut => $mut_method:path }) => {
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
    };
}

impl_as_ref! {
    BStr { ref => BStr::new, mut => BStr::new_mut }
}

impl_as_ref! {
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
#[derive(Clone)]
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
    pub(crate) fn new(self_index: usize, parent_index: usize, depth: usize) -> Self {
        JointPrivateData {
            self_index,
            parent_index,
            depth,
        }
    }

    #[inline]
    pub(crate) fn empty() -> Self {
        JointPrivateData::new(0, 0, 0)
    }

    #[inline]
    pub(crate) fn new_default() -> Self {
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
    pub(crate) joints: &'a AtomicRefCell<Vec<JointData>>,
    pub(crate) clips: &'a AtomicRefCell<Clips>,
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
    pub(crate) fn iter_root(
        joints: &'a AtomicRefCell<Vec<JointData>>,
        clips: &'a AtomicRefCell<Clips>,
    ) -> Self {
        Joints {
            joints,
            clips,
            current_joint: 0,
            from_child: None,
        }
    }

    pub(crate) fn iter_children(joint: &Joint<'a>) -> Self {
        let first_child = joint
            .skeleton
            .borrow()
            .iter()
            .find(|jd| {
                if let Some(p) = jd.private_data() {
                    p.parent_index == joint.self_index
                } else {
                    false
                }
            })
            .map(JointData::index)
            .unwrap();

        Joints {
            joints: joint.skeleton,
            clips: joint.clips,
            current_joint: joint.data().index(),
            from_child: Some(first_child),
        }
    }

    /// Finds the `Joint` named `joint_name`, or `None` if it doesn't exist.
    #[inline]
    pub fn find_by_name(&mut self, joint_name: &str) -> Option<Joint<'a>> {
        self.find(|b| b.data().name() == joint_name)
    }

    pub(crate) fn nth_child(
        joint: &Joint<'a>,
        joints: &[JointData],
        child: usize,
    ) -> Option<usize> {
        unimplemented!()
    }

    pub(crate) fn get_next_joint(&self, j: Option<&Joint<'a>>) -> usize {
        //if self.
        unimplemented!()
    }
}

impl<'a> Iterator for Joints<'a> {
    type Item = Joint<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_joint >= self.joints.borrow().len() {
            return None;
        }

        let joint = Some(Joint {
            self_index: self.current_joint,
            skeleton: self.joints,
            clips: self.clips,
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
pub struct JointsMut<'a> {
    pub(crate) joints: Joints<'a>,
    pub(crate) _boo: PhantomData<&'a mut Bvh>,
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
    pub(crate) self_index: usize,
    /// Skeleton which the joint is part of.
    pub(crate) skeleton: &'a AtomicRefCell<Vec<JointData>>,
    /// Motion clip data relevant to the skeleton.
    pub(crate) clips: &'a AtomicRefCell<Clips>,
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
        Joints::iter_children(&self)
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
    pub(crate) fn from_joint(joint: Joint<'a>) -> Self {
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
