#[cfg(feature = "bindings")]
mod error {
    use cbindgen::Error as BindgenError;
    #[cfg(feature = "ctests")]
    use cc::Error as CcError;
    use std::{env::VarError, error, fmt, io};

    #[allow(unused)]
    #[derive(Debug)]
    pub enum Error {
        Bindgen(BindgenError),
        #[cfg(feature = "ctests")]
        Cc(CcError),
        Io(io::Error),
        Env(VarError),
        Unspecified,
    }

    impl error::Error for Error {
        #[inline]
        fn source(&self) -> Option<&(dyn error::Error + 'static)> {
            match *self {
                Error::Bindgen(ref e) => Some(e),
                // Error::Cc(ref e) => Some(e),
                Error::Io(ref e) => Some(e),
                Error::Env(ref e) => Some(e),
                _ => None,
            }
        }

        #[inline]
        fn description(&self) -> &str {
            match *self {
                Error::Bindgen(ref e) => e.description(),
                // Error::Cc(ref e) => e.description(),
                Error::Io(ref e) => e.description(),
                Error::Env(ref e) => e.description(),
                #[cfg(feature = "ctests")]
                Error::Cc(_) => "A c compiler error occurred",
                Error::Unspecified => "An unspecified error occurred",
            }
        }
    }

    impl fmt::Display for Error {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match *self {
                Error::Bindgen(ref e) => fmt::Display::fmt(e, f),
                #[cfg(feature = "ctests")]
                Error::Cc(_) => f.write_str(error::Error::description(self)),
                Error::Unspecified => f.write_str(error::Error::description(self)),
                Error::Io(ref e) => fmt::Display::fmt(e, f),
                Error::Env(ref e) => fmt::Display::fmt(e, f),
            }
        }
    }

    #[cfg(feature = "ctests")]
    impl From<CcError> for Error {
        #[inline]
        fn from(e: CcError) -> Self {
            Error::Cc(e)
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
    use cbindgen::{self, Config, Language};
    use std::{
        env::{self, VarError},
        path::PathBuf,
    };

    let crate_dir = env::var("CARGO_MANIFEST_DIR")?;
    let mut bindings = cbindgen::generate(crate_dir)?;

    let mut header_path = target_dir()?;
    header_path.push("include/bvh_anim/bvh_anim.h");

    bindings.write_to_file(header_path);

    build_and_run_ctests()?;

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

    #[cfg(feature = "ctests")]
    fn build_and_run_ctests() -> Result<(), error::Error> {
        use cc::Build;
        use std::{fs, io};

        let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);

        let ctests_dir = crate_dir.join("ctests");
        if !ctests_dir.exists() {
            println!("Skipping ctests");
            return Err(error::Error::Unspecified);
        }

        let entries = fs::read_dir(ctests_dir)?;
        let mut include_dir = target_dir()?;
        include_dir.push("include");

        let mut out_dir = PathBuf::from(env::var("OUT_DIR")?);
        out_dir.push("ctests");
        panic!("{:?}", out_dir);

        for entry in entries {
            let entry = entry?;
            let entry_path = entry.path();

            match entry_path.extension() {
                Some(ext) if ext == "c" || ext == "cpp" => (),
                _ => continue,
            }

            let file_name = entry_path.file_name().unwrap();

            Build::new()
                .file(&entry_path)
                .include(&include_dir)
                .out_dir(&out_dir)
                .try_compile(file_name.to_str().unwrap())?;
        }

        Ok(())
    }

    #[cfg(not(feature = "ctests"))]
    fn build_and_run_ctests() -> Result<(), error::Error> {
        Ok(())
    }

    Ok(())
}

#[cfg(not(feature = "bindings"))]
fn main() {}
