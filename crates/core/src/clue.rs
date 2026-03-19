use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    geometry::{Offset, TOUCHING_NEIGHBOR_OFFSETS},
    types::{Answer, Name, Role},
};

#[rustfmt::skip]
pub const NONSENSE_TEXTS: [&str; 28] = [
    "I taught my goldfish to play chess.",
    "My socks filed a formal complaint today.",
    "I scheduled a meeting with my shadow.",
    "My umbrella only works on sunny days.",
    "I ironed my cereal this morning by accident.",
    "The curtains gossip whenever I leave the room.",
    "The clock asked for a day off this morning.",
    "I charged my houseplant but it still won't start.",
    "I updated my toaster and now it speaks French.",
    "My WiFi works better when I compliment it.",
    "My GPS keeps suggesting I move permanently.",
    "My voicemail started answering back with follow-up questions.",
    "My phone ran out of battery so I bought a new one.",
    "I left my car on read and now it won't start.",
    "My fitness tracker is disappointed but not surprised.",
    "My autocorrect has started winning the arguments.",
    "I put my to-do list on my to-do list.",
    "I asked Siri for directions and she sighed first.",
    "The roomba mapped my apartment and found it lacking.",
    "My spam folder has better conversations than I do.",
    "My laptop fan kicks in whenever I open my budget.",
    "The ATM laughed before showing me my balance.",
    "The loading bar finished but I wasn't ready yet.",
    "My smart speaker pretends not to hear me sometimes.",
    "I deleted the app but the app did not delete me.",
    "The captcha asked if I was a robot and I hesitated.",
    "My phone died mid-excuse and honestly it was convincing.",
    "My phone autocorrected my name and I considered keeping it.",
];

#[rustfmt::skip]
pub const CRIMINAL_NONSENSE_TEXTS: [&str; 26] = [
    "I stole the spotlight and I'm not giving it back.",
    "I plead the fifth but my face pled guilty.",
    "My getaway car is double parked right now.",
    "My lawyer said to stop helping the prosecution.",
    "I returned to the scene because I forgot my keys.",
    "The heist went perfectly except for everything after it.",
    "I robbed a bank and they offered me a credit card.",
    "My mugshot is my best photo and that says a lot.",
    "I laundered my money with my actual laundry accidentally.",
    "The witness described me as suspiciously average looking.",
    "I wrote my ransom note in comic sans by mistake.",
    "My ankle monitor gets better reception than my phone does.",
    "I stole a calendar and got twelve months for it.",
    "My criminal record has better continuity than my resume.",
    "The detective and I are on a first name basis.",
    "My accomplice left a Yelp review of the heist.",
    "The security camera caught my good side for once.",
    "I asked for a lawyer and they sent my ex.",
    "The interrogation lasted longer than my last relationship did.",
    "I confessed because the silence was getting awkward honestly.",
    "I got caught because I stopped to pet the dog.",
    "The police sketch artist really captured my essence though.",
    "I turned myself in because the line was shorter.",
    "The evidence locker has more of my stuff than I do.",
    "I robbed the wrong place and still got employee of the month.",
    "The undercover cop was the most interesting person at my party.",
];

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
pub enum Column {
    A,
    B,
    C,
    D,
}

impl Column {
    pub const fn index(self) -> u8 {
        match self {
            Self::A => 0,
            Self::B => 1,
            Self::C => 2,
            Self::D => 3,
        }
    }
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::A => f.write_str("A"),
            Self::B => f.write_str("B"),
            Self::C => f.write_str("C"),
            Self::D => f.write_str("D"),
        }
    }
}

