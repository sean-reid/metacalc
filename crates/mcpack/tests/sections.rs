use mcpack::library::{
    self, ENTRY_LEN, TRAILER_LEN, TRAILER_MAGIC, TRAILER_VERSION, crc32, decode, pack, pack_kernel,
};
use mcpack::mm::{self, Database, Grammar, StmtId};

const LOGIC: &str = "
$c ( ) -> wff |- setvar A. $.
$v ph ps ch x $.
wph $f wff ph $.
wps $f wff ps $.
wch $f wff ch $.
vx $f setvar x $.
wi $a wff ( ph -> ps ) $.
wal $a wff A. x ph $.
ax-1 $a |- ( ph -> ( ps -> ph ) ) $.
ax-2 $a |- ( ( ph -> ( ps -> ch ) ) -> ( ( ph -> ps ) -> ( ph -> ch ) ) ) $.
${
  min $e |- ph $.
  maj $e |- ( ph -> ps ) $.
  ax-mp $a |- ps $.
$}
${
  $d x ph $.
  ax-5 $a |- ( ph -> A. x ph ) $.
$}
${
  a1i.1 $e |- ph $.
  a1i $p |- ( ps -> ph ) $= wph wps wph wi a1i.1 wph wps ax-1 ax-mp $.
$}
id $p |- ( ph -> ph ) $= wph wph wph wi wph wph wph ax-1 wph wph wph wi ax-1 mpd $.
";

fn db() -> Database {
    mm::parse(LOGIC.as_bytes().to_vec()).expect("the logic parses")
}

fn closure(db: &Database) -> Vec<StmtId> {
    let grammar = Grammar::new(db, &[("|-", "wff")]).expect("the logic has a grammar");
    let roots: Vec<StmtId> = (0..db.stmts.len() as StmtId)
        .filter(|&id| db.stmt(id).kind.is_assertion() && db.stmt(id).label != "id")
        .collect();
    mm::closure(db, &grammar, &roots).expect("the logic closes")
}

fn reseal(bytes: &mut [u8]) {
    let crc = crc32(&bytes[8..]);
    bytes[4..8].copy_from_slice(&crc.to_le_bytes());
}

#[test]
fn a_kernel_only_library_has_no_trailer_and_reports_no_sections() {
    let db = db();
    let bytes = pack_kernel(&db, &closure(&db)).unwrap();
    assert_ne!(&bytes[bytes.len() - 4..], TRAILER_MAGIC);
    assert_eq!(decode(&bytes).unwrap().sections, None);
}

#[test]
fn the_trailer_closes_the_file_and_the_length_and_crc_cover_it() {
    let db = db();
    let kernel = pack_kernel(&db, &closure(&db)).unwrap();
    let bytes = pack(&db, &closure(&db)).unwrap();
    assert_eq!(&bytes[bytes.len() - 4..], TRAILER_MAGIC);
    assert_eq!(bytes[bytes.len() - 5], TRAILER_VERSION);
    let length = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    assert_eq!(length, bytes.len());
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        crc32(&bytes[8..])
    );
    let sections = decode(&bytes).unwrap().sections.expect("a trailer");
    let count = bytes[bytes.len() - TRAILER_LEN] as usize;
    assert_eq!(sections.table.len(), count);
    assert_eq!(&bytes[..4], &kernel[..4]);
    assert_eq!(&bytes[12..kernel.len()], &kernel[12..]);
    library::check(&bytes).expect("the kernel accepts a library with sections");
}

#[test]
fn every_byte_of_the_trailer_and_table_decodes_or_errors() {
    let db = db();
    let bytes = pack(&db, &closure(&db)).unwrap();
    let count = bytes[bytes.len() - TRAILER_LEN] as usize;
    let first = bytes.len() - TRAILER_LEN - count * ENTRY_LEN;
    for at in first..bytes.len() {
        for flip in [0x01u8, 0x80, 0xFF] {
            let mut hostile = bytes.clone();
            hostile[at] ^= flip;
            reseal(&mut hostile);
            let _ = decode(&hostile);
        }
    }
}
