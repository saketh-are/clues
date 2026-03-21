use serde::{Deserialize, Serialize};

use crate::puzzle::{Puzzle, Puzzle3D};

/// Versioned persistence entrypoint for authored puzzles.
///
/// The persisted document is intentionally close to the runtime `Puzzle` type.
/// If a future incompatible storage change is needed, add a new enum variant
/// rather than mutating `V1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "version", content = "puzzle", rename_all = "snake_case")]
pub enum StoredPuzzle {
    V1(Puzzle),
    V1ThreeD(Puzzle3D),
}

impl From<&Puzzle> for StoredPuzzle {
    fn from(puzzle: &Puzzle) -> Self {
        Self::V1(puzzle.clone())
    }
}

impl From<&Puzzle3D> for StoredPuzzle {
    fn from(puzzle: &Puzzle3D) -> Self {
        Self::V1ThreeD(puzzle.clone())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::{
        clue::{CellFilter, CellSelector, Clue, Count, Direction},
        puzzle::{Cell, Puzzle, Puzzle3D, PuzzleValidationError, RenamePuzzleCellError, Visibility},
        types::Answer,
    };

    use super::StoredPuzzle;

    #[test]
    fn stored_puzzle_serializes_with_explicit_version() {
        let puzzle = StoredPuzzle::V1(Puzzle {
            author: None,
            cells: vec![vec![
                Cell {
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    emoji: None,
                    clue: Clue::CountCells {
                        selector: CellSelector::Neighbor {
                            name: "Ben".to_string(),
                        },
                        answer: Answer::Innocent,
                        count: Count::Number(3),
                        filter: CellFilter::Any,
                    },
                    answer: Answer::Criminal,
                    state: Visibility::Revealed,
                },
                Cell {
                    name: "Ben".to_string(),
                    role: "Baker".to_string(),
                    emoji: None,
                    clue: Clue::DirectRelation {
                        name: "Ada".to_string(),
                        answer: Answer::Criminal,
                        direction: Direction::Left,
                    },
                    answer: Answer::Innocent,
                    state: Visibility::Hidden,
                },
            ]],
        });

        let value = serde_json::to_value(&puzzle).unwrap();

        assert_eq!(
            value,
            json!({
                "version": "v1",
                "puzzle": {
                    "cells": [[
                        {
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
                            "state": "revealed"
                        },
                        {
                            "name": "Ben",
                            "role": "Baker",
                            "clue": {
                                "kind": "direct_relation",
                                "name": "Ada",
                                "answer": "criminal",
                                "direction": "left"
                            },
                            "answer": "innocent",
                            "state": "hidden"
                        }
                    ]]
                }
            }),
        );
    }

    #[test]
    fn stored_puzzle_round_trips_through_json() {
        let json_value = json!({
            "version": "v1",
            "puzzle": {
                "cells": [[
                    {
                        "name": "Solo",
                        "role": "Guard",
                        "clue": {
                            "kind": "nonsense",
                            "text": "This is nonsense."
                        },
                        "answer": "innocent",
                        "state": "hidden"
                    }
                ]]
            }
        });

        let puzzle: StoredPuzzle = serde_json::from_value(json_value.clone()).unwrap();
        let reserialized: Value = serde_json::to_value(puzzle).unwrap();

        assert_eq!(reserialized, json_value);
    }

    #[test]
    fn stored_puzzle_supports_arbitrary_nonsense_text() {
        let json_value = json!({
            "version": "v1",
            "puzzle": {
                "cells": [[
                    {
                        "name": "Solo",
                        "role": "Guard",
                        "clue": {
                            "kind": "nonsense",
                            "text": "Editor's note: \"Definitely not suspicious.\""
                        },
                        "answer": "innocent",
                        "state": "revealed"
                    }
                ]]
            }
        });

        let puzzle: StoredPuzzle = serde_json::from_value(json_value.clone()).unwrap();
        let reserialized: Value = serde_json::to_value(puzzle).unwrap();

        assert_eq!(reserialized, json_value);
    }

    #[test]
    fn stored_puzzle_round_trips_runtime_puzzle() {
        let puzzle = Puzzle {
            author: None,
            cells: vec![vec![
                Cell {
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    emoji: None,
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
                    emoji: None,
                    clue: Clue::Nonsense {
                        text: "Hmm".to_string(),
                    },
                    answer: Answer::Innocent,
                    state: Visibility::Revealed,
                },
            ]],
        };

        let stored = StoredPuzzle::from(&puzzle);

        assert_eq!(stored, StoredPuzzle::V1(puzzle));
    }

    #[test]
    fn stored_puzzle_round_trips_runtime_puzzle_3d() {
        let puzzle = Puzzle3D {
            author: None,
            cells: vec![vec![vec![
                Cell {
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    emoji: None,
                    clue: Clue::Nonsense {
                        text: "Hmm".to_string(),
                    },
                    answer: Answer::Criminal,
                    state: Visibility::Hidden,
                },
                Cell {
                    name: "Ben".to_string(),
                    role: "Baker".to_string(),
                    emoji: None,
                    clue: Clue::DirectRelation {
                        name: "Ada".to_string(),
                        answer: Answer::Criminal,
                        direction: Direction::Left,
                    },
                    answer: Answer::Innocent,
                    state: Visibility::Revealed,
                },
            ]]],
        };

        let stored = StoredPuzzle::from(&puzzle);

        assert_eq!(stored, StoredPuzzle::V1ThreeD(puzzle));
    }

    #[test]
    fn rename_cell_updates_names_and_clue_references() {
        let mut puzzle = Puzzle {
            author: None,
            cells: vec![vec![
                Cell {
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    emoji: None,
                    clue: Clue::DirectRelation {
                        name: "Ben".to_string(),
                        answer: Answer::Innocent,
                        direction: Direction::Left,
                    },
                    answer: Answer::Criminal,
                    state: Visibility::Hidden,
                },
                Cell {
                    name: "Ben".to_string(),
                    role: "Baker".to_string(),
                    emoji: None,
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
                    state: Visibility::Hidden,
                },
            ]],
        };

        puzzle.rename_cell("Ben", "Bianca").unwrap();

        assert_eq!(puzzle.cells[0][1].name, "Bianca");
        assert_eq!(
            puzzle.cells[0][0].clue,
            Clue::DirectRelation {
                name: "Bianca".to_string(),
                answer: Answer::Innocent,
                direction: Direction::Left,
            }
        );
        assert_eq!(
            puzzle.cells[0][1].clue,
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
        let mut puzzle = Puzzle {
            author: None,
            cells: vec![vec![
                Cell {
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    emoji: None,
                    clue: Clue::Nonsense {
                        text: "x".to_string(),
                    },
                    answer: Answer::Criminal,
                    state: Visibility::Hidden,
                },
                Cell {
                    name: "Ben".to_string(),
                    role: "Baker".to_string(),
                    emoji: None,
                    clue: Clue::Nonsense {
                        text: "y".to_string(),
                    },
                    answer: Answer::Innocent,
                    state: Visibility::Hidden,
                },
            ]],
        };

        let error = puzzle.rename_cell("Ada", "Ben").unwrap_err();
        assert_eq!(
            error,
            RenamePuzzleCellError::DuplicateName("Ben".to_string())
        );
    }

    #[test]
    fn puzzle_validation_rejects_duplicate_names() {
        let puzzle = Puzzle {
            author: None,
            cells: vec![vec![
                Cell {
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    emoji: None,
                    clue: Clue::Nonsense {
                        text: "x".to_string(),
                    },
                    answer: Answer::Innocent,
                    state: Visibility::Hidden,
                },
                Cell {
                    name: "Ada".to_string(),
                    role: "Baker".to_string(),
                    emoji: None,
                    clue: Clue::Nonsense {
                        text: "y".to_string(),
                    },
                    answer: Answer::Criminal,
                    state: Visibility::Hidden,
                },
            ]],
        };

        let error = puzzle.validate().unwrap_err();

        assert_eq!(
            error,
            PuzzleValidationError::DuplicateName("Ada".to_string())
        );
    }

    #[test]
    fn puzzle_validation_rejects_ragged_rows() {
        let puzzle = Puzzle {
            author: None,
            cells: vec![
                vec![Cell {
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    emoji: None,
                    clue: Clue::Nonsense {
                        text: "x".to_string(),
                    },
                    answer: Answer::Innocent,
                    state: Visibility::Hidden,
                }],
                vec![
                    Cell {
                        name: "Ben".to_string(),
                        role: "Baker".to_string(),
                        emoji: None,
                        clue: Clue::Nonsense {
                            text: "y".to_string(),
                        },
                        answer: Answer::Criminal,
                        state: Visibility::Hidden,
                    },
                    Cell {
                        name: "Cora".to_string(),
                        role: "Guard".to_string(),
                        emoji: None,
                        clue: Clue::Nonsense {
                            text: "z".to_string(),
                        },
                        answer: Answer::Innocent,
                        state: Visibility::Hidden,
                    },
                ],
            ],
        };

        let error = puzzle.validate().unwrap_err();

        assert_eq!(
            error,
            PuzzleValidationError::RaggedRow {
                row: 1,
                expected: 1,
                actual: 2,
            }
        );
    }
}
