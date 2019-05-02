#![allow(unused)]

//! Contains options for `bvh` file formatting.

use bstr::{BStr, BString, B};
use crate::{duation_to_fractional_seconds, Bvh, Frame, Frames, Joint, Joints};
use mint::Vector3;
use smallvec::SmallVec;
use std::{
    fmt,
    io::{self, Write},
    iter, mem,
    num::NonZeroUsize,
};

/// Specify formatting options for writing a `Bvh`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WriteOptions {
    /// Which indentation style to use for nested bones.
    pub indent: IndentStyle,
    /// Which style new line terminator to use when writing the `bvh`.
    pub line_terminator: LineTerminator,
    /// Number of significant figures to use when writing `OFFSET` values.
    pub offset_significant_figures: usize,
    /// Number of significant figures to use when writing the `Frame Time` value.
    pub frame_time_significant_figures: usize,
    /// Number of significant figures to use when writing `MOTION` values.
    pub motion_values_significant_figures: usize,
    #[doc(hidden)]
    _nonexhaustive: (),
}

impl Default for WriteOptions {
    #[inline]
    fn default() -> Self {
        WriteOptions {
            indent: Default::default(),
            line_terminator: Default::default(),
            offset_significant_figures: 5,
            frame_time_significant_figures: 7,
            motion_values_significant_figures: 2,
            _nonexhaustive: (),
        }
    }
}

