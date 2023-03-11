use std::fmt::{Debug, Display};

use log::{debug, log_enabled};
use ndarray::{Array2, Axis};

use crate::{
    activation::Activation,
    assignment::{Assignment, AssignmentError},
    fact_solver::ContradictionKind,
    index::RunePosition,
    rule::{RuleKind, ValidateTupleError},
    RuneLock,
};

use super::{
    view::{ChooseView, View},
    DebugInfo, Fact, FactKind, FactReason,
};

#[derive(Clone, Debug, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug)]
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
                    FactKind::Contradiction(_) => Err(FactError::Contradiction(handle.clone())),
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
            SingleFactIntegrationResult::Integrated(_) => {
                loop {
                    let mut changed = false;
                    debug!("==\n==\n== Unique per RunePosition");
                    if let ConsolidationResult::Changes =
                        self.consolidate_unique_per_view::<RunePosition>()?
                    {
                        changed = true
                    }
                    if log_enabled!(log::Level::Debug) {
                        self.info_dump();
                    }
                    debug!("==\n==\n== Unique per Activation");
                    if let ConsolidationResult::Changes =
                        self.consolidate_unique_per_view::<Activation>()?
                    {
                        changed = true
                    }
                    if log_enabled!(log::Level::Debug) {
                        self.info_dump();
                    }
                    debug!("==\n==\n== Rules");
                    if let ConsolidationResult::Changes = self.consolidate_rules(lock)? {
                        changed = true
                    }
                    if log_enabled!(log::Level::Debug) {
                        self.info_dump();
                    }

                    debug!("Changes? {:?}", changed);
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

            //If the new rule already exists...
            if existing.kind == fact.kind {}

            //Integrate current into existing
            match (&existing.kind, &fact.kind) {
                //New Fact is equivalent to one that already exists
                (FactKind::ActivationMustBeOn, FactKind::ActivationMustBeOn)
                | (FactKind::Contradiction(_), FactKind::Contradiction(_))
                | (FactKind::ActivationCannotBeOn, FactKind::ActivationCannotBeOn) => {
                    //It already exists. Fine. (We could see which one has the shorter reasoning,
                    //but who careessss) (If we did that shorter thingy we have to take care not to
                    //run into circular reasoning with consolidate)
                    return SingleFactIntegrationResult::Unchanged(existing_handle.clone());
                }
                //A newcoming Contradictin overwrites All
                (_, FactKind::Contradiction(_)) => {
                    let handle = FactHandle(self.facts.len());
                    debug!("Created Contradiction {:?}: {:?}", handle, fact);
                    self.facts.push(fact);
                    *existing_fact = Some(handle);

                    return SingleFactIntegrationResult::Integrated(handle);
                }
                //An existing Contradiction cannot be overwritten
                (FactKind::Contradiction(_), _) => {
                    return SingleFactIntegrationResult::Unchanged(existing_handle.clone());
                }
                //New Fact contradicts with old Fact
                (FactKind::ActivationCannotBeOn, FactKind::ActivationMustBeOn)
                | (FactKind::ActivationMustBeOn, FactKind::ActivationCannotBeOn) => {
                    let new_handle = FactHandle(self.facts.len());
                    self.facts.push(fact.clone());

                    let contradiction = Fact {
                        kind: FactKind::Contradiction(ContradictionKind::ContradictingRequirements),
                        reasons: vec![
                            FactReason::Fact(
                                existing_handle.clone(),
                                DebugInfo {
                                    origin: "integrate_single_fact",
                                },
                            ),
                            FactReason::Fact(
                                new_handle,
                                DebugInfo {
                                    origin: "integrate_single_fact",
                                },
                            ),
                        ],
                        ..fact
                    };
                    let contradicting_handle = FactHandle(self.facts.len());
                    debug!(
                        "Created Contradiction {:?}: {}",
                        contradicting_handle, contradiction
                    );
                    self.facts.push(contradiction);

                    return SingleFactIntegrationResult::Integrated(contradicting_handle.clone());
                }
            }
        } else {
            let handle = FactHandle(self.facts.len());
            debug!("Created Fact {:?}: {:?}", handle, fact);
            self.facts.push(fact);
            *existing_fact = Some(handle);

            return SingleFactIntegrationResult::Integrated(handle);
        };
    }

    fn consolidate_unique_per_view<T: View + ChooseView>(
        &mut self,
    ) -> Result<ConsolidationResult, FactError>
    where
        T::Complement: PartialEq + Copy + Debug,
        T: Copy + Debug,
    {
        let mut integrations = Vec::new();
        for (view, complements) in self
            .fact_lookup
            .lanes(T::Complement::axis())
            .into_iter()
            .enumerate()
        {
            let view = T::from_usize(view);
            debug!("Consolidating View {:?}", view);

            let must_be_fact = complements
                .iter()
                .enumerate()
                .find_map(|(complement, fact)| {
                    let complement = T::Complement::from_usize(complement);
                    if let Some(fact) = fact {
                        match self.facts.get(fact.0).unwrap().kind {
                            FactKind::ActivationMustBeOn => Some((fact, complement)),
                            _ => None,
                        }
                    } else {
                        None
                    }
                });

            //If there is a must-be fact, then set all others to can't be with the mustbe as the reason
            //If there are multiple-must bes the integrate_single_fact takes care of the
            //contradiction
            if let Some((must_be_fact, must_be_complement)) = must_be_fact {
                for (complement, _) in complements.iter().enumerate() {
                    let complement = T::Complement::from_usize(complement);
                    //Jump over the one that has the MustBe Fact
                    if complement == must_be_complement {
                        continue;
                    }
                    integrations.push(Fact {
                        kind: FactKind::ActivationCannotBeOn,
                        activation: T::choose_activation(view, complement),
                        position: T::choose_position(view, complement),
                        reasons: vec![FactReason::Fact(
                            must_be_fact.clone(),
                            DebugInfo {
                                origin: "consolidate_views must_be_fact",
                            },
                        )],
                    });
                }
            } else {
                //If there is only one place left -> Introduce a MustBe with all other places as
                //Reasons
                let mut possibility = Possibilities::None;
                let mut reasons = Vec::new();

                for (complement, fact) in complements.iter().enumerate() {
                    let complement = T::Complement::from_usize(complement);
                    if let Some(fact) = fact {
                        match self.facts.get(fact.0).unwrap().kind {
                            FactKind::Contradiction(_) => {
                                //TODO WHat ?
                            }
                            FactKind::ActivationMustBeOn => {
                                panic!("We searched earlier for MustBeOn fields and found none, and now there is one?!");
                            }
                            FactKind::ActivationCannotBeOn => {
                                reasons.push(FactReason::Fact(
                                    fact.clone(),
                                    DebugInfo {
                                        origin: "consolidate_views only_one_place_left",
                                    },
                                ));
                            }
                        }
                    } else {
                        possibility.add(complement);
                    }
                }

                debug!("Found Possibilities: {:?}", possibility);

                match possibility {
                    Possibilities::Single(possibility) => {
                        integrations.push(Fact {
                            kind: FactKind::ActivationMustBeOn,
                            activation: T::choose_activation(view, possibility),
                            position: T::choose_position(view, possibility),
                            reasons,
                        });
                    }
                    Possibilities::None => {
                        for (complement, _) in complements.iter().enumerate() {
                            let complement = T::Complement::from_usize(complement);
                            integrations.push(Fact {
                                kind: FactKind::Contradiction(ContradictionKind::NoOptionsLeft),
                                activation: T::choose_activation(view, complement),
                                position: T::choose_position(view, complement),
                                reasons: reasons.clone(),
                            })
                        }
                    }
                    _ => {}
                }
            }
        }

        self.integrate_consolidation(integrations)
    }

    pub fn info_dump(&self) {
        println!("Current knowledge:= ======= ======");
        for (i, f) in self.facts.iter().enumerate() {
            println!("Fact {}, {:?}", i, f);
        }
        println!("[..] means Must Be, X..X means Contradiction, others mean CannotBe");
        print!("    {:3}", "");
        for i in 0..self.fact_lookup.shape()[1] {
            print!("| {:^5} ", i);
        }
        println!("");
        for (position, activations) in self
            .fact_lookup
            .lanes(Activation::axis())
            .into_iter()
            .enumerate()
        {
            print!("Pos {:3}", position);
            for (_, fact) in activations.iter().enumerate() {
                match fact {
                    Some(it) => {
                        let fact = &self.facts[it.0];
                        match fact.kind {
                            FactKind::Contradiction(_) => {
                                print!("|X{:^5}X", it.0);
                            }
                            FactKind::ActivationCannotBeOn => {
                                print!("| {:^5} ", it.0);
                            }
                            FactKind::ActivationMustBeOn => {
                                print!("|[{:^5}]", it.0);
                            }
                        }
                    }
                    None => print!("| {:^5} ", " "),
                }
            }
            println!("");
        }
    }

    fn consolidate_rules(&mut self, lock: &RuneLock) -> Result<ConsolidationResult, FactError> {
        let mut integrations = Vec::new();
        //Check if the fixed_assignment is valid (We don't need to do that, as internal
        //inconsistencies will com up in the second state anyways.)

        if log_enabled!(log::Level::Debug) {
            self.info_dump();
        }

        //Second check the implications of the current assignment
        for ((given_position, given_activation), fact) in self.givens() {
            debug!(
                "Given: {:?} {:?} through {:?}",
                given_position, given_activation, fact
            );
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
                            if *this == given_activation {
                                debug!(
                                    "Consolidating Rule: {:?} in config {:?}-{:?} for {:?}@{:?}",
                                    rule, this, other, given_activation, given_position
                                );
                                for possibility in self.possibilities_for(*other) {
                                    // possibility == position and similar shenanigans can happen,
                                    // because it might be that the previous consolidation has
                                    // inserted a fact, that hasn't had the chance to be
                                    // consolidated as well yet. Therefore Invalid Assignment
                                    // Errors can happen
                                    match rule.validate_tuple(
                                        lock,
                                        (given_position, given_activation),
                                        (possibility, *other),
                                    ) {
                                        Ok(_) => {}
                                        Err(_) => integrations.push(Fact {
                                            kind: FactKind::ActivationCannotBeOn,
                                            activation: *other,
                                            position: possibility,
                                            reasons: vec![
                                                FactReason::Fact(
                                                    fact,
                                                    DebugInfo {
                                                        origin: "consolidate_rules",
                                                    },
                                                ),
                                                FactReason::Rule(rule_index),
                                            ],
                                        }),
                                    }
                                }
                            }
                            //The rule has this activation as its first, so it has implications
                            //on the second.
                        }
                    }
                    RuleKind::RuneFollowsImmediately { first, .. } => {
                        let given_rune = lock.runes[given_position];
                        for (rune, affected_activation) in [
                            (first, given_activation.next()),
                            // (second, given_activation.prev()),
                        ] {
                            if given_rune == *rune {
                                match affected_activation {
                                    Ok(affected_activation) => {
                                        for possibility in
                                            self.possibilities_for(affected_activation)
                                        {
                                            match rule.validate_tuple(
                                                lock,
                                                (given_position, given_activation),
                                                (possibility, affected_activation),
                                            ) {
                                                Ok(_) => {}
                                                Err(_) => integrations.push(Fact {
                                                    kind: FactKind::ActivationCannotBeOn,
                                                    activation: affected_activation,
                                                    position: possibility,
                                                    reasons: vec![
                                                        FactReason::Fact(
                                                            fact,
                                                            DebugInfo {
                                                                origin: "consolidate_rules runes",
                                                            },
                                                        ),
                                                        FactReason::Rule(rule_index),
                                                    ],
                                                }),
                                            }
                                        }
                                    }
                                    Err(_) => integrations.push(Fact {
                                        kind: FactKind::ActivationCannotBeOn,
                                        activation: given_activation,
                                        position: given_position,
                                        reasons: vec![
                                            FactReason::Fact(
                                                fact,
                                                DebugInfo {
                                                    origin: "consolidate_rules runes",
                                                },
                                            ),
                                            FactReason::Rule(rule_index),
                                        ],
                                    }),
                                }
                            }
                        }
                    }
                }
            }
        }
        return self.integrate_consolidation(integrations);
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

    pub fn possibilities_for<'a, T: View + Debug>(
        &'a self,
        view: T,
    ) -> impl Iterator<Item = T::Complement> + 'a
    where
        T::Complement: Debug,
    {
        debug!("possibilities for {:?}", view);
        //TODO Test if that is the correct axis
        self.fact_lookup
            .index_axis(T::axis(), view.index())
            .into_iter()
            .enumerate()
            .filter_map(|(complement, handle)| {
                let complement = T::Complement::from_usize(complement);
                debug!("@ {:?} {:?}", complement, handle);
                if let Some(handle) = handle {
                    match self.facts.get(handle.0).unwrap().kind {
                        FactKind::Contradiction(_) => None,
                        FactKind::ActivationCannotBeOn => None,
                        FactKind::ActivationMustBeOn => Some(complement),
                    }
                } else {
                    Some(complement)
                }
            })
            .map(|it| {
                debug!(" => {:?}", it);
                it
            })
    }

    pub fn explain(&self, fact_handle: FactHandle, lock: &RuneLock, max_depth: usize) {
        fn explain_fact(
            db: &FactDb,
            lock: &RuneLock,
            handle: FactHandle,
            current_depth: usize,
            max_depth: usize,
        ) {
            if current_depth > max_depth {
                return;
            }
            match db.facts.get(handle.0) {
                Some(fact) => {
                    println!("{}: {}", handle, fact);
                    let mut reasons = fact.reasons.clone();
                    reasons.sort_by_key(|a| match a {
                        FactReason::Fact(handle, _) => 10000 + handle.0,
                        FactReason::Rule(rule) => 10 + rule,
                        FactReason::Assumption => 0,
                    });
                    for reason in reasons {
                        match reason {
                            FactReason::Fact(fact, _debug_info) => {
                                // print!("{0:1$}  -> (from {2})", "", inset, debug_info.origin);
                                if current_depth + 1 <= max_depth {
                                    print!("{0:1$}  -> ", "", current_depth * 4);
                                    explain_fact(
                                        db,
                                        lock,
                                        fact.clone(),
                                        current_depth + 1,
                                        max_depth,
                                    )
                                }
                            }
                            FactReason::Rule(rule) => {
                                println!(
                                    "{0:1$}  -> Rule {2} '{3}'",
                                    "",
                                    current_depth * 4,
                                    rule,
                                    lock.rules[rule]
                                )
                            }
                            FactReason::Assumption => {
                                println!("{0:1$}  -> Fact is Assumed", "", current_depth * 4)
                            }
                        }
                    }
                }
                None => println!("Unknown handle {}", handle),
            }
        }

        explain_fact(self, lock, fact_handle, 0, max_depth);
    }

    pub fn get(&self, fact: FactHandle) -> Option<&Fact> {
        self.facts.get(fact.0)
    }
}

