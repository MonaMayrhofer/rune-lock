use std::fmt::Display;

use thiserror::Error;

use crate::{
    activation::Activation,
    assignment::{Assignment, AssignmentError},
    index::{RunePosition, MAX_SANTOR, MIN_SANTOR},
    rune::Rune,
    RuneLock,
};

#[derive(Debug, Clone, Copy)]
pub enum RuleKind {
    Alwanese {
        first: Activation,
        second: Activation,
    },
    AntakianConjugates {
        first: Activation,
        second: Activation,
    },
    AlwaneseConjugates {
        first: Activation,
        second: Activation,
    },
    DifferentRunes {
        first: Activation,
        second: Activation,
    },
    AntakianTwins {
        first: Activation,
        second: Activation,
    },
    IncreaseSantor {
        first: Activation,
        second: Activation,
    },
    RuneFollowsImmediately {
        first: Rune,
        second: Rune,
    },
    Max0Conductive {
        first: Activation,
        second: Activation,
    },
}

impl Display for RuleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleKind::Alwanese { first, second } => {
                write!(f, "{} & {} are Alwanese", first, second)
            }
            RuleKind::AntakianConjugates { first, second } => {
                write!(f, "{} & {} are Antakian Conjugates", first, second)
            }
            RuleKind::AlwaneseConjugates { first, second } => {
                write!(f, "{} & {} are Alwanese Conjugates", first, second)
            }
            RuleKind::DifferentRunes { first, second } => {
                write!(f, "{} & {} are Different Runes", first, second)
            }
            RuleKind::AntakianTwins { first, second } => {
                write!(f, "{} & {} are Antakian Twins", first, second)
            }
            RuleKind::IncreaseSantor { first, second } => {
                write!(f, "{} & {} increase Santor", first, second)
            }
            RuleKind::RuneFollowsImmediately { first, second } => {
                write!(f, "{} immediately follows {}", second, first)
            }
            RuleKind::Max0Conductive { first, second } => {
                write!(f, "{} & {} are max 0 Conductive", first, second)
            }
        }
    }
}

pub trait ActivationRuleKindHelpers {
    fn alwanese(self) -> RuleKind;
    fn antakian_conjugate(self) -> RuleKind;
    fn alwanese_conjugate(self) -> RuleKind;
    fn different_runes(self) -> RuleKind;
    fn antakian_twins(self) -> RuleKind;
    fn increase_santor(self) -> RuleKind;
    fn max_0_conductive(self) -> RuleKind;
}

impl ActivationRuleKindHelpers for (u8, u8) {
    fn alwanese(self) -> RuleKind {
        RuleKind::Alwanese {
            first: Activation::from_human(self.0).expect("rule activations should be valid"),
            second: Activation::from_human(self.1).expect("rule activations should be valid"),
        }
    }

    fn antakian_conjugate(self) -> RuleKind {
        RuleKind::AntakianConjugates {
            first: Activation::from_human(self.0).expect("rule activations should be valid"),
            second: Activation::from_human(self.1).expect("rule activations should be valid"),
        }
    }

    fn alwanese_conjugate(self) -> RuleKind {
        RuleKind::AlwaneseConjugates {
            first: Activation::from_human(self.0).unwrap(),
            second: Activation::from_human(self.1).unwrap(),
        }
    }

    fn different_runes(self) -> RuleKind {
        RuleKind::DifferentRunes {
            first: Activation::from_human(self.0).unwrap(),
            second: Activation::from_human(self.1).unwrap(),
        }
    }

    fn antakian_twins(self) -> RuleKind {
        RuleKind::AntakianTwins {
            first: Activation::from_human(self.0).unwrap(),
            second: Activation::from_human(self.1).unwrap(),
        }
    }

    fn increase_santor(self) -> RuleKind {
        RuleKind::IncreaseSantor {
            first: Activation::from_human(self.0).unwrap(),
            second: Activation::from_human(self.1).unwrap(),
        }
    }

    fn max_0_conductive(self) -> RuleKind {
        RuleKind::Max0Conductive {
            first: Activation::from_human(self.0).unwrap(),
            second: Activation::from_human(self.1).unwrap(),
        }
    }
}

