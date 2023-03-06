use std::num::ParseIntError;

use thiserror::Error;

use crate::{
    activation::{Activation, ActivationError},
    index::RunePosition,
};

#[derive(Debug, Error)]
pub enum SolverCommandError {
    #[error("Unknown Command: {0}")]
    UnknownCommand(String),
    #[error("Not enough arguments. Expected {expected}")]
    NotEnoughArguments { expected: usize },
    #[error("Argument could not be parsed as a number: {0}")]
    NumberFormat(#[from] ParseIntError),
    #[error("Activation is invalid: {0}")]
    ActivationInvalid(#[from] ActivationError),
}
pub enum SolverCommand {
    View {
        node: usize,
    },
    Assume {
        position: RunePosition,
        activation: Activation,
    },
    TryInPosition {
        position: RunePosition,
    },
    TryActivation {
        activation: Activation,
    },
}

impl SolverCommand {
    pub fn parse(text: &str) -> Result<Self, SolverCommandError> {
        let (command, args) = text.split_once(' ').unwrap_or((text, ""));

        match command {
            "assume" => {
                let (position, activation) = args
                    .split_once(' ')
                    .ok_or(SolverCommandError::NotEnoughArguments { expected: 2 })?;
                let position = position.parse::<usize>()?;
                let activation = activation.parse::<u8>()?;
                let position = RunePosition::new(position);
                let activation = Activation::from_human(activation)?;

                Ok(SolverCommand::Assume {
                    position,
                    activation,
                })
            }
            "view" => {
                let node = args.parse::<usize>()?;
                Ok(SolverCommand::View { node })
            }
            "tryposition" => {
                let position = args.parse::<usize>()?;
                let position = RunePosition::new(position);
                Ok(Self::TryInPosition { position })
            }
            "tryactivation" => {
                let act = args.parse::<u8>()?;
                let act = Activation::from_human(act)?;
                Ok(Self::TryActivation { activation: act })
            }
            _ => Err(SolverCommandError::UnknownCommand(command.into())),
        }
    }
}