impl Display for FactHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "F{}", self.0)
    }
}

impl Display for Fact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            FactKind::Contradiction(k) => match k {
                ContradictionKind::ContradictingRequirements => write!(
                    f,
                    "{} has contradicting facts regarding position {}",
                    self.activation, self.position
                ),
                ContradictionKind::NoOptionsLeft => {
                    write!(
                        f,
                        "{} or {} has no options left to go",
                        self.activation, self.position
                    )
                }
            },
            FactKind::ActivationCannotBeOn => {
                write!(f, "{} cannot be on {}", self.activation, self.position)
            }
            FactKind::ActivationMustBeOn => {
                write!(f, "{} must be on {}", self.activation, self.position)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ndarray::Axis;

    use crate::{activation::Activation, fact_solver::view::View, index::RunePosition};

    use super::FactDb;

    #[test]
    fn test_axis_lanes() {
        //We have 3 runes and 9 activations.
        let db = FactDb::new(3, 9);

        dbg!(&db.fact_lookup);

        //Lookup 2nd Rune 7th Activation
        let rune_2_activation_7 = db.fact_lookup[[2, 7]];
        dbg!(rune_2_activation_7);

        //Lookup Activations for Rune 1 (Should be 9 long)
        let activations_for_rune = db.fact_lookup.index_axis(RunePosition::axis(), 1);
        dbg!(activations_for_rune);
        assert_eq!(activations_for_rune.len(), 9);

        //Lookup Runes for Activation 1 (Should be 3 long)
        let runes_for_activation = db.fact_lookup.index_axis(Activation::axis(), 1);
        dbg!(runes_for_activation);
        assert_eq!(runes_for_activation.len(), 3);

        //Iterate Runes with their activations
        //Index is the rune, collection is of activations
        let activations_for_rune: Vec<_> = db
            .fact_lookup
            .lanes(Activation::axis())
            .into_iter()
            .enumerate()
            .collect();
        assert_eq!(activations_for_rune.len(), 3);
        assert_eq!(activations_for_rune[0].1.len(), 9);

        //Iterate Activations with their runes
        //Index is the activation, collection is of runes
        let runes_for_activation: Vec<_> = db
            .fact_lookup
            .lanes(RunePosition::axis())
            .into_iter()
            .enumerate()
            .collect();
        assert_eq!(runes_for_activation.len(), 9);
        assert_eq!(runes_for_activation[0].1.len(), 3);
    }

    #[test]
    fn test_indexed_iter() {
        let db = FactDb::new(3, 9);

        for ((a, b), _) in db.fact_lookup.indexed_iter() {
            dbg!((a, b));
        }
    }
    //We have 3 runes and 9 activations.
}
