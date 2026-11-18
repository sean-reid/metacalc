use super::{
    Kernel, KernelError, Limits, ReadError, SectionCheckError, SectionError, decode, read_sections,
    verify_sections,
};

#[derive(Debug, PartialEq, Eq)]
pub enum CheckError {
    /// The library does not load.
    Load(KernelError),
    /// Labels of the assertions the kernel rejected, with the reason.
    Assertions(Vec<(String, KernelError)>),
    /// The library's trailer or one of its sections is damaged.
    Damaged(SectionError),
    /// The library has sections and does not decode.
    Read(ReadError),
    /// A section says something its records do not.
    Sections(SectionCheckError),
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckError::Load(e) => write!(f, "the packed library does not load: {e:?}"),
            CheckError::Damaged(e) => e.fmt(f),
            CheckError::Read(e) => e.fmt(f),
            CheckError::Sections(e) => write!(f, "a section is wrong: {e}"),
            CheckError::Assertions(failed) => {
                for (label, e) in failed {
                    writeln!(f, "{label}: {e:?}")?;
                }
                write!(f, "{} assertions failed the kernel check", failed.len())
            }
        }
    }
}

/// Runs the kernel over every assertion of a library, then checks its sections against its
/// records, and returns how many assertions there are.
pub fn check(bytes: &[u8]) -> Result<usize, CheckError> {
    let mut kernel = Kernel::new(bytes, Limits::UNBOUNDED).map_err(CheckError::Load)?;
    let mut failed = Vec::new();
    for id in 0..kernel.assertions() {
        if let Err(e) = kernel.verify(id) {
            let label = String::from_utf8_lossy(kernel.label(id).unwrap_or_default());
            failed.push((label.into_owned(), e));
        }
    }
    if !failed.is_empty() {
        return Err(CheckError::Assertions(failed));
    }
    let sections = read_sections(bytes).map_err(CheckError::Damaged)?;
    if sections.is_some() {
        let library = decode(bytes).map_err(CheckError::Read)?;
        verify_sections(&library).map_err(CheckError::Sections)?;
    }
    Ok(kernel.assertions())
}