impl WriteOptions {
    /// Create a new `WriteOptions` with default values.
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// Output the `Bvh` file to the `writer` with the given options.
    pub fn write<W: Write>(&self, bvh: &Bvh, writer: &mut W) -> io::Result<()> {
        let mut curr_chunk = BString::new();
        let mut curr_bytes_written = 0usize;
        let mut curr_string_len = 0usize;
        let mut iter_state = WriteOptionsIterState::new();

        while self.next_chunk(bvh, &mut curr_chunk, &mut iter_state) != false {
            let bytes: &[u8] = curr_chunk.as_ref();
            curr_string_len += bytes.len();
            curr_bytes_written += writer.write(bytes)?;

            if curr_bytes_written != curr_string_len {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Data has been dropped while writing to file",
                ));
            }
        }
        writer.flush()
    }

    /// Output the `Bvh` file to the `string` with the given options.
    pub fn write_to_string(&self, bvh: &Bvh) -> BString {
        let mut curr_chunk = BString::new();
        let mut out_string = BString::new();
        let mut iter_state = WriteOptionsIterState::new();

        while self.next_chunk(bvh, &mut curr_chunk, &mut iter_state) != false {
            out_string.push(&curr_chunk);
        }

        out_string
    }

    /// Sets `indent` on `self` to the new `IndentStyle`.
    #[inline]
    pub const fn with_indent(self, indent: IndentStyle) -> Self {
        WriteOptions { indent, ..self }
    }

    /// Sets `line_terminator` on `self` to the new `LineTerminator`.
    #[inline]
    pub fn with_line_terminator(self, line_terminator: LineTerminator) -> Self {
        WriteOptions {
            line_terminator,
            ..self
        }
    }

    /// Sets `offset_significant_figures` on `self` to the new `offset_significant_figures`.
    #[inline]
    pub const fn with_offset_significant_figures(self, offset_significant_figures: usize) -> Self {
        WriteOptions {
            offset_significant_figures,
            ..self
        }
    }

    /// Sets `motion_values_significant_figures` on `self` to the new `motion_values_significant_figures`.
    #[inline]
    pub const fn with_frame_time_significant_figures(
        self,
        frame_time_significant_figures: usize,
    ) -> Self {
        WriteOptions {
            frame_time_significant_figures,
            ..self
        }
    }

    /// Sets `motion_values_significant_figures` on `self` to the new `motion_values_significant_figures`.
    #[inline]
    pub const fn with_motion_values_significant_figures(
        self,
        motion_values_significant_figures: usize,
    ) -> Self {
        WriteOptions {
            motion_values_significant_figures,
            ..self
        }
    }

    // @TODO: Refactor all of this
    /// Get the next text chunk of the written bvh file. This function is
    /// structured so that the `chunk` string can be continually
    /// re-used without allocating and de-allocating memory.
    ///
    /// # Returns
    ///
    /// Returns `true` when there are still more lines available,
    /// `false` when all lines have been extracted.
    fn next_chunk<'a, 'b: 'a>(
        &self,
        bvh: &'b Bvh,
        chunk: &mut BString,
        iter_state: &'a mut WriteOptionsIterState<'b>,
    ) -> bool {
        chunk.clear();

        match *iter_state {
            WriteOptionsIterState::WriteHierarchy { ref mut written } => {
                if !*written {
                    *chunk = BString::from("HIERARCHY");
                    chunk.push(self.line_terminator.as_str());
                    *written = true;
                } else {
                    let mut joints = bvh.joints();
                    *iter_state = WriteOptionsIterState::WriteJoints {
                        current_joint: joints.next(),
                        joints,
                        wrote_name: false,
                        wrote_offset: false,
                        wrote_channels: false,
                    };
                }
            }
            WriteOptionsIterState::WriteJoints {
                current_joint: None,
                ..
            } => {
                *iter_state = WriteOptionsIterState::WriteMotion { written: false };
            }
            WriteOptionsIterState::WriteJoints {
                ref mut joints,
                ref mut current_joint,
                ref mut wrote_name,
                ref mut wrote_offset,
                ref mut wrote_channels,
            } => {
                if let Some(ref joint) = current_joint {
                    let joint_data = joint.data();
                    let mut depth = joint_data.depth();
                    if *wrote_name {
                        depth += 1
                    }

                    match (&mut *wrote_name, &mut *wrote_offset, &mut *wrote_channels) {
                        (&mut false, _, _) => {
                            // @TODO: Contribute `Extend` impl for `BString` to avoid the `Vec`
                            // allocation
                            chunk.push(self.indent.prefix_chars(depth).collect::<Vec<_>>());
                            if joint_data.is_root() {
                                chunk.push(B("ROOT "));
                            } else {
                                chunk.push(B("JOINT "));
                            }
                            chunk.push(joint_data.name());
                            chunk.push(self.line_terminator.as_str());
                            chunk.push(self.indent.prefix_chars(depth).collect::<Vec<_>>());
                            chunk.push("{");
                            chunk.push(self.line_terminator.as_str());

                            *wrote_name = true;
                        }
                        (&mut true, &mut false, _) => {
                            // @TODO: Contribute `Extend` impl for `BString` to avoid the `Vec`
                            // allocation
                            chunk.push(self.indent.prefix_chars(depth).collect::<Vec<_>>());

                            let Vector3 { x, y, z } = joint_data.offset();
                            chunk.push(format!(
                                "OFFSET {:.*} {:.*} {:.*}",
                                self.offset_significant_figures,
                                x,
                                self.offset_significant_figures,
                                y,
                                self.offset_significant_figures,
                                z,
                            ));
                            chunk.push(self.line_terminator.as_str());
                            *wrote_offset = true;
                        }
                        (&mut true, &mut true, &mut false) => {
                            // @TODO: Contribute `Extend` impl for `BString` to avoid the `Vec`
                            // allocation
                            chunk.push(self.indent.prefix_chars(depth).collect::<Vec<_>>());

                            let channels = joint_data.channels();
                            let channels_str = channels
                                .iter()
                                .map(|ch| ch.channel_type().as_str())
                                .collect::<SmallVec<[_; 6]>>()
                                .join(" ");

                            chunk.push(format!("CHANNELS {} {}", channels.len(), channels_str));
                            chunk.push(self.line_terminator.as_str());
                            *wrote_channels = true;
                        }
                        (&mut true, &mut true, &mut true) => {
                            if let Some(end_site) = joint_data.end_site() {
                                let Vector3 { x, y, z } = end_site;
                                chunk.push(self.indent.prefix_chars(depth).collect::<Vec<_>>());
                                chunk.push("End Site");
                                chunk.push(self.line_terminator.as_str());

                                chunk.push(self.indent.prefix_chars(depth).collect::<Vec<_>>());
                                chunk.push("{");
                                chunk.push(self.line_terminator.as_str());

                                chunk.push(self.indent.prefix_chars(depth + 1).collect::<Vec<_>>());
                                chunk.push(format!(
                                    "OFFSET {:.*} {:.*} {:.*}",
                                    self.offset_significant_figures,
                                    x,
                                    self.offset_significant_figures,
                                    y,
                                    self.offset_significant_figures,
                                    z,
                                ));
                                chunk.push(self.line_terminator.as_str());

                                chunk.push(self.indent.prefix_chars(depth).collect::<Vec<_>>());
                                chunk.push("}");
                                chunk.push(self.line_terminator.as_str());

                                let next_joint = joints.next();
                                let prev_joint = mem::replace(current_joint, next_joint).unwrap();

                                let (curr_depth, mut depth_difference) =
                                    if let Some(ref curr_j) = *current_joint {
                                        let curr_depth = curr_j.data().depth();
                                        (curr_depth, Some(prev_joint.data().depth() - curr_depth))
                                    } else {
                                        (0, Some(prev_joint.data().depth()))
                                    };

                                while let Some(d) = depth_difference {
                                    chunk.push(
                                        self.indent
                                            .prefix_chars(curr_depth + d)
                                            .collect::<Vec<_>>(),
                                    );
                                    chunk.push("}");
                                    chunk.push(self.line_terminator.as_str());
                                    depth_difference =
                                        depth_difference.and_then(|d| d.checked_sub(1));
                                }
                            } else {
                                *current_joint = joints.next();
                            }
                            *wrote_name = false;
                            *wrote_offset = false;
                            *wrote_channels = false;
                        }
                        _ => {}
                    }
                }
            }
            WriteOptionsIterState::WriteMotion { ref mut written } => {
                if !*written {
                    *chunk = BString::from("MOTION");
                    chunk.push(self.line_terminator.as_bstr());
                    *written = true;
                } else {
                    *iter_state = WriteOptionsIterState::WriteNumFrames { written: false };
                }
            }
            WriteOptionsIterState::WriteNumFrames { ref mut written } => {
                if !*written {
                    *chunk = BString::from(format!("Frames: {}", bvh.num_frames()));
                    chunk.push(self.line_terminator.as_bstr());
                    *written = true;
                } else {
                    *iter_state = WriteOptionsIterState::WriteFrameTime { written: false };
                }
            }
            WriteOptionsIterState::WriteFrameTime { ref mut written } => {
                if !*written {
                    *chunk = BString::from(format!(
                        "Frame Time: {:.*}",
                        self.frame_time_significant_figures,
                        duation_to_fractional_seconds(bvh.frame_time())
                    ));
                    chunk.push(self.line_terminator.as_bstr());
                    *written = true;
                } else {
                    let mut frames = bvh.frames();
                    *iter_state = WriteOptionsIterState::WriteFrames {
                        current_frame: frames.next(),
                        frames,
                    };
                }
            }
            WriteOptionsIterState::WriteFrames {
                ref mut current_frame,
                ref mut frames,
            } => match current_frame {
                None => return false,
                Some(frame) => {
                    let motion_values = frame
                        .as_slice()
                        .iter()
                        .map(|motion| {
                            format!("{:.*}", self.motion_values_significant_figures, motion)
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    *chunk = BString::from(motion_values.as_str());
                    chunk.push(self.line_terminator.as_bstr());
                    *current_frame = frames.next();
                }
            },
        }

        true
    }
}

enum WriteOptionsIterState<'a> {
    WriteHierarchy {
        written: bool,
    },
    WriteJoints {
        joints: Joints<'a>,
        current_joint: Option<Joint<'a>>,
        wrote_name: bool,
        wrote_offset: bool,
        wrote_channels: bool,
    },
    WriteMotion {
        written: bool,
    },
    WriteNumFrames {
        written: bool,
    },
    WriteFrameTime {
        written: bool,
    },
    WriteFrames {
        frames: Frames<'a>,
        current_frame: Option<&'a Frame>,
    },
}

