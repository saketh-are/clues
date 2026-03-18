use serde::{Deserialize, Serialize};

use crate::{
    clue::Clue,
    types::{Answer, Name, Role},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Hidden,
    Revealed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub name: Name,
    pub role: Role,
    pub clue: Clue,
    pub answer: Answer,
    pub state: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Puzzle {
    pub cells: Vec<Vec<Cell>>,
}
