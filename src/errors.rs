use crate::Axis;
use std::{error::Error as StdError, fmt, io, num::ParseFloatError};

#[derive(Debug)]
pub enum LoadError {
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
    MissingMotion,
}

impl LoadError {
    /// Returns the line of the `Bvh` file where the error occurred.
    pub fn line(&self) -> Option<usize> {
        match *self {
            LoadError::MissingJointName { line } => Some(line),
            LoadError::UnexpectedChannelsSection { line } => Some(line),
            LoadError::ParseChannelError { line, .. } => Some(line),
            LoadError::UnexpectedOffsetSection { line } => Some(line),
            LoadError::ParseOffsetError { line, .. } => Some(line),
            LoadError::MissingOffsetAxis { line, .. } => Some(line),
            _ => None,
        }
    }
}

impl From<io::Error> for LoadError {
    #[inline]
    fn from(e: io::Error) -> Self {
        LoadError::Io(e)
    }
}

impl fmt::Display for LoadError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LoadError::Io(ref e) => fmt::Display::fmt(&e, f),
            LoadError::MissingRoot => f.write_str("The root heirarchy could not be found"),
            LoadError::MissingJointName { line } => f.write_str("Unknown error"),
            LoadError::UnexpectedChannelsSection { line } => f.write_str("Unknown error"),
            LoadError::ParseChannelError { ref error, line } => f.write_str("Unknown error"),
            LoadError::UnexpectedOffsetSection { line } => f.write_str("Unknown error"),
            LoadError::ParseOffsetError {
                ref parse_float_error,
                axis,
                line,
            } => f.write_str("Unknown error"),
            LoadError::MissingOffsetAxis { axis, line } => f.write_str("Unknown error"),
            LoadError::MissingMotion => f.write_str("Unknown error"),
        }
    }
}

impl StdError for LoadError {
    #[inline]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            LoadError::Io(ref e) => Some(e),
            LoadError::ParseChannelError { ref error, .. } => Some(error),
            LoadError::ParseOffsetError {
                ref parse_float_error,
                ..
            } => Some(parse_float_error),
            _ => None,
        }
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
