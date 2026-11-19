use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

#[path = "build_support/spasm.rs"]
mod spasm_run;

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
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GetCSC", "WAIT=2"],
    ),
    (
        "os_call.z80",
        "os_getcsc_blind.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GetCSC"],
    ),
    (
        "os_iff.z80",
        "os_iff_getcsc_off.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GetCSC"],
    ),
    (
        "os_iff.z80",
        "os_iff_getcsc_on.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GetCSC", "ON"],
    ),
    (
        "os_iff.z80",
        "os_iff_put_c_off.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_PutC"],
    ),
    (
        "os_iff.z80",
        "os_iff_put_c_on.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_PutC", "ON"],
    ),
    (
        "os_iff.z80",
        "os_iff_clr_lcd_full_off.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_ClrLCDFull"],
    ),
    (
        "os_iff.z80",
        "os_iff_gr_buf_cpy_off.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GrBufCpy"],
    ),
    (
        "os_iff.z80",
        "os_iff_clr_lcd_full_on.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_ClrLCDFull", "ON"],
    ),
    (
        "os_iff.z80",
        "os_iff_gr_buf_cpy_on.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GrBufCpy", "ON"],
    ),
    (
        "os_clobber.z80",
        "os_clobber_interrupt.bin",
        &["ENTRY=$9000", "RESULT=$A000", "INTERRUPT"],
    ),
    (
        "os_clobber.z80",
        "os_clobber_put_c.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_PutC"],
    ),
    (
        "os_call.z80",
        "os_exit.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_JForceCmdNoChar",
            "JUMP",
            "IM1",
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
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GrBufCpy"],
    ),
    (
        "os_lcd.z80",
        "os_clr_lcd_full.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_ClrLCDFull"],
    ),
    (
        "os_lcd.z80",
        "os_gr_buf_cpy_mode4.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GrBufCpy", "MODE=4"],
    ),
    (
        "os_lcd.z80",
        "os_gr_buf_cpy_mode6.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GrBufCpy", "MODE=6"],
    ),
    (
        "os_lcd.z80",
        "os_gr_buf_cpy_mode7.bin",
        &["ENTRY=$9000", "RESULT=$A000", "SERVICE=_GrBufCpy", "MODE=7"],
    ),
    (
        "os_lcd.z80",
        "os_clr_lcd_full_mode4.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_ClrLCDFull",
            "MODE=4",
        ],
    ),
    (
        "os_lcd.z80",
        "os_clr_lcd_full_mode6.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_ClrLCDFull",
            "MODE=6",
        ],
    ),
    (
        "os_lcd.z80",
        "os_gr_buf_cpy_ints_mode4.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_GrBufCpy",
            "MODE=4",
            "INTERRUPTS",
        ],
    ),
    (
        "os_lcd.z80",
        "os_gr_buf_cpy_ints_mode5.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_GrBufCpy",
            "MODE=5",
            "INTERRUPTS",
        ],
    ),
    (
        "os_lcd.z80",
        "os_gr_buf_cpy_ints_mode6.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_GrBufCpy",
            "MODE=6",
            "INTERRUPTS",
        ],
    ),
    (
        "os_lcd.z80",
        "os_gr_buf_cpy_ints_mode7.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_GrBufCpy",
            "MODE=7",
            "INTERRUPTS",
        ],
    ),
    (
        "os_lcd.z80",
        "os_clr_lcd_full_ints_mode4.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_ClrLCDFull",
            "MODE=4",
            "INTERRUPTS",
        ],
    ),
    (
        "os_lcd.z80",
        "os_clr_lcd_full_ints_mode5.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_ClrLCDFull",
            "MODE=5",
            "INTERRUPTS",
        ],
    ),
    (
        "os_lcd.z80",
        "os_clr_lcd_full_ints_mode6.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_ClrLCDFull",
            "MODE=6",
            "INTERRUPTS",
        ],
    ),
    (
        "os_lcd.z80",
        "os_clr_lcd_full_ints_mode7.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_ClrLCDFull",
            "MODE=7",
            "INTERRUPTS",
        ],
    ),
    (
        "os_lcd.z80",
        "os_clr_lcd_full_mode7.bin",
        &[
            "ENTRY=$9000",
            "RESULT=$A000",
            "SERVICE=_ClrLCDFull",
            "MODE=7",
        ],
    ),
    (
        "os_service.z80",
        "os_chk_find_sym.bin",
        &["ENTRY=$9000", "RESULT=$9100", "SERVICE=_ChkFindSym"],
    ),
    (
        "os_service.z80",
        "os_enough_mem.bin",
        &["ENTRY=$9000", "RESULT=$9100", "SERVICE=_EnoughMem"],
    ),
    (
        "os_service.z80",
        "os_create_app_var.bin",
        &["ENTRY=$9000", "RESULT=$9100", "SERVICE=_CreateAppVar"],
    ),
    (
        "os_service.z80",
        "os_del_var_arc.bin",
        &["ENTRY=$9000", "RESULT=$9100", "SERVICE=_DelVarArc"],
    ),
    (
        "os_service.z80",
        "os_arc_unarc.bin",
        &["ENTRY=$9000", "RESULT=$9100", "SERVICE=_Arc_Unarc"],
    ),
    (
        "os_service.z80",
        "os_create_r_list.bin",
        &["ENTRY=$9000", "RESULT=$9100", "SERVICE=_CreateRList"],
    ),
    (
        "os_service.z80",
        "os_load_de_ind_paged.bin",
        &["ENTRY=$9000", "RESULT=$9100", "SERVICE=_LoadDEIndPaged"],
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
    (
        "os_on_irq.z80",
        "os_on_irq_flags.bin",
        &["ENTRY=$9000", "RESULT=$A000", "IYVAL=$89F0"],
    ),
    (
        "os_on_irq.z80",
        "os_on_irq_other.bin",
        &["ENTRY=$9000", "RESULT=$A000", "IYVAL=$BEEF"],
    ),
];

