use std::path::Path;
use std::process::Command;

/// Assembles through scripts/spasm.sh, which fails on a nonzero exit or any error or warning
/// line. A failure holds the assembler's own words; a missing shell or wrapper is a failure too.
pub fn assemble(src: &Path, out: &Path, defines: &[&str], args: &[&str]) -> Result<(), String> {
    let wrapper = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/spasm.sh");
    let mut cmd = Command::new("sh");
    cmd.arg(&wrapper);
    for d in defines {
        cmd.arg(format!("-D{d}"));
    }
    match cmd.args(args).arg("-N").arg(src).arg(out).output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )
        .trim()
        .replace('\n', " ")),
        Err(e) => Err(format!("cannot run {}: {e}", wrapper.display())),
    }
}
