const WIDTH: usize = 79;

/// Appends tokens to a string, breaking lines before they pass `WIDTH` columns.
pub(super) struct Lines<'a> {
    out: &'a mut String,
    indent: usize,
    used: usize,
    empty: bool,
}

impl<'a> Lines<'a> {
    pub(super) fn new(out: &'a mut String, indent: usize) -> Self {
        out.extend(std::iter::repeat_n(' ', indent));
        Lines {
            out,
            indent,
            used: indent,
            empty: true,
        }
    }

    fn newline(&mut self) {
        self.out.push('\n');
        self.out.extend(std::iter::repeat_n(' ', self.indent + 2));
        self.used = self.indent + 2;
        self.empty = true;
    }

    fn separate(&mut self) {
        if !self.empty {
            self.out.push(' ');
            self.used += 1;
        }
    }

    pub(super) fn token(&mut self, token: &str) {
        if !self.empty && self.used + 1 + token.len() > WIDTH {
            self.newline();
        }
        self.separate();
        self.out.push_str(token);
        self.used += token.len();
        self.empty = false;
    }

    /// Letters may split anywhere, so each line is filled to the margin.
    pub(super) fn run(&mut self, letters: &str) {
        let mut rest = letters;
        while !rest.is_empty() {
            let room = WIDTH.saturating_sub(self.used + usize::from(!self.empty));
            if room == 0 {
                self.newline();
                continue;
            }
            let (head, tail) = rest.split_at(room.min(rest.len()));
            self.separate();
            self.out.push_str(head);
            self.used += head.len();
            self.empty = false;
            rest = tail;
        }
    }

    pub(super) fn finish(self) {
        self.out.push('\n');
    }
}
