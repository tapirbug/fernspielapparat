pub use err::Error;
use std::process::{Command, Output};

/// Tries to call the given command with --version and
/// captures its output and status.
///
/// Returns the output if version successfully detected.
///
/// Returns error in case of unsuccessful exit.
pub fn detect_version(cmd: &str) -> Result<Output, Error> {
    detect_version_with_arg(cmd, Some("--version"))
}

pub fn detect_version_with_arg(program: &str, arg: Option<&str>) -> Result<Output, Error> {
    let mut cmd = Command::new(program);

    if let Some(arg) = arg {
        cmd.arg(arg);
    }

    cmd.output()
        .or_else(|cause| Err(Error::version_detect_io(program, cause)))
        .and_then(|output| {
            if output.status.success() {
                Ok(output)
            } else {
                Err(Error::unsuccessful_exit(program, output))
            }
        })
}

mod err {
    use failure::{Backtrace, Fail};
    use std::io;
    use std::process::Output;

    #[derive(Fail, Debug)]
    pub enum Error {
        #[fail(display = "{} was found, but reported an error.", cmd)]
        UnsuccessfulExit {
            cmd: String,
            output: Output,
            backtrace: Backtrace,
        },
        #[fail(
            display = "{} seems to not be installed. Attempt to check version failed with I/O error: {}",
            cmd, cause
        )]
        VersionDetectIO {
            cmd: String,
            #[fail(cause)]
            cause: io::Error,
            backtrace: Backtrace,
        },
    }

    impl Error {
        pub fn unsuccessful_exit(cmd: &str, output: Output) -> Error {
            Error::UnsuccessfulExit {
                cmd: cmd.into(),
                output,
                backtrace: Backtrace::new(),
            }
        }

        pub fn version_detect_io(cmd: &str, cause: io::Error) -> Error {
            Error::VersionDetectIO {
                cmd: cmd.into(),
                cause,
                backtrace: Backtrace::new(),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn detect_cargo_version() {
        detect_version("cargo").expect("cargo could not be detected with detect_version()");
    }

    #[test]
    fn detect_nonsense_version() {
        let nonsense_version_err = detect_version("wrdlbrnft_42")
            .expect_err("expected wrdlbrnft_42 to be something nonsensable and err");

        match nonsense_version_err {
            Error::VersionDetectIO { cmd, .. } => assert_eq!(cmd, "wrdlbrnft_42"),
            other => panic!(
                "Expected VersionDetectIO error but got something else: {}",
                other
            ),
        }
    }

    #[cfg(unix)]
    #[test]
    fn detect_unsupported_cmd() {
        let false_version_err = detect_version("false")
            .expect_err("Expected false to exit unsuccessfully on any unix, making it unsupported by our method.");

        match false_version_err {
            Error::UnsuccessfulExit { .. } => (),
            other => panic!(
                "Expected UnsuccessfulExit error but got something else: {}",
                other
            ),
        }
    }
}
