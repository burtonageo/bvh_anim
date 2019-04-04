#![allow(unused)]

//! Contains options for `bvh` file formatting.

use crate::Bvh;
use std::{
    fmt,
    io::{self, Write},
    iter,
    num::NonZeroUsize,
};

/// Specify formatting options for writing a `Bvh`.
#[derive(Clone, Default, Debug, Eq, Hash, PartialEq)]
pub struct WriteOptions {
    /// Which indentation style to use for nested bones.
    pub indent: IndentStyle,
    /// Which style new line terminator to use when writing the `bvh`.
    pub line_terminator: LineTerminator,
    #[doc(hidden)]
    _nonexhaustive: (),
}

impl WriteOptions {
    /// Create a new `WriteOptions` with default values.
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// Output the `Bvh` file to the `writer` with the given options.
    pub fn write<W: Write>(&self, bvh: &Bvh, writer: &mut W) -> io::Result<()> {
        let mut curr_line = String::new();
        let mut curr_bytes_written = 0usize;
        let mut curr_string_len = 0usize;
        let mut iter_state = WriteOptionsIterState::new(bvh);

        while self.next_line(bvh, &mut curr_line, &mut iter_state) != false {
            let bytes: &[u8] = curr_line.as_ref();
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
    pub fn write_to_string(&self, bvh: &Bvh) -> String {
        let mut curr_line = String::new();
        let mut out_string = String::new();
        let mut iter_state = WriteOptionsIterState::new(bvh);

        while self.next_line(bvh, &mut curr_line, &mut iter_state) != false {
            out_string.push_str(&curr_line);
        }

        out_string
    }

    #[inline]
    pub fn with_indent(self, indent: IndentStyle) -> Self {
        WriteOptions { indent, ..self }
    }

    #[inline]
    pub fn with_line_terminator(self, line_terminator: LineTerminator) -> Self {
        WriteOptions {
            line_terminator,
            ..self
        }
    }

    /// Get the next line of the written bvh file. This function is
    /// structured so that the `line` string can be continually
    /// re-used without allocating and de-allocating memory.
    ///
    /// # Returns
    ///
    /// Returns `true` when there are still more lines available,
    /// `false` when all lines have been extracted.
    fn next_line(
        &self,
        bvh: &Bvh,
        line: &mut String,
        iter_state: &mut WriteOptionsIterState,
    ) -> bool {
        line.clear();
        false
    }
}

enum WriteOptionsIterState<'a> {
    WriteBones { bvh: &'a Bvh, curr_bone: usize },
    WriteMotion { bvh: &'a Bvh, curr_frame: usize },
}

impl<'a> WriteOptionsIterState<'a> {
    #[inline]
    fn new(bvh: &'a Bvh) -> Self {
        WriteOptionsIterState::WriteBones { bvh, curr_bone: 0 }
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
    fn prefix_chars(&self) -> impl Iterator<Item = u8> {
        match *self {
            IndentStyle::NoIndentation => iter::repeat(b'\0').take(0),
            IndentStyle::Tabs => iter::repeat(b'\t').take(1),
            IndentStyle::Spaces(n) => iter::repeat(b' ').take(n.get()),
        }
    }
}

/// Create a new `IndentStyle` using a single tab.
impl Default for IndentStyle {
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
    /// * On Unix, use Unix-style line endings (`'\n'`).
    /// * On Windows, use Windows-style line endings (`'\r\n'`).
    Native,
}

#[cfg(target_os = "windows")]
impl LineTerminator {
    /// Return the characters of the `LineTerminator` as a `str`.
    #[inline]
    pub fn as_str(&self) -> &str {
        match *self {
            LineTerminator::Unix => "\n",
            LineTerminator::Windows | LineTerminator::Native => "\r\n",
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl LineTerminator {
    /// Return the characters of the `LineTerminator` as a `str`.
    #[inline]
    pub fn as_str(&self) -> &str {
        match *self {
            LineTerminator::Native | LineTerminator::Unix => "\n",
            LineTerminator::Windows => "\r\n",
        }
    }
}

impl Default for LineTerminator {
    #[inline]
    fn default() -> Self {
        LineTerminator::Native
    }
}

impl fmt::Display for LineTerminator {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
