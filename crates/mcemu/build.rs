use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

const ROM_SIZE: usize = 0x20_0000;
const PAGE: usize = 0x4000;
const BOOT_PAGE: usize = 0x7F;

/// Test routines: source, output name, spasm defines.
const ROUTINES: &[(&str, &str, &[&str])] = &[
    ("sum.z80", "sum_ram.bin", &["ENTRY=$9000", "RESULT=$A000"]),
    ("sum.z80", "sum_flash.bin", &["ENTRY=$4000", "RESULT=$A000"]),
    ("lcd_busy.z80", "lcd_busy.bin", &[]),
    ("lcd_busy.z80", "lcd_wait.bin", &["WAIT"]),
    ("keys.z80", "keys.bin", &["RESULT=$A000"]),
    ("count.z80", "count.bin", &["ENTRY=$9000"]),
    ("on_key.z80", "on_key.bin", &["RESULT=$A000"]),
];

/// Routines that call the stub's OS services: source, output name, spasm defines.
const OS_ROUTINES: &[(&str, &str, &[&str])] = &[
    (
        "os_call.z80",
        "os_getcsc.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GetCSC"],
    ),
    (
        "os_call.z80",
        "os_exit.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_JForceCmdNoChar",
            "JUMP",
        ],
    ),
    (
        "os_call.z80",
        "os_bcall_missing.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=$4A4A"],
    ),
    (
        "os_call.z80",
        "os_bjump_missing.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=$4B4B", "JUMP"],
    ),
    (
        "os_lcd.z80",
        "os_gr_buf_cpy.bin",
        &["ENTRY=$9000", "SERVICE=_GrBufCpy"],
    ),
    (
        "os_lcd.z80",
        "os_clr_lcd_full.bin",
        &["ENTRY=$9000", "SERVICE=_ClrLCDFull"],
    ),
    (
        "os_clobber.z80",
        "os_clobber_getcsc.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GetCSC"],
    ),
    (
        "os_clobber.z80",
        "os_clobber_clr_lcd_full.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_ClrLCDFull"],
    ),
    (
        "os_clobber.z80",
        "os_clobber_gr_buf_cpy.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GrBufCpy"],
    ),
    (
        "os_reliant.z80",
        "os_reliant.bin",
        &["ENTRY=$9000", "RESULT=$A000"],
    ),
    ("os_irq.z80", "os_irq.bin", &["ENTRY=$9000", "RESULT=$A000"]),
];

fn fail(msg: &str) -> ! {
    eprintln!("error: {msg}");
    exit(1);
}

fn tilem_sources() -> Vec<PathBuf> {
    println!("cargo:rerun-if-env-changed=TILEM_SRC");
    let Some(root) = env::var_os("TILEM_SRC") else {
        fail("TILEM_SRC is not set; run scripts/fetch-tilem.sh and point it at the checkout");
    };
    let emu = Path::new(&root).join("emu");
    if !emu.join("tilem.h").is_file() {
        fail(&format!(
            "TILEM_SRC={} has no emu/tilem.h; run scripts/fetch-tilem.sh",
            Path::new(&root).display()
        ));
    }
    println!("cargo:rerun-if-changed={}", emu.display());
    let mut files = Vec::new();
    let mut dirs = vec![emu.clone()];
    for entry in fs::read_dir(&emu).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() && path.file_name().unwrap().to_string_lossy().starts_with('x') {
            dirs.push(path);
        }
    }
    for dir in dirs {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "c") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn compile_tilem() {
    let files = tilem_sources();
    let emu = Path::new(&env::var_os("TILEM_SRC").unwrap()).join("emu");
    cc::Build::new()
        .files(&files)
        .file("csrc/harness.c")
        .include(&emu)
        .include(emu.join("xz"))
        .define("HAVE_STDINT_H", "1")
        .define("HAVE_INTTYPES_H", "1")
        .define("HAVE_STRING_H", "1")
        .define("PACKAGE_VERSION", "\"pinned\"")
        .define("restrict", "__restrict")
        .define("_POSIX_C_SOURCE", "200809L")
        .warnings(false)
        .opt_level(2)
        .compile("tilem");
    println!("cargo:rerun-if-changed=csrc/harness.c");
}

