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
    ClueScoreTerms, DEFAULT_COLS, DEFAULT_ROWS, GenerateError, GeneratedPuzzle, MAX_CELL_COUNT,
    generate_puzzle, generate_puzzle_with_seed, generate_puzzle_with_seed_and_size,
};
pub use geometry::{BoardShape, Offset, Position, TOUCHING_NEIGHBOR_OFFSETS};
pub use puzzle::{Cell, Puzzle, Visibility};
pub use solver::{
    ClueAnalysis, ForcedAnswer, SolveError, analyze_clues, analyze_puzzle, analyze_revealed_puzzle,
};
pub use storage::{
    RenameStoredCellError, StoredCellV1, StoredPositionV1, StoredPuzzle,
    StoredPuzzleConversionError, StoredPuzzleV1,
};
pub use types::{Answer, NAMES, Name, ROLES, Role};
