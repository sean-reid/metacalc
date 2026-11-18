use std::fmt;

/// A 1-bit image, row-major, `true` meaning ink.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Bitmap {
    width: usize,
    height: usize,
    cells: Vec<bool>,
}

impl Bitmap {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![false; width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Pixels outside the bitmap read as blank.
    pub fn get(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height && self.cells[y * self.width + x]
    }

    /// Writes outside the bitmap are ignored.
    pub fn set(&mut self, x: usize, y: usize, ink: bool) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = ink;
        }
    }

    pub fn rows(&self) -> impl Iterator<Item = &[bool]> {
        self.cells.chunks(self.width.max(1)).take(self.height)
    }

    /// Copies the ink of `src` onto `self` with its top left corner at `(x, y)`.
    pub fn blit(&mut self, src: &Bitmap, x: usize, y: usize) {
        for sy in 0..src.height {
            for sx in 0..src.width {
                if src.get(sx, sy) {
                    self.set(x + sx, y + sy, true);
                }
            }
        }
    }

    pub(crate) fn from_rows(rows: &[Vec<bool>]) -> Self {
        let height = rows.len();
        let width = rows.first().map_or(0, Vec::len);
        Self {
            width,
            height,
            cells: rows.concat(),
        }
    }
}

impl fmt::Display for Bitmap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, row) in self.rows().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            for &ink in row {
                f.write_str(if ink { "#" } else { "." })?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Bitmap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Bitmap {}x{}", self.width, self.height)?;
        fmt::Display::fmt(self, f)
    }
}
