use crate::tifile::PAGE;
use std::io::Write;

/// Progress of the transfer: a percentage rewritten in place on a terminal, one line per page
/// otherwise.
pub struct Progress<W: Write> {
    out: W,
    tty: bool,
    pages: usize,
    total: usize,
    shown: usize,
    line_open: bool,
}

impl<W: Write> Progress<W> {
    pub fn new(out: W, tty: bool, pages: usize) -> Progress<W> {
        Progress {
            out,
            tty,
            pages,
            total: pages * PAGE,
            shown: 0,
            line_open: false,
        }
    }

    /// Reports `sent` bytes of the app image acknowledged.
    pub fn update(&mut self, sent: usize) {
        let sent = sent.min(self.total);
        if self.tty {
            let percent = sent * 100 / self.total;
            if percent != self.shown {
                self.shown = percent;
                let _ = write!(
                    self.out,
                    "\rsending {percent}% ({}/{} pages)",
                    sent / PAGE,
                    self.pages
                );
                let _ = self.out.flush();
                self.line_open = true;
            }
        } else {
            let done = sent / PAGE;
            while self.shown < done {
                self.shown += 1;
                let _ = writeln!(self.out, "page {} of {} sent", self.shown, self.pages);
            }
        }
    }

    /// Ends the in-place line.
    pub fn finish(&mut self) {
        if self.line_open {
            let _ = writeln!(self.out);
            self.line_open = false;
        }
    }
}
