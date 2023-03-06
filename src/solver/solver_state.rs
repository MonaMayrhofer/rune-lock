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

use super::{
    field_state::FieldState, ActivationPossibility, DeduceWithAssumptionResult,
    DeductionIterationResult, SolverError,
};

#[derive(Clone)]
pub enum SolverStateAction {
    Assume {
        activation: Activation,
        position: RunePosition,
    },
    RuleOut {
        activation: Activation,
        position: RunePosition,
    },
    Default,
}

#[derive(Clone)]
pub struct SolverState {
    state: [FieldState; 12],
    action: SolverStateAction,
}

impl Display for SolverState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (position, state) in self.state.iter().enumerate() {
            let position = RunePosition::new(position);
            write!(f, "{:2}: {}\n", position, state)?;
        }
        Ok(())
    }
}

impl Default for SolverState {
    fn default() -> Self {
        SolverState {
            action: SolverStateAction::Default,
            state: [
                FieldState::default(),
                FieldState::default(),
                FieldState::default(),
                FieldState::default(),
                FieldState::default(),
                FieldState::default(),
                FieldState::default(),
                FieldState::default(),
                FieldState::default(),
                FieldState::default(),
                FieldState::default(),
                FieldState::default(),
            ],
        }
    }
}

impl SolverState {
    pub fn with_assumed(
        &self,
        lock: &RuneLock,
        position: RunePosition,
        to_be: Activation,
    ) -> Result<SolverState, SolverError> {
        let mut new_state = self.clone();
        new_state.action = SolverStateAction::Assume {
            activation: to_be,
            position,
        };
        new_state.assume(position, to_be)?;
        new_state.prune_state(lock);
        Ok(new_state)
    }

    pub fn deduce_with_assumption(
        &self,
        lock: &RuneLock,
        position: RunePosition,
        to_be: Activation,
    ) -> Result<DeduceWithAssumptionResult, SolverError> {
        let mut substates = vec![self.with_assumed(lock, position, to_be)?];
        loop {
            let result = substates.last().unwrap().deduce(lock);
            match result {
                DeductionIterationResult::Unsolvable { reason } => {
                    return Ok(DeduceWithAssumptionResult::Unsolvable { reason });
                }
                DeductionIterationResult::MadeDeductions(deduced) => {
                    substates.push(deduced);
                }
                DeductionIterationResult::Indecisive => {
                    return Ok(DeduceWithAssumptionResult::Done(substates));
                }
            }
        }
    }

    pub fn possible_activations_of(&self, pos: RunePosition) -> Vec<Activation> {
        match &self.state[pos] {
            FieldState::Unsure(possibilities) => {
                let mut poss: Vec<_> = possibilities.iter().cloned().collect();
                poss.sort();
                poss
            }
            FieldState::Assumed(_) | FieldState::Deduced(_) => Vec::new(),
        }
    }

    pub fn possible_positions_of(&self, activation: Activation) -> Vec<RunePosition> {
        let mut positions = Vec::with_capacity(12);
        for (position, state) in self.state.iter().enumerate() {
            let position = RunePosition::new(position);

            match state {
                FieldState::Unsure(possibilities) => positions.push(position),
                FieldState::Assumed(_) | FieldState::Deduced(_) => {}
            }
        }
        positions.sort();
        positions
    }

    pub fn fixed_assignments(&self) -> Assignment {
        let i = self.state.iter().map(|it| match it {
            FieldState::Deduced(activation) => Some(*activation),
            FieldState::Assumed(activation) => Some(*activation),
            FieldState::Unsure(_) => None,
        });
        Assignment::from_iter(i).expect("solver should only contain valid assignment states")
    }

    fn assume(&mut self, position: RunePosition, to_be: Activation) -> Result<(), SolverError> {
        match self.state[position] {
            FieldState::Assumed(old) => {
                return Err(SolverError::AssumeAtOldAssumption {
                    activation: to_be,
                    new_position: position,
                    old_assumption: old,
                })
            }
            FieldState::Deduced(old) => {
                return Err(SolverError::AssumeAtOldDeduction {
                    activation: to_be,
                    new_position: position,
                    old_deduction: old,
                })
            }
            FieldState::Unsure(_) => {}
        }
        for (old_position, activation) in self.state.iter().enumerate() {
            let old_position = RunePosition::new(old_position);
            match activation {
                FieldState::Assumed(old) if *old == to_be => {
                    return Err(SolverError::ActivationAlreadyAssumed {
                        activation: to_be,
                        new_position: position,
                        old_position,
                    })
                }
                FieldState::Deduced(old) if *old == to_be => {
                    return Err(SolverError::ActivationAlreadyDeduced {
                        activation: to_be,
                        new_position: position,
                        old_position,
                    })
                }
                _ => {}
            }
        }
        self.state[position] = FieldState::Assumed(to_be);
        Ok(())
    }

