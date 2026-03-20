use std::collections::HashSet;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenamePuzzleCellError {
    MissingName(Name),
    DuplicateName(Name),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PuzzleValidationError {
    EmptyPuzzle,
    EmptyRow {
        row: usize,
    },
    RaggedRow {
        row: usize,
        expected: usize,
        actual: usize,
    },
    DuplicateName(Name),
}

impl Puzzle {
    pub fn rename_cell(
        &mut self,
        current_name: &str,
        updated_name: &str,
    ) -> Result<(), RenamePuzzleCellError> {
        if current_name != updated_name
            && self
                .cells
                .iter()
                .flatten()
                .any(|cell| cell.name == updated_name)
        {
            return Err(RenamePuzzleCellError::DuplicateName(
                updated_name.to_string(),
            ));
        }

        let mut renamed = false;
        for row in &mut self.cells {
            for cell in row {
                if cell.name == current_name {
                    cell.name = updated_name.to_string();
                    renamed = true;
                }

                cell.clue.rename_name_references(current_name, updated_name);
            }
        }

        if renamed {
            Ok(())
        } else {
            Err(RenamePuzzleCellError::MissingName(current_name.to_string()))
        }
    }

    pub fn validate(&self) -> Result<(), PuzzleValidationError> {
        let expected_cols = self
            .cells
            .first()
            .ok_or(PuzzleValidationError::EmptyPuzzle)?
            .len();
        if expected_cols == 0 {
            return Err(PuzzleValidationError::EmptyRow { row: 0 });
        }

        let mut seen_names = HashSet::new();

        for (row_index, row) in self.cells.iter().enumerate() {
            if row.is_empty() {
                return Err(PuzzleValidationError::EmptyRow { row: row_index });
            }

            if row.len() != expected_cols {
                return Err(PuzzleValidationError::RaggedRow {
                    row: row_index,
                    expected: expected_cols,
                    actual: row.len(),
                });
            }

            for cell in row {
                if !seen_names.insert(cell.name.clone()) {
                    return Err(PuzzleValidationError::DuplicateName(cell.name.clone()));
                }
            }
        }

        Ok(())
    }
}
