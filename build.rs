#[cfg(feature = "bindings")]
mod error {
    use cbindgen::Error as BindgenError;
    use std::{env::VarError, error, fmt, io};

    #[derive(Debug)]
    pub enum Error {
        Bindgen(BindgenError),
        Io(io::Error),
        Env(VarError),
    }

    impl error::Error for Error {
        #[inline]
        fn source(&self) -> Option<&(dyn error::Error + 'static)> {
            match *self {
                Error::Bindgen(ref e) => Some(e),
                Error::Io(ref e) => Some(e),
                Error::Env(ref e) => Some(e),
            }
        }

        #[inline]
        fn description(&self) -> &str {
            match *self {
                Error::Bindgen(ref e) => e.description(),
                Error::Io(ref e) => e.description(),
                Error::Env(ref e) => e.description(),
            }
        }
    }

    impl fmt::Display for Error {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match *self {
                Error::Bindgen(ref e) => fmt::Display::fmt(e, f),
                Error::Io(ref e) => fmt::Display::fmt(e, f),
                Error::Env(ref e) => fmt::Display::fmt(e, f),
            }
        }
    }

    impl From<io::Error> for Error {
        #[inline]
        fn from(e: io::Error) -> Self {
            Error::Io(e)
        }
    }

    impl From<BindgenError> for Error {
        #[inline]
        fn from(e: BindgenError) -> Self {
            Error::Bindgen(e)
        }
    }

    impl From<VarError> for Error {
        #[inline]
        fn from(e: VarError) -> Self {
            Error::Env(e)
        }
    }
}

#[cfg(feature = "bindings")]
fn main() -> Result<(), error::Error> {
    use cbindgen;
    use std::{
        env::{self, VarError},
        path::PathBuf,
    };

    let crate_dir = env::var("CARGO_MANIFEST_DIR")?;
    let bindings = cbindgen::generate(crate_dir)?;

    let mut header_path = target_dir()?;
    header_path.push("include/bvh_anim/bvh_anim.h");

    bindings.write_to_file(header_path);

    #[inline]
    fn target_dir() -> Result<PathBuf, VarError> {
        env::var("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .or_else(|_| {
                env::var("CARGO_MANIFEST_DIR")
                    .map(PathBuf::from)
                    .map(|p| p.join("target"))
            })
    }

    Ok(())
}

#[cfg(not(feature = "bindings"))]
fn main() {}
