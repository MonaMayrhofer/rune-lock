use std::fmt::Display;

use ndarray::{Array2, Axis};

use crate::{
    activation::Activation,
    assignment::{Assignment, AssignmentError},
    index::RunePosition,
    rule::RuleKind,
    RuneLock,
};

use super::{view::View, Fact, FactKind, FactReason};

#[derive(Clone, Debug, Copy)]
pub struct FactHandle(usize);
impl FactHandle {
    pub fn from_raw(node: usize) -> FactHandle {
        Self(node)
    }
}

#[derive(Clone)]
pub struct FactDb {
    facts: Vec<Fact>,
    fact_lookup: Array2<Option<FactHandle>>,
}

pub enum Possibilities<T> {
    None,
    Single(T),
    Multiple,
}

pub enum ConsolidationResult {
    Changes,
    Unchanged,
}

pub enum SingleFactIntegrationResult {
    Unchanged(FactHandle),
    Integrated(FactHandle),
}

pub enum FactError {
    Contradiction(FactHandle),
}

impl SingleFactIntegrationResult {
    fn expect_without_contradiction(self, db: &FactDb) -> Result<Self, FactError> {
        match self {
            SingleFactIntegrationResult::Unchanged(handle)
            | SingleFactIntegrationResult::Integrated(handle) => {
                match db.facts.get(handle.0).unwrap().kind {
                    FactKind::Contradiction => Err(FactError::Contradiction(handle.clone())),
                    FactKind::ActivationCannotBeOn | FactKind::ActivationMustBeOn => Ok(self),
                }
            }
        }
    }
}

impl<T> Possibilities<T> {
    pub fn add(&mut self, it: T) {
        *self = match self {
            Possibilities::None => Possibilities::Single(it),
            Possibilities::Single(_) => Possibilities::Multiple,
            Possibilities::Multiple => Possibilities::Multiple,
        }
    }
}

impl FactDb {
    pub fn new(runes: usize, activations: usize) -> Self {
        Self {
            facts: Vec::new(),
            fact_lookup: Array2::from_elem((runes, activations), None),
        }
    }

    pub fn integrate_and_consolidate(
        &mut self,
        fact: Fact,
        lock: &RuneLock,
    ) -> Result<(), FactError> {
        match self
            .integrate_single_fact(fact)
            .expect_without_contradiction(self)?
        {
            SingleFactIntegrationResult::Unchanged(_) => Ok(()),
            SingleFactIntegrationResult::Integrated(handle) => {
                loop {
                    let mut changed = false;
                    if let ConsolidationResult::Changes = self.consolidate_positions()? {
                        changed = true
                    }
                    if let ConsolidationResult::Changes = self.consolidate_activations()? {
                        changed = true
                    }
                    if let ConsolidationResult::Changes = self.consolidate_rules(lock)? {
                        changed = true
                    }
                    if !changed {
                        break;
                    }
                }
                Ok(())
            }
        }
        //be mapped to corresponding contradictions.
    }

    ///Only operates on the position and activation of the supplied fact.
    ///Does no global reasoning. Only updatesthe information about that singular fact that we have.
    fn integrate_single_fact(&mut self, fact: Fact) -> SingleFactIntegrationResult {
        let existing_fact = &mut self.fact_lookup[[fact.position.index(), fact.activation.index()]];

        let handle = if let Some(existing_handle) = existing_fact {
            let existing = self.facts.get(existing_handle.0).unwrap();

            //Integrate current into existing
            match (&existing.kind, &fact.kind) {
                (FactKind::ActivationCannotBeOn, FactKind::ActivationMustBeOn)
                | (FactKind::ActivationMustBeOn, FactKind::ActivationCannotBeOn) => {
                    let new_handle = FactHandle(self.facts.len());
                    self.facts.push(fact.clone());

                    let contradiction = Fact {
                        kind: FactKind::Contradiction,
                        reasons: vec![
                            FactReason::Fact(existing_handle.clone()),
                            FactReason::Fact(new_handle),
                        ],
                        ..fact
                    };
                    let contradicting_handle = FactHandle(self.facts.len());
                    self.facts.push(contradiction);

                    return SingleFactIntegrationResult::Integrated(contradicting_handle.clone());
                }
                (FactKind::ActivationMustBeOn, FactKind::ActivationMustBeOn)
                | (FactKind::ActivationCannotBeOn, FactKind::ActivationCannotBeOn) => {
                    //It already exists. Fine. (We could see which one has the shorter reasoning,
                    //but who careessss) (If we did that shorter thingy we have to take care not to
                    //run into circular reasoning with consolidate)
                    return SingleFactIntegrationResult::Unchanged(existing_handle.clone());
                }
                (FactKind::Contradiction, _) | (_, FactKind::Contradiction) => {
                    return SingleFactIntegrationResult::Unchanged(existing_handle.clone());
                }
            }
        } else {
            let handle = FactHandle(self.facts.len());
            self.facts.push(fact);
            *existing_fact = Some(handle);

            return SingleFactIntegrationResult::Integrated(handle);
        };
    }

