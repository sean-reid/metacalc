use super::inspect::kept;
use super::{fail, parse_source, read, unusable};
use mcpack::library::{self, check};
use mcpack::manifest::{Manifest, Pin};
use mcpack::mm::{Database, StmtId};
use mcpack::{build as plan, page};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn pack_checked(db: &Database, kept: &[StmtId], output: &Path) -> Result<ExitCode, ExitCode> {
    let bytes = library::pack(db, kept).map_err(|e| fail(format_args!("{e:?}")))?;
    let assertions = check(&bytes).map_err(fail)?;
    std::fs::write(output, &bytes)
        .map_err(|e| unusable(format_args!("{}: {e}", output.display())))?;
    println!(
        "{assertions} assertions, {} bytes, {} pages of {} bytes",
        bytes.len(),
        bytes.len().div_ceil(library::CHUNK),
        library::CHUNK
    );
    Ok(ExitCode::SUCCESS)
}

pub fn pack(db: &Database, roots: &[String], output: &Path) -> Result<ExitCode, ExitCode> {
    pack_checked(db, &kept(db, roots)?, output)
}

fn manifest_and_pin(path: &Path) -> Result<(Manifest, Pin), ExitCode> {
    let manifest = Manifest::load(path).map_err(unusable)?;
    let pin = Pin::load(path).map_err(unusable)?;
    Ok((manifest, pin))
}

pub fn build(manifest: &Path, database: &Path, output: &Path) -> Result<ExitCode, ExitCode> {
    let (manifest, pin) = manifest_and_pin(manifest)?;
    let src = read(database)?;
    pin.check(&src)
        .map_err(|e| unusable(format_args!("{}: {e}", database.display())))?;
    let db = parse_source(database, src)?;
    let kept = plan::closure(&db, &manifest).map_err(|e| {
        if e.is_bad_input() {
            unusable(e)
        } else {
            fail(e)
        }
    })?;
    pack_checked(&db, &kept, output)
}

pub fn doc(
    manifest: &Path,
    library: &Path,
    output: &Path,
    require_unicode: bool,
) -> Result<ExitCode, ExitCode> {
    let (manifest, pin) = manifest_and_pin(manifest)?;
    let bytes = read(library)?;
    check(&bytes).map_err(|e| fail(format_args!("{}: {e}", library.display())))?;
    let broken = |e| fail(format_args!("{}: {e}", library.display()));
    let text = page::render(&manifest, &pin, &bytes).map_err(broken)?;
    let unmapped = page::unmapped(&bytes).map_err(broken)?;
    if require_unicode && !unmapped.is_empty() {
        return Err(fail(format_args!(
            "{}: no Unicode spelling for {}",
            library.display(),
            unmapped.join(" ")
        )));
    }
    std::fs::write(output, &text)
        .map_err(|e| unusable(format_args!("{}: {e}", output.display())))?;
    println!("{}: {} bytes", output.display(), text.len());
    Ok(ExitCode::SUCCESS)
}

pub fn summary(manifests: &[PathBuf], libdir: &Path) -> Result<ExitCode, ExitCode> {
    let mut libraries = Vec::new();
    for path in manifests {
        let (manifest, _) = manifest_and_pin(path)?;
        let file = libdir.join(format!("{}.mclib", manifest.name));
        let bytes = read(&file)?;
        check(&bytes).map_err(|e| fail(format_args!("{}: {e}", file.display())))?;
        libraries.push((manifest, bytes));
    }
    let table = page::table(libraries).map_err(unusable)?;
    print!("{table}");
    Ok(ExitCode::SUCCESS)
}

pub fn export(library: &Path, output: &Path) -> Result<ExitCode, ExitCode> {
    let bytes = read(library)?;
    check(&bytes).map_err(|e| fail(format_args!("{}: {e}", library.display())))?;
    let text =
        library::export(&bytes).map_err(|e| fail(format_args!("{}: {e}", library.display())))?;
    std::fs::write(output, &text)
        .map_err(|e| unusable(format_args!("{}: {e}", output.display())))?;
    println!("{}: {} bytes", output.display(), text.len());
    Ok(ExitCode::SUCCESS)
}
