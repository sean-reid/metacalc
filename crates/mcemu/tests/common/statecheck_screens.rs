// Included by statecheck.rs: the screens the probe draws, as the tests write them out.

pub fn start_screen(ver: &str, flags: u8, on: &str, off: &str, stale: bool) -> Vec<String> {
    [
        "statechk v1".to_string(),
        format!("ver {ver}"),
        format!("f33 {flags:02X}"),
        format!("ONSCRPT {on}"),
        format!("OFFSCRPT {off}"),
        if stale { "fill ok stale" } else { "fill ok" }.to_string(),
        "QUIT is a key".to_string(),
        "CLEAR exits".to_string(),
    ]
    .to_vec()
}

/// The lines for vars, stack and appd, a changed region as `@offset xcount`, then stat and ssc,
/// and the save check on the flag byte's line.
pub struct Report {
    pub vars: String,
    pub stack: String,
    pub appd: String,
    pub stat: &'static str,
    pub ssc: &'static str,
    pub save: &'static str,
}

impl Report {
    pub fn ok() -> Report {
        Report {
            vars: "ok".into(),
            stack: "ok".into(),
            appd: "ok".into(),
            stat: "ok",
            ssc: "same",
            save: "ok",
        }
    }
}

pub fn key_screen(count: u16, code: u8, flags: u8, r: &Report) -> Vec<String> {
    [
        format!("keys {count:05} k={code:02X}"),
        format!("vars {}", r.vars),
        format!("stack {}", r.stack),
        format!("appd {}", r.appd),
        format!("stat {}", r.stat),
        format!("ssc {}", r.ssc),
        format!("f33 {flags:02X} save {}", r.save),
        "CLEAR exits".to_string(),
    ]
    .to_vec()
}

/// A changed region as the key screen prints it: the first offset in hex, then the count.
pub fn changed(first: usize, count: usize) -> String {
    format!("@{first:02X} x{count:03}")
}

/// The addresses as `ADDR+length` runs.
pub fn runs(addrs: &[u16]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < addrs.len() {
        let mut j = i;
        while j + 1 < addrs.len() && addrs[j + 1] == addrs[j] + 1 {
            j += 1;
        }
        out.push(format!("{:04X}+{}", addrs[i], j - i + 1));
        i = j + 1;
    }
    out.join(" ")
}
