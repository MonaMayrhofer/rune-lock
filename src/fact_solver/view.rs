use std::any::TypeId;

use ndarray::Axis;

use crate::{activation::Activation, index::RunePosition};

pub trait View {
    type Complement: View;

    fn from_usize(it: usize) -> Self;
    fn axis() -> Axis;
    fn index(&self) -> usize;
}

pub trait ChooseView: View {
    fn choose_position(s: Self, c: Self::Complement) -> RunePosition;
    fn choose_activation(s: Self, c: Self::Complement) -> Activation;
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

impl ChooseView for RunePosition {
    fn choose_position(s: Self, c: Self::Complement) -> RunePosition {
        s
    }

    fn choose_activation(s: Self, c: Self::Complement) -> Activation {
        c
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
impl ChooseView for Activation {
    fn choose_position(s: Self, c: Self::Complement) -> RunePosition {
        c
    }

    fn choose_activation(s: Self, c: Self::Complement) -> Activation {
        s
    }
}
