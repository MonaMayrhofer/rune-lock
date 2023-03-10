pub mod activation;
pub mod assignment;
pub mod command;
pub mod fact_solver;
pub mod index;
pub mod rule;
pub mod rune;
pub mod solver;
pub mod solver_nodes;

use std::io;
use std::io::BufRead;

use assignment::Assignment;
use crossterm::style::Stylize;
use rule::ActivationRuleKindHelpers;
use rule::RuleKind;
use rune::Rune;
use thiserror::Error;

use crate::command::SolverCommand;
use crate::fact_solver::FactualSolver;

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
            (9, 10).antakian_twins(),
            (10, 11).increase_santor(),
            (11, 12).increase_santor(),
            (8, 10).antakian_twins(),
            (1, 12).alwanese(),
            RuleKind::RuneFollowsImmediately {
                first: Rune::new(0),
                second: Rune::new(1),
            },
        ],
    };

    let mut solver = FactualSolver::new(&lock);
    // let mut assignment = Assignment::new([None; 12]).unwrap();
    let stdin = io::stdin();

    println!("{}", "Rune Lock".red());

    // solver_ui(&solver, &lock);
    solver.display_ui();
    for line in stdin.lock().lines() {
        if let Ok(line) = line {
            //Parse Line
            let command = SolverCommand::parse(line.as_str());
            match command {
                Err(err) => println!("Didn't understand command: {}", err),
                Ok(command) => match command {
                    SolverCommand::View { node } => match solver.get_tree_handle(node) {
                        Ok(handle) => solver.set_current(handle),
                        Err(err) => println!("{}", err),
                    },
                    SolverCommand::Assume {
                        position,
                        activation,
                    } => {
                        solver.assume(activation, position);
                    }
                    SolverCommand::TryInPosition { position } => {
                        solver.try_possibilities(position);
                    }
                    SolverCommand::TryActivation { activation } => {
                        solver.try_possibilities(activation);
                    }
                    SolverCommand::Explain { fact_handle } => {
                        solver.explain(fact_handle, 10);
                    }
                    SolverCommand::Dump => solver.dump_knowledge(),
                },
            }
        } else {
            break;
        }

        solver.display_ui();
        println!("==============================");
    }
}
