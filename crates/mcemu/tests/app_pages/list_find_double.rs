//! A letter typed twice in a row: prefixes with a doubled letter through the rig, which waits out the
//! OS's key gap, are the model's; and on the reference OS the same letter pressed again 30 and 50 ms
//! after its release, where the OS drops the second inside about 40 ms.

use crate::list_find_rig::*;
use crate::list_real::PROP;
use crate::list_rig::*;
use crate::shellrig::*;

/// The prefixes, up to the cap, of labels that have two equal characters in a row, each ending at the
/// second of the pair or past it.
fn doubled(labels: &[String]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for label in labels {
        let chars: Vec<char> = label.chars().take(8).collect();
        if let Some(at) = chars.windows(2).position(|w| w[0] == w[1]) {
            let prefix: String = chars[..at + 2].iter().collect();
            if !found.contains(&prefix) {
                found.push(prefix);
            }
        }
    }
    found
}

#[test]
fn prefixes_with_a_doubled_letter_typed_through_the_rig_are_the_models() {
    let built = proofs_three();
    let labels = labels_of(built);
    let prefixes = doubled(&labels);
    println!(
        "{} prefixes with a doubled letter, {:?}",
        prefixes.len(),
        &prefixes[..prefixes.len().min(6)]
    );
    assert!(prefixes.len() >= 3, "{} prefixes", prefixes.len());
    let mut list = Lister::open(built, None, 1);
    list.settle();
    let (mut hit, mut miss) = (0, 0);
    for prefix in &prefixes {
        list.f2();
        list.type_text(prefix);
        if list.find().missed {
            miss += 1;
        } else {
            hit += 1;
        }
        list.skip_the_wait();
        list.settle();
    }
    println!("doubled letters: {hit} hit, {miss} missed");
    assert!(hit >= 1 && miss + hit == prefixes.len());
}

/// T-states of 6 MHz in `ms` milliseconds.
const fn ms(n: u32) -> u32 {
    n * 6_000
}

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_the_same_letter_pressed_again_after_30_and_50_ms_is_dropped_and_taken() {
    let mut taken = Vec::new();
    for gap in [30, 50] {
        let mut list = Lister::on_rom(PROP);
        list.open_on_rom();
        let label = labels_of(list.built)[list.items()[0]].clone();
        let c = label.chars().next().unwrap();
        let scan = scans_for(c)[0];
        list.tap(scan);
        list.run.run(ms(gap));
        let keys = list.run.scr_keys();
        list.run.allow_raw_press();
        list.run.m().press_key(scan);
        list.run.run(ms(60));
        list.run.release(scan);
        list.run.run(ms(400));
        let frame = list.run.frames().last().cloned().unwrap();
        let len = usize::from(frame[7] & 0x0F);
        println!(
            "{gap} ms after the release: byte 7 {:02X}, characters {:?}, the app took {} more key(s)",
            frame[7],
            String::from_utf8_lossy(&frame[8..8 + len.min(8)]),
            list.run.scr_keys() - keys
        );
        assert!(len >= 1, "the first press was taken");
        assert_eq!(frame[8], c as u8);
        taken.push((gap, len, list.run.scr_keys() - keys));
    }
    assert_eq!(
        taken,
        [(30, 1, 0), (50, 2, 1)],
        "dropped at 30 ms, taken at 50 ms"
    );
}
