use super::{
    Kernel, KernelError, Limits, ReadError, SectionCheckError, SectionError, WORK_LIMIT, decode,
    labels, locate_sections, verify_sections,
};

/// The most working memory `check` lets a proof use, a multiple of the calculator's buffers in
/// `kernel/ram.inc`, so a proof too big for the calculator still checks on the host and one that
/// would take gigabytes is refused.
pub const HOST_LIMITS: Limits = Limits {
    stack_syms: 131072,
    stack_entries: 8192,
    saved_syms: 131072,
    saved_entries: 8192,
};

fn describe(error: &KernelError) -> String {
    match error {
        KernelError::OutOfMemory => {
            let l = HOST_LIMITS;
            format!(
                "OutOfMemory: the proof needs more than {} stack symbols, {} stack entries, {} saved symbols or {} saved entries",
                l.stack_syms, l.stack_entries, l.saved_syms, l.saved_entries
            )
        }
        KernelError::WorkLimit => format!(
            "WorkLimit: the disjoint-variable checks need more than {WORK_LIMIT} units of work"
        ),
        other => format!("{other:?}"),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CheckError {
    /// The library does not load.
    Load(KernelError),
    /// Labels of the assertions the kernel rejected, with the reason.
    Assertions(Vec<(String, KernelError)>),
    /// An assertion label that is not a Metamath label, repeats or is also a symbol.
    Label(String),
    /// The library's trailer or one of its sections is damaged.
    Damaged(SectionError),
    /// The kernel accepts the library and the decoder does not, for instance because records
    /// overlap or share an offset.
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
                    writeln!(f, "{}: {}", crate::mm::shown(label.as_bytes()), describe(e))?;
                }
                write!(f, "{} assertions failed the kernel check", failed.len())
            }
            CheckError::Label(label) => write!(
                f,
                "\"{}\" cannot be a unique label",
                crate::mm::shown(label.as_bytes())
            ),
        }
    }
}

/// Runs the kernel over every assertion of a library within `HOST_LIMITS` and `WORK_LIMIT`, checks
/// its labels, then checks its sections against its records, and returns how many assertions there
/// are.
pub fn check(bytes: &[u8]) -> Result<usize, CheckError> {
    let (count, rejected) = check_each(bytes, false)?;
    debug_assert!(rejected.is_empty());
    Ok(count)
}

/// The labels of the assertions the kernel rejects, with the reason, in library order.
pub(super) type Rejected = Vec<(String, KernelError)>;

/// `check`, except that with `tolerate` an assertion the kernel rejects does not fail the check:
/// the library's labels and sections are checked all the same and the rejected assertions come
/// back with the count.
pub(super) fn check_each(bytes: &[u8], tolerate: bool) -> Result<(usize, Rejected), CheckError> {
    let mut kernel = Kernel::new(bytes, HOST_LIMITS).map_err(CheckError::Load)?;
    let mut failed = Vec::new();
    for id in 0..kernel.assertions() {
        if let Err(e) = kernel.verify(id) {
            let label = String::from_utf8_lossy(kernel.label(id).unwrap_or_default());
            failed.push((label.into_owned(), e));
        }
    }
    if !failed.is_empty() && !tolerate {
        return Err(CheckError::Assertions(failed));
    }
    if let Some(label) = labels::first_bad(&kernel) {
        return Err(CheckError::Label(label));
    }
    let sections = locate_sections(bytes).map_err(CheckError::Damaged)?;
    let library = decode(bytes).map_err(CheckError::Read)?;
    if sections.is_some() {
        verify_sections(&library).map_err(CheckError::Sections)?;
    }
    Ok((kernel.assertions(), failed))
}
