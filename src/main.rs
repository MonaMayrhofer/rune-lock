pub mod activation;
pub mod assignment;
pub mod index;
pub mod rule;
pub mod rune;
pub mod solver;

use std::io;
use std::io::BufRead;
use std::result;

use activation::Activation;
use assignment::{print_assignment, Assignment};
use crossterm::style::Stylize;
use rule::ActivationRuleKindHelpers;
use rule::RuleKind;
use rune::Rune;
use thiserror::Error;

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

    print_assignment(&solver.peek().fixed_assignments());
    for line in stdin.lock().lines() {
        if let Ok(line) = line {
            //Parse Line
            let (command, args) = line.split_once(' ').unwrap_or((&line[..], ""));
            match command {
                "try" => {}
                "back" => {
                    println!("Go back.");
                    if let Some(err) = solver.back().err() {
                        println!("Could not go back: {}", err);
                    }
                }
                "set" => {
                    if let Some((field, target)) = args.split_once(' ') {
                        println!("{} to {}", field, target);
                        if let Ok(field) = field.parse::<usize>() {
                            if let Ok(activation) = target.parse::<u8>() {
                                if let Ok(activation) = Activation::from_human(activation) {
                                    let position = RunePosition::new(field);
                                    if let Some(err) = solver.fix(&lock, position, activation).err()
                                    {
                                        println!("Could not fix: {}", err);
                                    } else {
                                        println!("Automatically running deductions.");
                                        let result = solver.iterate_deductions(&lock);
                                        match result {
                                            solver::DeductionResult::Unsolvable => {
                                                println!("{}", "No solution found.".red())
                                            }
                                            solver::DeductionResult::Indecisive => {
                                                println!(
                                                    "Unclear State, Manual Assumptions required."
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => println!("Unknown command."),
            }
            let fixed = solver.peek().fixed_assignments();
            print_assignment(&fixed);
            match lock.validate(&fixed) {
                Err(err) => println!("Invalid Assignment: {}", err),
                Ok(_) => println!("Valid State."),
            }
            println!("{}", solver.peek());
        } else {
            break;
        }

        println!("==============================");
    }
}
