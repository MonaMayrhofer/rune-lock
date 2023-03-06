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
        SolverNodeAction, SolverNodeData, SolverNodeHandle, SolverNodes, SolverNodesError,
    },
    RuneLock,
};

#[derive(Clone)]
enum FieldState {
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

pub enum ExploreResult {
    Unsolvable { reason: String },
    Indecisive,
}

impl Display for ExploreResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            ExploreResult::Unsolvable{reason} => write!(f, "This assignment turns out to be impossible. Removed it from the possibility lists. ({})", reason),
            ExploreResult::Indecisive => write!(f, "This assignment leads to another unclear situation. More assumptions required."),
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
                        deduction_chain: steps,
                        action: SolverNodeAction::Assume {
                            position,
                            activation: assume_to_be,
                        },
                    },
                );
                Ok(ExploreResult::Indecisive)
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
}
