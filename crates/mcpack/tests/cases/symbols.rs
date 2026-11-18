use crate::raw::{self, Case};
use mcpack::library::{CheckError, Kernel, KernelError, Limits, check};
use std::path::Path;
use std::process::Command;

fn model(case: &Case) -> Result<(), KernelError> {
    Kernel::new(&case.bytes, Limits::UNBOUNDED).map(|_| ())
}

#[test]
fn a_symbol_table_must_end_inside_the_first_page() {
    for case in raw::crossing_cases() {
        assert_eq!(model(&case), case.init, "{}", case.name);
    }
}

#[test]
fn symbol_names_are_checked_at_load() {
    for case in raw::name_cases() {
        assert_eq!(model(&case), case.init, "{}", case.name);
    }
}

#[test]
fn typecode_bytes_are_checked_at_load() {
    for case in raw::typecode_cases() {
        assert_eq!(model(&case), case.init, "{}", case.name);
    }
}

#[test]
fn check_refuses_the_library_whose_variables_shadow_its_constants() {
    let bytes = raw::library(&[b"T.", b"F."], &[b"F.", b"T."], &[b"ax-1"]);
    assert_eq!(check(&bytes), Err(CheckError::Load(KernelError::BadSymbol)));
}

#[test]
fn check_refuses_labels_the_kernel_does_not_look_at() {
    for (what, bytes, named) in raw::bad_label_libraries() {
        let mut kernel = Kernel::new(&bytes, Limits::UNBOUNDED).expect(what);
        for id in 0..kernel.assertions() {
            assert_eq!(kernel.verify(id), Ok(()), "{what}");
        }
        assert_eq!(
            check(&bytes),
            Err(CheckError::Label(named.into())),
            "{what}"
        );
    }
}

#[test]
fn check_accepts_every_character_of_the_label_alphabet() {
    let labels: [&[u8]; 3] = [b"a.B-c_9", b"0", b"Z"];
    let bytes = raw::library(&[b"T"], &[b"x"], &labels);
    assert_eq!(check(&bytes), Ok(3));
}

#[test]
fn doc_refuses_a_library_with_a_repeated_label() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("repeated-label");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let manifest = "name = \"t\"\ntitle = \"T\"\ndescription = \"d\"\norder = 1\nroots = [\"a\"]\n";
    let pin = "repository = \"https://example.org/r\"\ncommit = \"abc\"\npath = \"x.mm\"\nsha256 = \"0\"\n";
    std::fs::write(dir.join("t.toml"), manifest).unwrap();
    std::fs::write(dir.join("source.toml"), pin).unwrap();
    let library = dir.join("t.mclib");
    std::fs::write(&library, raw::library(&[b"T"], &[], &[b"ax-1", b"ax-1"])).unwrap();
    let page = dir.join("t.md");
    let out = Command::new(env!("CARGO_BIN_EXE_mcpack"))
        .arg("doc")
        .arg(dir.join("t.toml"))
        .arg("--library")
        .arg(&library)
        .arg("-o")
        .arg(&page)
        .output()
        .expect("mcpack runs");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        format!("{}: \"ax-1\" cannot be a unique label\n", library.display())
    );
    assert!(!page.exists());
}
