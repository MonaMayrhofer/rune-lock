pub mod assumption_tree;
pub mod fact_db;
pub mod view;

use std::fmt::{Display, Formatter};

use slotmap::{new_key_type, SlotMap};

use crate::{activation::Activation, index::RunePosition, RuneLock};

use self::{
    assumption_tree::{AssumptionTree, AssumptionTreeNodeHandle},
    fact_db::{FactDb, FactHandle},
};

#[derive(Clone, Copy, Debug)]
pub struct DebugInfo {
    pub origin: &'static str,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum FactKind {
    Contradiction,
    ActivationCannotBeOn,
    ActivationMustBeOn,
}

#[derive(Clone, Copy, Debug)]
pub enum FactReason {
    Fact(FactHandle, DebugInfo),
    Rule(usize),
    Assumption,
}

#[derive(Clone, Debug)]
pub struct Fact {
    kind: FactKind,
    activation: Activation,
    position: RunePosition,
    reasons: Vec<FactReason>,
}

#[derive(Debug, Clone, Copy)]
pub enum SolverAction {
    Assume {
        position: RunePosition,
        activation: Activation,
    },
    Root,
}

#[derive(Debug, Clone, Copy)]
pub enum SolverStateState {
    Unexplored,
    Contradicts(FactHandle),
}

impl Display for SolverAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverAction::Assume {
                position,
                activation,
            } => write!(f, "Assume {position} = {activation}")?,
            SolverAction::Root => write!(f, "Root")?,
        }
        Ok(())
    }
}

impl Display for SolverStateState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverStateState::Contradicts(fact) => write!(f, "✘ ({})", fact),
            SolverStateState::Unexplored => write!(f, " "),
            // SolverStateState::Solved => write!(f, "✔"),
        }
    }
}

#[derive(Clone)]
struct FactSolverState {
    facts: FactDb,
    action: SolverAction,
    state: SolverStateState,
}

impl Display for FactSolverState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}], {}", self.state, self.action)
    }
}

pub struct FactualSolver<'a> {
    lock: &'a RuneLock,
    states: AssumptionTree<FactSolverState>,
    current: AssumptionTreeNodeHandle,
}

impl<'a> FactualSolver<'a> {
    pub fn new(lock: &'a RuneLock) -> Self {
        let (tree, root) = AssumptionTree::new(FactSolverState {
            facts: FactDb::new(12, 12),
            action: SolverAction::Root,
            state: SolverStateState::Unexplored,
        });
        Self {
            lock,
            states: tree,
            current: root,
        }
    }

    pub fn assume(&mut self, activation: Activation, position: RunePosition) {
        let mut derived_facts = self.states[self.current].facts.clone();
        let state = match derived_facts.integrate_and_consolidate(
            Fact {
                kind: FactKind::ActivationMustBeOn {},
                reasons: vec![FactReason::Assumption],
                position,
                activation,
            },
            self.lock,
        ) {
            Ok(_) => SolverStateState::Unexplored,
            Err(err) => match err {
                fact_db::FactError::Contradiction(reason) => SolverStateState::Contradicts(reason),
            },
        };

        self.current = self.states.insert_child(
            self.current,
            FactSolverState {
                facts: derived_facts,
                action: SolverAction::Assume {
                    position,
                    activation,
                },
                state,
            },
        );
    }

    pub fn display_ui(&self) {
        println!("{}", self.states);
        let fixed = self.states[self.current].facts.fixed_assignment().unwrap();
        fixed.print();
        match self.lock.validate(&fixed) {
            Err(err) => println!("Invalid Assignment: {}", err),
            Ok(_) => println!("Valid State."),
        }
        println!("{}", self.states[self.current].facts);
    }

    pub fn explain(&self, fact_handle: FactHandle) {
        println!("Explaining Fact: {} in state {}", fact_handle, self.current);
        let db = &self.states[self.current].facts;
        db.explain(fact_handle);
    }
}
