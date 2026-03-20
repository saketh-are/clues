use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    clue::Clue,
    geometry::BoardShape,
    puzzle::{Cell, Puzzle, Visibility},
    types::{Answer, Name},
};

/// Versioned persistence entrypoint for authored puzzles.
///
/// `StoredPuzzleV1` should be treated as immutable once real puzzle documents
/// depend on it. Additive fields that older readers can safely ignore are
/// acceptable, but semantic or structural changes should land in `V2`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "version", content = "puzzle", rename_all = "snake_case")]
pub enum StoredPuzzle {
    V1(StoredPuzzleV1),
}

/// Shareable authored puzzle definition.
///
/// This is intentionally separate from the runtime `Puzzle` type:
/// - positions are explicit instead of inferred from nested array order
/// - clues refer to cells by `name`, so editors must rewrite clue references
///   when a person is renamed
/// - authored reveal state is represented as `initially_revealed`
/// - no player progress or generator/debug fields live here
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredPuzzleV1 {
    pub board: BoardShape,
    pub cells: Vec<StoredCellV1>,
}

/// Zero-based row/column position inside the stored board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StoredPositionV1 {
    pub row: u8,
    pub col: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameStoredCellError {
    MissingName(Name),
    DuplicateName(Name),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredPuzzleConversionError {
    CellCountMismatch { expected: usize, actual: usize },
    PositionOutOfBounds(StoredPositionV1),
    DuplicatePosition(StoredPositionV1),
    MissingPosition(StoredPositionV1),
    DuplicateName(Name),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredCellV1 {
    pub position: StoredPositionV1,
    pub name: String,
    pub role: String,
    pub clue: Clue,
    pub answer: Answer,
    #[serde(default)]
    pub initially_revealed: bool,
}

impl StoredPuzzleV1 {
    pub fn rename_cell(
        &mut self,
        current_name: &str,
        updated_name: &str,
    ) -> Result<(), RenameStoredCellError> {
        if current_name != updated_name && self.cells.iter().any(|cell| cell.name == updated_name) {
            return Err(RenameStoredCellError::DuplicateName(
                updated_name.to_string(),
            ));
        }

        let mut renamed = false;
        for cell in &mut self.cells {
            if cell.name == current_name {
                cell.name = updated_name.to_string();
                renamed = true;
            }

            cell.clue.rename_name_references(current_name, updated_name);
        }

        if renamed {
            Ok(())
        } else {
            Err(RenameStoredCellError::MissingName(current_name.to_string()))
        }
    }
}

impl From<&Puzzle> for StoredPuzzleV1 {
    fn from(puzzle: &Puzzle) -> Self {
        let board = BoardShape::new(
            puzzle.cells.len() as u8,
            puzzle
                .cells
                .first()
                .map(|row| row.len())
                .unwrap_or_default() as u8,
        );
        let cells = puzzle
            .cells
            .iter()
            .enumerate()
            .flat_map(|(row_index, row)| {
                row.iter()
                    .enumerate()
                    .map(move |(col_index, cell)| StoredCellV1 {
                        position: StoredPositionV1 {
                            row: row_index as u8,
                            col: col_index as u8,
                        },
                        name: cell.name.clone(),
                        role: cell.role.clone(),
                        clue: cell.clue.clone(),
                        answer: cell.answer,
                        initially_revealed: cell.state == Visibility::Revealed,
                    })
            })
            .collect();

        Self { board, cells }
    }
}

impl From<&Puzzle> for StoredPuzzle {
    fn from(puzzle: &Puzzle) -> Self {
        Self::V1(StoredPuzzleV1::from(puzzle))
    }
}

impl TryFrom<StoredPuzzleV1> for Puzzle {
    type Error = StoredPuzzleConversionError;

    fn try_from(stored: StoredPuzzleV1) -> Result<Self, Self::Error> {
        let StoredPuzzleV1 { board, cells } = stored;
        let expected_cell_count = board.rows as usize * board.cols as usize;
        if cells.len() != expected_cell_count {
            return Err(StoredPuzzleConversionError::CellCountMismatch {
                expected: expected_cell_count,
                actual: cells.len(),
            });
        }

        let mut grid = vec![vec![None; board.cols as usize]; board.rows as usize];
        let mut seen_names = HashSet::with_capacity(expected_cell_count);

        for stored_cell in cells {
            let StoredCellV1 {
                position,
                name,
                role,
                clue,
                answer,
                initially_revealed,
            } = stored_cell;

            if position.row >= board.rows || position.col >= board.cols {
                return Err(StoredPuzzleConversionError::PositionOutOfBounds(position));
            }

            if !seen_names.insert(name.clone()) {
                return Err(StoredPuzzleConversionError::DuplicateName(name));
            }

            let slot = &mut grid[position.row as usize][position.col as usize];
            if slot.is_some() {
                return Err(StoredPuzzleConversionError::DuplicatePosition(position));
            }

            *slot = Some(Cell {
                name,
                role,
                clue,
                answer,
                state: if initially_revealed {
                    Visibility::Revealed
                } else {
                    Visibility::Hidden
                },
            });
        }

        let cells = grid
            .into_iter()
            .enumerate()
            .map(|(row_index, row)| {
                row.into_iter()
                    .enumerate()
                    .map(|(col_index, cell)| {
                        cell.ok_or(StoredPuzzleConversionError::MissingPosition(
                            StoredPositionV1 {
                                row: row_index as u8,
                                col: col_index as u8,
                            },
                        ))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Puzzle { cells })
    }
}

impl TryFrom<StoredPuzzle> for Puzzle {
    type Error = StoredPuzzleConversionError;

    fn try_from(stored: StoredPuzzle) -> Result<Self, Self::Error> {
        match stored {
            StoredPuzzle::V1(puzzle) => puzzle.try_into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::{
        clue::{CellFilter, CellSelector, Clue, Count, Direction},
        geometry::BoardShape,
        puzzle::{Cell, Puzzle, Visibility},
        types::Answer,
    };

    use super::{
        RenameStoredCellError, StoredCellV1, StoredPositionV1, StoredPuzzle,
        StoredPuzzleConversionError, StoredPuzzleV1,
    };

    #[test]
    fn stored_puzzle_v1_serializes_with_explicit_version() {
        let puzzle = StoredPuzzle::V1(StoredPuzzleV1 {
            board: BoardShape::new(2, 2),
            cells: vec![
                StoredCellV1 {
                    position: StoredPositionV1 { row: 0, col: 0 },
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    clue: Clue::CountCells {
                        selector: CellSelector::Neighbor {
                            name: "Ben".to_string(),
                        },
                        answer: Answer::Innocent,
                        count: Count::Number(3),
                        filter: CellFilter::Any,
                    },
                    answer: Answer::Criminal,
                    initially_revealed: true,
                },
                StoredCellV1 {
                    position: StoredPositionV1 { row: 0, col: 1 },
                    name: "Ben".to_string(),
                    role: "Baker".to_string(),
                    clue: Clue::DirectRelation {
                        name: "Ada".to_string(),
                        answer: Answer::Criminal,
                        direction: Direction::Left,
                    },
                    answer: Answer::Innocent,
                    initially_revealed: false,
                },
            ],
        });

        let value = serde_json::to_value(&puzzle).unwrap();

        assert_eq!(
            value,
            json!({
                "version": "v1",
                "puzzle": {
                    "board": {
                        "rows": 2,
                        "cols": 2
                    },
                    "cells": [
                        {
                            "position": {
                                "row": 0,
                                "col": 0
                            },
                            "name": "Ada",
                            "role": "Detective",
                            "clue": {
                                "kind": "count_cells",
                                "selector": {
                                    "kind": "neighbor",
                                    "name": "Ben"
                                },
                                "answer": "innocent",
                                "count": {
                                    "kind": "number",
                                    "value": 3
                                },
                                "filter": "any"
                            },
                            "answer": "criminal",
                            "initially_revealed": true
                        },
                        {
                            "position": {
                                "row": 0,
                                "col": 1
                            },
                            "name": "Ben",
                            "role": "Baker",
                            "clue": {
                                "kind": "direct_relation",
                                "name": "Ada",
                                "answer": "criminal",
                                "direction": "left"
                            },
                            "answer": "innocent",
                            "initially_revealed": false
                        }
                    ]
                }
            }),
        );
    }

    #[test]
    fn stored_puzzle_v1_round_trips_through_json() {
        let json_value = json!({
            "version": "v1",
            "puzzle": {
                "board": { "rows": 1, "cols": 1 },
                "cells": [
                    {
                        "position": { "row": 0, "col": 0 },
                        "name": "Solo",
                        "role": "Guard",
                        "clue": {
                            "kind": "nonsense",
                            "text": "This is nonsense."
                        },
                        "answer": "innocent",
                        "initially_revealed": false
                    }
                ]
            }
        });

        let puzzle: StoredPuzzle = serde_json::from_value(json_value.clone()).unwrap();
        let reserialized: Value = serde_json::to_value(puzzle).unwrap();

        assert_eq!(reserialized, json_value);
    }

    #[test]
    fn stored_puzzle_v1_supports_arbitrary_nonsense_text() {
        let json_value = json!({
            "version": "v1",
            "puzzle": {
                "board": { "rows": 1, "cols": 1 },
                "cells": [
                    {
                        "position": { "row": 0, "col": 0 },
                        "name": "Solo",
                        "role": "Guard",
                        "clue": {
                            "kind": "nonsense",
                            "text": "Editor's note: \"Definitely not suspicious.\""
                        },
                        "answer": "innocent",
                        "initially_revealed": true
                    }
                ]
            }
        });

        let puzzle: StoredPuzzle = serde_json::from_value(json_value.clone()).unwrap();
        let reserialized: Value = serde_json::to_value(puzzle).unwrap();

        assert_eq!(reserialized, json_value);
    }

    #[test]
    fn rename_cell_updates_names_and_clue_references() {
        let mut puzzle = StoredPuzzleV1 {
            board: BoardShape::new(1, 2),
            cells: vec![
                StoredCellV1 {
                    position: StoredPositionV1 { row: 0, col: 0 },
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    clue: Clue::DirectRelation {
                        name: "Ben".to_string(),
                        answer: Answer::Innocent,
                        direction: Direction::Left,
                    },
                    answer: Answer::Criminal,
                    initially_revealed: false,
                },
                StoredCellV1 {
                    position: StoredPositionV1 { row: 0, col: 1 },
                    name: "Ben".to_string(),
                    role: "Baker".to_string(),
                    clue: Clue::NamedCountCells {
                        name: "Ben".to_string(),
                        selector: CellSelector::Neighbor {
                            name: "Ada".to_string(),
                        },
                        answer: Answer::Innocent,
                        number: 1,
                        filter: CellFilter::Any,
                    },
                    answer: Answer::Innocent,
                    initially_revealed: false,
                },
            ],
        };

        puzzle.rename_cell("Ben", "Bianca").unwrap();

        assert_eq!(puzzle.cells[1].name, "Bianca");
        assert_eq!(
            puzzle.cells[0].clue,
            Clue::DirectRelation {
                name: "Bianca".to_string(),
                answer: Answer::Innocent,
                direction: Direction::Left,
            }
        );
        assert_eq!(
            puzzle.cells[1].clue,
            Clue::NamedCountCells {
                name: "Bianca".to_string(),
                selector: CellSelector::Neighbor {
                    name: "Ada".to_string(),
                },
                answer: Answer::Innocent,
                number: 1,
                filter: CellFilter::Any,
            }
        );
    }

    #[test]
    fn rename_cell_rejects_duplicate_target_name() {
        let mut puzzle = StoredPuzzleV1 {
            board: BoardShape::new(1, 2),
            cells: vec![
                StoredCellV1 {
                    position: StoredPositionV1 { row: 0, col: 0 },
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    clue: Clue::Nonsense {
                        text: "x".to_string(),
                    },
                    answer: Answer::Criminal,
                    initially_revealed: false,
                },
                StoredCellV1 {
                    position: StoredPositionV1 { row: 0, col: 1 },
                    name: "Ben".to_string(),
                    role: "Baker".to_string(),
                    clue: Clue::Nonsense {
                        text: "y".to_string(),
                    },
                    answer: Answer::Innocent,
                    initially_revealed: false,
                },
            ],
        };

        let error = puzzle.rename_cell("Ada", "Ben").unwrap_err();
        assert_eq!(
            error,
            RenameStoredCellError::DuplicateName("Ben".to_string())
        );
    }

    #[test]
    fn stored_puzzle_v1_round_trips_runtime_puzzle() {
        let puzzle = Puzzle {
            cells: vec![vec![
                Cell {
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    clue: Clue::CountCells {
                        selector: CellSelector::Neighbor {
                            name: "Ben".to_string(),
                        },
                        answer: Answer::Innocent,
                        count: Count::Number(1),
                        filter: CellFilter::Any,
                    },
                    answer: Answer::Criminal,
                    state: Visibility::Hidden,
                },
                Cell {
                    name: "Ben".to_string(),
                    role: "Baker".to_string(),
                    clue: Clue::Nonsense {
                        text: "Hmm".to_string(),
                    },
                    answer: Answer::Innocent,
                    state: Visibility::Revealed,
                },
            ]],
        };

        let stored = StoredPuzzle::from(&puzzle);
        let restored = Puzzle::try_from(stored).unwrap();

        assert_eq!(restored, puzzle);
    }

    #[test]
    fn stored_puzzle_v1_rejects_duplicate_names_when_loading_runtime_puzzle() {
        let stored = StoredPuzzle::V1(StoredPuzzleV1 {
            board: BoardShape::new(1, 2),
            cells: vec![
                StoredCellV1 {
                    position: StoredPositionV1 { row: 0, col: 0 },
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    clue: Clue::Nonsense {
                        text: "x".to_string(),
                    },
                    answer: Answer::Innocent,
                    initially_revealed: false,
                },
                StoredCellV1 {
                    position: StoredPositionV1 { row: 0, col: 1 },
                    name: "Ada".to_string(),
                    role: "Baker".to_string(),
                    clue: Clue::Nonsense {
                        text: "y".to_string(),
                    },
                    answer: Answer::Criminal,
                    initially_revealed: false,
                },
            ],
        });

        let error = Puzzle::try_from(stored).unwrap_err();
        assert_eq!(
            error,
            StoredPuzzleConversionError::DuplicateName("Ada".to_string())
        );
    }
}