    fn prune_state(&mut self, lock: &RuneLock) {
        let assignment = self.fixed_assignments();

        for (position, state) in self.state.iter_mut().enumerate() {
            let position = RunePosition::new(position);
            match state {
                FieldState::Deduced(_) => {}
                FieldState::Assumed(_) => {}
                FieldState::Unsure(possibilities) => possibilities.retain(|possibility| {
                    if assignment.contains(*possibility) {
                        false
                    } else {
                        let mut assignment = assignment.clone();
                        assignment.assign(position, *possibility);
                        match lock.validate(&assignment) {
                            Ok(_) => true,
                            Err(_) => false,
                        }
                    }
                }),
            }
        }
    }

    fn deduce(&self, lock: &RuneLock) -> DeductionIterationResult {
        let mut activation_possibility = [0; 12].map(|_| ActivationPossibility::None);
        for (position, state) in self.state.iter().enumerate() {
            let position = RunePosition::new(position);
            match state {
                FieldState::Unsure(possibilities) => {
                    if possibilities.len() == 1 {
                        let activation = *possibilities.iter().next().unwrap();
                        activation_possibility[activation.index()] =
                            ActivationPossibility::ForcedAt(position);
                    } else {
                        for possibility in possibilities.iter() {
                            activation_possibility[possibility.index()] =
                                match activation_possibility[possibility.index()] {
                                    ActivationPossibility::AlreadyDeduced => {
                                        ActivationPossibility::AlreadyDeduced
                                    }
                                    ActivationPossibility::AlreadyAssumed => {
                                        ActivationPossibility::AlreadyAssumed
                                    }
                                    ActivationPossibility::None => {
                                        ActivationPossibility::ExactlyOne(position)
                                    }
                                    ActivationPossibility::ExactlyOne(_) => {
                                        ActivationPossibility::MoreThanOne
                                    }
                                    ActivationPossibility::MoreThanOne => {
                                        ActivationPossibility::MoreThanOne
                                    }
                                    ActivationPossibility::ForcedAt(p) => {
                                        ActivationPossibility::ForcedAt(p)
                                    }
                                }
                        }
                    }
                }
                FieldState::Deduced(a) => {
                    activation_possibility[a.index()] = ActivationPossibility::AlreadyDeduced
                }
                FieldState::Assumed(a) => {
                    activation_possibility[a.index()] = ActivationPossibility::AlreadyAssumed
                }
            }
        }

        let mut deduced_state = self.clone();

        let mut changed = false;
        for (activation, possibilities) in activation_possibility.into_iter().enumerate() {
            let activation = Activation::new(activation as u8).unwrap();
            match possibilities {
                ActivationPossibility::ForcedAt(pos) | ActivationPossibility::ExactlyOne(pos) => {
                    match deduced_state.state[pos] {
                        FieldState::Assumed(assumed) => {
                            return DeductionIterationResult::Unsolvable {
                                reason: format!(
                                    "{} has to be at {}, but that has been assumed to be {}",
                                    activation, pos, assumed
                                ),
                            }
                        }
                        FieldState::Deduced(deduced) => {
                            return DeductionIterationResult::Unsolvable {
                                reason: format!(
                                    "{} has to be at {}, but that has been deduced to be {}",
                                    activation, pos, deduced
                                ),
                            }
                        }
                        _ => {}
                    }
                    changed = true;
                    deduced_state.state[pos] = FieldState::Deduced(activation);
                }
                ActivationPossibility::None => {
                    return DeductionIterationResult::Unsolvable {
                        reason: format!("{} has no possible position left.", activation),
                    }
                }
                _ => {}
            }
        }
        if changed {
            deduced_state.prune_state(lock);
            return DeductionIterationResult::MadeDeductions(deduced_state);
        } else {
            return DeductionIterationResult::Indecisive;
        }
    }

    pub fn ruled_out(&self, position: RunePosition, assume_to_be: Activation) -> SolverState {
        let mut state = self.state.clone();
        match &mut state[position] {
            FieldState::Assumed(_) | FieldState::Deduced(_) => {
                panic!("Cannot rule something out that is fixed.")
            }
            FieldState::Unsure(probs) => {
                probs.remove(&assume_to_be);
            }
        }
        SolverState {
            state,
            action: SolverStateAction::RuleOut {
                activation: assume_to_be,
                position,
            },
        }
    }
}
