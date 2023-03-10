use std::fmt::Display;

#[derive(Debug, PartialEq, Copy, Clone, Hash, Eq)]
pub struct Rune(u8);

impl Rune {
    pub fn new(id: u8) -> Self {
        Self(id)
    }
}

impl Display for Rune {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            0 => write!(f, "Z"),
            1 => write!(f, "V"),
            2 => write!(f, "S"),
            3 => write!(f, "C"),
            _ => write!(f, "{}", self.0),
        }
    }
}
