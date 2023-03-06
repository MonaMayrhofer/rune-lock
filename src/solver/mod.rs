pub mod field_state;
pub mod solver_state;

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

use self::solver_state::SolverState;

enum ActivationPossibility {
    AlreadyDeduced,
    None,
    ForcedAt(RunePosition),
    ExactlyOne(RunePosition),
    MoreThanOne,
    AlreadyAssumed,
}

enum DeductionIterationResult {
    Unsolvable { reason: String },
    MadeDeductions(SolverState),
    Indecisive,
}

pub enum DeductionResult {
    Unsolvable,
    Indecisive,
}

pub enum DeduceWithAssumptionResult {
    Unsolvable { reason: String },
    Done(Vec<SolverState>),
    Solved(Vec<SolverState>),
}

#[derive(Debug, Error)]
pub enum SolverError {
    #[error("Cannot assume that {activation} is at position {new_position}, which is already assumed to be {old_assumption}")]
    AssumeAtOldAssumption {
        activation: Activation,
        new_position: RunePosition,
        old_assumption: Activation,
    },
    #[error("Cannot assume that {activation} is at position {new_position}, which is already deduced to be {old_deduction}")]
    AssumeAtOldDeduction {
        activation: Activation,
        new_position: RunePosition,
        old_deduction: Activation,
    },
    #[error("Cannot assume that {activation} is at position {new_position}, when it is already assumed to be at {old_position}")]
    ActivationAlreadyAssumed {
        activation: Activation,
        new_position: RunePosition,
        old_position: RunePosition,
    },
    #[error("Cannot assume that {activation} is at position {new_position}, when it is already deduced to be at {old_position}")]
    ActivationAlreadyDeduced {
        activation: Activation,
        new_position: RunePosition,
        old_position: RunePosition,
    },
    #[error("Cannot go back before the initial state.")]
    PopInitialState,
}

pub enum ExploreResult {
    Unsolvable { reason: String },
    Indecisive,
    Solved { solution: SolverNodeHandle },
}

impl Display for ExploreResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExploreResult::Unsolvable { reason } => write!(
                f,
                "This assignment turns out to be impossible. ({})",
                reason
            ),
            ExploreResult::Indecisive => write!(
                f,
                "This assignment leads to another unclear situation. More assumptions required."
            ),
            ExploreResult::Solved { solution } => {
                write!(f, "The assignment {} solves the lock.", solution)
            }
        }
    }
}

pub struct Solver {
    nodes: SolverNodes,
    current: SolverNodeHandle,
}

impl Solver {
    pub fn new() -> Self {
        let (nodes, root) = SolverNodes::new(SolverState::default());
        Self {
            nodes,
            current: root,
        }
    }

    pub fn explore(
        &mut self,
        lock: &RuneLock,
        position: RunePosition,
        assume_to_be: Activation,
    ) -> Result<ExploreResult, SolverError> {
        let result = self
            .peek()
            .deduce_with_assumption(lock, position, assume_to_be);

        match result {
            Ok(DeduceWithAssumptionResult::Unsolvable { reason }) => {
                self.nodes[self.current].rule_out(position, assume_to_be);

                Ok(ExploreResult::Unsolvable { reason })
            }
            Ok(DeduceWithAssumptionResult::Done(steps)) => {
                self.current = self.nodes.insert_child(
                    self.current,
                    SolverNodeData {
                        state: SolverNodeState::Alive,
                        deduction_chain: steps,
                        action: SolverNodeAction::Assume {
                            position,
                            activation: assume_to_be,
                        },
                    },
                );
                Ok(ExploreResult::Indecisive)
            }
            Ok(DeduceWithAssumptionResult::Solved(steps)) => {
                println!("AAAHHHH SOLVED");
                self.nodes[self.current].rule_out(position, assume_to_be);
                let solution = self.nodes.insert_child(
                    self.current,
                    SolverNodeData {
                        state: SolverNodeState::Solved,
                        deduction_chain: steps,
                        action: SolverNodeAction::Assume {
                            position,
                            activation: assume_to_be,
                        },
                    },
                );
                self.current = solution;
                Ok(ExploreResult::Solved { solution })
            }
            Err(err) => Err(err),
        }
    }

    pub fn peek(&self) -> &SolverState {
        &self.nodes[self.current].deduction_chain.last().unwrap()
    }

    pub fn print_nodes(&self) {
        println!("{}", self.nodes);
        println!("{}", self.current);
    }

    pub fn view(&mut self, node: usize) -> Result<(), SolverNodesError> {
        self.current = self.nodes.get_handle(node)?;
        Ok(())
    }

    pub fn try_activation(
        &mut self,
        lock: &RuneLock,
        activation: Activation,
    ) -> Result<ExploreResult, SolverError> {
        let state = self.peek();
        let to_try = state.possible_positions_of(activation);
        println!("to try: {:?}", to_try);
        let mut solved = false;
        for i in to_try {
            match self.explore(lock, i, activation)? {
                ExploreResult::Unsolvable { reason } => {
                    println!("Assumption {} in {} is false: {}", activation, i, reason);
                }
                ExploreResult::Indecisive => return Ok(ExploreResult::Indecisive),
                ExploreResult::Solved { .. } => {
                    solved = true;
                }
            }
        }

        if solved {
            Ok(ExploreResult::Indecisive)
        } else {
            self.nodes[self.current].state = SolverNodeState::Unsolvable;
            let assumption = self.nodes[self.current].action;
            self.current = self.nodes.parent_of(self.current).unwrap();
            if let SolverNodeAction::Assume {
                position,
                activation,
            } = assumption
            {
                self.nodes[self.current].rule_out(position, activation)
            }

            Ok(ExploreResult::Unsolvable {
                reason: format!(
                    "Activation {} has no position it can be assigned to.",
                    activation
                ),
            })
        }
    }

    pub fn try_in_position(
        &mut self,
        lock: &RuneLock,
        position: RunePosition,
    ) -> Result<ExploreResult, SolverError> {
        let state = self.peek();
        let to_try = state.possible_activations_of(position);
        let mut solved = false;
        for i in to_try {
            match self.explore(lock, position, i)? {
                ExploreResult::Unsolvable { reason } => {
                    println!("Assumption {} in {} is false: {}", i, position, reason);
                }
                ExploreResult::Indecisive => return Ok(ExploreResult::Indecisive),
                ExploreResult::Solved { .. } => {
                    solved = true;
                }
            }
        }

        if solved {
            Ok(ExploreResult::Indecisive)
        } else {
            self.nodes[self.current].state = SolverNodeState::Unsolvable;
            let assumption = self.nodes[self.current].action;
            self.current = self.nodes.parent_of(self.current).unwrap();
            if let SolverNodeAction::Assume {
                position,
                activation,
            } = assumption
            {
                self.nodes[self.current].rule_out(position, activation)
            }

            Ok(ExploreResult::Unsolvable {
                reason: format!(
                    "Positon {} has no activation that can be assigned to it.",
                    position
                ),
            })
        }
    }
}