    fn consolidate_activations(&mut self) -> Result<ConsolidationResult, FactError> {
        let mut integrations = Vec::new();
        for (activation, positions) in self.fact_lookup.columns().into_iter().enumerate() {
            let activation = Activation::new(activation as u8).unwrap();

            let must_be_fact = positions.iter().find_map(|fact| {
                if let Some(fact) = fact {
                    match self.facts.get(fact.0).unwrap().kind {
                        FactKind::ActivationMustBeOn => Some(fact),
                        _ => None,
                    }
                } else {
                    None
                }
            });

            //If there is a must-be fact, then set all others to can't be with the mustbe as the reason
            //If there are multiple-must bes the integrate_single_fact takes care of the
            //contradiction
            if let Some(must_be_fact) = must_be_fact {
                for (position, _) in positions.iter().enumerate() {
                    let position = RunePosition::new(position);
                    integrations.push(Fact {
                        kind: FactKind::ActivationCannotBeOn,
                        activation,
                        position,
                        reasons: vec![FactReason::Fact(must_be_fact.clone())],
                    });
                }
            } else {
                //If there is only one place left -> Introduce a MustBe with all other places as
                //Reasons
                let mut possibility = Possibilities::None;
                let mut reasons = Vec::new();

                for (position, fact) in positions.iter().enumerate() {
                    let position = RunePosition::new(position);
                    if let Some(fact) = fact {
                        match self.facts.get(fact.0).unwrap().kind {
                            FactKind::Contradiction => {
                                //TODO WHat ?
                            }
                            FactKind::ActivationMustBeOn => {
                                panic!("We searched earlier for MustBeOn fields and found none, and now there is one?!");
                            }
                            FactKind::ActivationCannotBeOn => {
                                reasons.push(FactReason::Fact(fact.clone()));
                            }
                        }
                    } else {
                        possibility.add(position);
                    }
                }

                if let Possibilities::Single(possibility) = possibility {
                    integrations.push(Fact {
                        kind: FactKind::ActivationMustBeOn,
                        activation,
                        position: possibility,
                        reasons,
                    });
                }
            }
        }

        self.integrate_consolidation(integrations)
    }

    fn consolidate_positions(&mut self) -> Result<ConsolidationResult, FactError> {
        let mut integrations = Vec::new();
        for (position, activations) in self.fact_lookup.rows().into_iter().enumerate() {
            let position = RunePosition::new(position);

            let must_be_fact = activations.iter().find_map(|fact| {
                if let Some(fact) = fact {
                    match self.facts.get(fact.0).unwrap().kind {
                        FactKind::ActivationMustBeOn => Some(fact),
                        _ => None,
                    }
                } else {
                    None
                }
            });

            //If there is a must-be fact, then set all others to can't be with the mustbe as the reason
            //If there are multiple-must bes the integrate_single_fact takes care of the
            //contradiction
            if let Some(must_be_fact) = must_be_fact {
                for (activation, _) in activations.iter().enumerate() {
                    let activation = Activation::new(activation as u8).unwrap();
                    integrations.push(Fact {
                        kind: FactKind::ActivationCannotBeOn,
                        activation,
                        position,
                        reasons: vec![FactReason::Fact(must_be_fact.clone())],
                    });
                }
            } else {
                //If there is only one place left -> Introduce a MustBe with all other places as
                //Reasons
                let mut possibility = Possibilities::None;
                let mut reasons = Vec::new();

                for (activation, fact) in activations.iter().enumerate() {
                    let activation = Activation::new(activation as u8).unwrap();
                    if let Some(fact) = fact {
                        match self.facts.get(fact.0).unwrap().kind {
                            FactKind::Contradiction => {
                                //TODO WHat ?
                            }
                            FactKind::ActivationMustBeOn => {
                                panic!("We searched earlier for MustBeOn fields and found none, and now there is one?!");
                            }
                            FactKind::ActivationCannotBeOn => {
                                reasons.push(FactReason::Fact(fact.clone()));
                            }
                        }
                    } else {
                        possibility.add(activation);
                    }
                }

                if let Possibilities::Single(possibility) = possibility {
                    integrations.push(Fact {
                        kind: FactKind::ActivationMustBeOn,
                        activation: possibility,
                        position,
                        reasons,
                    });
                }
            }
        }

        self.integrate_consolidation(integrations)
    }

