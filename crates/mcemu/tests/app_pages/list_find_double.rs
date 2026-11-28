//! A letter typed twice in a row: prefixes with a doubled letter, typed through the rig, which waits
//! out the OS's key gap, are the model's on the stub and, ignored, on the reference OS.

use crate::list_find_arms::single;
use crate::list_find_rig::*;
use crate::list_real::PROP;
use crate::list_rig::*;

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

#[test]
#[ignore = "needs the reference ROM image and the built libraries: MC_LIBRARIES=<dir holding prop.mclib, pred.mclib, ring.mclib>"]
fn on_the_reference_os_a_doubled_letter_typed_through_the_rig_puts_both_letters_in_the_frame() {
    let mut list = Lister::on_rom(PROP);
    list.open_on_rom();
    let prefixes: Vec<String> = doubled(&labels_of(list.built))
        .into_iter()
        .filter(|p| p.chars().all(single))
        .collect();
    assert!(prefixes.len() >= 3, "{} prefixes", prefixes.len());
    println!("{} prefixes to type, {:?}", prefixes.len(), &prefixes[..3]);
    for prefix in &prefixes[..3] {
        list.f2();
        for (n, c) in prefix.chars().enumerate() {
            assert!(list.press_char(c), "{prefix:?}: {c:?} drew a frame");
            list.same_find(&format!("{prefix:?}, {n} typed"));
        }
        let frame = list.run.frames().last().cloned().unwrap();
        let len = usize::from(frame[7] & 0x0F);
        assert_eq!(len, prefix.len(), "{prefix:?}: byte 7 is {:02X}", frame[7]);
        assert_eq!(
            &frame[8..8 + len],
            prefix.as_bytes(),
            "{prefix:?}: the characters"
        );
        list.skip_the_wait();
        list.settle();
    }
}
