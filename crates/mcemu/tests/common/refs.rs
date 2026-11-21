//! What the library reader of mcpack says each assertion's reference list is, as the u16 entries a
//! record stores: 8000h plus a dummy variable's symbol number, or the number of an earlier assertion.

use mcpack::library::{Reference, decode};

/// The stored entries of every assertion, in order; an axiom's list is empty.
pub fn stored(bytes: &[u8]) -> Vec<Vec<u16>> {
    let library = decode(bytes).expect("the library decodes");
    library
        .assertions
        .iter()
        .map(|assertion| {
            assertion
                .references
                .iter()
                .map(|reference| match reference {
                    Reference::Assertion(n) => u16::try_from(*n).expect("an assertion number"),
                    Reference::Invalid(raw) => *raw,
                    Reference::Dummy(variable) => {
                        let at = library.variables.iter().position(|v| v == variable);
                        0x8000
                            + u16::try_from(
                                library.constants.len() + at.expect("a declared variable"),
                            )
                            .unwrap()
                    }
                })
                .collect()
        })
        .collect()
}

/// The entries as the bytes a record stores them in.
pub fn bytes_of(list: &[u16]) -> Vec<u8> {
    list.iter().flat_map(|entry| entry.to_le_bytes()).collect()
}
