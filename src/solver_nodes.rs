use std::{
    fmt::{Display, Formatter},
    ops::{Index, IndexMut},
};

use thiserror::Error;

use crate::{activation::Activation, index::RunePosition, solver::solver_state::SolverState};

#[derive(Debug, Clone, Copy)]
pub enum SolverNodeAction {
    Assume {
        position: RunePosition,
        activation: Activation,
    },
    Root,
}
#[derive(Debug)]
pub enum SolverNodeState {
    Unsolvable,
    Alive,
    Solved,
}

pub struct SolverNodeData {
    pub deduction_chain: Vec<SolverState>,
    pub action: SolverNodeAction,
    pub state: SolverNodeState,
}

struct SolverNode {
    pub parent: Option<SolverNodeHandle>,
    pub data: SolverNodeData,
    children: Vec<SolverNodeHandle>,
}

impl SolverNodeData {
    pub fn rule_out(&mut self, position: RunePosition, activation: Activation) {
        let last_state = self
            .deduction_chain
            .last()
            .unwrap()
            .ruled_out(position, activation);
        self.deduction_chain.push(last_state);
    }
}

#[derive(Copy, Clone)]
pub struct SolverNodeHandle(usize);

pub struct SolverNodes {
    nodes: Vec<SolverNode>,
}

#[derive(Debug, Error)]
pub enum SolverNodesError {
    #[error("Node {0} does not exist")]
    UnknownNode(usize),
}

impl SolverNodes {
    pub fn new(initial: SolverState) -> (Self, SolverNodeHandle) {
        (
            Self {
                nodes: vec![SolverNode {
                    parent: None,
                    data: SolverNodeData {
                        state: SolverNodeState::Alive,
                        deduction_chain: vec![initial],
                        action: SolverNodeAction::Root,
                    },
                    children: vec![],
                }],
            },
            SolverNodeHandle(0),
        )
    }

    pub fn insert_child(
        &mut self,
        parent: SolverNodeHandle,
        child: SolverNodeData,
    ) -> SolverNodeHandle {
        self.nodes.push(SolverNode {
            parent: Some(parent),
            data: child,
            children: vec![],
        });

        let child_handle = SolverNodeHandle(self.nodes.len() - 1);

        self.nodes[parent.0].children.push(child_handle);

        return child_handle;
    }

    pub fn get_handle(&self, node: usize) -> Result<SolverNodeHandle, SolverNodesError> {
        if node >= self.nodes.len() {
            return Err(SolverNodesError::UnknownNode(node));
        }
        return Ok(SolverNodeHandle(node));
    }

    pub fn parent_of(&self, node: SolverNodeHandle) -> Option<SolverNodeHandle> {
        self.nodes[node.0].parent
    }
}

impl Index<SolverNodeHandle> for SolverNodes {
    type Output = SolverNodeData;

    fn index(&self, index: SolverNodeHandle) -> &Self::Output {
        &self.nodes.index(index.0).data
    }
}

impl IndexMut<SolverNodeHandle> for SolverNodes {
    fn index_mut(&mut self, index: SolverNodeHandle) -> &mut Self::Output {
        &mut self.nodes.index_mut(index.0).data
    }
}

impl Display for SolverNodeAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverNodeAction::Assume {
                position,
                activation,
            } => write!(f, "Assume {position} = {activation}")?,
            SolverNodeAction::Root => write!(f, "Root")?,
        }
        Ok(())
    }
}

impl Display for SolverNodeState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverNodeState::Unsolvable => write!(f, "✘"),
            SolverNodeState::Alive => write!(f, " "),
            SolverNodeState::Solved => write!(f, "✔"),
        }
    }
}

impl Display for SolverNodeData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.state, self.action)
    }
}

impl Display for SolverNodes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fn do_fmt(
            handle: SolverNodeHandle,
            nodes: &SolverNodes,
            indent: usize,
            f: &mut Formatter<'_>,
        ) -> std::fmt::Result {
            write!(f, "{0:1$} - ({3}) {2}\n", "", indent, nodes[handle], handle)?;
            for child in nodes.nodes[handle.0].children.iter() {
                do_fmt(*child, nodes, indent + 2, f)?;
            }
            Ok(())
        }

        do_fmt(SolverNodeHandle(0), self, 0, f)
    }
}

impl Display for SolverNodeHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
