use std::{collections::HashSet, fmt::Display};

use crate::{activation::Activation, assignment::Assignment, index::RunePosition, RuneLock};

#[derive(Clone)]
enum FieldState {
    Fixed(Activation),
    Unsure(HashSet<Activation>),
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
            FieldState::Fixed(a) => write!(f, "{}", a),
            FieldState::Unsure(p) => {
                write!(f, "[")?;
                for i in p.iter() {
                    write!(f, "{} ", i)?;
                }
                write!(f, "]")?;
                Ok(())
            }
        }
    }
}

#[derive(Clone)]
pub struct SolverState {
    state: [FieldState; 12],
}

impl Display for SolverState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (position, state) in self.state.iter().enumerate() {
            let position = RunePosition::new(position);
            write!(f, "{}: {}\n", position, state)?;
        }
        Ok(())
    }
}

impl Default for SolverState {
    fn default() -> Self {
        SolverState {
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
    AlreadyFixed,
    None,
    ForcedAt(RunePosition),
    ExactlyOne(RunePosition),
    MoreThanOne,
}

enum DeductionResult {
    Unsolvable,
    MadeDeductions(SolverState),
    Done,
}

impl SolverState {
    pub fn with_fixed(
        &self,
        lock: &RuneLock,
        position: RunePosition,
        to_be: Activation,
    ) -> SolverState {
        let mut new_state = self.clone();
        new_state.fix(position, to_be);
        new_state.prune_state(lock);
        new_state
    }

    pub fn fixed_assignments(&self) -> Assignment {
        let i = self.state.iter().map(|it| match it {
            FieldState::Fixed(activation) => Some(*activation),
            FieldState::Unsure(_) => None,
        });
        Assignment::from_iter(i).expect("solver should only contain valid assignment states")
    }

    fn fix(&mut self, position: RunePosition, to_be: Activation) {
        match self.state[position] {
            FieldState::Fixed(_) => panic!("Cannot double fix a field."),
            _ => {}
        }
        self.state[position] = FieldState::Fixed(to_be);
    }

    fn prune_state(&mut self, lock: &RuneLock) {
        let assignment = self.fixed_assignments();

        for (position, state) in self.state.iter_mut().enumerate() {
            let position = RunePosition::new(position);
            match state {
                FieldState::Fixed(_) => {}
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

    fn deduce(&self, lock: &RuneLock) -> DeductionResult {
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
                                    ActivationPossibility::AlreadyFixed => {
                                        ActivationPossibility::AlreadyFixed
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
                FieldState::Fixed(a) => {
                    activation_possibility[a.index()] = ActivationPossibility::AlreadyFixed
                }
            }
        }

        let mut deduced_state = self.clone();

        let mut changed = false;
        for (activation, possibilities) in activation_possibility.into_iter().enumerate() {
            let activation = Activation::new(activation as u8).unwrap();
            match possibilities {
                ActivationPossibility::ForcedAt(pos) | ActivationPossibility::ExactlyOne(pos) => {
                    changed = true;
                    deduced_state.fix(pos, activation)
                }
                ActivationPossibility::None => return DeductionResult::Unsolvable,
                _ => {}
            }
        }
        if changed {
            deduced_state.prune_state(lock);
            return DeductionResult::MadeDeductions(deduced_state);
        } else {
            return DeductionResult::Done;
        }
    }
}

enum SolverStateType {
    Assumed { state: SolverState },
    Deduced { substates: Vec<SolverState> },
}

pub struct Solver {
    states: Vec<SolverStateType>,
}

impl Solver {
    pub fn new() -> Self {
        Self {
            states: vec![SolverStateType::Assumed {
                state: SolverState::default(),
            }],
        }
    }
    pub fn fix(&mut self, lock: &RuneLock, position: RunePosition, to_be: Activation) {
        let new_state = self.peek().with_fixed(lock, position, to_be);
        self.states
            .push(SolverStateType::Assumed { state: new_state });
    }

    pub fn peek(&self) -> &SolverState {
        match self
            .states
            .last()
            .expect("the solver should never contain 0 states.")
        {
            SolverStateType::Assumed { state } => &state,
            SolverStateType::Deduced { substates } => substates
                .last()
                .expect("the deduction should contain at least one step"),
        }
    }

    pub fn back(&mut self) {
        if self.states.len() == 1 {
            panic!("Cannot pop initial state.")
        }
        self.states.pop();
    }

    pub fn iterate_deductions(&mut self, lock: &RuneLock) {
        let mut substates = Vec::new();
        let mut last = self.peek();
        loop {
            let result = last.deduce(lock);
            match result {
                DeductionResult::Unsolvable => todo!(),
                DeductionResult::MadeDeductions(deduced) => {
                    substates.push(deduced);
                    last = substates.last().unwrap();
                }
                DeductionResult::Done => break,
            }
        }
        if !substates.is_empty() {
            self.states.push(SolverStateType::Deduced { substates })
        }
    }
}
