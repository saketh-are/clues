pub mod clue;
pub mod generator;
pub mod geometry;
pub mod puzzle;
pub mod solver;
pub mod storage;
pub mod types;

pub use clue::{
    CellFilter, CellSelector, Clue, Column, Comparison, Count, Direction, Line, Parity,
    PersonGroup, PersonPredicate, Quantifier,
};
pub use generator::{
    ClueScoreTerms, DEFAULT_COLS, DEFAULT_ROWS, GenerateError, GeneratedPuzzle, GeneratedPuzzle3D,
    MAX_CELL_COUNT, generate_puzzle, generate_puzzle_3d_with_seed_and_size,
    generate_puzzle_with_seed, generate_puzzle_with_seed_and_size,
    suggest_clue_for_known_tile,
};
pub use geometry::{BoardShape, Offset, Position, TOUCHING_NEIGHBOR_OFFSETS};
pub use puzzle::{Cell, Puzzle, Puzzle3D, PuzzleValidationError, RenamePuzzleCellError, Visibility};
pub use solver::{
    ClueAnalysis, ClueAnalysis3D, ForcedAnswer, SolveError, analyze_clues, analyze_clues_3d,
    analyze_puzzle, analyze_puzzle_3d, analyze_revealed_puzzle, analyze_revealed_puzzle_3d,
};
pub use storage::StoredPuzzle;
pub use types::{Answer, NAMES, Name, ROLES, Role};
