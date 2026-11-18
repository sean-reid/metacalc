use super::ParseError;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 100;
pub(super) const MAX_FILE: u64 = 64 << 20;
pub(super) const MAX_TOTAL: u64 = 256 << 20;

/// Where a run of the expanded text came from.
struct Chunk {
    at: usize,
    file: usize,
    line: usize,
}

/// A database with every `$[ file $]` replaced by the text of the file.
pub(super) struct Expanded {
    pub text: Vec<u8>,
    chunks: Vec<Chunk>,
    names: Vec<Option<String>>,
}

impl Expanded {
    /// The file name and line of a byte of the expanded text; `None` names the top-level file.
    pub fn locate(&self, pos: usize) -> (Option<&str>, usize) {
        let index = self
            .chunks
            .partition_point(|c| c.at <= pos)
            .saturating_sub(1);
        let chunk = &self.chunks[index];
        let end = pos.min(self.text.len());
        let start = chunk.at.min(end);
        let lines = self.text[start..end]
            .iter()
            .filter(|&&b| b == b'\n')
            .count();
        (self.names[chunk.file].as_deref(), chunk.line + lines)
    }
}

struct Expander<'a> {
    base: Option<&'a Path>,
    out: Expanded,
    included: HashSet<PathBuf>,
    open: Vec<PathBuf>,
    max_file: u64,
    max_total: u64,
}

/// Replaces each inclusion command of `src` with the file it names, once per file. Names are
/// relative to the directory of `top` and may leave it. Each included file must be a regular file
/// of at most `max_file` bytes, and the expanded text at most `max_total`. Without `top`, an
/// inclusion is refused.
pub(super) fn expand(
    src: &[u8],
    top: Option<&Path>,
    max_file: u64,
    max_total: u64,
) -> Result<Expanded, ParseError> {
    let mut expander = Expander {
        base: top.map(|path| path.parent().unwrap_or(Path::new(""))),
        out: Expanded {
            text: Vec::with_capacity(src.len()),
            chunks: Vec::new(),
            names: Vec::new(),
        },
        included: HashSet::new(),
        open: top.map(canonical).into_iter().collect(),
        max_file,
        max_total,
    };
    expander.file(None, src)?;
    Ok(expander.out)
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn tokens(src: &[u8]) -> impl Iterator<Item = (usize, usize)> + '_ {
    let mut pos = 0;
    std::iter::from_fn(move || {
        while pos < src.len() && src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos == src.len() {
            return None;
        }
        let start = pos;
        while pos < src.len() && !src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        Some((start, pos))
    })
}

fn line_at(src: &[u8], pos: usize) -> usize {
    1 + src[..pos].iter().filter(|&&b| b == b'\n').count()
}

impl Expander<'_> {
    fn file(&mut self, name: Option<&str>, src: &[u8]) -> Result<(), ParseError> {
        let file = self.out.names.len();
        self.out.names.push(name.map(str::to_owned));
        let fail = |pos: usize, message: &str| ParseError {
            file: name.map(str::to_owned),
            line: line_at(src, pos),
            message: message.to_string(),
        };
        let mut copied = 0;
        let mut in_comment = false;
        let mut found = tokens(src);
        while let Some((start, end)) = found.next() {
            match (&src[start..end], in_comment) {
                (b"$(", false) => in_comment = true,
                (b"$)", true) => in_comment = false,
                (_, true) => {}
                (b"$[", false) => {
                    let (Some((from, to)), Some(close)) = (found.next(), found.next()) else {
                        return Err(fail(start, "$[ is not closed by $]"));
                    };
                    if &src[close.0..close.1] != b"$]" {
                        return Err(fail(start, "$[ takes one file name and $]"));
                    }
                    let target = String::from_utf8_lossy(&src[from..to]).into_owned();
                    self.chunk(file, src, copied, start);
                    self.out.text.push(b' ');
                    self.include(&target).map_err(|e| match e {
                        Failure::Parse(e) => e,
                        Failure::Here(message) => fail(start, &message),
                    })?;
                    self.out.text.push(b' ');
                    copied = close.1;
                }
                _ => {}
            }
        }
        if in_comment {
            return Err(fail(src.len(), "unterminated comment"));
        }
        self.chunk(file, src, copied, src.len());
        Ok(())
    }

    fn chunk(&mut self, file: usize, src: &[u8], from: usize, to: usize) {
        self.out.chunks.push(Chunk {
            at: self.out.text.len(),
            file,
            line: line_at(src, from),
        });
        self.out.text.extend_from_slice(&src[from..to]);
    }

    fn include(&mut self, name: &str) -> Result<(), Failure> {
        let Some(base) = self.base else {
            return Err(Failure::Here("file inclusion is not supported here".into()));
        };
        let path = base.join(name);
        let target = std::fs::canonicalize(&path)
            .map_err(|e| Failure::Here(format!("cannot read {name}: {e}")))?;
        let meta = std::fs::metadata(&target)
            .map_err(|e| Failure::Here(format!("cannot read {name}: {e}")))?;
        if !meta.is_file() {
            return Err(Failure::Here(format!("{name} is not a regular file")));
        }
        if self.open.contains(&target) {
            return Err(Failure::Here(format!("{name} includes itself")));
        }
        if self.included.contains(&target) {
            return Ok(());
        }
        if self.open.len() >= MAX_DEPTH {
            return Err(Failure::Here(format!(
                "inclusions nest more than {MAX_DEPTH} deep"
            )));
        }
        let mut src = Vec::new();
        std::fs::File::open(&target)
            .and_then(|f| f.take(self.max_file + 1).read_to_end(&mut src))
            .map_err(|e| Failure::Here(format!("cannot read {name}: {e}")))?;
        if src.len() as u64 > self.max_file {
            return Err(Failure::Here(format!(
                "{name} is over {} bytes",
                self.max_file
            )));
        }
        if self.out.text.len() as u64 + src.len() as u64 > self.max_total {
            return Err(Failure::Here(format!(
                "the included files pass {} bytes at {name}",
                self.max_total
            )));
        }
        let text = tokens(&src)
            .next()
            .is_none_or(|(a, b)| src[a..b].iter().all(u8::is_ascii_graphic));
        if !text {
            return Err(Failure::Here(format!("{name} is not a Metamath database")));
        }
        self.included.insert(target.clone());
        self.open.push(target);
        let done = self.file(Some(name), &src).map_err(Failure::Parse);
        self.open.pop();
        done
    }
}

enum Failure {
    /// An error inside the included file, already placed.
    Parse(ParseError),
    /// An error with the command that names the file.
    Here(String),
}
