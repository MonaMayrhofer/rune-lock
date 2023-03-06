pub mod activation;
pub mod assignment;
pub mod command;
pub mod index;
pub mod rule;
pub mod rune;
pub mod solver;
pub mod solver_nodes;

use std::io;
use std::io::BufRead;

use activation::Activation;
use assignment::{print_assignment, Assignment};
use crossterm::style::Stylize;
use rule::ActivationRuleKindHelpers;
use rule::RuleKind;
use rune::Rune;
use thiserror::Error;

use crate::command::SolverCommand;
use crate::index::RunePosition;
use crate::solver::Solver;

pub struct RuneLock {
    //Address: Outer Circle, then Inner Circle
    runes: [Rune; 12],
    rules: Vec<RuleKind>,
}

#[derive(Debug, Error)]
enum RuneLockError {
    #[error("Rule {0} was violated: {1}")]
    RuleViolated(usize, RuleKind),
    #[error("Rule {0} is not fulfillable: {1}")]
    RuleUnfulfillable(usize, RuleKind),
}

impl RuneLock {
    fn validate(&self, assignment: &Assignment) -> Result<(), RuneLockError> {
        //Build IndexOf Array
        for (rule_no, rule) in self.rules.iter().enumerate() {
            rule.validate(self, assignment).map_err(|err| match err {
                rule::RuleError::Violated => RuneLockError::RuleViolated(rule_no, *rule),
                rule::RuleError::Unfulfillable => RuneLockError::RuleUnfulfillable(rule_no, *rule),
            })?;
        }

        Ok(())
    }
}

fn solver_ui(solver: &Solver, lock: &RuneLock) {
    solver.print_nodes();
    let fixed = solver.peek().fixed_assignments();
    print_assignment(&fixed);
    match lock.validate(&fixed) {
        Err(err) => println!("Invalid Assignment: {}", err),
        Ok(_) => println!("Valid State."),
    }
    println!("{}", solver.peek());
}

fn main() {
    //Z = 0
    //V = 1
    //S = 2
    //C = 3

    let lock = RuneLock {
        runes: [
            //Outer circle
            0, 2, 1, 3, 2, 1, //Inner Circle
            3, 2, 1, 0, 2, 1,
        ]
        .map(|it| Rune::new(it)),
        rules: vec![
            (1, 2).alwanese(),
            (2, 3).antakian_conjugate(),
            (3, 4).alwanese(),
            (6, 7).alwanese_conjugate(),
            (6, 8).antakian_conjugate(),
            (7, 8).different_runes(),
            (9, 10).alwanese(),
            (10, 11).increase_santor(),
            (11, 12).increase_santor(),
            (8, 10).antakian_twins(),
            (1, 12).max_0_conductive(),
            RuleKind::RuneFollowsImmediately {
                first: Rune::new(0),
                second: Rune::new(1),
            },
        ],
    };

    let mut solver = Solver::new();
    // let mut assignment = Assignment::new([None; 12]).unwrap();
    let stdin = io::stdin();

    println!("{}", "Rune Lock".red());

    solver_ui(&solver, &lock);
    for line in stdin.lock().lines() {
        if let Ok(line) = line {
            //Parse Line
            let command = SolverCommand::parse(line.as_str());
            match command {
                Err(err) => println!("Didn't understand command: {}", err),
                Ok(command) => match command {
                    SolverCommand::View { node } => match solver.view(node) {
                        Ok(_) => {}
                        Err(err) => println!("{}", err),
                    },
                    SolverCommand::Assume {
                        position,
                        activation,
                    } => {
                        let result = solver.explore(&lock, position, activation);
                        match result {
                            Ok(result) => println!("Result: {}", result),
                            Err(err) => println!("Error: {}", err),
                        }
                    }
                },
            }
        } else {
            break;
        }

        solver_ui(&solver, &lock);
        println!("==============================");
    }
}
