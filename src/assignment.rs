use std::ops::{Index, IndexMut};

use crossterm::style::{Color, ResetColor, SetForegroundColor};
use thiserror::Error;

use crate::{activation::Activation, index::RunePosition};

#[derive(Clone)]
pub struct Assignment {
    activation_of_position: [Option<Activation>; 12],
    position_of_activation: [Option<RunePosition>; 12],
}

#[derive(Debug, Error)]
pub enum AssignmentError {
    #[error("Activation {activation} was assigned twice to {position_a} and {position_b}")]
    ActivationDoubleAssigned {
        activation: Activation,
        position_a: RunePosition,
        position_b: RunePosition,
    },
    // #[error("Position {position} was assigned twice to {activation_a} and {activation_b}")]
    // PositionDoubleAssigned {
    //     position: RunePosition,
    //     activation_a: Activation,
    //     activation_b: Activation,
    // },
}

impl Assignment {
    pub fn from_iter(
        mut assignment: impl Iterator<Item = Option<Activation>>,
    ) -> Result<Self, AssignmentError> {
        let mut target = [None; 12];
        for i in target.iter_mut() {
            *i = assignment.next().expect("iterator should be long enough");
        }
        assert!(
            assignment.next().is_none(),
            "iterator should not be too long"
        );
        Self::new(target)
    }
    pub fn new(assignment: [Option<Activation>; 12]) -> Result<Self, AssignmentError> {
        let mut position_of = [None; 12];
        for (position, a) in assignment.iter().enumerate() {
            let position = RunePosition::new(position);
            if let Some(a) = a {
                if let Some(old) = position_of[a.index()] {
                    return Err(AssignmentError::ActivationDoubleAssigned {
                        activation: *a,
                        position_a: position,
                        position_b: old,
                    });
                }
                position_of[a.index()] = Some(position);
            }
        }

        Ok(Self {
            activation_of_position: assignment,
            position_of_activation: position_of,
        })
    }

    pub fn position_of(&self, number: Activation) -> Option<RunePosition> {
        self.position_of_activation[number.index()]
    }

    pub fn assign(&mut self, position: RunePosition, activation: Activation) {
        if let Some(old) = self.position_of_activation[activation.index()] {
            self.activation_of_position[old] = None;
        }
        if let Some(old) = self.activation_of_position[position] {
            self.position_of_activation[old.index()] = None;
        }
        self.position_of_activation[activation.index()] = Some(position);
        self.activation_of_position[position] = Some(activation);
    }

    pub fn contains(&self, a: Activation) -> bool {
        self.activation_of_position.iter().any(|it| *it == Some(a))
    }
}

impl Index<RunePosition> for Assignment {
    type Output = Option<Activation>;

    fn index(&self, index: RunePosition) -> &Self::Output {
        &self.activation_of_position[index]
    }
}
impl IndexMut<RunePosition> for Assignment {
    fn index_mut(&mut self, index: RunePosition) -> &mut Self::Output {
        &mut self.activation_of_position[index]
    }
}

pub fn print_assignment(assignment: &Assignment) {
    let assignment: Vec<_> = assignment
        .activation_of_position
        .iter()
        .enumerate()
        .map(|(index, it)| match it {
            Some(it) => format!("{:3}", format!("{}", it)),
            None => format!(
                "{}{:3}{}",
                SetForegroundColor(Color::DarkGrey),
                index,
                ResetColor,
            ),
        })
        .collect();
    println!(
        include_str!("hexagon.txt"),
        assignment[0],
        assignment[1],
        assignment[2],
        assignment[3],
        assignment[4],
        assignment[5],
        assignment[6],
        assignment[7],
        assignment[8],
        assignment[9],
        assignment[10],
        assignment[11],
    )
}
