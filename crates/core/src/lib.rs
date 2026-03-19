pub mod clue;
pub mod generator;
pub mod geometry;
pub mod puzzle;
pub mod solver;
pub mod types;

pub use clue::{
    CellFilter, CellSelector, Clue, Column, Comparison, Count, Direction, Line, Parity,
    PersonGroup, PersonPredicate, Quantifier,
};
pub use generator::{
    ClueScoreTerms, GenerateError, GeneratedPuzzle, generate_puzzle, generate_puzzle_with_seed,
};
pub use geometry::{BoardShape, Offset, Position, TOUCHING_NEIGHBOR_OFFSETS};
pub use puzzle::{Cell, Puzzle, Visibility};
pub use solver::{
    ClueAnalysis, ForcedAnswer, SolveError, analyze_clues, analyze_puzzle, analyze_revealed_puzzle,
};
pub use types::{Answer, NAMES, Name, ROLES, Role};