fn spasm_ok(src: &Path, out: &Path, defines: &[&str], args: &[&str]) -> bool {
    let mut cmd = Command::new(env::var_os("SPASM").unwrap_or_else(|| "spasm".into()));
    for d in defines {
        cmd.arg(format!("-D{d}"));
    }
    cmd.args(args)
        .arg("-N")
        .arg(src)
        .arg(out)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn spasm(src: &Path, out: &Path, defines: &[&str], args: &[&str]) {
    println!("cargo:rerun-if-changed={}", src.display());
    let mut cmd = Command::new(env::var_os("SPASM").unwrap_or_else(|| "spasm".into()));
    for d in defines {
        cmd.arg(format!("-D{d}"));
    }
    let status = cmd.args(args).arg("-N").arg(src).arg(out).output();
    match status {
        Ok(o) if o.status.success() => {}
        Ok(o) => fail(&format!(
            "spasm failed on {}: {}",
            src.display(),
            String::from_utf8_lossy(&o.stdout).trim().replace('\n', " ")
        )),
        Err(e) => fail(&format!("cannot run spasm (set SPASM to its path): {e}")),
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=SPASM");
    compile_tilem();

    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let dir = Path::new("stub");
    let os = out.join("os.bin");
    let boot = out.join("boot.bin");
    spasm(&dir.join("os.z80"), &os, &[], &["-L"]);
    write_stub_labels(&out.join("os.lab"), &out.join("stub_labels.rs"));
    spasm(&dir.join("boot.z80"), &boot, &[], &[]);
    let mut rom = vec![0xFF; ROM_SIZE];
    for (bin, at) in [(os, 0), (boot, BOOT_PAGE * PAGE)] {
        let bytes = fs::read(bin).unwrap();
        rom[at..at + bytes.len()].copy_from_slice(&bytes);
    }
    fs::write(out.join("stub.rom"), rom).unwrap();

    for (src, name, defines) in ROUTINES {
        spasm(
            &Path::new("tests/routines").join(src),
            &out.join(name),
            defines,
            &[],
        );
    }
    for (src, name, defines) in OS_ROUTINES {
        spasm(
            &Path::new("tests/routines").join(src),
            &out.join(name),
            defines,
            &["-I", "../../app/src"],
        );
    }
    assemble_kernel(&out);
    assemble_ui(&out);
    spasm(
        &Path::new("tests/routines").join("launch_state.z80"),
        &out.join("launch_state.8xk"),
        &[],
        &["-I", "../../app/src", "-L"],
    );
    for name in ["sparecheck_test", "sparescreen_test"] {
        spasm(
            &Path::new("tests/routines").join(format!("{name}.z80")),
            &out.join(format!("{name}.bin")),
            &[],
            &["-I", "../../app/src", "-I", "../../probe", "-L"],
        );
    }
    println!("cargo:rerun-if-changed=../../probe/sparecore.inc");
    println!("cargo:rerun-if-changed=../../probe/sparescreen.inc");
    println!("cargo:rerun-if-changed=../../app/src/equates.inc");
    println!("cargo:rerun-if-changed=../../app/src/header.inc");
}

/// The stub addresses the library needs, as constants named after their labels.
fn write_stub_labels(labels: &Path, out: &Path) {
    const WANTED: &[&str] = &["APP_EXIT", "FAULT_BCALL", "FAULT_BJUMP", "OS_INIT"];
    let text = fs::read_to_string(labels).unwrap();
    let mut code = String::new();
    for name in WANTED {
        let value = text
            .lines()
            .find_map(|l| l.split_once(" = $").filter(|(k, _)| k == name))
            .unwrap_or_else(|| fail(&format!("the stub has no label {name}")))
            .1;
        code.push_str(&format!("pub const {name}: u16 = 0x{value};\n"));
    }
    fs::write(out, code).unwrap();
}

/// The kernel on its own, and with the test entry points and a label file for the tests.
fn assemble_kernel(out: &Path) {
    let dir = Path::new("../../kernel");
    for entry in fs::read_dir(dir).unwrap().flatten() {
        println!("cargo:rerun-if-changed={}", entry.path().display());
    }
    let include = dir.to_str().unwrap();
    spasm(
        &dir.join("kernel.z80"),
        &out.join("kernel_standalone.bin"),
        &[],
        &["-I", include],
    );
    let assembles = |base: &str| {
        spasm_ok(
            &dir.join("kernel.z80"),
            &out.join("kernel_base_probe.bin"),
            &[&format!("KERNEL_RAM={base}")],
            &["-I", include],
        )
    };
    assert!(
        assembles("$CE00"),
        "a kernel RAM base of $CE00 does not assemble"
    );
    for base in ["$BF00", "$C080", "$CF00", "$F100"] {
        assert!(!assembles(base), "a kernel RAM base of {base} assembled");
    }
    for (name, defines) in [
        ("kernel_test", &[][..]),
        ("kernel_test_moved", &["KERNEL_RAM=$C400"][..]),
    ] {
        spasm(
            Path::new("tests/routines/kernel_test.z80"),
            &out.join(format!("{name}.bin")),
            defines,
            &["-I", include, "-L"],
        );
    }
}

/// The canvas with the built-in font, and with its RAM block moved; each with the label file the
/// tests read.
fn assemble_ui(out: &Path) {
    let dir = Path::new("../../ui");
    for entry in fs::read_dir(dir).unwrap().flatten() {
        println!("cargo:rerun-if-changed={}", entry.path().display());
    }
    let font_dir = out.join("ui_font");
    fs::create_dir_all(&font_dir).unwrap();
    let table = mcglyph::z80::Table::new(mcglyph::Font::builtin()).unwrap_or_else(|e| fail(&e));
    fs::write(font_dir.join("font.inc"), table.include("ui_font")).unwrap();
    let include = dir.to_str().unwrap();
    let assemble = |defines: &[&str], output: &Path| {
        spasm_ok(
            &dir.join("ui.z80"),
            output,
            defines,
            &["-I", include, "-I", font_dir.to_str().unwrap(), "-L"],
        )
    };
    for (name, defines) in [
        ("ui_test", &[][..]),
        ("ui_moved", &["UI_FRAME=$A000", "UI_VARS=$A500"][..]),
    ] {
        if !assemble(defines, &out.join(format!("{name}.bin"))) {
            fail(&format!("spasm failed on the canvas image {name}"));
        }
    }
    assert!(
        !assemble(&["UI_VARS=$9400"], &out.join("ui_probe.bin")),
        "overlapping canvas blocks assembled"
    );
}
