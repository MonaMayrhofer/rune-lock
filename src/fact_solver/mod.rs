pub mod assumption_tree;
mod explainer;
pub mod fact_db;
pub mod view;

use std::fmt::{Debug, Display, Formatter};

use slotmap::{new_key_type, SlotMap};

use crate::{
    activation::Activation, fact_solver::explainer::explain_fact, index::RunePosition, RuneLock,
};

use self::{
    assumption_tree::{AssumptionTree, AssumptionTreeError, AssumptionTreeNodeHandle},
    fact_db::{FactDb, FactError::Contradiction, FactHandle},
    view::{ChooseView, View},
};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct DebugInfo {
    pub origin: &'static str,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum FactKind {
    Contradiction,
    ActivationCannotBeOn,
    ActivationMustBeOn,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
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
                Contradiction(reason) => SolverStateState::Contradicts(reason),
            },
        };
        println!(
            "================================================================ {:?}!",
            state
        );

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

    pub fn try_possibilities<T: View + Debug + ChooseView + Clone>(&mut self, it: T)
    where
        T::Complement: Debug + Clone,
    {
        let current = self.current;
        let current_facts = &self.states[current].facts;
        let possibilities: Vec<_> = current_facts.possibilities_for(it.clone()).collect();

        for possibility in possibilities {
            self.assume(
                T::choose_activation(it.clone(), possibility.clone()),
                T::choose_position(it.clone(), possibility.clone()),
            );
            self.current = current;
        }
    }

    pub fn get_tree_handle(
        &self,
        node_id: usize,
    ) -> Result<AssumptionTreeNodeHandle, AssumptionTreeError> {
        self.states.get_handle(node_id)
    }

    pub fn set_current(&mut self, new: AssumptionTreeNodeHandle) {
        self.current = new;
    }

    pub fn display_ui(&self) {
        println!("{}", self.states);
        println!("Current State: {}", self.current);
        let fixed = self.states[self.current].facts.fixed_assignment().unwrap();
        fixed.print();
        match self.lock.validate(&fixed) {
            Err(err) => println!("Invalid Assignment: {}", err),
            Ok(_) => println!("Valid State."),
        }
        println!("{}", self.states[self.current].facts);
    }

    pub fn explain(&self, fact_handle: FactHandle, max_depth: usize) {
        println!("Explaining Fact: {} in state {}", fact_handle, self.current);
        let db = &self.states[self.current].facts;
        db.explain(fact_handle, self.lock, max_depth);
        println!("============");
        explain_fact(fact_handle, &db, &self.lock);
    }

    pub fn dump_knowledge(&self) {
        self.states[self.current].facts.info_dump();
    }
}