impl WriteOptionsIterState<'_> {
    #[inline]
    fn new() -> Self {
        WriteOptionsIterState::WriteHierarchy { written: false }
    }
}

impl Default for WriteOptionsIterState<'_> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Specify indentation style to use when writing the `Bvh` joints.
///
/// By default, this value is set to 1 tab.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum IndentStyle {
    /// Do not indent nested joints.
    NoIndentation,
    /// Use a single tab (`'\t'`) for indentation.
    Tabs,
    /// Use `n` spaces for indentation.
    Spaces(NonZeroUsize),
}

impl IndentStyle {
    /// Create a new `IndentStyle` with `n` preceeding spaces.
    ///
    /// If `n` is `0`, then `IndentStyle::NoIndentation` is returned.
    #[inline]
    pub fn with_spaces(n: usize) -> Self {
        NonZeroUsize::new(n)
            .map(IndentStyle::Spaces)
            .unwrap_or(IndentStyle::NoIndentation)
    }

    /// Return an `Iterator` which yields bytes corresponding to the ascii
    /// chars which form the `String` this indentation style would take.
    #[inline]
    fn prefix_chars(&self, depth: usize) -> impl Iterator<Item = u8> {
        match *self {
            IndentStyle::NoIndentation => iter::repeat(b'\0').take(0),
            IndentStyle::Tabs => iter::repeat(b'\t').take(depth),
            IndentStyle::Spaces(n) => iter::repeat(b' ').take(n.get() * depth),
        }
    }
}