/// Kernel images with the test entry stub: output name and spasm defines. The first is the
/// default layout; each is a layout the checks in assemble_kernel accept.
const IMAGES: &[(&str, &[&str])] = &[
    ("kernel_test", &[]),
    (
        "kernel_test_moved",
        &["KERNEL_RAM_LOW=$C100", "KERNEL_RAM_HIGH=$E000"],
    ),
    (
        "kernel_test_top",
        &["KERNEL_RAM_LOW=$EE00", "KERNEL_RAM_HIGH=$C100"],
    ),
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
        .define("time", "mcemu_time")
        .define("_POSIX_C_SOURCE", "200809L")
        .warnings(false)
        .opt_level(2)
        .compile("tilem");
    println!("cargo:rerun-if-changed=csrc/harness.c");
}

fn spasm_try(src: &Path, out: &Path, defines: &[&str], args: &[&str]) -> Result<(), String> {
    spasm_run::assemble(src, out, defines, args)
}

fn spasm(src: &Path, out: &Path, defines: &[&str], args: &[&str]) {
    println!("cargo:rerun-if-changed={}", src.display());
    if let Err(why) = spasm_try(src, out, defines, args) {
        fail(&format!("spasm failed on {}: {why}", src.display()));
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=SPASM");
    println!("cargo:rerun-if-changed=../../scripts/spasm.sh");
    println!("cargo:rerun-if-changed=build_support/spasm.rs");
    compile_tilem();

    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let dir = Path::new("stub");
    let os = out.join("os.bin");
    let boot = out.join("boot.bin");
    for part in ["os.z80", "vars.z80", "varsvc.z80"] {
        println!("cargo:rerun-if-changed={}", dir.join(part).display());
    }
    spasm(&dir.join("os.z80"), &os, &[], &["-L", "-I", "stub"]);
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
    assemble_reader(&out);
    spasm(
        &Path::new("tests/routines").join("launch_state.z80"),
        &out.join("launch_state.8xk"),
        &[],
        &["-I", "../../app/src", "-L"],
    );
    for name in ["sparecheck_test", "sparescreen_test", "kerncheck_test"] {
        spasm(
            &Path::new("tests/routines").join(format!("{name}.z80")),
            &out.join(format!("{name}.bin")),
            &[],
            &[
                "-I",
                "../../app/src",
                "-I",
                "../../probe",
                "-I",
                "../../kernel",
                "-L",
            ],
        );
    }
    spasm(
        Path::new("../../probe/storecheck.z80"),
        &out.join("storecheck.8xk"),
        &[],
        &["-I", "../../app/src", "-I", "../../probe", "-L"],
    );
    for f in [
        "../../probe/storecheck.z80",
        "../../probe/storedraw.inc",
        "../../app/src/store.inc",
        "../../app/src/store_count.inc",
        "../../app/src/store_open.inc",
    ] {
        println!("cargo:rerun-if-changed={f}");
    }
    println!("cargo:rerun-if-changed=../../probe/sparecore.inc");
    println!("cargo:rerun-if-changed=../../probe/sparescreen.inc");
    println!("cargo:rerun-if-changed=../../probe/printnum.inc");
    println!("cargo:rerun-if-changed=../../probe/kerndriver.inc");
    println!("cargo:rerun-if-changed=../../probe/onbreak.inc");
    println!("cargo:rerun-if-changed=../../app/src/libpage.inc");
    println!("cargo:rerun-if-changed=../../app/src/equates.inc");
    println!("cargo:rerun-if-changed=../../app/src/header.inc");
}

/// The stub addresses the library needs, as constants named after their labels.
fn write_stub_labels(labels: &Path, out: &Path) {
    const WANTED: &[&str] = &[
        "APP_EXIT",
        "APP_RETURN",
        "FAULT_BCALL",
        "FAULT_BJUMP",
        "FAULT_ERROR",
        "FAULT_LCD",
        "OS_INIT",
        "HOST_INIT",
        "HOST_FIND",
        "HOST_CREATE",
        "HOST_ARCHIVE",
        "HOST_DELETE",
        "HOST_GROW",
        "VS_HEAP_END",
        "VS_VAT_LOW",
        "VS_RESERVE",
        "VS_ARC_NEXT",
        "ARC_PAGE",
    ];
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
    let assemble = |defines: &[&str]| {
        spasm_try(
            &dir.join("kernel.z80"),
            &out.join("kernel_base_probe.bin"),
            defines,
            &["-I", include],
        )
    };
    let overlap = "the low and high regions overlap";
    for (defines, refusal) in [
        (&["KERNEL_RAM_LOW=$C000", "KERNEL_RAM_HIGH=$E000"][..], None),
        (&["KERNEL_RAM_LOW=$EE00", "KERNEL_RAM_HIGH=$C000"][..], None),
        (&["KERNEL_RAM_LOW=$C000", "KERNEL_RAM_HIGH=$D200"][..], None),
        (
            &["KERNEL_RAM_LOW=$C000", "KERNEL_RAM_HIGH=$D100"][..],
            Some(overlap),
        ),
        (&["KERNEL_RAM_LOW=$D000"][..], Some(overlap)),
        (
            &["KERNEL_RAM_LOW=$BF00"][..],
            Some("KERNEL_RAM_LOW must be in bank C"),
        ),
        (
            &["KERNEL_RAM_LOW=$C080"][..],
            Some("KERNEL_RAM_LOW must be a multiple of $100"),
        ),
        (
            &["KERNEL_RAM_LOW=$EF00"][..],
            Some("the low region does not fit below $10000"),
        ),
        (
            &["KERNEL_RAM_HIGH=$BF00"][..],
            Some("KERNEL_RAM_HIGH must be in bank C"),
        ),
        (
            &["KERNEL_RAM_HIGH=$E100"][..],
            Some("the high region does not fit below $10000"),
        ),
        (
            &["KERNEL_RAM_HIGH=$D880"][..],
            Some("KERNEL_RAM_HIGH must be a multiple of $100"),
        ),
        (&["KERNEL_RAM_PAGE=7"][..], None),
        (&["KERNEL_RAM_PAGE=2"][..], None),
        (
            &["KERNEL_RAM_PAGE=8"][..],
            Some("there are no pages above 7"),
        ),
        (
            &["KERNEL_RAM_PAGE=0"][..],
            Some("pages 0 and 1 belong to TI-OS"),
        ),
        (
            &["KERNEL_RAM_PAGE=1"][..],
            Some("pages 0 and 1 belong to TI-OS"),
        ),
        (&["KERNEL_RAM_LOW=$EE00", "KERNEL_RAM_HIGH=$C100"][..], None),
        (
            &["KERNEL_RAM_LOW=$EE00", "KERNEL_RAM_HIGH=$E000"][..],
            Some(overlap),
        ),
        (
            &["WORK_LIMIT=130815"][..],
            Some("WORK_LIMIT must be at least"),
        ),
        (&["WORK_LIMIT=1"][..], Some("WORK_LIMIT must be at least")),
        (&["WORK_LIMIT=130816"][..], None),
        (&["WORK_LIMIT=16777215"][..], None),
        (&["WORK_LIMIT=16777216"][..], Some("WORK_LIMIT must fit")),
    ] {
        match (assemble(defines), refusal) {
            (Ok(()), None) => {}
            (Err(why), Some(want)) if why.contains(want) => {}
            (got, _) => fail(&format!(
                "assembling the kernel with {defines:?} gave {got:?}, wanted {}",
                refusal.map_or("success", |want| want)
            )),
        }
    }
    for (name, defines) in IMAGES {
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
    fs::write(font_dir.join("font.inc"), mcglyph::z80::fonts_include()).unwrap();
    let include = dir.to_str().unwrap();
    let assemble = |defines: &[&str], output: &Path| {
        spasm_try(
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
        if let Err(why) = assemble(defines, &out.join(format!("{name}.bin"))) {
            fail(&format!("spasm failed on the canvas image {name}: {why}"));
        }
    }
    match assemble(&["UI_VARS=$9400"], &out.join("ui_probe.bin")) {
        Err(why) if why.contains("UI_FRAME and UI_VARS overlap") => {}
        other => fail(&format!(
            "overlapping canvas blocks gave {other:?}, not an overlap error"
        )),
    }
    const BOX_ERROR: &str =
        "a soft key box must span one or two whole bytes between its edge bytes";
    match assemble(&["UI_WIDTH=250"], &out.join("ui_probe.bin")) {
        Err(why) if why.contains(BOX_ERROR) => {}
        other => fail(&format!(
            "soft key boxes too wide for the flip gave {other:?}, not a box error"
        )),
    }
}

/// The library reader with the test driver: as shipped, with the kernel beside it, with its
/// state and bounce buffer moved, and with a bounce buffer that runs past the end of the kernel's
/// region. Out-of-place addresses must not assemble, each with its own error.
fn assemble_reader(out: &Path) {
    let dir = Path::new("../../lib");
    for entry in fs::read_dir(dir).unwrap().flatten() {
        println!("cargo:rerun-if-changed={}", entry.path().display());
    }
    println!("cargo:rerun-if-changed=tests/routines/reader_test.z80");
    println!("cargo:rerun-if-changed=tests/routines/ramplan.z80");
    let src = Path::new("tests/routines/reader_test.z80");
    let args = ["-I", "../../kernel", "-I", "../..", "-L"];
    for (name, defines) in [
        ("reader_test", &[][..]),
        ("reader_kernel_test", &["WITH_KERNEL"][..]),
        (
            "reader_moved",
            &["LIB_STATE=$B000", "LIB_BOUNCE=$D100", "LIB_SPARE_PAGE=5"][..],
        ),
        (
            "reader_straddle",
            &["LIB_BOUNCE=$FE80", "LIB_BOUNCE_ANYWHERE"][..],
        ),
    ] {
        spasm(src, &out.join(format!("{name}.bin")), defines, &args);
    }
    spasm(
        Path::new("tests/routines/ramplan.z80"),
        &out.join("ramplan.bin"),
        &[],
        &["-I", "../../app/src", "-I", "../..", "-L"],
    );
    let regions = "LIB_BOUNCE must lie inside the kernel regions";
    for (defines, refusal) in [
        (&["LIB_BOUNCE=$C500"][..], None),
        (&["LIB_BOUNCE=$D600"][..], None),
        (&["LIB_BOUNCE=$DF00"][..], None),
        (&["LIB_BOUNCE=$FE80", "LIB_BOUNCE_ANYWHERE"][..], None),
        (
            &["LIB_BOUNCE=$BF00"][..],
            Some("LIB_BOUNCE must be in bank C"),
        ),
        (
            &["LIB_BOUNCE=$FF80"][..],
            Some("LIB_BOUNCE must leave 256 bytes below $10000"),
        ),
        (&["LIB_BOUNCE=$C400"][..], Some(regions)),
        (&["LIB_BOUNCE=$D601"][..], Some(regions)),
        (&["LIB_BOUNCE=$D700"][..], Some(regions)),
        (&["LIB_BOUNCE=$DE80"][..], Some(regions)),
        (&["LIB_BOUNCE=$FE80"][..], Some(regions)),
        (
            &["LIB_STATE=$7F00"][..],
            Some("LIB_STATE must be in bank B"),
        ),
        (
            &["LIB_STATE=$BE00"][..],
            Some("LIB_STATE and its index must end below $C000"),
        ),
        (
            &["LIB_SPARE_PAGE=8"][..],
            Some("LIB_SPARE_PAGE must be a RAM page"),
        ),
    ] {
        match (
            spasm_try(src, &out.join("reader_probe.bin"), defines, &args),
            refusal,
        ) {
            (Ok(()), None) => {}
            (Err(why), Some(want)) if why.contains(want) => {}
            (got, _) => fail(&format!(
                "assembling the reader with {defines:?} gave {got:?}, wanted {}",
                refusal.map_or("success", |want| want)
            )),
        }
    }
}
