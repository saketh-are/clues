pub mod clue;
pub mod geometry;
pub mod puzzle;
pub mod solver;
pub mod types;

pub use clue::{
    CellFilter, CellSelector, Clue, Column, Comparison, Count, Direction, Line, Parity,
    PersonGroup, PersonPredicate, Quantifier,
};
pub use geometry::{BoardShape, Offset, Position, TOUCHING_NEIGHBOR_OFFSETS};
pub use puzzle::{Cell, Puzzle, Visibility};
pub use solver::{analyze_clues, analyze_puzzle, ClueAnalysis, ForcedAnswer, SolveError};
pub use types::{Answer, NAMES, Name, ROLES, Role};