const fn display_row(row: u8) -> u8 {
    row.saturating_add(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Line {
    Row(u8),
    Col(Column),
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Row(row) => write!(f, "row {}", display_row(*row)),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CellSelector {
    Board,
    Neighbor { name: Name },
    Direction { name: Name, direction: Direction },
    Row { row: u8 },
    Col { col: Column },
    Between { first_name: Name, second_name: Name },
    SharedNeighbor { first_name: Name, second_name: Name },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Count {
    Number(i32),
    AtLeast(i32),
    Parity(Parity),
}

impl Count {
    fn describe(self, noun: &str) -> String {
        match self {
            Self::Number(number) => format!("{number} {noun}"),
            Self::AtLeast(number) => format!("at least {number} {noun}"),
            Self::Parity(Parity::Odd) => format!("an odd number of {noun}"),
            Self::Parity(Parity::Even) => format!("an even number of {noun}"),
        }
    }
}

impl CellSelector {
    fn text(&self, answer: Answer, count: Count, filter: CellFilter) -> String {
        match self {
            Self::Board => format!(
                "there are {}{}",
                count.describe(&format!("{answer}s")),
                filter.suffix(),
            ),
            Self::Neighbor { name } => format!(
                "{name} has {} neighbors{}",
                count.describe(&answer.to_string()),
                filter.suffix(),
            ),
            Self::Direction { name, direction } => format!(
                "there are {} {direction} {name}{}",
                count.describe(&format!("{answer}s")),
                filter.suffix(),
            ),
            Self::Row { row } => format!(
                "Row {} has {}{}",
                display_row(*row),
                count.describe(&format!("{answer}s")),
                filter.suffix(),
            ),
            Self::Col { col } => format!(
                "Col {col} has {}{}",
                count.describe(&format!("{answer}s")),
                filter.suffix(),
            ),
            Self::Between {
                first_name,
                second_name,
            } => format!(
                "there are {} between {first_name} and {second_name}{}",
                count.describe(&format!("{answer}s")),
                filter.suffix(),
            ),
            Self::SharedNeighbor {
                first_name,
                second_name,
            } => format!(
                "{first_name} and {second_name} share {} neighbors{}",
                count.describe(&answer.to_string()),
                filter.suffix(),
            ),
        }
    }

    pub const fn neighbor_offsets(&self) -> &'static [Offset] {
        match self {
            Self::Board => &[],
            Self::Neighbor { .. } | Self::SharedNeighbor { .. } => &TOUCHING_NEIGHBOR_OFFSETS,
            Self::Direction { .. } | Self::Row { .. } | Self::Col { .. } | Self::Between { .. } => {
                &[]
            }
        }
    }

    pub const fn direction_offset(&self) -> Option<Offset> {
        match self {
            Self::Direction { direction, .. } => Some(direction.offset()),
            Self::Board
            | Self::Neighbor { .. }
            | Self::Row { .. }
            | Self::Col { .. }
            | Self::Between { .. }
            | Self::SharedNeighbor { .. } => None,
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

    fn describe_in(self, noun: &str, first_scope: &str, second_scope: &str) -> String {
        match self {
            Self::More => format!("there are more {noun} in {first_scope} than in {second_scope}"),
            Self::Fewer => {
                format!("there are fewer {noun} in {first_scope} than in {second_scope}")
            }
            Self::Equal => {
                format!("there are as many {noun} in {first_scope} as in {second_scope}")
            }
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

fn singular_role(role: &str) -> String {
    role.to_lowercase()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PersonGroup {
    Any,
    Filter { filter: CellFilter },
    Line { line: Line },
    Role { role: Role },
}

impl PersonGroup {
    fn text(&self, singular: bool) -> String {
        match self {
            Self::Any => {
                if singular {
                    "person".to_string()
                } else {
                    "people".to_string()
                }
            }
            Self::Filter {
                filter: CellFilter::Any,
            } => {
                if singular {
                    "person".to_string()
                } else {
                    "people".to_string()
                }
            }
            Self::Filter {
                filter: CellFilter::Edge,
            } => {
                if singular {
                    "person on the edges".to_string()
                } else {
                    "people on the edges".to_string()
                }
            }
            Self::Filter {
                filter: CellFilter::Corner,
            } => {
                if singular {
                    "person in the corners".to_string()
                } else {
                    "people in the corners".to_string()
                }
            }
            Self::Line { line } => {
                if singular {
                    format!("person in {line}")
                } else {
                    format!("people in {line}")
                }
            }
            Self::Role { role } => {
                if singular {
                    singular_role(role)
                } else {
                    pluralize_role(role)
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PersonPredicate {
    Neighbor {
        answer: Answer,
        count: Count,
        filter: CellFilter,
    },
    DirectRelation {
        answer: Answer,
        direction: Direction,
    },
}

impl PersonPredicate {
    fn text(&self, singular: bool) -> String {
        match self {
            Self::Neighbor {
                answer,
                count,
                filter,
            } => format!(
                "{} {} neighbors{}",
                if singular { "has" } else { "have" },
                count.describe(&answer.to_string()),
                filter.suffix(),
            ),
            Self::DirectRelation { answer, direction } => format!(
                "{} directly {direction} {}",
                if singular { "is" } else { "are" },
                answer_with_article(*answer),
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Quantifier {
    Exactly(i32),
}

impl Quantifier {
    fn text(self) -> String {
        match self {
            Self::Exactly(count) => format!("Exactly {count}"),
        }
    }

    fn is_singular(self) -> bool {
        match self {
            Self::Exactly(1) => true,
            Self::Exactly(_) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Clue {
    Nonsense {
        text: String,
    },
    CountCells {
        selector: CellSelector,
        answer: Answer,
        count: Count,
        filter: CellFilter,
    },
    Connected {
        answer: Answer,
        line: Line,
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
    LineComparison {
        first_line: Line,
        second_line: Line,
        answer: Answer,
        comparison: Comparison,
    },
    Quantified {
        quantifier: Quantifier,
        group: PersonGroup,
        predicate: PersonPredicate,
    },
}

impl Clue {
    pub fn text(&self) -> String {
        match self {
            Self::Nonsense { text } => text.clone(),
            Self::CountCells {
                selector,
                answer,
                count,
                filter,
            } => selector.text(*answer, *count, *filter),
            Self::Connected { answer, line } => {
                format!("all {answer}s in {line} are connected")
            }
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
            Self::LineComparison {
                first_line,
                second_line,
                answer,
                comparison,
            } => comparison.describe_in(
                &format!("{answer}s"),
                &first_line.to_string(),
                &second_line.to_string(),
            ),
            Self::Quantified {
                quantifier,
                group,
                predicate,
            } => format!(
                "{} {} {}",
                quantifier.text(),
                group.text(quantifier.is_singular()),
                predicate.text(quantifier.is_singular()),
            ),
        }
    }

    pub const fn neighbor_offsets(&self) -> &'static [Offset] {
        match self {
            Self::Nonsense { .. } => &[],
            Self::CountCells { selector, .. } => selector.neighbor_offsets(),
            Self::Connected { .. }
            | Self::DirectRelation { .. }
            | Self::RoleCount { .. }
            | Self::RolesComparison { .. }
            | Self::LineComparison { .. }
            | Self::Quantified { .. } => &[],
        }
    }

    pub const fn direction_offset(&self) -> Option<Offset> {
        match self {
            Self::Nonsense { .. } => None,
            Self::CountCells { selector, .. } => selector.direction_offset(),
            Self::DirectRelation { direction, .. } => Some(direction.offset()),
            Self::Connected { .. }
            | Self::RoleCount { .. }
            | Self::RolesComparison { .. }
            | Self::LineComparison { .. }
            | Self::Quantified { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CellFilter, CellSelector, Clue, Column, Comparison, Count, Direction, Line, NONSENSE_TEXTS,
        Parity, PersonGroup, PersonPredicate, Quantifier,
    };
    use crate::{geometry::Offset, types::Answer};

    #[test]
    fn neighbor_renders_number_puzzle_text() {
        let clue = Clue::CountCells {
            selector: CellSelector::Neighbor {
                name: "Ada".to_string(),
            },
            answer: Answer::Innocent,
            count: Count::Number(3),
            filter: CellFilter::Any,
        };

        assert_eq!(clue.text(), "Ada has 3 innocent neighbors");
    }

    #[test]
    fn nonsense_renders_puzzle_text() {
        let clue = Clue::Nonsense {
            text: NONSENSE_TEXTS[0].to_string(),
        };

        assert_eq!(clue.text(), NONSENSE_TEXTS[0]);
    }

    #[test]
    fn board_renders_corner_filtered_parity_puzzle_text() {
        let clue = Clue::CountCells {
            selector: CellSelector::Board,
            answer: Answer::Innocent,
            count: Count::Parity(Parity::Even),
            filter: CellFilter::Corner,
        };

        assert_eq!(
            clue.text(),
            "there are an even number of innocents in the corners"
        );
    }

    #[test]
    fn neighbor_renders_edge_filtered_parity_puzzle_text() {
        let clue = Clue::CountCells {
            selector: CellSelector::Neighbor {
                name: "Ada".to_string(),
            },
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
        let clue = Clue::CountCells {
            selector: CellSelector::Direction {
                name: "Ada".to_string(),
                direction: Direction::Below,
            },
            answer: Answer::Innocent,
            count: Count::Number(2),
            filter: CellFilter::Any,
        };

        assert_eq!(clue.text(), "there are 2 innocents below Ada");
    }

    #[test]
    fn direction_renders_parity_puzzle_text() {
        let clue = Clue::CountCells {
            selector: CellSelector::Direction {
                name: "Ada".to_string(),
                direction: Direction::Below,
            },
            answer: Answer::Innocent,
            count: Count::Parity(Parity::Even),
            filter: CellFilter::Any,
        };

        assert_eq!(
            clue.text(),
            "there are an even number of innocents below Ada"
        );
    }

    #[test]
    fn row_renders_at_least_puzzle_text() {
        let clue = Clue::CountCells {
            selector: CellSelector::Row { row: 2 },
            answer: Answer::Innocent,
            count: Count::AtLeast(2),
            filter: CellFilter::Any,
        };

        assert_eq!(clue.text(), "Row 3 has at least 2 innocents");
    }

    #[test]
    fn row_renders_edge_filtered_number_puzzle_text() {
        let clue = Clue::CountCells {
            selector: CellSelector::Row { row: 2 },
            answer: Answer::Innocent,
            count: Count::Number(2),
            filter: CellFilter::Edge,
        };

        assert_eq!(clue.text(), "Row 3 has 2 innocents on the edges");
    }

    #[test]
    fn neighbor_renders_corner_filtered_puzzle_text() {
        let clue = Clue::CountCells {
            selector: CellSelector::Neighbor {
                name: "Ada".to_string(),
            },
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
        let clue = Clue::CountCells {
            selector: CellSelector::Col { col: Column::C },
            answer: Answer::Criminal,
            count: Count::Parity(Parity::Even),
            filter: CellFilter::Any,
        };

        assert_eq!(clue.text(), "Col C has an even number of criminals");
    }

    #[test]
    fn connected_renders_puzzle_text() {
        let clue = Clue::Connected {
            answer: Answer::Criminal,
            line: Line::Row(2),
        };

        assert_eq!(clue.text(), "all criminals in row 3 are connected");
    }

    #[test]
    fn connected_col_renders_puzzle_text() {
        let clue = Clue::Connected {
            answer: Answer::Criminal,
            line: Line::Col(Column::B),
        };

        assert_eq!(clue.text(), "all criminals in col B are connected");
    }

    #[test]
    fn between_renders_puzzle_text() {
        let clue = Clue::CountCells {
            selector: CellSelector::Between {
                first_name: "Ada".to_string(),
                second_name: "Ben".to_string(),
            },
            answer: Answer::Innocent,
            count: Count::Number(2),
            filter: CellFilter::Any,
        };

        assert_eq!(clue.text(), "there are 2 innocents between Ada and Ben");
    }

    #[test]
    fn shared_neighbor_renders_puzzle_text() {
        let clue = Clue::CountCells {
            selector: CellSelector::SharedNeighbor {
                first_name: "Ada".to_string(),
                second_name: "Ben".to_string(),
            },
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
    fn line_comparison_renders_puzzle_text() {
        let clue = Clue::LineComparison {
            first_line: Line::Row(1),
            second_line: Line::Col(Column::B),
            answer: Answer::Innocent,
            comparison: Comparison::More,
        };

        assert_eq!(
            clue.text(),
            "there are more innocents in row 2 than in col B",
        );
    }

    #[test]
    fn exactly_one_corner_person_renders_puzzle_text() {
        let clue = Clue::Quantified {
            quantifier: Quantifier::Exactly(1),
            group: PersonGroup::Filter {
                filter: CellFilter::Corner,
            },
            predicate: PersonPredicate::Neighbor {
                answer: Answer::Innocent,
                count: Count::Number(2),
                filter: CellFilter::Any,
            },
        };

        assert_eq!(
            clue.text(),
            "Exactly 1 person in the corners has 2 innocent neighbors",
        );
    }

    #[test]
    fn exactly_one_role_renders_direct_relation_puzzle_text() {
        let clue = Clue::Quantified {
            quantifier: Quantifier::Exactly(1),
            group: PersonGroup::Role {
                role: "Sleuth".to_string(),
            },
            predicate: PersonPredicate::DirectRelation {
                answer: Answer::Innocent,
                direction: Direction::Left,
            },
        };

        assert_eq!(
            clue.text(),
            "Exactly 1 sleuth is directly left of an innocent",
        );
    }

    #[test]
    fn exactly_one_row_person_renders_puzzle_text() {
        let clue = Clue::Quantified {
            quantifier: Quantifier::Exactly(1),
            group: PersonGroup::Line { line: Line::Row(2) },
            predicate: PersonPredicate::Neighbor {
                answer: Answer::Innocent,
                count: Count::Number(4),
                filter: CellFilter::Any,
            },
        };

        assert_eq!(
            clue.text(),
            "Exactly 1 person in row 3 has 4 innocent neighbors",
        );
    }

    #[test]
    fn exactly_two_corner_people_render_puzzle_text() {
        let clue = Clue::Quantified {
            quantifier: Quantifier::Exactly(2),
            group: PersonGroup::Filter {
                filter: CellFilter::Corner,
            },
            predicate: PersonPredicate::Neighbor {
                answer: Answer::Innocent,
                count: Count::Number(2),
                filter: CellFilter::Any,
            },
        };

        assert_eq!(
            clue.text(),
            "Exactly 2 people in the corners have 2 innocent neighbors",
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
