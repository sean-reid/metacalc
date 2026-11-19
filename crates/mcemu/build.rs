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
    assemble_layout_probes(&out);
    assemble_probe_ram_probes(&out);
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
    match spasm_try(
        &Path::new("tests/routines").join("kerncheck_test.z80"),
        &out.join("kerncheck_probe.bin"),
        &["KD_SCRATCH_SIZE=35"],
        &[
            "-I",
            "../../app/src",
            "-I",
            "../../probe",
            "-I",
            "../../kernel",
        ],
    ) {
        Err(why)
            if why.contains(
                "the driver's scratch overlaps, leaves a gap, or runs past KD_SCRATCH_SIZE",
            ) => {}
        other => fail(&format!(
            "a driver scratch too small for its variables gave {other:?}"
        )),
    }
    let font_dir = out.join("ui_font");
    spasm(
        &Path::new("tests/routines").join("kerncheck_test.z80"),
        &out.join("kerncheck_ui.bin"),
        &["WITH_UI"],
        &[
            "-I",
            "../../app/src",
            "-I",
            "../../probe",
            "-I",
            "../../kernel",
            "-I",
            "../../ui",
            "-I",
            "../..",
            "-I",
            font_dir.to_str().unwrap(),
            "-L",
        ],
    );
    spasm(
        Path::new("../../probe/ramprobe.z80"),
        &out.join("ramprobe.bin"),
        &[],
        &["-I", "../../app/src", "-I", "../../probe", "-L"],
    );
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
    println!("cargo:rerun-if-changed=../../probe/storeram.inc");
    println!("cargo:rerun-if-changed=../../probe/sparecore.inc");
    println!("cargo:rerun-if-changed=../../probe/sparescreen.inc");
    println!("cargo:rerun-if-changed=../../probe/printnum.inc");
    println!("cargo:rerun-if-changed=../../probe/kerndriver.inc");
    println!("cargo:rerun-if-changed=../../probe/onbreak.inc");
    println!("cargo:rerun-if-changed=../../app/src/libpage.inc");
    println!("cargo:rerun-if-changed=../../app/src/equates.inc");
    println!("cargo:rerun-if-changed=../../app/src/header.inc");
    assemble_key_apps(&out);
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
        &["-I", include, "-I", "../../app/src"],
    );
    let assemble = |defines: &[&str]| {
        spasm_try(
            &dir.join("kernel.z80"),
            &out.join("kernel_base_probe.bin"),
            defines,
            &["-I", include, "-I", "../../app/src"],
        )
    };
    let overlap = "KERNEL_RAM_LOW and KERNEL_RAM_HIGH overlap";
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
            Some("KERNEL_RAM_LOW runs past bank C"),
        ),
        (
            &["KERNEL_RAM_LOW=$C080"][..],
            Some("KERNEL_RAM_LOW must be a multiple of $100"),
        ),
        (
            &["KERNEL_RAM_LOW=$EF00"][..],
            Some("KERNEL_RAM_LOW runs past bank C"),
        ),
        (
            &["KERNEL_RAM_HIGH=$BF00"][..],
            Some("KERNEL_RAM_HIGH runs past bank C"),
        ),
        (
            &["KERNEL_RAM_HIGH=$E100"][..],
            Some("KERNEL_RAM_HIGH runs past bank C"),
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
        (&["CACHE_IDS=$2100"][..], None),
        (
            &["CACHE_IDS=$2080"][..],
            Some("CACHE_IDS must be a multiple of $100"),
        ),
        (
            &["CACHE_IDS=$1FFF"][..],
            Some("CACHE_IDS must be a multiple of $100"),
        ),
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
            &["-I", include, "-I", "../../app/src", "-L"],
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
    println!("cargo:rerun-if-changed=../../app/src/ram_map.inc");
    let font_dir = out.join("ui_font");
    fs::create_dir_all(&font_dir).unwrap();
    fs::write(font_dir.join("font.inc"), mcglyph::z80::fonts_include()).unwrap();
    let include = dir.to_str().unwrap();
    let assemble = |defines: &[&str], output: &Path| {
        spasm_try(
            &dir.join("ui.z80"),
            output,
            defines,
            &[
                "-I",
                include,
                "-I",
                font_dir.to_str().unwrap(),
                "-I",
                "../../app/src",
                "-I",
                "../..",
                "-L",
            ],
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
    for (defines, want) in [
        (&["UI_VARS=$9326", "FM_SCRATCH=$A000"][..], None),
        (&["UI_VARS=$9640"][..], None),
        (&["UI_FRAME=$9572"][..], None),
        (&["UI_FRAME=$9B71"][..], None),
        (&["KD_SCRATCH=$9640"][..], None),
        (&["LIB_STATE=$9896"][..], None),
        (&["KERNEL_RAM_HIGH=$D700"][..], None),
        (&["UI_VARS=$9340"][..], Some("UI_FRAME and UI_VARS overlap")),
        (
            &["UI_VARS=$9880"][..],
            Some("UI_VARS and KD_SCRATCH overlap"),
        ),
        (
            &["UI_VARS=$9B60"][..],
            Some("UI_VARS and LIB_STATE overlap"),
        ),
        (
            &["UI_FRAME=$9860"][..],
            Some("UI_FRAME and KD_SCRATCH overlap"),
        ),
        (
            &["UI_FRAME=$9B00"][..],
            Some("UI_FRAME and LIB_STATE overlap"),
        ),
        (
            &["UI_FRAME=$9B70"][..],
            Some("UI_FRAME and LIB_STATE overlap"),
        ),
        (
            &["LIB_STATE=$9890"][..],
            Some("KD_SCRATCH and LIB_STATE overlap"),
        ),
        (&["UI_VARS=$BFF0"][..], Some("UI_VARS runs past bank B")),
        (&["UI_VARS=$7FF0"][..], Some("UI_VARS runs past bank B")),
        (
            &["KERNEL_RAM_HIGH=$C000"][..],
            Some("KERNEL_RAM_LOW and KERNEL_RAM_HIGH overlap"),
        ),
        (
            &["KERNEL_RAM_LOW=$F000"][..],
            Some("KERNEL_RAM_LOW runs past bank C"),
        ),
        (&["UI_FRAME=$BF00"][..], Some("UI_FRAME runs past bank B")),
        (&["UI_FRAME=$7F00"][..], Some("UI_FRAME runs past bank B")),
        (
            &["KD_SCRATCH=$BFF0"][..],
            Some("KD_SCRATCH runs past bank B"),
        ),
        (
            &["KD_SCRATCH=$7FF0"][..],
            Some("KD_SCRATCH runs past bank B"),
        ),
        (&["LIB_STATE=$BF00"][..], Some("LIB_STATE runs past bank B")),
        (&["LIB_STATE=$7F00"][..], Some("LIB_STATE runs past bank B")),
        (
            &["UI_FRAME_SIZE=$2FF"][..],
            Some("UI_FRAME_SIZE in ram_map.inc must be UI_STRIDE*UI_HEIGHT"),
        ),
        (&["FM_LOW=$C700", "FM_LOW_SIZE=$1000"][..], None),
        (&["FM_LOW_SIZE=$1000"][..], None),
        (
            &["FM_LOW=$C500"][..],
            Some("FM_LOW must not start below KERNEL_RAM_LOW + $100"),
        ),
        (
            &["FM_LOW=$C700"][..],
            Some("FM_LOW runs outside the kernel's low region"),
        ),
        (
            &["FM_LOW_SIZE=$1101"][..],
            Some("FM_LOW runs outside the kernel's low region"),
        ),
        (&["FM_SCRATCH=$9100"][..], None),
        (&["FM_SCRATCH=$8000"][..], None),
        (&["FM_SCRATCH=$9240"][..], None),
        (&["FM_SCRATCH=$BF00"][..], None),
        (&["FM_RAM_SIZE=$1F00"][..], None),
        (&["FM_RAM=$E000"][..], None),
        (
            &["FM_SCRATCH=$86F0"][..],
            Some("FM_SCRATCH and UI_VARS overlap"),
        ),
        (
            &["FM_SCRATCH=$9300"][..],
            Some("FM_SCRATCH and UI_FRAME overlap"),
        ),
        (
            &["FM_SCRATCH=$9800"][..],
            Some("FM_SCRATCH and KD_SCRATCH overlap"),
        ),
        (
            &["FM_SCRATCH=$9900"][..],
            Some("FM_SCRATCH and LIB_STATE overlap"),
        ),
        (
            &["FM_SCRATCH=$BF80"][..],
            Some("FM_SCRATCH runs past bank B"),
        ),
        (
            &["FM_SCRATCH=$7F80"][..],
            Some("FM_SCRATCH runs past bank B"),
        ),
        (
            &["FM_RAM_SIZE=$2000"][..],
            Some("FM_RAM and LIB_BOUNCE overlap"),
        ),
        (&["FM_RAM=$FE00"][..], Some("FM_RAM and LIB_BOUNCE overlap")),
        (
            &["FM_RAM=$DE00"][..],
            Some("FM_RAM runs outside the kernel's high region"),
        ),
        (
            &["FM_RAM=$EA00"][..],
            Some("FM_RAM runs outside the kernel's high region"),
        ),
        (&["UI_VARS_SIZE=$30"][..], None),
        (
            &["UI_VARS_SIZE=$19"][..],
            Some("the UI variables overlap, leave a gap, or run past UI_VARS_SIZE"),
        ),
    ] {
        match (assemble(defines, &out.join("ui_probe.bin")), want) {
            (Ok(()), None) => {}
            (Err(why), Some(want)) if why.contains(want) => {}
            (got, _) => fail(&format!(
                "the RAM map with {defines:?} gave {got:?}, wanted {}",
                want.unwrap_or("success")
            )),
        }
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
    let font_dir = out.join("ui_font");
    let args = [
        "-I",
        "../../kernel",
        "-I",
        "../../app/src",
        "-I",
        "../..",
        "-I",
        font_dir.to_str().unwrap(),
        "-L",
    ];
    let foreign = format!(
        "LIB_FONT_FINGERPRINT=${:04X}",
        mcglyph::z80::Table::builtin().fingerprint() ^ 1
    );
    for (name, defines) in [
        ("reader_test", &[][..]),
        ("reader_foreign", &[foreign.as_str()][..]),
        ("reader_kernel_test", &["WITH_KERNEL"][..]),
        (
            "reader_moved",
            &[
                "LIB_STATE=$B000",
                "LIB_BOUNCE=$F800",
                "LIB_SPARE_PAGE=5",
                "FM_RAM_SIZE=$1000",
            ][..],
        ),
        (
            "reader_straddle",
            &["LIB_BOUNCE=$FE80", "LIB_BOUNCE_ANYWHERE"][..],
        ),
    ] {
        spasm(src, &out.join(format!("{name}.bin")), defines, &args);
    }
    let mut fm_args = args.to_vec();
    fm_args.extend(["-I", "../../ui"]);
    for entry in fs::read_dir("../../fm").unwrap().flatten() {
        println!("cargo:rerun-if-changed={}", entry.path().display());
    }
    spasm(
        src,
        &out.join("fm_test.bin"),
        &["WITH_FM", "IMAGE_ORG=$2000"],
        &fm_args,
    );
    spasm(
        src,
        &out.join("fm_short.bin"),
        &["WITH_FM", "IMAGE_ORG=$2000", "FM_RUN_MAX=8"],
        &fm_args,
    );
    for (defines, refusal) in [
        (&["FM_RUN_MAX=1"][..], None),
        (&["FM_RUN_MAX=127"][..], None),
        (&["FM_RUN_MAX=0"][..], Some("FM_RUN_MAX must be 1 to 128")),
        (&["FM_RUN_MAX=129"][..], Some("FM_RUN_MAX must be 1 to 128")),
        (
            &["FM_LOW=$C700", "FM_LOW_SIZE=$D00", "FM_LOW_FREE=0"][..],
            None,
        ),
        (
            &["FM_LOW=$C800", "FM_LOW_SIZE=$D00", "FM_LOW_FREE=0"][..],
            Some("the statement headers must stay off the kernel's stack page"),
        ),
        (&["FM_RAM_SIZE=$1E00"][..], None),
        (&["FM_VARS_SIZE=$100"][..], None),
        (&["FM_RAM_SIZE=$1F00"][..], None),
        (&["FM_RAM=$E000"][..], None),
        (&["FM_LOW_SIZE=$1080"][..], None),
        (
            &["FM_LOW_SIZE=$107F"][..],
            Some("fm must leave $400 bytes of FM_LOW unused"),
        ),
        (
            &["FM_LOW=$C800", "FM_LOW_SIZE=$F00"][..],
            Some("the statement headers must stay off the kernel's stack page"),
        ),
        (
            &["FM_LOW=$C7F0", "FM_LOW_SIZE=$F00"][..],
            Some("FM_LOW must be a multiple of $100"),
        ),
        (
            &["FM_LOW_SIZE=$C7F"][..],
            Some("FM_LOW_SIZE in ram_map.inc is smaller than what fm lays out"),
        ),
        (
            &["FM_LOW=$C680", "FM_LOW_SIZE=$1000"][..],
            Some("FM_LOW must be a multiple of $100"),
        ),
        (
            &["FM_RAM_SIZE=$1DFF"][..],
            Some("FM_RAM_SIZE in ram_map.inc is smaller than what fm lays out"),
        ),
        (
            &["FM_RAM=$DF80"][..],
            Some("FM_RAM must be a multiple of $100"),
        ),
        (
            &["FM_VARS_SIZE=$101"][..],
            Some("FM_VARS_SIZE must leave the stack at FM_RAM + $100 alone"),
        ),
    ] {
        let mut defines = defines.to_vec();
        defines.extend(["WITH_FM", "IMAGE_ORG=$2000"]);
        expect(src, &out.join("fm_probe.bin"), &defines, &fm_args, refusal);
    }
    spasm(
        Path::new("tests/routines/ramplan.z80"),
        &out.join("ramplan.bin"),
        &[],
        &["-I", "../../app/src", "-I", "../..", "-L"],
    );
    let regions = "LIB_BOUNCE must lie inside the kernel's saved symbols buffer";
    for (defines, refusal) in [
        (&[][..], None),
        (&["LIB_BOUNCE=$F300", "FM_RAM_SIZE=$1400"][..], None),
        (&["LIB_BOUNCE=$F800", "FM_RAM_SIZE=$1400"][..], None),
        (&["LIB_BOUNCE=$C500"][..], Some(regions)),
        (&["LIB_BOUNCE=$D600"][..], Some(regions)),
        (&["LIB_BOUNCE=$DF00"][..], Some(regions)),
        (&["LIB_BOUNCE=$F200"][..], Some(regions)),
        (&["KERNEL_RAM_LOW=$EE00", "KERNEL_RAM_HIGH=$C100"][..], None),
        (
            &[
                "KERNEL_RAM_LOW=$EE00",
                "KERNEL_RAM_HIGH=$C100",
                "LIB_BOUNCE=$E100",
            ][..],
            Some(regions),
        ),
        (
            &["LIB_STATE_SIZE=$2D4"][..],
            Some("the reader's state overlaps, leaves a gap, or runs past LIB_STATE_SIZE"),
        ),
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
        (
            &["FONT_AFTER_LIB"][..],
            Some("UI_FONT_FINGERPRINT must be defined before lib/lib.z80"),
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

/// Assembles `source` with `defines` and requires success for `None`, or the message for `Some`.
fn expect(source: &Path, out: &Path, defines: &[&str], args: &[&str], want: Option<&str>) {
    match (spasm_try(source, out, defines, args), want) {
        (Ok(()), None) => {}
        (Err(why), Some(want)) if why.contains(want) => {}
        (got, _) => fail(&format!(
            "assembling {} with {defines:?} gave {got:?}, wanted {}",
            source.display(),
            want.unwrap_or("success")
        )),
    }
}

/// Each probe refuses RAM of its own that leaves bank B, overlaps another of its regions or overlaps
/// a map region its image links, and accepts the default addresses and a move clear of them. Each
/// size is pinned by the highest base that assembles and the next one up that does not. The checks
/// sit at the end of the image, so the canvas, fm, the reader and the kernel driver are each
/// included before the probe's regions and between its regions and its checks.
fn assemble_probe_ram_probes(out: &Path) {
    for f in ["sparecheck.z80", "ramcheck.inc", "storeguard.inc"] {
        println!("cargo:rerun-if-changed=../../probe/{f}");
    }
    let args = [
        "-I",
        "../../app/src",
        "-I",
        "../../probe",
        "-I",
        "../../kernel",
        "-I",
        "../../ui",
        "-I",
        "../..",
    ];
    let bank = |probe: &str| format!("{probe}'s RAM runs past bank B");
    let parts = [
        ("ui/ram.inc", "RAM overlaps UI_VARS", "S_PORT5=$86F0"),
        ("ui/ram.inc", "RAM overlaps UI_FRAME", "S_PORT5=$9340"),
        ("fm/ram.inc", "RAM overlaps FM_SCRATCH", "S_PORT5=$8710"),
        ("lib/ram.inc", "RAM overlaps LIB_STATE", "S_PORT5=$98A0"),
    ];
    for (probe, top, size) in [("ramprobe", "$BFE8", 24), ("sparecheck", "$BFED", 19)] {
        let source = Path::new("../../probe").join(format!("{probe}.z80"));
        let bin = out.join("probe_ram_probe.bin");
        let ok = |d: &[&str]| expect(&source, &bin, d, &args, None);
        let fails = |d: &[&str], want: &str| expect(&source, &bin, d, &args, Some(want));
        ok(&[]);
        ok(&["S_PORT5=$8000"]);
        ok(&[&format!("S_PORT5={top}")]);
        let past = format!("S_PORT5=${:X}", 0xC000 - size + 1);
        fails(&[&past], &bank(probe));
        fails(&["S_PORT5=$7FFF"], &bank(probe));
        let wrapper = out.join(format!("{probe}_parts_probe.z80"));
        for (part, want, define) in parts {
            fs::write(
                &wrapper,
                format!("#include \"{part}\"\n#include \"{probe}.z80\"\n"),
            )
            .unwrap();
            expect(&wrapper, &bin, &[], &args, None);
            expect(
                &wrapper,
                &bin,
                &[define],
                &args,
                Some(&format!("the probe's {want}")),
            );
        }
        fs::write(&wrapper, format!("#include \"{probe}.z80\"\n")).unwrap();
        expect(
            &wrapper,
            &bin,
            &["KD_NEXT"],
            &args,
            Some("the probe's RAM overlaps KD_SCRATCH"),
        );
        expect(&wrapper, &bin, &["KD_NEXT", "S_PORT5=$9B80"], &args, None);
    }

    let driver = fs::read_to_string("tests/routines/kerncheck_test.z80").unwrap();
    let driver_bare = driver.replacen("#include \"equates.inc\"\n", "", 1);
    let equates = "#include \"equates.inc\"\n";
    let store = "#include \"storeram.inc\"\n";
    let guard = "#include \"storeguard.inc\"\n";
    let ui = "RAM overlaps UI_VARS";
    let frame = "RAM overlaps UI_FRAME";
    let fm = "RAM overlaps FM_SCRATCH";
    let reader = "RAM overlaps LIB_STATE";
    let kd = "RAM overlaps KD_SCRATCH";
    type Case<'a> = (Vec<&'a str>, Option<&'a str>);
    let images: Vec<(&str, String, String, Vec<Case>)> = vec![
        (
            "canvas",
            format!("{equates}#include \"ui/ram.inc\"\n{store}{guard}"),
            format!("{equates}{store}#include \"ui/ram.inc\"\n{guard}"),
            vec![
                (vec!["SCRIPT=$8C00"], None),
                (vec![], Some(ui)),
                (vec!["SCRIPT=$8C00", "STATE=$86F0"], Some(ui)),
                (vec!["SCRIPT=$8C00", "LOG=$86F0"], Some(ui)),
                (vec!["SCRIPT=$9340"], Some(frame)),
                (vec!["SCRIPT=$8C00", "STATE=$9340"], Some(frame)),
                (vec!["SCRIPT=$8C00", "LOG=$9340"], Some(frame)),
            ],
        ),
        (
            "fm",
            format!("{equates}#include \"fm/ram.inc\"\n{store}{guard}"),
            format!("{equates}{store}#include \"fm/ram.inc\"\n{guard}"),
            vec![
                (vec!["SCRIPT=$8C00"], None),
                (vec![], Some(fm)),
                (vec!["SCRIPT=$8C00", "STATE=$8710"], Some(fm)),
                (vec!["SCRIPT=$8C00", "LOG=$8710"], Some(fm)),
            ],
        ),
        (
            "reader",
            format!("{equates}#include \"lib/ram.inc\"\n{store}{guard}"),
            format!("{equates}{store}#include \"lib/ram.inc\"\n{guard}"),
            vec![
                (vec!["LOG=$9B80"], None),
                (vec![], Some(reader)),
                (vec!["LOG=$9B80", "STATE=$98A0"], Some(reader)),
                (vec!["LOG=$9B80", "SCRIPT=$98A0"], Some(reader)),
            ],
        ),
        (
            "driver",
            format!("{driver}{store}{guard}"),
            format!("{equates}{store}{driver_bare}{guard}"),
            vec![
                (vec!["LOG=$9B80"], None),
                (vec![], Some(kd)),
                (vec!["LOG=$9B80", "STATE=$9880"], Some(kd)),
                (vec!["LOG=$9B80", "SCRIPT=$9880"], Some(kd)),
            ],
        ),
    ];
    for (name, before, after, cases) in &images {
        let bin = out.join(format!("store_{name}_probe.bin"));
        for (order, text) in [("before", before), ("after", after)] {
            let source = out.join(format!("store_{name}_{order}_probe.z80"));
            fs::write(&source, text).unwrap();
            for (defines, want) in cases {
                expect(&source, &bin, defines, &args, *want);
            }
        }
    }

    let real = Path::new("../../probe/storecheck.z80");
    let real_bin = out.join("store_real_probe.bin");
    expect(real, &real_bin, &[], &args, None);
    expect(
        real,
        &real_bin,
        &["STATE=$9340"],
        &args,
        Some("storecheck's RAM overlaps UI_FRAME"),
    );

    let source = out.join("store_alone_probe.z80");
    fs::write(&source, format!("{equates}{store}{guard}")).unwrap();
    let bin = out.join("store_alone_probe.bin");
    let past = |what: &str| format!("storecheck's {what} runs past bank B");
    let top = |size: u32| format!("${:X}", 0xC000 - size);
    let over = |size: u32| format!("${:X}", 0xC000 - size + 1);
    for (name, size) in [("SCRIPT", 640), ("STATE", 47), ("LOG", 709)] {
        let ok = [format!("{name}={}", top(size)), format!("{name}=$8000")];
        let bad = [format!("{name}={}", over(size)), format!("{name}=$7FFF")];
        for d in &ok {
            expect(&source, &bin, &[d], &args, None);
        }
        for d in &bad {
            expect(&source, &bin, &[d], &args, Some(&past(name)));
        }
        expect(
            &source,
            &bin,
            &[&format!("{name}=$9340")],
            &args,
            Some("storecheck's RAM overlaps UI_FRAME"),
        );
    }
    expect(&source, &bin, &[], &args, None);
    for (defines, want) in [
        (
            &["STATE=$8700"][..],
            "storecheck's SCRIPT and STATE overlap",
        ),
        (&["LOG=$8700"][..], "storecheck's SCRIPT and LOG overlap"),
        (&["STATE=$9880"][..], "storecheck's STATE and LOG overlap"),
    ] {
        expect(&source, &bin, defines, &args, Some(want));
    }
}

/// Each layout refuses a variable at an offset already taken, inside another, past the region and
/// after an undeclared gap, and accepts one that follows the last or follows declared padding; an
/// image that includes equates.inc refuses a map that disagrees with the OS names.
fn assemble_layout_probes(out: &Path) {
    println!("cargo:rerun-if-changed=../../app/src/equates.inc");
    let probe_args = [
        "-I",
        "../../app/src",
        "-I",
        "../../probe",
        "-I",
        "../../kernel",
    ];
    let driver = Path::new("tests/routines/kerncheck_test.z80");
    for (defines, want) in [
        (
            &["UI_FRAME=$9400"][..],
            "UI_FRAME in ram_map.inc must be plotSScreen",
        ),
        (
            &["KD_SCRATCH=$9880"][..],
            "KD_SCRATCH in ram_map.inc must be appBackUpScreen",
        ),
    ] {
        expect(
            driver,
            &out.join("equates_probe.bin"),
            defines,
            &probe_args,
            Some(want),
        );
    }

    let font_dir = out.join("ui_font");
    let font = font_dir.to_str().unwrap();
    let ui_args = [
        "-I",
        "../../ui",
        "-I",
        "../..",
        "-I",
        "../../app/src",
        "-I",
        font,
    ];
    let reader_args = [
        "-I",
        "../../kernel",
        "-I",
        "../../app/src",
        "-I",
        "../..",
        "-I",
        font,
    ];
    let reader = fs::read_to_string("tests/routines/reader_test.z80").unwrap();
    let driver_source = fs::read_to_string(driver).unwrap();
    for (prefix, base, args, size, end, message) in [
        (
            "UI",
            "#include \"ui/ram.inc\"\n".to_string(),
            &ui_args[..],
            "UI_VARS_SIZE=$30",
            0x1A,
            "the UI variables overlap, leave a gap, or run past UI_VARS_SIZE",
        ),
        (
            "FM",
            "#include \"fm/ram.inc\"\n".to_string(),
            &ui_args[..],
            "FM_VARS_SIZE=$6A",
            0x67,
            "the fm variables overlap, leave a gap, or run past FM_VARS_SIZE",
        ),
        (
            "KD",
            driver_source,
            &probe_args[..],
            "KD_SCRATCH_SIZE=40",
            36,
            "the driver's scratch overlaps, leaves a gap, or runs past KD_SCRATCH_SIZE",
        ),
        (
            "LR",
            reader,
            &reader_args[..],
            "LIB_STATE_SIZE=$2E0",
            0x2D5,
            "the reader's state overlaps, leaves a gap, or runs past LIB_STATE_SIZE",
        ),
    ] {
        let source = out.join(format!("{prefix}_layout_probe.z80"));
        let go_back = format!("{prefix}_AT must not go back");
        let go_past = format!("{prefix}_AT must not go back or past");
        for (lines, defines, want) in [
            (
                format!("{prefix}_VAR(probe_next, ${end:X}, 1)"),
                vec![size],
                None,
            ),
            (
                format!(
                    "{prefix}_AT(${:X})\n{prefix}_VAR(probe_next, ${:X}, 1)",
                    end + 2,
                    end + 2
                ),
                vec![size],
                None,
            ),
            (
                format!("{prefix}_VAR(probe_same, ${:X}, 1)", end - 1),
                vec![size],
                Some(message),
            ),
            (
                format!("{prefix}_VAR(probe_inside, $1, 1)"),
                vec![size],
                Some(message),
            ),
            (
                format!("{prefix}_VAR(probe_gap, ${:X}, 1)", end + 2),
                vec![size],
                Some(message),
            ),
            (
                format!("{prefix}_VAR(probe_past, ${end:X}, 1)"),
                vec![],
                Some(message),
            ),
            (
                format!("{prefix}_AT($0)"),
                vec![size],
                Some(go_back.as_str()),
            ),
            (
                format!("{prefix}_AT(${:X})", end + 1),
                vec![],
                Some(go_past.as_str()),
            ),
            (format!("{prefix}_AT(${end:X})"), vec![], None),
        ] {
            fs::write(&source, format!("{base}\n{lines}\n")).unwrap();
            expect(&source, &out.join("layout_probe.bin"), &defines, args, want);
        }
    }
}

/// The apps that wait for keys: one logging what the OS's `_GetKey` returns, one running
/// `app_key` over the canvas.
fn assemble_key_apps(out: &Path) {
    println!("cargo:rerun-if-changed=../../app/src/keys.inc");
    let font_dir = out.join("ui_font");
    let include = [
        "-I",
        "../../app/src",
        "-I",
        "../../ui",
        "-I",
        "../..",
        "-I",
        font_dir.to_str().unwrap(),
        "-L",
    ];
    for (src, name) in [("get_key_log", "get_key_log"), ("keys_app", "keys_app")] {
        spasm(
            &Path::new("tests/routines").join(format!("{src}.z80")),
            &out.join(format!("{name}.8xk")),
            &[],
            &include,
        );
    }
}
