/// A key press. `F1` to `F5` are the five keys under the screen, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Clear,
    F1,
    F2,
    F3,
    F4,
    F5,
    Letter(char),
}
