#[cfg(feature = "ffi")]
mod error {
    use cbindgen::Error as BindgenError;
    use std::{error, env::VarError, fmt, io};

    #[derive(Debug)]
    pub enum Error {
        Bindgen(BindgenError),
        Io(io::Error),
        Env(VarError),
    }

    impl error::Error for Error {
        fn source(&self) -> Option<&(dyn error::Error + 'static)> {
            match *self {
                Error::Bindgen(ref e) => Some(e),
                Error::Io(ref e) => Some(e),
                Error::Env(ref e) => Some(e),
            }
        }
    }

    impl fmt::Display for Error {
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

#[cfg(feature = "ffi")]
fn main() -> Result<(), error::Error> {
    use cbindgen;
    use std::{
        env,
        path::PathBuf,
    };

    let crate_dir = env::var("CARGO_MANIFEST_DIR")?;
    let bindings = cbindgen::generate(crate_dir)?;

    let mut header_path: PathBuf = env::var("OUT_DIR")?.into();
    header_path.push("bvh_anim.h");

    bindings.write_to_file(header_path);
    Ok(())
}

#[cfg(not(feature = "ffi"))]
fn main() {}
