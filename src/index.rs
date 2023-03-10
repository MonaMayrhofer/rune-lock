use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub struct RunePosition(usize);

impl Display for RunePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

//This expects both elements of the "vertical" sector to have a higher santor than
//the elements in the side sectors - which works if the rings are equally spaced.
pub const SANTOR: [u32; 12] = [
    //Outer Circle
    7, 5, 2, 0, 2, 5, //Inner Circle
    6, 4, 3, 1, 3, 4,
];
pub const MAX_SANTOR: u32 = 7;
pub const MIN_SANTOR: u32 = 7;

impl RunePosition {
    pub fn new(index: usize) -> Self {
        assert!(index < 12);
        Self(index)
    }

    pub fn antiakian_conjugate(&self) -> RunePosition {
        if self.0 < 6 {
            return RunePosition::new((self.0 + 3) % 6);
        } else {
            return RunePosition::new((self.0 + 3) % 6 + 6);
        }
    }

    pub fn antakian_conjugate_of(&self, other: RunePosition) -> bool {
        self.antakian_twins(other) && self.alwanese_conjugate_of(other)
    }

    pub fn alwanese_of(&self, other: RunePosition) -> bool {
        let distance = (12 + self.0 - other.0).rem_euclid(6);
        distance <= 2 && distance > 0
    }

    pub fn alwanese_conjugate_of(&self, other: RunePosition) -> bool {
        self.0 % 6 == (other.0 + 3) % 6
    }

    pub fn antakian_twins(&self, other: RunePosition) -> bool {
        (self.0 < 6) == (other.0 < 6)
    }

    pub fn increases_santor(&self, other: RunePosition) -> bool {
        self.santor() < other.santor()
    }

    pub fn santor(&self) -> u32 {
        SANTOR[self.0]
    }

    pub fn max_0_conductive(&self, two: RunePosition) -> bool {
        return match self.antakian_twins(two) {
            true => (self.0 + 1) % 6 == two.0 % 6 || (two.0 + 1) % 6 == self.0 % 6,
            false => (self.0 + 6) % 12 == two.0,
        };
    }

    pub fn index(&self) -> usize {
        self.0
    }
}

impl<T> Index<RunePosition> for [T; 12] {
    type Output = <Self as Index<usize>>::Output;

    fn index(&self, index: RunePosition) -> &Self::Output {
        self.index(index.0)
    }
}

impl<T> IndexMut<RunePosition> for [T; 12] {
    fn index_mut(&mut self, index: RunePosition) -> &mut Self::Output {
        self.index_mut(index.0)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use itertools::Itertools;

    use super::RunePosition;

    fn test_pairs(
        pass: HashSet<(usize, usize)>,
        test: impl Fn(RunePosition, RunePosition) -> bool,
    ) {
        for a in 0..12 {
            for b in 0..12 {
                let expected = pass.get(&(a, b)).is_some();
                let actual = test(RunePosition::new(a), RunePosition::new(b));
                assert_eq!(
                    actual,
                    expected,
                    "Expected operation to be {} for {:?}",
                    expected,
                    (a, b)
                );
            }
        }
    }

    #[test]
    pub fn test_antakian_conjugates() {
        test_pairs(
            HashSet::from([
                (0, 3),
                (1, 4),
                (2, 5),
                (3, 0),
                (4, 1),
                (5, 2),
                (6, 9),
                (7, 10),
                (8, 11),
                (9, 6),
                (10, 7),
                (11, 8),
            ]),
            |a, b| a.antakian_conjugate_of(b),
        );
    }

    #[test]
    pub fn test_alwanese_of() {
        test_pairs(
            HashSet::from([
                (0, 1),
                (0, 2),
                (0, 7),
                (0, 8),
                (6, 1),
                (6, 2),
                (6, 7),
                (6, 8),
                (1, 2),
                (1, 3),
                (1, 8),
                (1, 9),
                (7, 2),
                (7, 3),
                (7, 8),
                (7, 9),
                (2, 3),
                (2, 4),
                (2, 9),
                (2, 10),
                (8, 3),
                (8, 4),
                (8, 9),
                (8, 10),
                (3, 4),
                (3, 5),
                (3, 10),
                (3, 11),
                (9, 4),
                (9, 5),
                (9, 10),
                (9, 11),
                (4, 5),
                (4, 0),
                (4, 11),
                (4, 6),
                (10, 5),
                (10, 0),
                (10, 11),
                (10, 6),
                (5, 0),
                (5, 1),
                (5, 6),
                (5, 7),
                (11, 0),
                (11, 1),
                (11, 6),
                (11, 7),
            ]),
            |a, b| b.alwanese_of(a),
        );
    }

    #[test]
    pub fn test_alwanese_conjugates() {
        test_pairs(
            HashSet::from([
                (0, 3),
                (0, 9),
                (1, 4),
                (1, 10),
                (2, 5),
                (2, 11),
                (3, 0),
                (3, 6),
                (4, 1),
                (4, 7),
                (5, 2),
                (5, 8),
                (6, 9),
                (6, 3),
                (7, 10),
                (7, 4),
                (8, 11),
                (8, 5),
                (9, 6),
                (9, 0),
                (10, 7),
                (10, 1),
                (11, 8),
                (11, 2),
            ]),
            |a, b| a.alwanese_conjugate_of(b),
        );
    }

    #[test]
    pub fn test_antakian_twins() {
        test_pairs(
            HashSet::from_iter(
                (0..6)
                    .into_iter()
                    .permutations(2)
                    .map(|it| (it[0], it[1]))
                    .chain((6..12).permutations(2).map(|it| (it[0], it[1])))
                    .chain((0..12).map(|it| (it, it))),
            ),
            |a, b| a.antakian_twins(b),
        );
    }

    #[test]
    pub fn test_conductivity() {
        test_pairs(
            HashSet::from_iter(
                (0..6)
                    .into_iter()
                    .flat_map(|num| [(num, (num + 1) % 6), ((num + 1) % 6, num)])
                    .chain((0..6).into_iter().flat_map(|num| {
                        [(num + 6, (num + 1) % 6 + 6), ((num + 1) % 6 + 6, num + 6)]
                    }))
                    .chain((0..12).into_iter().map(|num| (num, (num + 6) % 12))),
            ),
            |a, b| a.max_0_conductive(b),
        );
    }
}
