use super::Kernel;
use std::collections::HashSet;

/// The first label that is empty, has a byte outside the Metamath label alphabet, repeats an
/// earlier label or is also a symbol name. The kernel does not look at labels.
pub(super) fn first_bad(kernel: &Kernel) -> Option<String> {
    let mut seen: HashSet<&[u8]> = kernel.symbol_names().iter().copied().collect();
    let alphabet = |b: &u8| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.');
    (0..kernel.assertions()).find_map(|id| {
        let label = kernel.label(id).ok()?;
        let valid = !label.is_empty() && label.iter().all(alphabet);
        (!valid || !seen.insert(label)).then(|| String::from_utf8_lossy(label).into_owned())
    })
}
