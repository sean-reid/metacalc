//! The checking screen's CRC run: the chunks it counts for libraries at and around the multiples
//! of 16 KB, and ON held through a kernel chunk call, which stops the run and lets a second Run
//! start it again from the first chunk.

use crate::app::*;
use crate::checking_frames::*;
use crate::checkrig::*;
use crate::shellrig::*;
use crate::table;
use mcemu::Machine;

/// The sizes of the libraries the chunk count is checked on, with the chunks of 16,384 bytes each
/// takes: exactly one chunk, one byte over, exactly two, exactly four, one byte over four.
const SIZES: [(usize, usize); 5] = [
    (16_384, 1),
    (16_385, 2),
    (32_768, 2),
    (65_536, 4),
    (65_537, 5),
];

/// The first reading frame of a library of each size shows the model's chunk total, the screen
/// keeps the same total, and the kernel is called for exactly that many chunks, so the count at
/// kn_read is the kernel's own at each side of a multiple of 16 KB.
#[test]
fn the_chunk_count_is_the_kernels_at_each_side_of_a_multiple_of_16_kb() {
    for (size, chunks) in SIZES {
        let library = library_of_size(size);
        assert_eq!(library.len(), size, "the library is built to the size");
        let count = usize::from(table::header_of(&library).1);
        let built = staged(&format!("pages-size-{size}"), &library, &[count]);
        assert_eq!(chunks_of(built), chunks, "{size} bytes: the host's count");
        let mut run = Run::boot(built, None);
        run.m().press_key(F2);
        let (first, values) = run.next_frame();
        assert_eq!(values, None, "{size} bytes: no table yet");
        shows(
            &format!("{size} bytes: the first reading frame"),
            &first,
            &reading_model(built, 0, 0),
        );
        let at = run.shell.app.label("CKS_CHUNKS");
        assert_eq!(
            run.peek(at, 2),
            u16::try_from(chunks).unwrap().to_le_bytes(),
            "{size} bytes: the screen's total"
        );
        run.m().release_key(F2);
        let mut frames = 1;
        while run.next_frame().1.is_none() {
            frames += 1;
            assert!(frames < 20, "{size} bytes: the run never ended");
        }
        assert_eq!(
            frames,
            chunks + 1,
            "{size} bytes: the push, crc_begin and the chunks but the last, whose frame is the steps'"
        );
        assert_eq!(
            run.crc_calls(),
            (1, u16::try_from(chunks).unwrap()),
            "{size} bytes: the kernel read every chunk once"
        );
    }
}

/// ON held through a kernel chunk call, with two chunks read: the call answers CK_PAUSED, the screen
/// pops, no variable is created, and a second Run starts the CRC run again from the first chunk,
/// reads every chunk and finishes the check.
#[test]
fn on_through_a_kernel_chunk_call_pops_creates_nothing_and_a_second_run_reads_from_the_start() {
    let built = padded();
    let chunks = u16::try_from(chunks_of(built)).unwrap();
    assert!(chunks >= 4);
    let n = usize::from(table::header_of(built.library.as_ref().unwrap()).1);
    let verdicts = host_verdicts(built.library.as_ref().unwrap());
    let mut run = Run::boot(built, None);
    let vars = run.vars();
    run.m().press_key(F2);
    for _ in 0..4 {
        run.next_blit();
    }
    run.m().release_key(F2);
    assert_eq!(run.crc_calls(), (1, 2), "two chunks are read");
    run.run_to("far_kernel_crc_chunk", WAIT);
    run.m().press_key(Machine::KEY_ON);
    run.next_blit();
    assert_eq!(run.depth(), 1, "the screen popped");
    assert_eq!(run.top_kind(), KIND_LIBRARIES);
    assert_eq!(run.crc_calls(), (1, 3), "the chunk call was made");
    assert_eq!(run.byte("APP_ON"), 0, "CK_PAUSED stopped it, not APP_ON");
    assert_eq!(run.vars(), vars, "no variable was created");
    run.m().release_key(Machine::KEY_ON);
    run.run(2_000_000);
    assert_eq!((run.depth(), run.vars()), (1, vars), "the app stayed");
    run.m().press_key(F2);
    let values = loop {
        if let (_, Some(values)) = run.next_frame()
            && values.iter().all(|&v| v != 0)
        {
            break values;
        }
    };
    run.m().release_key(F2);
    assert_eq!(
        run.crc_calls(),
        (2, 3 + chunks),
        "the second run began again and read every chunk"
    );
    assert_eq!(values, expected_values(built, &verdicts, n));
    assert_eq!(usize::from(run.verifies()), n);
}