#[derive(Debug, Error)]
pub enum RuleError {
    #[error("Rule is violated")]
    Violated,
    #[error("Rule is not fulfillable in this situation")]
    Unfulfillable,
}

#[derive(Debug, Error)]
pub enum ValidateTupleError {
    #[error("{0}")]
    InvalidAssignment(#[from] AssignmentError),
    #[error("{0}")]
    RuleError(#[from] RuleError),
}

impl RuleKind {
    pub fn validate(&self, lock: &RuneLock, assignment: &Assignment) -> Result<(), RuleError> {
        match self {
            RuleKind::Alwanese { first, second } => match (
                assignment.position_of(*first),
                assignment.position_of(*second),
            ) {
                (Some(one), Some(two)) if !two.alwanese_of(one) => Err(RuleError::Violated),
                _ => Ok(()),
            },
            RuleKind::AntakianConjugates { first, second } => match (
                assignment.position_of(*first),
                assignment.position_of(*second),
            ) {
                (None, Some(three)) if assignment[three.antiakian_conjugate()].is_some() => {
                    Err(RuleError::Unfulfillable)
                }
                (Some(two), None) if assignment[two.antiakian_conjugate()].is_some() => {
                    Err(RuleError::Unfulfillable)
                }
                (Some(one), Some(two)) if !one.antakian_conjugate_of(two) => {
                    Err(RuleError::Violated)
                }
                _ => Ok(()),
            },
            RuleKind::AlwaneseConjugates { first, second } => match (
                assignment.position_of(*first),
                assignment.position_of(*second),
            ) {
                (Some(one), Some(two)) if !one.alwanese_conjugate_of(two) => {
                    Err(RuleError::Violated)
                }
                _ => Ok(()),
            },
            RuleKind::DifferentRunes { first, second } => match (
                assignment.position_of(*first),
                assignment.position_of(*second),
            ) {
                (Some(one), Some(two)) if lock.runes[one] == lock.runes[two] => {
                    return Err(RuleError::Violated)
                }
                _ => Ok(()),
            },
            RuleKind::AntakianTwins { first, second } => match (
                assignment.position_of(*first),
                assignment.position_of(*second),
            ) {
                (Some(one), Some(two)) if !one.antakian_twins(two) => {
                    return Err(RuleError::Violated)
                }
                _ => Ok(()),
            },
            RuleKind::IncreaseSantor { first, second } => {
                match (
                    assignment.position_of(*first),
                    assignment.position_of(*second),
                ) {
                    (Some(f), Some(s)) if !f.increases_santor(s) => Err(RuleError::Violated),
                    (Some(f), None) if f.santor() == MAX_SANTOR => Err(RuleError::Unfulfillable),
                    (None, Some(s)) if s.santor() == MIN_SANTOR => Err(RuleError::Unfulfillable),
                    _ => Ok(()),
                }
            }
            RuleKind::RuneFollowsImmediately { first, second } => {
                for (position, rune) in lock.runes.iter().enumerate() {
                    let position = RunePosition::new(position);
                    if rune == first {
                        if let Some(first_assignment) = assignment[position] {
                            let next = first_assignment
                                .next()
                                .map_err(|_| RuleError::Unfulfillable)?; //TODO Check in bounds
                            let second_position = assignment.position_of(next);
                            match second_position {
                                Some(second_position) if lock.runes[second_position] != *second => {
                                    return Err(RuleError::Violated)
                                }
                                _ => {}
                            }
                        }
                    }
                }
                return Ok(());
            }
            RuleKind::Max0Conductive { first, second } => match (
                assignment.position_of(*first),
                assignment.position_of(*second),
            ) {
                (Some(one), Some(two)) if !one.max_0_conductive(two) => {
                    return Err(RuleError::Violated)
                }
                _ => Ok(()),
            },
        }
    }

    pub fn validate_tuple(
        &self,
        lock: &RuneLock,
        a: (RunePosition, Activation),
        b: (RunePosition, Activation),
    ) -> Result<(), ValidateTupleError> {
        //TODO Speed this up
        let fake_assignment = Assignment::from_tuple_iter([a, b].into_iter())?;
        self.validate(lock, &fake_assignment)?;
        Ok(())
    }
}
