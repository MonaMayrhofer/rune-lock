use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
};

use thiserror::Error;

use crate::{
    activation::Activation,
    assignment::Assignment,
    index::RunePosition,
    solver_nodes::{
        SolverNodeAction, SolverNodeData, SolverNodeHandle, SolverNodeState, SolverNodes,
        SolverNodesError,
    },
    RuneLock,
};

#[derive(Clone)]
pub enum FieldState {
    Assumed(Activation),
    Unsure(HashSet<Activation>),
    Deduced(Activation),
}

impl Default for FieldState {
    fn default() -> Self {
        Self::Unsure(HashSet::from_iter(
            (0..12).map(|it| Activation::new(it).unwrap()),
        ))
    }
}

impl Display for FieldState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldState::Deduced(a) => write!(f, "{}", a),
            FieldState::Assumed(a) => write!(f, "{}?", a),
            FieldState::Unsure(p) => {
                write!(f, "[")?;
                let mut act: Vec<_> = p.iter().collect();
                act.sort();
                for i in act {
                    write!(f, "{} ", i)?;
                }
                write!(f, "]")?;
                Ok(())
            }
        }
    }
}
