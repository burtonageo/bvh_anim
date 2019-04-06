//! Errors which may occur when manipulating `Bvh` files.

use bstr::BString;
use crate::Axis;
use lexical::Error as LexicalError;
use std::{error::Error as StdError, fmt, io};

/// Errors which may arise when loading a `Bvh` file from
/// a `Reader`.
#[derive(Debug)]
pub enum LoadError {
    /// An error occurred when loading the joints hierarchy.
    Joints(LoadJointsError),
    /// An error occurred when loading the motion values.
    Motion(LoadMotionError),
}

impl fmt::Display for LoadError {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmtr, "{}: {}", self.description(), self.source().unwrap())
    }
}

impl StdError for LoadError {
    #[inline]
    fn description(&self) -> &str {
        match *self {
            LoadError::Joints(_) => "Could not load hierarchy",
            LoadError::Motion(_) => "Could not load motion",
        }
    }

    #[inline]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            LoadError::Joints(ref e) => Some(e),
            LoadError::Motion(ref e) => Some(e),
        }
    }
}

impl From<LoadJointsError> for LoadError {
    #[inline]
    fn from(e: LoadJointsError) -> Self {
        LoadError::Joints(e)
    }
}

impl From<LoadMotionError> for LoadError {
    #[inline]
    fn from(e: LoadMotionError) -> Self {
        LoadError::Motion(e)
    }
}

/// Represents an error which occurred when loading the `Joints` of the
/// bvh file.
#[derive(Debug)]
pub enum LoadJointsError {
    /// An I/O error occurred.
    Io(io::Error),
    /// The skeletal hierarchy is missing the `Root` joint.
    MissingRoot,
    /// A name could not be found for the `Joint`.
    MissingJointName {
        /// Line number in the source bvh where the error occurred.
        line: usize,
    },
    /// A `CHANNELS` section was encountered in the wrong location.
    UnexpectedChannelsSection {
        /// Line number in the source bvh where the error occurred.
        line: usize,
    },
    /// The number of channels could not be parsed in a `CHANNELS` section.
    ParseNumChannelsError {
        /// The parse error, if there was a malformed string to parse.
        error: Option<LexicalError>,
        /// Line number in the source bvh where the error occurred.
        line: usize,
    },
    /// A channel type could not be parsed in the `CHANNELS` section.
    ParseChannelError {
        /// The parse error.
        error: ParseChannelError,
        /// Line number in the source bvh where the error occurred.
        line: usize,
    },
    /// An `OFFSET` section was encountered in the wrong location.
    UnexpectedOffsetSection {
        /// Line number in the source bvh where the error occurred.
        line: usize,
    },
    /// An axis in the `OFFSET` section could not be parsed into a value.
    ParseOffsetError {
        /// The parse error.
        parse_float_error: LexicalError,
        /// The axis of the offset which could not be parsed.
        axis: Axis,
        /// Line number in the source bvh where the error occurred.
        line: usize,
    },
    /// An `OFFSET` section was missing an axis in the offset vector.
    MissingOffsetAxis {
        /// The smallest axis which was missing.
        axis: Axis,
        /// Line number in the source bvh where the error occurred.
        line: usize,
    },
}

impl LoadJointsError {
    /// Returns the line of the `Bvh` file where the error occurred.
    pub fn line(&self) -> Option<usize> {
        match *self {
            LoadJointsError::MissingJointName { line } => Some(line),
            LoadJointsError::UnexpectedChannelsSection { line } => Some(line),
            LoadJointsError::ParseNumChannelsError { line, .. } => Some(line),
            LoadJointsError::ParseChannelError { line, .. } => Some(line),
            LoadJointsError::UnexpectedOffsetSection { line } => Some(line),
            LoadJointsError::ParseOffsetError { line, .. } => Some(line),
            LoadJointsError::MissingOffsetAxis { line, .. } => Some(line),
            _ => None,
        }
    }
}

impl From<io::Error> for LoadJointsError {
    #[inline]
    fn from(e: io::Error) -> Self {
        LoadJointsError::Io(e)
    }
}

impl fmt::Display for LoadJointsError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LoadJointsError::Io(ref e) => fmt::Display::fmt(&e, f),
            LoadJointsError::MissingRoot => f.write_str("The root heirarchy could not be found"),
            LoadJointsError::MissingJointName { line } => {
                write!(f, "{}: the name is missing from the joints section", line)
            }
            LoadJointsError::UnexpectedChannelsSection { line } => write!(
                f,
                "{}: unexpectedly encountered a \"CHANNELS\" section",
                line
            ),
            LoadJointsError::ParseNumChannelsError { ref error, line } => match error {
                Some(ref e) => write!(f, "{}: could not parse the number of channels: {}", line, e),
                None => write!(f, "{}: could not find the number of channels", line),
            },
            LoadJointsError::ParseChannelError { ref error, line } => {
                write!(f, "{}: could not parse channel: {}", line, error)
            }
            LoadJointsError::UnexpectedOffsetSection { line } => write!(
                f,
                "{}: unexpectedly encountered an \"OFFSET\" section",
                line
            ),
            LoadJointsError::ParseOffsetError {
                ref parse_float_error,
                axis,
                line,
            } => write!(
                f,
                "{}: could not parse the {}-axis offset: {}",
                line, axis, parse_float_error
            ),
            LoadJointsError::MissingOffsetAxis { axis, line } => {
                write!(f, "{}: the {}-axis offset value is missing", line, axis)
            }
        }
    }
}