    fn consolidate_rules(&mut self, lock: &RuneLock) -> Result<ConsolidationResult, FactError> {
        let mut integrations = Vec::new();
        //Check if the fixed_assignment is valid (We don't need to do that, as internal
        //inconsistencies will com up in the second state anyways.)

        //Second check the implications of the current assignment
        for ((position, activation), fact) in self.givens() {
            //Get all rules which affect this given
            for (rule_index, rule) in lock.rules.iter().enumerate() {
                match rule {
                    RuleKind::Alwanese { first, second }
                    | RuleKind::AntakianConjugates { first, second }
                    | RuleKind::AlwaneseConjugates { first, second }
                    | RuleKind::DifferentRunes { first, second }
                    | RuleKind::AntakianTwins { first, second }
                    | RuleKind::IncreaseSantor { first, second }
                    | RuleKind::Max0Conductive { first, second } => {
                        for (this, other) in [(first, second), (second, first)] {
                            if *this == activation {
                                for possibility in self.possibilities_for(*other) {
                                    match rule.validate_tuple(
                                        lock,
                                        (position, activation),
                                        (possibility, *other),
                                    ) {
                                        Ok(_) => {}
                                        Err(err) => match err {
                                            crate::rule::RuleError::Violated
                                            | crate::rule::RuleError::Unfulfillable => integrations
                                                .push(Fact {
                                                    kind: FactKind::ActivationCannotBeOn,
                                                    activation: *other,
                                                    position: possibility,
                                                    reasons: vec![
                                                        FactReason::Fact(fact),
                                                        FactReason::Rule(rule_index),
                                                    ],
                                                }),
                                        },
                                    }
                                }
                            }
                            //The rule has this activation as its first, so it has implications
                            //on the second.
                        }
                    }
                    RuleKind::RuneFollowsImmediately { first, second } => {
                        println!("RuneFollowsImmediately is not yet implemented for consolidation");
                    }
                }
            }
        }
        self.integrate_consolidation(integrations)
    }

    fn integrate_consolidation(
        &mut self,
        integrations: Vec<Fact>,
    ) -> Result<ConsolidationResult, FactError> {
        let mut result = ConsolidationResult::Unchanged;
        for f in integrations {
            let new_fact = self
                .integrate_single_fact(f)
                .expect_without_contradiction(self)?;
            match new_fact {
                SingleFactIntegrationResult::Unchanged(_) => {}
                SingleFactIntegrationResult::Integrated(_) => result = ConsolidationResult::Changes,
            }
        }
        Ok(result)
    }

    fn givens<'a>(&'a self) -> impl Iterator<Item = ((RunePosition, Activation), FactHandle)> + 'a {
        self.fact_lookup
            .indexed_iter()
            .filter_map(|((position, activation), fact)| {
                if let Some(fact) = fact {
                    match self.facts.get(fact.0).unwrap().kind {
                        FactKind::ActivationMustBeOn => Some((
                            (
                                RunePosition::new(position),
                                Activation::new(activation as u8).unwrap(),
                            ),
                            fact.clone(),
                        )),
                        _ => None,
                    }
                } else {
                    None
                }
            })
    }

    pub fn fixed_assignment(&self) -> Result<Assignment, AssignmentError> {
        Assignment::from_tuple_iter(self.givens().map(|it| it.0))
    }

    fn possibilities_for<'a, T: View>(
        &'a self,
        activation: T,
    ) -> impl Iterator<Item = T::Complement> + 'a {
        //TODO Test if that is the correct axis
        self.fact_lookup
            .index_axis(T::axis(), activation.index())
            .into_iter()
            .enumerate()
            .filter_map(|(position, handle)| {
                let complement = T::Complement::from_usize(position);
                if let Some(handle) = handle {
                    match self.facts.get(handle.0).unwrap().kind {
                        FactKind::Contradiction => None,
                        FactKind::ActivationCannotBeOn => None,
                        FactKind::ActivationMustBeOn => Some(complement),
                    }
                } else {
                    Some(complement)
                }
            })
    }

    pub fn explain(&self, fact_handle: FactHandle) {
        fn explain_fact(db: &FactDb, handle: FactHandle, inset: usize) {
            match db.facts.get(handle.0) {
                Some(fact) => {
                    println!("{}: {}", handle, fact);
                    for reason in fact.reasons.iter() {
                        match reason {
                            FactReason::Fact(fact) => {
                                print!("{0:1$}  -> ", "", inset);
                                explain_fact(db, fact.clone(), inset + 4)
                            }
                            FactReason::Rule(rule) => {
                                println!("{0:1$}  -> Rule {2}", "", inset, rule)
                            }
                            FactReason::Assumption => {
                                println!("{0:1$}  -> Fact is Assumed", "", inset)
                            }
                        }
                    }
                }
                None => println!("Unknown handle {}", handle),
            }
        }

        explain_fact(self, fact_handle, 0);
    }
}

impl Display for FactDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TODO")
    }
}

impl Display for FactHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "F{}", self.0)
    }
}

impl Display for Fact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verb = match self.kind {
            FactKind::Contradiction => "caused a Contradiction on",
            FactKind::ActivationCannotBeOn => "cannot be on",
            FactKind::ActivationMustBeOn => "must be on",
        };
        write!(f, "{} {} {}", self.position, verb, self.activation)
    }
}
