use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    geometry::{Offset, TOUCHING_NEIGHBOR_OFFSETS},
    types::{Answer, Name, Role},
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
pub enum Line {
    Row(u8),
    Col(u8),
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Row(row) => write!(f, "row {row}"),
            Self::Col(col) => write!(f, "col {col}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Parity {
    Odd,
    Even,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellFilter {
    Any,
    Edge,
    Corner,
}

impl CellFilter {
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Any => "",
            Self::Edge => " on the edges",
            Self::Corner => " in the corners",
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    More,
    Fewer,
    Equal,
}

impl Comparison {
    fn describe(self, left: &str, right: &str) -> String {
        match self {
            Self::More => format!("there are more {left} than there are {right}"),
            Self::Fewer => format!("there are fewer {left} than there are {right}"),
            Self::Equal => format!("there are as many {left} as there are {right}"),
        }
    }
}

fn pluralize_role(role: &str) -> String {
    let role = role.to_lowercase();

    if role.ends_with("ch")
        || role.ends_with("sh")
        || role.ends_with('s')
        || role.ends_with('x')
        || role.ends_with('z')
    {
        format!("{role}es")
    } else {
        format!("{role}s")
    }
}

fn answer_roles(answer: Answer, role: &str) -> String {
    format!("{answer} {}", pluralize_role(role))
}

fn answer_with_article(answer: Answer) -> &'static str {
    match answer {
        Answer::Criminal => "a criminal",
        Answer::Innocent => "an innocent",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Clue {
    Neighbor {
        name: Name,
        answer: Answer,
        count: Count,
        filter: CellFilter,
    },
    Direction {
        name: Name,
        answer: Answer,
        direction: Direction,
        count: Count,
        filter: CellFilter,
    },
    Row {
        row: u8,
        answer: Answer,
        count: Count,
        filter: CellFilter,
    },
    Col {
        col: u8,
        answer: Answer,
        count: Count,
        filter: CellFilter,
    },
    Connected {
        answer: Answer,
        line: Line,
    },
    Between {
        first_name: Name,
        second_name: Name,
        answer: Answer,
        count: Count,
    },
    SharedNeighbor {
        first_name: Name,
        second_name: Name,
        answer: Answer,
        count: Count,
        filter: CellFilter,
    },
    DirectRelation {
        name: Name,
        answer: Answer,
        direction: Direction,
    },
    RoleCount {
        role: Role,
        answer: Answer,
        count: Count,
    },
    RolesComparison {
        first_role: Role,
        second_role: Role,
        answer: Answer,
        comparison: Comparison,
    },
}

impl Clue {
    pub fn text(&self) -> String {
        match self {
            Self::Neighbor {
                name,
                answer,
                count,
                filter,
            } => format!(
                "{name} has {} neighbors{}",
                count.describe(&answer.to_string()),
                filter.suffix(),
            ),
            Self::Direction {
                name,
                answer,
                direction,
                count,
                filter,
            } => format!(
                "there are {} {direction} {name}{}",
                count.describe(&format!("{answer}s")),
                filter.suffix(),
            ),
            Self::Row {
                row,
                answer,
                count,
                filter,
            } => format!(
                "Row {row} has {}{}",
                count.describe(&format!("{answer}s")),
                filter.suffix(),
            ),
            Self::Col {
                col,
                answer,
                count,
                filter,
            } => format!(
                "Col {col} has {}{}",
                count.describe(&format!("{answer}s")),
                filter.suffix(),
            ),
            Self::Connected { answer, line } => {
                format!("all {answer}s in {line} are connected")
            }
            Self::Between {
                first_name,
                second_name,
                answer,
                count,
            } => format!(
                "there are {} between {first_name} and {second_name}",
                count.describe(&format!("{answer}s")),
            ),
            Self::SharedNeighbor {
                first_name,
                second_name,
                answer,
                count,
                filter,
            } => format!(
                "{first_name} and {second_name} share {} neighbors{}",
                count.describe(&answer.to_string()),
                filter.suffix(),
            ),
            Self::DirectRelation {
                name,
                answer,
                direction,
            } => format!(
                "there is {} directly {direction} {name}",
                answer_with_article(*answer),
            ),
            Self::RoleCount {
                role,
                answer,
                count,
            } => format!("there are {}", count.describe(&answer_roles(*answer, role))),
            Self::RolesComparison {
                first_role,
                second_role,
                answer,
                comparison,
            } => comparison.describe(
                &answer_roles(*answer, first_role),
                &answer_roles(*answer, second_role),
            ),
        }
    }

    pub const fn neighbor_offsets(&self) -> &'static [Offset] {
        match self {
            Self::Neighbor { .. } | Self::SharedNeighbor { .. } => &TOUCHING_NEIGHBOR_OFFSETS,
            Self::Direction { .. }
            | Self::Row { .. }
            | Self::Col { .. }
            | Self::Connected { .. }
            | Self::Between { .. }
            | Self::DirectRelation { .. }
            | Self::RoleCount { .. }
            | Self::RolesComparison { .. } => &[],
        }
    }

    pub const fn direction_offset(&self) -> Option<Offset> {
        match self {
            Self::Direction { direction, .. } | Self::DirectRelation { direction, .. } => {
                Some(direction.offset())
            }
            Self::Neighbor { .. }
            | Self::Row { .. }
            | Self::Col { .. }
            | Self::Connected { .. }
            | Self::Between { .. }
            | Self::SharedNeighbor { .. }
            | Self::RoleCount { .. }
            | Self::RolesComparison { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CellFilter, Clue, Comparison, Count, Direction, Line, Parity};
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
            filter: CellFilter::Any,
        };

        assert_eq!(clue.text(), "Ada has 3 innocent neighbors");
    }

    #[test]
    fn neighbor_renders_edge_filtered_parity_puzzle_text() {
        let clue = Clue::Neighbor {
            name: "Ada".to_string(),
            answer: Answer::Innocent,
            count: Count::Parity(Parity::Odd),
            filter: CellFilter::Edge,
        };

        assert_eq!(
            clue.text(),
            "Ada has an odd number of innocent neighbors on the edges",
        );
    }

    #[test]
    fn direction_renders_number_puzzle_text() {
        let clue = Clue::Direction {
            name: "Ada".to_string(),
            answer: Answer::Innocent,
            direction: Direction::Below,
            count: Count::Number(2),
            filter: CellFilter::Any,
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
            filter: CellFilter::Any,
        };

        assert_eq!(clue.text(), "there are an even number of innocents below Ada");
    }

    #[test]
    fn row_renders_edge_filtered_number_puzzle_text() {
        let clue = Clue::Row {
            row: 2,
            answer: Answer::Innocent,
            count: Count::Number(2),
            filter: CellFilter::Edge,
        };

        assert_eq!(clue.text(), "Row 2 has 2 innocents on the edges");
    }

    #[test]
    fn neighbor_renders_corner_filtered_puzzle_text() {
        let clue = Clue::Neighbor {
            name: "Ada".to_string(),
            answer: Answer::Innocent,
            count: Count::Parity(Parity::Odd),
            filter: CellFilter::Corner,
        };

        assert_eq!(
            clue.text(),
            "Ada has an odd number of innocent neighbors in the corners",
        );
    }

    #[test]
    fn col_renders_parity_puzzle_text() {
        let clue = Clue::Col {
            col: 3,
            answer: Answer::Criminal,
            count: Count::Parity(Parity::Even),
            filter: CellFilter::Any,
        };

        assert_eq!(clue.text(), "Col 3 has an even number of criminals");
    }

    #[test]
    fn connected_renders_puzzle_text() {
        let clue = Clue::Connected {
            answer: Answer::Criminal,
            line: Line::Row(2),
        };

        assert_eq!(clue.text(), "all criminals in row 2 are connected");
    }

    #[test]
    fn between_renders_puzzle_text() {
        let clue = Clue::Between {
            first_name: "Ada".to_string(),
            second_name: "Ben".to_string(),
            answer: Answer::Innocent,
            count: Count::Number(2),
        };

        assert_eq!(clue.text(), "there are 2 innocents between Ada and Ben");
    }

    #[test]
    fn shared_neighbor_renders_puzzle_text() {
        let clue = Clue::SharedNeighbor {
            first_name: "Ada".to_string(),
            second_name: "Ben".to_string(),
            answer: Answer::Innocent,
            count: Count::Parity(Parity::Odd),
            filter: CellFilter::Any,
        };

        assert_eq!(
            clue.text(),
            "Ada and Ben share an odd number of innocent neighbors",
        );
    }

    #[test]
    fn direct_relation_renders_puzzle_text() {
        let clue = Clue::DirectRelation {
            name: "Ben".to_string(),
            answer: Answer::Innocent,
            direction: Direction::Left,
        };

        assert_eq!(clue.text(), "there is an innocent directly left of Ben");
    }

    #[test]
    fn role_count_renders_puzzle_text() {
        let clue = Clue::RoleCount {
            role: "Coach".to_string(),
            answer: Answer::Criminal,
            count: Count::Number(2),
        };

        assert_eq!(clue.text(), "there are 2 criminal coaches");
    }

    #[test]
    fn roles_comparison_renders_more_puzzle_text() {
        let clue = Clue::RolesComparison {
            first_role: "Coach".to_string(),
            second_role: "Coder".to_string(),
            answer: Answer::Criminal,
            comparison: Comparison::More,
        };

        assert_eq!(
            clue.text(),
            "there are more criminal coaches than there are criminal coders",
        );
    }

    #[test]
    fn roles_comparison_renders_fewer_puzzle_text() {
        let clue = Clue::RolesComparison {
            first_role: "Coach".to_string(),
            second_role: "Coder".to_string(),
            answer: Answer::Criminal,
            comparison: Comparison::Fewer,
        };

        assert_eq!(
            clue.text(),
            "there are fewer criminal coaches than there are criminal coders",
        );
    }

    #[test]
    fn roles_comparison_renders_equal_puzzle_text() {
        let clue = Clue::RolesComparison {
            first_role: "Coach".to_string(),
            second_role: "Coder".to_string(),
            answer: Answer::Criminal,
            comparison: Comparison::Equal,
        };

        assert_eq!(
            clue.text(),
            "there are as many criminal coaches as there are criminal coders",
        );
    }

    #[test]
    fn direction_has_an_offset_for_each_cardinal_direction() {
        assert_eq!(Direction::Above.offset(), Offset::new(-1, 0));
        assert_eq!(Direction::Below.offset(), Offset::new(1, 0));
        assert_eq!(Direction::Left.offset(), Offset::new(0, -1));
        assert_eq!(Direction::Right.offset(), Offset::new(0, 1));
    }
}