/// Create a new `IndentStyle` using a single tab.
impl Default for IndentStyle {
    #[inline]
    fn default() -> Self {
        IndentStyle::Tabs
    }
}

/// Represents which line terminator style to use when writing a `Bvh` file.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LineTerminator {
    /// Use Unix-style line endings (`'\n'`).
    Unix,
    /// Use Windows-style line endings (`'\r\n'`).
    Windows,
}

impl LineTerminator {
    /// Get the line terminator style native to the current OS:
    ///
    /// * On Windows, this returns `LineTerminator::Windows`.
    /// * Otherwise, this returns `LineTerminator::Unix`.
    #[cfg(target_os = "windows")]
    #[inline]
    pub fn native() -> Self {
        LineTerminator::Windows
    }

    /// Get the line terminator style native to the current OS:
    ///
    /// * On Windows, this returns `LineTerminator::Windows`.
    /// * Otherwise, this returns `LineTerminator::Unix`.
    #[cfg(not(target_os = "windows"))]
    #[inline]
    pub fn native() -> Self {
        LineTerminator::Unix
    }

    /// Return the characters of the `LineTerminator` as a `&str`.
    #[inline]
    pub fn as_str(&self) -> &str {
        match *self {
            LineTerminator::Unix => "\n",
            LineTerminator::Windows => "\r\n",
        }
    }

    /// Returns the escaped characters of the `LineTerminator` as a `&str`.
    #[inline]
    pub fn as_escaped_str(&self) -> &str {
        match *self {
            LineTerminator::Unix => r"\n",
            LineTerminator::Windows => r"\r\n",
        }
    }

    /// Return the characters of the `LineTerminator` as a `&BStr`.
    #[inline]
    pub fn as_bstr(&self) -> &BStr {
        B(self.as_str())
    }

    /// Returns the escaped characters of the `LineTerminator` as a `&BStr`.
    #[inline]
    pub fn as_escaped_bstr(&self) -> &BStr {
        B(self.as_escaped_str())
    }
}

/// Returns the native line terminator for the current OS.
impl Default for LineTerminator {
    #[inline]
    fn default() -> Self {
        LineTerminator::native()
    }
}

impl fmt::Display for LineTerminator {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
