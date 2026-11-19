//! An include that names a terminal device must not make it the controlling terminal of the reader.

#![cfg(unix)]

use mcpack::mm::parse_at;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CHILD: &str = "MCPACK_INCLUDE_TERMINAL_CHILD";

fn has_controlling_terminal() -> bool {
    let tty = std::ffi::CString::new("/dev/tty").unwrap();
    let fd = unsafe {
        libc::open(
            tty.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOCTTY,
        )
    };
    if fd >= 0 {
        unsafe { libc::close(fd) };
        return true;
    }
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ENXIO)
    );
    false
}

/// The slave path of a new pty, with the master held open.
fn pty() -> (libc::c_int, std::path::PathBuf) {
    unsafe {
        let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
        assert!(master >= 0, "{}", std::io::Error::last_os_error());
        assert_eq!(libc::grantpt(master), 0);
        assert_eq!(libc::unlockpt(master), 0);
        let name = libc::ptsname(master);
        assert!(!name.is_null());
        let path = std::ffi::CStr::from_ptr(name).to_str().unwrap().into();
        (master, path)
    }
}

#[test]
fn the_child_that_includes_a_terminal() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    assert!(
        !has_controlling_terminal(),
        "the child starts with a terminal"
    );
    let (master, slave) = pty();
    let held = std::ffi::CString::new(slave.to_str().unwrap()).unwrap();
    let held = unsafe {
        libc::open(
            held.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOCTTY,
        )
    };
    assert!(held >= 0, "{}", std::io::Error::last_os_error());
    let dir = std::env::temp_dir().join(format!("mcpack-tty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let top = dir.join("top.mm");
    let text = format!("$[ {} $]", slave.display());
    std::fs::write(&top, &text).unwrap();
    let refused = parse_at(&top, text.into_bytes())
        .err()
        .map(|e| e.to_string());
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(
        refused.is_some_and(|m| m.contains("is not a regular file")),
        "the include of a terminal is refused"
    );
    assert!(
        !has_controlling_terminal(),
        "the include took a controlling terminal"
    );
    unsafe {
        libc::close(held);
        libc::close(master);
    }
}

#[test]
fn an_included_terminal_does_not_become_the_controlling_terminal() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "the_child_that_includes_a_terminal",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        command.pre_exec(|| match libc::setsid() {
            -1 => Err(std::io::Error::last_os_error()),
            _ => Ok(()),
        });
    }
    let mut child = command.spawn().unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("the child did not finish");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let out = child.wait_with_output().unwrap();
    assert!(
        status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));
}
