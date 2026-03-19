use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    geometry::{Offset, TOUCHING_NEIGHBOR_OFFSETS},
    types::{Answer, Name},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Above,
    Below,
    Left,
    Right,
}

impl Direction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Above => "above",
            Self::Below => "below",
            Self::Left => "left of",
            Self::Right => "right of",
        }
    }

    pub const fn offset(self) -> Offset {
        match self {
            Self::Above => Offset::new(-1, 0),
            Self::Below => Offset::new(1, 0),
            Self::Left => Offset::new(0, -1),
            Self::Right => Offset::new(0, 1),
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Parity {
    Odd,
    Even,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Count {
    Number(i32),
    Parity(Parity),
}

impl Count {
    fn describe(self, noun: &str) -> String {
        match self {
            Self::Number(number) => format!("{number} {noun}"),
            Self::Parity(Parity::Odd) => format!("an odd number of {noun}"),
            Self::Parity(Parity::Even) => format!("an even number of {noun}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Clue {
    Neighbor {
        name: Name,
        answer: Answer,
        count: Count,
    },
    Direction {
        name: Name,
        answer: Answer,
        direction: Direction,
        count: Count,
    },
}

impl Clue {
    pub fn text(&self) -> String {
        match self {
            Self::Neighbor {
                name,
                answer,
                count,
            } => format!("{name} has {} neighbors", count.describe(&answer.to_string())),
            Self::Direction {
                name,
                answer,
                direction,
                count,
            } => format!("there are {} {direction} {name}", count.describe(&format!("{answer}s"))),
        }
    }

    pub const fn neighbor_offsets(&self) -> &'static [Offset] {
        match self {
            Self::Neighbor { .. } => &TOUCHING_NEIGHBOR_OFFSETS,
            Self::Direction { .. } => &[],
        }
    }

    pub const fn direction_offset(&self) -> Option<Offset> {
        match self {
            Self::Neighbor { .. } => None,
            Self::Direction { direction, .. } => Some(direction.offset()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Clue, Count, Direction, Parity};
    use crate::{
        geometry::Offset,
        types::Answer,
    };

    #[test]
    fn neighbor_renders_number_puzzle_text() {
        let clue = Clue::Neighbor {
            name: "Ada".to_string(),
            answer: Answer::Innocent,
            count: Count::Number(3),
        };

        assert_eq!(clue.text(), "Ada has 3 innocent neighbors");
    }

    #[test]
    fn neighbor_renders_parity_puzzle_text() {
        let clue = Clue::Neighbor {
            name: "Ada".to_string(),
            answer: Answer::Innocent,
            count: Count::Parity(Parity::Odd),
        };

        assert_eq!(clue.text(), "Ada has an odd number of innocent neighbors");
    }

    #[test]
    fn direction_renders_number_puzzle_text() {
        let clue = Clue::Direction {
            name: "Ada".to_string(),
            answer: Answer::Innocent,
            direction: Direction::Below,
            count: Count::Number(2),
        };

        assert_eq!(clue.text(), "there are 2 innocents below Ada");
    }

    #[test]
    fn direction_renders_parity_puzzle_text() {
        let clue = Clue::Direction {
            name: "Ada".to_string(),
            answer: Answer::Innocent,
            direction: Direction::Below,
            count: Count::Parity(Parity::Even),
        };

        assert_eq!(clue.text(), "there are an even number of innocents below Ada");
    }

    #[test]
    fn direction_has_an_offset_for_each_cardinal_direction() {
        assert_eq!(Direction::Above.offset(), Offset::new(-1, 0));
        assert_eq!(Direction::Below.offset(), Offset::new(1, 0));
        assert_eq!(Direction::Left.offset(), Offset::new(0, -1));
        assert_eq!(Direction::Right.offset(), Offset::new(0, 1));
    }
}
