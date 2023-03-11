use std::collections::HashMap;

use itertools::Itertools;

use crate::{
    activation::Activation, fact_solver::ContradictionKind, index::RunePosition, rule::RuleKind,
    RuneLock,
};

use super::{
    fact_db::{FactDb, FactHandle},
    Fact, FactKind, FactReason,
};

/*
#[derive(Hash, PartialEq, Eq)]
enum ExplainedFactReason {
    Fact(ExplainedFact),
    Rule(RuleKind),
    Assumption,
}
#[derive(Hash, PartialEq, Eq)]
enum ExplainedFactOtherReason {
    Rule(RuleKind),
    Assumption,
}
impl ExplainedFactReason {
    fn from_reason(it: &FactReason, facts: &FactDb, lock: &RuneLock) -> Self {
        match it {
            FactReason::Fact(handle, _) => ExplainedFactReason::Fact(ExplainedFact::from(
                facts.get(*handle).unwrap(),
                facts,
                lock,
            )),
            FactReason::Rule(rule) => ExplainedFactReason::Rule(lock.rules[*rule].clone()),
            FactReason::Assumption => ExplainedFactReason::Assumption,
        }
    }
}

enum SimilarFacts {
    Activation {
        activation: Activation,
        positions: Vec<RunePosition>,
        reasons: Vec<ExplainedFactReason>,
    },
}

#[derive(PartialEq, Eq, Hash)]
struct FactReasonCollection {
    facts: HashMap<Vec<ExplainedFactReason>, ExplainedFact>,
}

impl FactReasonCollection {
    fn from_reasons(fact: Vec<Fact>, facts: &FactDb, lock: &RuneLock) -> Self {
        let mut map = HashMap::new();
        for f in fact {
            let reasons: Vec<ExplainedFactReason> = f
                .reasons
                .iter()
                .filter_map(|handle| match handle {
                    FactReason::Fact(fact, _) => Some(ExplainedFactReason::Fact(
                        ExplainedFact::from(&f, facts, lock),
                    )),
                    FactReason::Rule(rule) => Some(ExplainedFactReason::Rule(lock.rules[*rule])),
                    FactReason::Assumption => Some(ExplainedFactReason::Assumption),
                })
                .collect();
            map.insert(reasons, ExplainedFact::from(&f, facts, lock));
        }
        Self { facts: map }
    }
}

#[derive(Hash, PartialEq, Eq)]
struct ExplainedFact {
    kind: FactKind,
    activation: Activation,
    position: RunePosition,
    reasons: FactReasonCollection,
    other_reasons: Vec<ExplainedFactOtherReason>
}

impl ExplainedFact {
    fn from(fact: &Fact, facts: &FactDb, lock: &RuneLock) -> Self {
        let reasons = reasons: fact
                .reasons
                .iter()
                // .map(|it| ExplainedFactReason::from_reason(it, facts, lock))
                .map(|it| )
                .collect(),

        ExplainedFact {
            kind: fact.kind.clone(),
            activation: fact.activation.clone(),
            position: fact.position.clone(),
            reasons: fact
                .reasons
                .iter()
                .map(|it| ExplainedFactReason::from_reason(it, facts, lock))
                .collect(),
        }
    }
}
*/

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct FactExceptPosition {
    kind: FactKind,
    activation: Activation,
    reasons: Vec<FactReason>,
}

impl FactExceptPosition {
    fn from(value: &Fact) -> Self {
        Self {
            kind: value.kind,
            activation: value.activation,
            reasons: value.reasons.clone(),
        }
    }
}
pub fn explain_fact(fact_handle: FactHandle, facts: &FactDb, lock: &RuneLock) {
    explain_fact_d(fact_handle, facts, lock, 0)
}

pub fn explain_fact_d(fact_handle: FactHandle, facts: &FactDb, lock: &RuneLock, depth: usize) {
    let fact = match facts.get(fact_handle) {
        Some(i) => i,
        None => {
            println!("Unknown Fact: {}", fact_handle);
            return;
        }
    };

    let mut similar_but_position: HashMap<FactExceptPosition, Vec<(FactHandle, RunePosition)>> =
        HashMap::new();

    let fact_reasons = fact
        .reasons
        .iter()
        .filter_map(|it| {
            if let FactReason::Fact(handle, _) = it {
                Some((handle, facts.get(*handle).unwrap()))
            } else {
                None
            }
        })
        .collect_vec();

    for (handle, f) in fact_reasons {
        let fep = FactExceptPosition::from(f);
        let positions = similar_but_position
            .entry(fep)
            .or_insert_with(|| Vec::new());
        positions.push((*handle, f.position));
    }

    let inset = depth * 4;
    println!("{0}: {1}", fact_handle, fact);

    for reason in fact.reasons.iter() {
        match reason {
            FactReason::Fact(_, _) => {} //Handled Later
            FactReason::Rule(_) | FactReason::Assumption => {
                print_fact_reason(reason, facts, lock, depth);
            }
        }
    }

    for (fact, position) in similar_but_position.iter() {
        let positions = position.iter().map(|(_, p)| format!("{}", p)).join(", ");
        let verb = match fact.kind {
            FactKind::Contradiction(k) => match k {
                ContradictionKind::ContradictingRequirements => format!(
                    "{} has contradicting facts regarding position {}",
                    fact.activation, positions
                ),
                ContradictionKind::NoOptionsLeft => {
                    format!("{} has no options left to go", fact.activation)
                }
            },
            FactKind::ActivationCannotBeOn => {
                format!("{} cannot be on {}", fact.activation, positions)
            }
            FactKind::ActivationMustBeOn => format!("{} must be on {}", fact.activation, positions),
        };

        println!("{:1$} -> {2}", "", inset, verb);

        let mut reasons = fact.reasons.clone();
        reasons.sort();
        for reason in reasons {
            print_fact_reason(&reason, facts, lock, depth + 1);
        }
    }
}

fn print_fact_reason(reason: &FactReason, facts: &FactDb, lock: &RuneLock, depth: usize) {
    let inset = depth * 4;
    match reason {
        FactReason::Fact(handle, _) => {
            print!("{:1$} -> ", "", inset);
            explain_fact_d(*handle, facts, lock, depth + 1);
        }
        FactReason::Rule(rule) => {
            println!(
                "{:1$} -> Rule {2}: '{3}'",
                "", inset, rule, lock.rules[*rule]
            )
        }
        FactReason::Assumption => println!("{:1$} -> Fact Assumed.", "", inset),
    }
}
