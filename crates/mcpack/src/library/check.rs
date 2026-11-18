use super::{Kernel, KernelError, Limits};

#[derive(Debug, PartialEq, Eq)]
pub enum CheckError {
    /// The library does not load.
    Load(KernelError),
    /// Labels of the assertions the kernel rejected, with the reason.
    Assertions(Vec<(String, KernelError)>),
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckError::Load(e) => write!(f, "the packed library does not load: {e:?}"),
            CheckError::Assertions(failed) => {
                for (label, e) in failed {
                    writeln!(f, "{label}: {e:?}")?;
                }
                write!(f, "{} assertions failed the kernel check", failed.len())
            }
        }
    }
}

/// Runs the kernel over every assertion of a library and returns how many there are.
pub fn check(bytes: &[u8]) -> Result<usize, CheckError> {
    let mut kernel = Kernel::new(bytes, Limits::UNBOUNDED).map_err(CheckError::Load)?;
    let mut failed = Vec::new();
    for id in 0..kernel.assertions() {
        if let Err(e) = kernel.verify(id) {
            let label = String::from_utf8_lossy(kernel.label(id).unwrap_or_default());
            failed.push((label.into_owned(), e));
        }
    }
    if failed.is_empty() {
        Ok(kernel.assertions())
    } else {
        Err(CheckError::Assertions(failed))
    }
}