impl StdError for LoadJointsError {
    #[inline]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            LoadJointsError::Io(ref e) => Some(e),
            LoadJointsError::ParseNumChannelsError { ref error, .. } => {
                error.as_ref().map(|e| e as &(dyn StdError + 'static))
            }
            LoadJointsError::ParseChannelError { ref error, .. } => Some(error),
            LoadJointsError::ParseOffsetError {
                ref parse_float_error,
                ..
            } => Some(parse_float_error),
            _ => None,
        }
    }
}

/// Represents an error which occurred when loading the motion of the
/// bvh file.
#[derive(Debug)]
pub enum LoadMotionError {
    /// An I/O error occurred.
    Io(io::Error),
    /// The `MOTION` section is missing in the bvh.
    MissingMotionSection,
    /// The "Number of Frames" section could not be parsed in the bvh.
    MissingNumFrames {
        /// The parse error, or `None` if there was no number to be parsed.
        parse_error: Option<LexicalError>,
    },
    /// The "Frame Time" section could not be parsed in the bvh.
    MissingFrameTime {
        /// The parse error, or `None` if there was no number to be parsed.
        parse_error: Option<LexicalError>,
    },
    /// The motion values section could not be parsed in the bvh.
    ParseMotionSection {
        /// The parse error, or `None` if there was no number to be parsed.
        parse_error: LexicalError,
    },
    /// There was a discrepancy between the number of motion values promised
    /// by the file and the actual amount.
    MotionCountMismatch {
        /// Actual number of motion values parsed.
        actual_total_motion_values: usize,
        /// Expected number of motion values.
        expected_total_motion_values: usize,
        /// Expected number of frames.
        expected_num_frames: usize,
        /// Expected number of clips.
        expected_num_clips: usize,
    },
}

impl fmt::Display for LoadMotionError {
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LoadMotionError::Io(ref e) => {
                fmt::Display::fmt(e, fmtr)
            }
            LoadMotionError::MissingMotionSection => fmtr.write_str(self.description()),
            LoadMotionError::MissingNumFrames { ref parse_error } => {
                if let Some(ref e) = parse_error {
                    write!(fmtr, "could not parse the num frames value: {}", e)
                } else {
                    fmtr.write_str(self.description())
                }
            }
            LoadMotionError::MissingFrameTime { ref parse_error } => {
                if let Some(ref e) = parse_error {
                    write!(fmtr, "could not parse the frame time: {}", e)
                } else {
                    fmtr.write_str(self.description())
                }
            }
            LoadMotionError::ParseMotionSection { ref parse_error } => {
                write!(fmtr, "{}: {}", self.description(), parse_error)
            }
            LoadMotionError::MotionCountMismatch  {
                actual_total_motion_values,
                expected_total_motion_values,
                expected_num_frames,
                expected_num_clips,
            } => {
                write!(
                    fmtr,
                    "expected to find {} motion values, found {} values (num frames = {}, num clips = {})",
                    expected_total_motion_values,
                    actual_total_motion_values,
                    expected_num_frames,
                    expected_num_clips)
            }
        }
    }
}

impl StdError for LoadMotionError {
    #[inline]
    fn description(&self) -> &str {
        match *self {
            LoadMotionError::Io(ref e) => e.description(),
            LoadMotionError::MissingMotionSection => {
                "the 'MOTION' section of the bvh file is missing"
            }
            LoadMotionError::MissingNumFrames { .. } => {
                "the number of frames section is missing from the bvh file"
            }
            LoadMotionError::MissingFrameTime { .. } => {
                "the frame time is missing from the bvh file"
            }
            LoadMotionError::ParseMotionSection { .. } => "could not parse the motion value",
            LoadMotionError::MotionCountMismatch { .. } => "unexpected number of motion values",
        }
    }

    #[inline]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            LoadMotionError::Io(ref e) => Some(e),
            LoadMotionError::MissingFrameTime {
                parse_error: Some(ref e),
            } => Some(e),
            LoadMotionError::ParseMotionSection { ref parse_error } => Some(parse_error),
            LoadMotionError::MissingNumFrames {
                parse_error: Some(ref e),
            } => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for LoadMotionError {
    #[inline]
    fn from(e: io::Error) -> Self {
        LoadMotionError::Io(e)
    }
}

/// Represents an error which may occur when attempting to parse a
/// `BString` into a `ChannelType`.
#[derive(Debug)]
pub struct ParseChannelError(
    // @TODO(burtonageo): Borrow the erroneous string when hrts
    // land.
    BString,
);

impl ParseChannelError {
    /// Get the `BString` which caused the parse error.
    #[inline]
    pub fn into_inner(self) -> BString {
        self.0
    }
}

impl<S: Into<BString>> From<S> for ParseChannelError {
    #[inline]
    fn from(s: S) -> Self {
        ParseChannelError(s.into())
    }
}

impl fmt::Display for ParseChannelError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?}", self.description(), &self.0)
    }
}

impl StdError for ParseChannelError {
    #[inline]
    fn description(&self) -> &str {
        "The channel could not be parsed from the given string"
    }
}
