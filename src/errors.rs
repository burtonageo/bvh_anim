use crate::Axis;
use std::{error::Error as StdError, fmt, io, num::{ParseFloatError, ParseIntError}};

#[derive(Debug)]
pub enum LoadError {
    Joints(LoadJointsError),
    Motion(LoadMotionError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LoadError::Joints(ref e) => {
                write!(fmtr, "Could not load hierarchy: {}", e)
            }
            LoadError::Motion(ref e) => {
                write!(fmtr, "Could not load motion: {}", e)
            }
        }
    }
}

impl StdError for LoadError {
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

#[derive(Debug)]
pub enum LoadJointsError {
    Io(io::Error),
    MissingRoot,
    MissingJointName {
        line: usize,
    },
    UnexpectedChannelsSection {
        line: usize,
    },
    ParseChannelError {
        error: ParseChannelError,
        line: usize,
    },
    UnexpectedOffsetSection {
        line: usize,
    },
    ParseOffsetError {
        parse_float_error: ParseFloatError,
        axis: Axis,
        line: usize,
    },
    MissingOffsetAxis {
        axis: Axis,
        line: usize,
    },
}

impl LoadJointsError {
    /// Returns the line of the `Bvh` file where the error occurred.
    pub fn line(&self) -> Option<usize> {
        match *self {
            LoadJointsError::MissingJointName { line } => Some(line),
            LoadJointsError::UnexpectedChannelsSection { line } => Some(line),
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
            LoadJointsError::MissingJointName { line } => f.write_str("Unknown error"),
            LoadJointsError::UnexpectedChannelsSection { line } => f.write_str("Unknown error"),
            LoadJointsError::ParseChannelError { ref error, line } => f.write_str("Unknown error"),
            LoadJointsError::UnexpectedOffsetSection { line } => f.write_str("Unknown error"),
            LoadJointsError::ParseOffsetError {
                ref parse_float_error,
                axis,
                line,
            } => f.write_str("Unknown error"),
            LoadJointsError::MissingOffsetAxis { axis, line } => f.write_str("Unknown error"),
        }
    }
}

impl StdError for LoadJointsError {
    #[inline]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            LoadJointsError::Io(ref e) => Some(e),
            LoadJointsError::ParseChannelError { ref error, .. } => Some(error),
            LoadJointsError::ParseOffsetError {
                ref parse_float_error,
                ..
            } => Some(parse_float_error),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum LoadMotionError {
    Io(io::Error),
    MissingMotionSection,
    MissingNumFrames {
        parse_error: Option<ParseIntError>,
    },
    MissingFrameTime {
        parse_error: Option<ParseFloatError>,
    },
    ParseMotionSection {
        parse_error: Option<ParseFloatError>,
    }
}

impl fmt::Display for LoadMotionError {
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        unimplemented!()
    }
}

impl StdError for LoadMotionError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            LoadMotionError::Io(ref e) => Some(e),
            LoadMotionError::MissingFrameTime { parse_error: Some(ref e) } => {
                Some(e)
            },
            LoadMotionError::ParseMotionSection { parse_error: Some(ref e) } => {
                Some(e)
            }
            LoadMotionError::MissingNumFrames { parse_error: Some(ref e) } => {
                Some(e)
            }
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

#[derive(Debug)]
pub struct ParseChannelError(
    // @TODO(burtonageo): Borrow the erroneous string when hrts
    // land.
    pub(crate) String,
);

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
