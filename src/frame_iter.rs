use crate::{Channel, errors::SetMotionError};
use std::{
    borrow::{Borrow, BorrowMut},
    fmt,
    iter::{DoubleEndedIterator, FusedIterator, Iterator, ExactSizeIterator},
    slice::{ChunksExact, ChunksExactMut},
    mem,
    ops::{Deref, DerefMut, Index, IndexMut, Range},
};

/// An iterator over the frames of a `Bvh`.
///
/// This type is created using the [`Bvh::frames`] method.
///
/// [`Bvh::frames`]: ./struct.Bvh.html#method.frames
#[derive(Debug)]
pub struct Frames<'a> {
    pub(crate) chunks: Option<ChunksExact<'a, f32>>,
}

impl<'a> Iterator for Frames<'a> {
    type Item = Frame<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.as_mut().and_then(|mut c| c.next().map(Frame))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chunks.as_ref().map(|c| c.size_hint()).unwrap_or_default()
    }
}

impl<'a> DoubleEndedIterator for Frames<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.chunks.as_mut().and_then(|mut c| c.next_back().map(Frame))
    }
}

impl<'a> ExactSizeIterator for Frames<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.chunks.as_ref().map(|c| c.len()).unwrap_or(0)
    }
}

impl<'a> FusedIterator for Frames<'a> {}

/// A mutable iterator over the frames of a `Bvh`.
///
/// This type is created using the [`Bvh::frames_mut`] method.
///
/// [`Bvh::frames`]: ./struct.Bvh.html#method.frames_mut
#[derive(Debug)]
pub struct FramesMut<'a> {
    pub(crate) chunks: Option<ChunksExactMut<'a, f32>>,
}

impl<'a> Iterator for FramesMut<'a> {
    type Item = FrameMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.as_mut().and_then(|mut c| c.next().map(FrameMut))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chunks.as_ref().map(|c| c.size_hint()).unwrap_or_default()
    }
}

impl<'a> DoubleEndedIterator for FramesMut<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.chunks.as_mut().and_then(|mut c| c.next_back().map(FrameMut))
    }
}

impl<'a> ExactSizeIterator for FramesMut<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.chunks.as_ref().map(|c| c.len()).unwrap_or(0)
    }
}

impl<'a> FusedIterator for FramesMut<'a> {}

/// A wrapper for a slice of motion values, so that they can be indexed by [`Channel`].
///
/// [`Channel`]: ./struct.Channel.html
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame<'a>(&'a [f32]);

impl<'a> Frame<'a> {
    /// Return the number of values in the `Frame`.
    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are `0` values in the `FrameMut`. Otherwise,
    /// returns `false`.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Convert the `Frame` into a `&[f32]`.
    #[inline]
    pub const fn as_slice(&self) -> &[f32] {
        self.0
    }

    /// Attempts to get the motion value at `channel`. Otherwise, returns `None`. 
    #[inline]
    pub fn get<B: Borrow<Channel>>(&self, channel: B) -> Option<&f32> {
        self.0.get(channel.borrow().motion_index)
    }
}

impl<'a> AsRef<[f32]> for Frame<'a> {
    #[inline]
    fn as_ref(&self) -> &[f32] {
        self.0
    }
}

impl<'a> Borrow<[f32]> for Frame<'a> {
    #[inline]
    fn borrow(&self) -> &[f32] {
        &*self.0
    }
}

impl<'a, B: Borrow<Channel>> Index<B> for Frame<'a> {
    type Output = f32;
    #[inline]
    fn index(&self, channel: B) -> &Self::Output {
        self.0.index(channel.borrow().motion_index)
    }
}

/// A wrapper for a mutable slice of motion values, so that they can be indexed by [`Channel`].
///
/// [`Channel`]: ./struct.Channel.html
#[derive(Debug, PartialEq)]
pub struct FrameMut<'a>(&'a mut [f32]);

impl<'a> FrameMut<'a> {
    /// Return the number of values in the `FrameMut`.
    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are `0` values in the `FrameMut`. Otherwise,
    /// returns `false`.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Convert the `FrameMut` into a `&[f32]`.
    #[inline]
    pub const fn as_slice(&self) -> &[f32] {
        &*self.0
    }

    /// Convert the `FrameMut` into a `&mut [f32]`.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        self.0
    }

    /// Attempts to get the motion value at `channel`. Otherwise, returns `None`. 
    #[inline]
    pub fn get<C: Borrow<Channel>>(&self, channel: C) -> Option<&f32> {
        self.0.get(channel.borrow().motion_index)
    }

    /// Attempts to return a mutable reference to the motion value at `channel`.
    /// Otherwise, returns `None`.
    #[inline]
    pub fn get_mut<C: Borrow<Channel>>(&mut self, channel: C) -> Option<&mut f32> {
        self.0.get_mut(channel.borrow().motion_index)
    }

    pub fn try_set_motion<C>(&mut self, channel: C, new_motion: f32) -> Result<f32, SetMotionError>
    where
        C: Borrow<Channel>,
    {
        let channel = channel.borrow();
        let motion = self.get_mut(channel).ok_or(SetMotionError::BadChannel(*channel))?;
        Ok(mem::replace(motion, new_motion))
    }
}

impl<'a> Borrow<[f32]> for FrameMut<'a> {
    #[inline]
    fn borrow(&self) -> &[f32] {
        &*self.0
    }
}

impl<'a> BorrowMut<[f32]> for FrameMut<'a> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [f32] {
        self.0
    }
}

impl<'a> AsRef<[f32]> for FrameMut<'a> {
    #[inline]
    fn as_ref(&self) -> &[f32] {
        &*self.0
    }
}

impl<'a> AsMut<[f32]> for FrameMut<'a> {
    #[inline]
    fn as_mut(&mut self) -> &mut [f32] {
        self.0
    }
}

impl<'a, B: Borrow<Channel>> Index<B> for FrameMut<'a> {
    type Output = f32;
    #[inline]
    fn index(&self, channel: B) -> &Self::Output {
        self.0.index(channel.borrow().motion_index)
    }
}

impl<'a, B: Borrow<Channel>> IndexMut<B> for FrameMut<'a> {
    #[inline]
    fn index_mut(&mut self, channel: B) -> &mut Self::Output {
        self.0.index_mut(channel.borrow().motion_index)
    }
}
