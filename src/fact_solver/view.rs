use ndarray::Axis;

use crate::{activation::Activation, index::RunePosition};

pub trait View {
    type Complement: View;

    fn from_usize(it: usize) -> Self;
    fn axis() -> Axis;
    fn index(&self) -> usize;
}

impl View for RunePosition {
    type Complement = Activation;

    fn from_usize(it: usize) -> Self {
        Self::new(it)
    }

    fn axis() -> Axis {
        Axis(0) //TODO CHeck if this is the correct assignment
    }

    fn index(&self) -> usize {
        self.index()
    }
}

impl View for Activation {
    type Complement = RunePosition;

    fn from_usize(it: usize) -> Self {
        Self::new(it as u8).unwrap()
    }

    fn axis() -> Axis {
        Axis(1)
    }

    fn index(&self) -> usize {
        self.index()
    }
}
