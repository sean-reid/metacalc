//! The whole send against the scripted calculator in `common`, which is written from the same
//! documents as the sender and so cannot catch a misreading of them. Time is virtual: the peer
//! adds to a clock instead of sleeping.

mod common;

use common::{Calc, PAGE, Peer, Signature, build_8xk, expected_image, sample_pages};
use mcsend::error::Error;
use mcsend::progress::Progress;
use mcsend::session::Session;
use mcsend::tifile::App;
use mcsend::transfer::transfer;
use std::time::Duration;

struct Outcome {
    result: Result<(), Error>,
    peer: Peer,
    stdout: String,
    progress: String,
}

fn run(cfg: Calc, timeout: u64, pages: &[Vec<u8>]) -> Outcome {
    let file = build_8xk("metacalc", pages, 0x73, 0x24, Signature::Valid);
    let app = App::parse(&file).unwrap();
    let mut peer = Peer::new(cfg);
    let mut out = Vec::new();
    let mut shown = Vec::new();
    let result = {
        let mut session = Session::new(&mut peer, Duration::from_secs(timeout));
        let mut progress = Progress::new(&mut shown, false, app.page_count());
        transfer(&mut session, &app, &mut progress, &mut out)
    };
    Outcome {
        result,
        peer,
        stdout: String::from_utf8(out).unwrap(),
        progress: String::from_utf8(shown).unwrap(),
    }
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

const SEQUENCE: [&str; 11] = [
    "negotiate",
    "ping",
    "params",
    "negotiate",
    "ping",
    "dir",
    "negotiate",
    "ping",
    "rts",
    "content",
    "eot",
];

#[test]
fn a_three_page_app_is_checked_listed_and_sent_whole() {
    let pages = sample_pages(3, 5000);
    let o = run(Calc::default(), 120, &pages);
    o.result.unwrap();
    assert_eq!(o.peer.image.unwrap(), expected_image(&pages));
    assert_eq!(o.peer.events, SEQUENCE);
    assert_eq!(o.peer.rts_name.as_deref(), Some("metacalc"));
    assert_eq!(
        o.stdout,
        "found TI-84 Plus Silver Edition, 1536 KB flash free\n"
    );
    assert_eq!(
        o.progress,
        "page 1 of 3 sent\npage 2 of 3 sent\npage 3 of 3 sent\n"
    );
}

#[test]
fn the_calculator_may_answer_the_end_of_transmission_with_its_verdict() {
    let ok = run(
        Calc {
            verdict: Some(1),
            ..Calc::default()
        },
        120,
        &sample_pages(1, 100),
    );
    ok.result.unwrap();
    let bad = run(
        Calc {
            verdict: Some(0),
            ..Calc::default()
        },
        120,
        &sample_pages(1, 100),
    );
    assert!(matches!(bad.result, Err(Error::Rejected)));
}

#[test]
fn delay_packets_are_waited_out_instead_of_timing_out() {
    let cfg = Calc {
        delays: vec![(200_000, 190); 3],
        ..Calc::default()
    };
    let o = run(cfg, 120, &sample_pages(2, 100));
    o.result.unwrap();
    assert!(o.peer.clock >= secs(570), "{:?}", o.peer.clock);
    assert_eq!(o.peer.waits.iter().filter(|&&w| w == secs(320)).count(), 3);
}

#[test]
fn silence_past_the_timeout_fails_with_the_timeout() {
    let cfg = Calc {
        silent_after_content: Some(secs(500)),
        ..Calc::default()
    };
    let o = run(cfg, 120, &sample_pages(1, 100));
    let err = o.result.unwrap_err();
    assert!(matches!(err, Error::Timeout(d) if d == secs(120)), "{err}");
    assert_eq!(*o.peer.waits.last().unwrap(), secs(120));
    assert!(err.to_string().contains("stopped answering for 120 s"));
    assert!(err.to_string().contains("--timeout"));
}

#[test]
fn the_timeout_option_sets_the_wait() {
    let cfg = Calc {
        silent_after_content: Some(secs(500)),
        ..Calc::default()
    };
    let o = run(cfg, 30, &sample_pages(1, 100));
    assert!(matches!(o.result, Err(Error::Timeout(d)) if d == secs(30)));
}

#[test]
fn a_delay_extends_the_wait_by_what_it_announces_and_no_more() {
    let cfg = Calc {
        delays: vec![(10_000, 500)],
        ..Calc::default()
    };
    let o = run(cfg, 120, &sample_pages(1, 100));
    assert!(matches!(o.result, Err(Error::Timeout(d)) if d == secs(130)));
}

#[test]
fn an_error_packet_mid_transfer_stops_the_send() {
    let cfg = Calc {
        error_at_content_packet: Some((40, 0x000C)),
        ..Calc::default()
    };
    let o = run(cfg, 120, &sample_pages(2, 100));
    let err = o.result.unwrap_err();
    assert!(matches!(err, Error::Calculator(0x000C)), "{err}");
    assert!(err.to_string().contains("0x000c"));
    assert!(o.peer.image.is_none());
}

#[test]
fn an_app_name_already_installed_is_refused_before_anything_is_sent() {
    let cfg = Calc {
        apps: vec!["Finance".into(), "metacalc".into()],
        ..Calc::default()
    };
    let o = run(cfg, 120, &sample_pages(1, 100));
    let err = o.result.unwrap_err();
    assert!(
        matches!(err, Error::Duplicate(ref n) if n == "metacalc"),
        "{err}"
    );
    assert!(err.to_string().contains("MEM, Mem Mgmt/Del, Apps"));
    assert!(!o.peer.events.contains(&"rts"));
}

#[test]
fn a_refusal_of_the_request_to_send_says_to_delete_the_app() {
    let cfg = Calc {
        refuse_rts: Some(0x000E),
        ..Calc::default()
    };
    let o = run(cfg, 120, &sample_pages(1, 100));
    let err = o.result.unwrap_err();
    assert!(matches!(err, Error::Refused { code: 0x000E, .. }), "{err}");
    let text = err.to_string();
    assert!(
        text.contains("metacalc") && text.contains("Mem Mgmt/Del"),
        "{text}"
    );
}

#[test]
fn a_disconnect_mid_transfer_is_reported_as_one() {
    let cfg = Calc {
        disconnect_at_content_packet: Some(10),
        ..Calc::default()
    };
    let o = run(cfg, 120, &sample_pages(2, 100));
    let err = o.result.unwrap_err();
    assert!(matches!(err, Error::Disconnected), "{err}");
    assert!(err.to_string().contains("disconnected"));
}

#[test]
fn another_model_is_refused_before_the_directory_is_read() {
    let cfg = Calc {
        product: 0x13,
        name: "TI-84 Plus CE",
        ..Calc::default()
    };
    let o = run(cfg, 120, &sample_pages(1, 100));
    let err = o.result.unwrap_err();
    assert!(matches!(err, Error::WrongModel(_)));
    assert!(err.to_string().contains("TI-84 Plus CE"));
    assert_eq!(o.peer.events, ["negotiate", "ping", "params"]);
    assert!(o.stdout.is_empty());
}

#[test]
fn a_calculator_that_allocates_a_small_buffer_gets_small_packets() {
    let pages = sample_pages(1, 3000);
    let o = run(
        Calc {
            alloc: 64,
            ..Calc::default()
        },
        120,
        &pages,
    );
    o.result.unwrap();
    assert_eq!(o.peer.image.unwrap(), expected_image(&pages));
}

#[test]
fn progress_rewrites_one_line_on_a_terminal() {
    let mut out = Vec::new();
    {
        let mut p = Progress::new(&mut out, true, 2);
        p.update(0);
        p.update(PAGE / 2);
        p.update(PAGE / 2 + 1);
        p.update(PAGE);
        p.update(2 * PAGE);
        p.finish();
    }
    let text = String::from_utf8(out).unwrap();
    assert_eq!(
        text,
        "\rsending 25% (0/2 pages)\rsending 50% (1/2 pages)\rsending 100% (2/2 pages)\n"
    );
}
