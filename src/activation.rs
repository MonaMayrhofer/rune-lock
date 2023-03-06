use std::fmt::Display;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Activation(u8);

impl Display for Activation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0 + 1)
    }
}

#[derive(Error, Debug)]
pub enum ActivationError {
    #[error("Activation is out of bounds")]
    OutOfBounds,
}

impl Activation {
    ///1 based
    pub fn from_human(one_based: u8) -> Result<Self, ActivationError> {
        if one_based < 1 {
            return Err(ActivationError::OutOfBounds);
        }
        Activation::new(one_based - 1)
    }

    pub fn new(zero_based: u8) -> Result<Self, ActivationError> {
        if zero_based >= 12 {
            Err(ActivationError::OutOfBounds)
        } else {
            Ok(Activation(zero_based))
        }
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }

    pub fn next(&self) -> Result<Self, ActivationError> {
        Activation::new(self.0 + 1)
    }
}
