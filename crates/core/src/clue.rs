use serde::{Deserialize, Serialize};

use crate::{
    geometry::{Offset, TOUCHING_NEIGHBOR_OFFSETS},
    types::{Answer, Name},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Clue {
    NeighborCount {
        name: Name,
        answer: Answer,
        number: i32,
    },
}

impl Clue {
    pub fn text(&self) -> String {
        match self {
            Self::NeighborCount {
                name,
                answer,
                number,
            } => format!("{name} has {number} {answer} neighbors"),
        }
    }

    pub const fn neighbor_offsets(&self) -> &'static [Offset] {
        match self {
            Self::NeighborCount { .. } => &TOUCHING_NEIGHBOR_OFFSETS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Clue;
    use crate::types::Answer;

    #[test]
    fn neighbor_count_renders_puzzle_text() {
        let clue = Clue::NeighborCount {
            name: "Ada".to_string(),
            answer: Answer::Innocent,
            number: 3,
        };

        assert_eq!(clue.text(), "Ada has 3 innocent neighbors");
    }
}
