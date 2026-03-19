use std::collections::HashMap;

use crate::{
    clue::{
        CellFilter, CellSelector, Clue, Column, Comparison, Count, Direction, Line, PersonGroup,
        PersonPredicate, Quantifier,
    },
    geometry::{BoardShape, Position},
    puzzle::{Puzzle, Visibility},
    types::{Answer, Name, Role},
};

type BitMask = u32;
const CELL_COUNT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForcedAnswer {
    Criminal,
    Innocent,
    Neither,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClueAnalysis {
    pub has_solution: bool,
    pub forced_answers: Vec<Vec<ForcedAnswer>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SolutionSet {
    pub analysis: ClueAnalysis,
    pub assignments: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveError {
    RaggedBoard,
    WrongCellCount(usize),
    DuplicateName(Name),
    MissingName(Name),
    InvalidRow(u8),
    InvalidColumn(Column),
}

#[derive(Debug, Clone)]
enum CompiledClue {
    Nonsense,
    Count {
        mask: BitMask,
        answer: Answer,
        count: Count,
    },
    Compare {
        left_mask: BitMask,
        right_mask: BitMask,
        answer: Answer,
        comparison: Comparison,
    },
    Connected {
        mask: BitMask,
        answer: Answer,
    },
    Quantified {
        group_mask: BitMask,
        predicate: CompiledPredicate,
        quantifier: Quantifier,
    },
}

#[derive(Debug, Clone)]
struct CompiledPredicate {
    masks_by_origin: [BitMask; CELL_COUNT],
    answer: Answer,
    count: Count,
}

#[derive(Debug, Clone)]
struct CompileContext {
    board: BoardShape,
    cols: u8,
    full_mask: BitMask,
    edge_mask: BitMask,
    corner_mask: BitMask,
    names: HashMap<Name, usize>,
    role_masks: HashMap<Role, BitMask>,
    orthogonal_masks: [BitMask; CELL_COUNT],
    positions: [Position; CELL_COUNT],
}

pub fn analyze_puzzle(puzzle: &Puzzle) -> Result<ClueAnalysis, SolveError> {
    let clues: Vec<Clue> = puzzle
        .cells
        .iter()
        .flat_map(|row| row.iter().map(|cell| cell.clue.clone()))
        .collect();

    analyze_clues(puzzle, &clues)
}

pub fn analyze_revealed_puzzle(puzzle: &Puzzle) -> Result<ClueAnalysis, SolveError> {
    let clues: Vec<Clue> = puzzle
        .cells
        .iter()
        .flat_map(|row| {
            row.iter()
                .filter(|cell| cell.state == Visibility::Revealed)
                .map(|cell| cell.clue.clone())
        })
        .collect();
    let (known_mask, known_innocent_mask) = known_masks_from_revealed_cells(puzzle)?;

    Ok(solve_clues_with_known_mask(puzzle, &clues, known_mask, known_innocent_mask)?.analysis)
}

pub fn analyze_clues(puzzle: &Puzzle, clues: &[Clue]) -> Result<ClueAnalysis, SolveError> {
    Ok(solve_clues_with_known_mask(puzzle, clues, 0, 0)?.analysis)
}

pub(crate) fn solve_clues_with_known_mask(
    puzzle: &Puzzle,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
) -> Result<SolutionSet, SolveError> {
    let context = CompileContext::new(puzzle)?;
    let compiled_clues = clues
        .iter()
        .map(|clue| context.compile_clue(clue))
        .collect::<Result<Vec<_>, _>>()?;

    let known_mask = known_mask & context.full_mask;
    let known_innocent_mask = known_innocent_mask & known_mask;
    let implicated_mask = compiled_clues
        .iter()
        .fold(0, |mask, clue| mask | context.implicated_mask(clue));
    let variable_mask = implicated_mask & !known_mask;
    let variable_bits = (0..CELL_COUNT)
        .map(|index| 1u32 << index)
        .filter(|bit| variable_mask & bit != 0)
        .collect::<Vec<_>>();
    let base_assignment = known_innocent_mask;
    let mut assignments = Vec::new();

    for subset in 0..(1u64 << variable_bits.len()) {
        let mut assignment = base_assignment;

        for (index, bit) in variable_bits.iter().enumerate() {
            if subset & (1u64 << index) != 0 {
                assignment |= *bit;
            }
        }

        if compiled_clues
            .iter()
            .all(|clue| context.assignment_satisfies_clue(assignment, clue))
        {
            assignments.push(assignment);
        }
    }

    Ok(SolutionSet {
        analysis: context.analysis_from_assignments(
            &assignments,
            known_mask,
            known_innocent_mask,
            variable_mask,
        ),
        assignments,
    })
}

fn known_masks_from_revealed_cells(puzzle: &Puzzle) -> Result<(BitMask, BitMask), SolveError> {
    let context = CompileContext::new(puzzle)?;
    let mut known_mask = 0;
    let mut known_innocent_mask = 0;

    for (row_index, row) in puzzle.cells.iter().enumerate() {
        for (col_index, cell) in row.iter().enumerate() {
            if cell.state != Visibility::Revealed {
                continue;
            }

            let bit = 1u32 << (row_index * context.cols as usize + col_index);
            known_mask |= bit;

            if cell.answer == Answer::Innocent {
                known_innocent_mask |= bit;
            }
        }
    }

    Ok((known_mask, known_innocent_mask))
}

impl CompileContext {
    fn analysis_from_assignments(
        &self,
        assignments: &[BitMask],
        known_mask: BitMask,
        known_innocent_mask: BitMask,
        variable_mask: BitMask,
    ) -> ClueAnalysis {
        let mut always_innocent = self.full_mask;
        let mut always_criminal = self.full_mask;

        for assignment in assignments {
            always_innocent &= *assignment;
            always_criminal &= (!assignment) & self.full_mask;
        }

        let forced_answers = (0..self.board.rows as usize)
            .map(|row| {
                (0..self.cols as usize)
                    .map(|col| {
                        if assignments.is_empty() {
                            return ForcedAnswer::Neither;
                        }

                        let bit = 1u32 << (row * self.cols as usize + col);
                        if known_mask & bit != 0 {
                            if known_innocent_mask & bit != 0 {
                                ForcedAnswer::Innocent
                            } else {
                                ForcedAnswer::Criminal
                            }
                        } else if variable_mask & bit == 0 {
                            ForcedAnswer::Neither
                        } else if always_innocent & bit != 0 {
                            ForcedAnswer::Innocent
                        } else if always_criminal & bit != 0 {
                            ForcedAnswer::Criminal
                        } else {
                            ForcedAnswer::Neither
                        }
                    })
                    .collect()
            })
            .collect();

        ClueAnalysis {
            has_solution: !assignments.is_empty(),
            forced_answers,
        }
    }

    fn implicated_mask(&self, clue: &CompiledClue) -> BitMask {
        match clue {
            CompiledClue::Nonsense => 0,
            CompiledClue::Count { mask, .. } => *mask,
            CompiledClue::Compare {
                left_mask,
                right_mask,
                ..
            } => *left_mask | *right_mask,
            CompiledClue::Connected { mask, .. } => *mask,
            CompiledClue::Quantified {
                group_mask,
                predicate,
                ..
            } => (0..CELL_COUNT)
                .filter(|index| group_mask & (1u32 << index) != 0)
                .fold(0, |mask, index| mask | predicate.masks_by_origin[index]),
        }
    }

    fn new(puzzle: &Puzzle) -> Result<Self, SolveError> {
        let rows = puzzle.cells.len();
        let cols = puzzle
            .cells
            .first()
            .map(|row| row.len())
            .unwrap_or_default();

        if puzzle.cells.iter().any(|row| row.len() != cols) {
            return Err(SolveError::RaggedBoard);
        }

        let cell_count = rows * cols;
        if cell_count != CELL_COUNT {
            return Err(SolveError::WrongCellCount(cell_count));
        }

        let board = BoardShape::new(rows as u8, cols as u8);
        let full_mask = (1u32 << CELL_COUNT) - 1;
        let mut positions = [Position::new(0, 0); CELL_COUNT];
        let mut names = HashMap::new();
        let mut role_masks: HashMap<Role, BitMask> = HashMap::new();
        let mut edge_mask = 0;
        let mut corner_mask = 0;

        for (row_index, row) in puzzle.cells.iter().enumerate() {
            for (col_index, cell) in row.iter().enumerate() {
                let index = row_index * cols + col_index;
                let position = Position::new(row_index as i16, col_index as i16);
                let bit = 1u32 << index;
                positions[index] = position;

                if names.insert(cell.name.clone(), index).is_some() {
                    return Err(SolveError::DuplicateName(cell.name.clone()));
                }

                role_masks
                    .entry(cell.role.clone())
                    .and_modify(|mask| *mask |= bit)
                    .or_insert(bit);

                if board.is_edge(position) {
                    edge_mask |= bit;
                }
                if board.is_corner(position) {
                    corner_mask |= bit;
                }
            }
        }

        let mut orthogonal_masks = [0; CELL_COUNT];
        for (index, position) in positions.iter().enumerate() {
            orthogonal_masks[index] =
                Self::positions_to_mask(&board.orthogonal_neighbors(*position), cols);
        }

        Ok(Self {
            board,
            cols: cols as u8,
            full_mask,
            edge_mask,
            corner_mask,
            names,
            role_masks,
            orthogonal_masks,
            positions,
        })
    }

    fn compile_clue(&self, clue: &Clue) -> Result<CompiledClue, SolveError> {
        match clue {
            Clue::Nonsense => Ok(CompiledClue::Nonsense),
            Clue::CountCells {
                selector,
                answer,
                count,
                filter,
            } => Ok(CompiledClue::Count {
                mask: self.selector_mask(selector)? & self.filter_mask(*filter),
                answer: *answer,
                count: *count,
            }),
            Clue::Connected { answer, line } => Ok(CompiledClue::Connected {
                mask: self.line_mask(*line)?,
                answer: *answer,
            }),
            Clue::DirectRelation {
                name,
                answer,
                direction,
            } => Ok(CompiledClue::Count {
                mask: self.direct_relation_mask(name, *direction)?,
                answer: *answer,
                count: Count::Number(1),
            }),
            Clue::RoleCount {
                role,
                answer,
                count,
            } => Ok(CompiledClue::Count {
                mask: self.role_mask(role),
                answer: *answer,
                count: *count,
            }),
            Clue::RolesComparison {
                first_role,
                second_role,
                answer,
                comparison,
            } => Ok(CompiledClue::Compare {
                left_mask: self.role_mask(first_role),
                right_mask: self.role_mask(second_role),
                answer: *answer,
                comparison: *comparison,
            }),
            Clue::LineComparison {
                first_line,
                second_line,
                answer,
                comparison,
            } => Ok(CompiledClue::Compare {
                left_mask: self.line_mask(*first_line)?,
                right_mask: self.line_mask(*second_line)?,
                answer: *answer,
                comparison: *comparison,
            }),
            Clue::Quantified {
                quantifier,
                group,
                predicate,
            } => Ok(CompiledClue::Quantified {
                group_mask: self.group_mask(group)?,
                predicate: self.compile_predicate(predicate)?,
                quantifier: *quantifier,
            }),
        }
    }

    fn compile_predicate(
        &self,
        predicate: &PersonPredicate,
    ) -> Result<CompiledPredicate, SolveError> {
        let mut masks_by_origin = [0; CELL_COUNT];

        match predicate {
            PersonPredicate::Neighbor {
                answer,
                count,
                filter,
            } => {
                for (index, position) in self.positions.iter().enumerate() {
                    masks_by_origin[index] = Self::positions_to_mask(
                        &self.board.touching_neighbors(*position),
                        self.cols as usize,
                    ) & self.filter_mask(*filter);
                }

                Ok(CompiledPredicate {
                    masks_by_origin,
                    answer: *answer,
                    count: *count,
                })
            }
            PersonPredicate::DirectRelation { answer, direction } => {
                let offset = match direction {
                    Direction::Above => Direction::Below.offset(),
                    Direction::Below => Direction::Above.offset(),
                    Direction::Left => Direction::Right.offset(),
                    Direction::Right => Direction::Left.offset(),
                };

                for (index, position) in self.positions.iter().enumerate() {
                    let shifted = position.shifted(offset);
                    if self.board.contains(shifted) {
                        masks_by_origin[index] = self.position_to_bit(shifted);
                    }
                }

                Ok(CompiledPredicate {
                    masks_by_origin,
                    answer: *answer,
                    count: Count::Number(1),
                })
            }
        }
    }

    fn assignment_satisfies_clue(&self, assignment: BitMask, clue: &CompiledClue) -> bool {
        match clue {
            CompiledClue::Nonsense => true,
            CompiledClue::Count {
                mask,
                answer,
                count,
            } => self.matches_count(assignment, *mask, *answer, *count),
            CompiledClue::Compare {
                left_mask,
                right_mask,
                answer,
                comparison,
            } => {
                let left = self.matching_count(assignment, *left_mask, *answer);
                let right = self.matching_count(assignment, *right_mask, *answer);

                match comparison {
                    Comparison::More => left > right,
                    Comparison::Fewer => left < right,
                    Comparison::Equal => left == right,
                }
            }
            CompiledClue::Connected { mask, answer } => self
                .matching_positions(assignment, *mask, *answer)
                .map(|selected| self.is_connected(selected))
                .unwrap_or(true),
            CompiledClue::Quantified {
                group_mask,
                predicate,
                quantifier,
            } => {
                let matching_people = (0..CELL_COUNT)
                    .filter(|index| group_mask & (1u32 << index) != 0)
                    .filter(|index| {
                        self.matches_count(
                            assignment,
                            predicate.masks_by_origin[*index],
                            predicate.answer,
                            predicate.count,
                        )
                    })
                    .count() as i32;

                match quantifier {
                    Quantifier::Exactly(expected) => matching_people == *expected,
                }
            }
        }
    }

    fn matching_positions(
        &self,
        assignment: BitMask,
        mask: BitMask,
        answer: Answer,
    ) -> Option<BitMask> {
        let selected = match answer {
            Answer::Innocent => assignment & mask,
            Answer::Criminal => ((!assignment) & self.full_mask) & mask,
        };

        if selected == 0 { None } else { Some(selected) }
    }

    fn is_connected(&self, selected: BitMask) -> bool {
        let start_index = selected.trailing_zeros() as usize;
        let mut frontier = 1u32 << start_index;
        let mut visited = 0;

        while frontier != 0 {
            let index = frontier.trailing_zeros() as usize;
            let bit = 1u32 << index;
            frontier &= !bit;

            if visited & bit != 0 {
                continue;
            }

            visited |= bit;
            frontier |= self.orthogonal_masks[index] & selected & !visited;
        }

        visited == selected
    }

    fn matches_count(
        &self,
        assignment: BitMask,
        mask: BitMask,
        answer: Answer,
        count: Count,
    ) -> bool {
        let matching = self.matching_count(assignment, mask, answer);

        match count {
            Count::Number(number) => matching == number as u32,
            Count::AtLeast(number) => matching >= number as u32,
            Count::Parity(crate::clue::Parity::Odd) => matching % 2 == 1,
            Count::Parity(crate::clue::Parity::Even) => matching % 2 == 0,
        }
    }

    fn matching_count(&self, assignment: BitMask, mask: BitMask, answer: Answer) -> u32 {
        let innocents = (assignment & mask).count_ones();

        match answer {
            Answer::Innocent => innocents,
            Answer::Criminal => mask.count_ones() - innocents,
        }
    }

    fn selector_mask(&self, selector: &CellSelector) -> Result<BitMask, SolveError> {
        Ok(match selector {
            CellSelector::Board => self.full_mask,
            CellSelector::Neighbor { name } => Self::positions_to_mask(
                &self.board.touching_neighbors(self.name_position(name)?),
                self.cols as usize,
            ),
            CellSelector::Direction { name, direction } => Self::positions_to_mask(
                &self
                    .board
                    .tiles_in_direction(self.name_position(name)?, direction.offset()),
                self.cols as usize,
            ),
            CellSelector::Row { row } => self.line_mask(Line::Row(*row))?,
            CellSelector::Col { col } => self.line_mask(Line::Col(*col))?,
            CellSelector::Between {
                first_name,
                second_name,
            } => {
                let first = self.name_position(first_name)?;
                let second = self.name_position(second_name)?;

                if first.row == second.row || first.col == second.col {
                    Self::positions_to_mask(
                        &self.board.positions_between(first, second),
                        self.cols as usize,
                    )
                } else {
                    0
                }
            }
            CellSelector::SharedNeighbor {
                first_name,
                second_name,
            } => Self::positions_to_mask(
                &self.board.common_neighbors(
                    self.name_position(first_name)?,
                    self.name_position(second_name)?,
                ),
                self.cols as usize,
            ),
        })
    }

    fn direct_relation_mask(
        &self,
        name: &str,
        direction: Direction,
    ) -> Result<BitMask, SolveError> {
        let shifted = self.name_position(name)?.shifted(direction.offset());
        Ok(if self.board.contains(shifted) {
            self.position_to_bit(shifted)
        } else {
            0
        })
    }

    fn group_mask(&self, group: &PersonGroup) -> Result<BitMask, SolveError> {
        Ok(match group {
            PersonGroup::Any => self.full_mask,
            PersonGroup::Filter { filter } => self.filter_mask(*filter),
            PersonGroup::Line { line } => self.line_mask(*line)?,
            PersonGroup::Role { role } => self.role_mask(role),
        })
    }

    fn role_mask(&self, role: &str) -> BitMask {
        self.role_masks.get(role).copied().unwrap_or(0)
    }

    fn line_mask(&self, line: Line) -> Result<BitMask, SolveError> {
        match line {
            Line::Row(row) => {
                if row >= self.board.rows {
                    return Err(SolveError::InvalidRow(row));
                }

                Ok(Self::positions_to_mask(
                    &self.board.row_positions(row),
                    self.cols as usize,
                ))
            }
            Line::Col(col) => {
                if col.index() >= self.cols {
                    return Err(SolveError::InvalidColumn(col));
                }

                Ok(Self::positions_to_mask(
                    &self.board.col_positions(col.index()),
                    self.cols as usize,
                ))
            }
        }
    }

    fn filter_mask(&self, filter: CellFilter) -> BitMask {
        match filter {
            CellFilter::Any => self.full_mask,
            CellFilter::Edge => self.edge_mask,
            CellFilter::Corner => self.corner_mask,
        }
    }

    fn name_position(&self, name: &str) -> Result<Position, SolveError> {
        let index = self
            .names
            .get(name)
            .copied()
            .ok_or_else(|| SolveError::MissingName(name.to_string()))?;

        Ok(self.positions[index])
    }

    fn position_to_bit(&self, position: Position) -> BitMask {
        1u32 << (position.row as usize * self.cols as usize + position.col as usize)
    }

    fn positions_to_mask(positions: &[Position], cols: usize) -> BitMask {
        positions.iter().fold(0, |mask, position| {
            mask | (1u32 << (position.row as usize * cols + position.col as usize))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        clue::{
            CellFilter, CellSelector, Clue, Column, Comparison, Count, Direction, Line,
            PersonGroup, PersonPredicate, Quantifier,
        },
        puzzle::{Cell, Puzzle, Visibility},
        solver::{
            ClueAnalysis, ForcedAnswer, analyze_clues, analyze_puzzle, analyze_revealed_puzzle,
            known_masks_from_revealed_cells, solve_clues_with_known_mask,
        },
        types::{Answer, NAMES},
    };

    fn test_puzzle() -> Puzzle {
        let roles = ["Sleuth", "Coder", "Coach", "Chef"];
        let dummy_clue = Clue::CountCells {
            selector: CellSelector::Board,
            answer: Answer::Innocent,
            count: Count::AtLeast(0),
            filter: CellFilter::Any,
        };

        let cells = (0..5)
            .map(|row| {
                (0..4)
                    .map(|col| {
                        let index = row * 4 + col;
                        Cell {
                            name: NAMES[index].to_string(),
                            role: roles[col].to_string(),
                            clue: dummy_clue.clone(),
                            answer: Answer::Innocent,
                            state: Visibility::Hidden,
                        }
                    })
                    .collect()
            })
            .collect();

        Puzzle { cells }
    }

    fn forced_at(analysis: &ClueAnalysis, row: usize, col: usize) -> ForcedAnswer {
        analysis.forced_answers[row][col]
    }

    #[test]
    fn board_wide_innocent_count_forces_every_tile_innocent() {
        let puzzle = test_puzzle();
        let clues = [Clue::CountCells {
            selector: CellSelector::Board,
            answer: Answer::Innocent,
            count: Count::Number(20),
            filter: CellFilter::Any,
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        assert!(
            analysis
                .forced_answers
                .iter()
                .flatten()
                .all(|forced| *forced == ForcedAnswer::Innocent)
        );
    }

    #[test]
    fn neighbor_selector_can_force_touching_neighbors() {
        let puzzle = test_puzzle();
        let anchor_name = puzzle.cells[1][1].name.clone();
        let clues = [Clue::CountCells {
            selector: CellSelector::Neighbor { name: anchor_name },
            answer: Answer::Innocent,
            count: Count::Number(8),
            filter: CellFilter::Any,
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        for (row, col) in [
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 0),
            (1, 2),
            (2, 0),
            (2, 1),
            (2, 2),
        ] {
            assert_eq!(forced_at(&analysis, row, col), ForcedAnswer::Innocent);
        }
        assert_eq!(forced_at(&analysis, 1, 1), ForcedAnswer::Neither);
    }

    #[test]
    fn direction_selector_can_force_a_full_ray() {
        let puzzle = test_puzzle();
        let anchor_name = puzzle.cells[0][0].name.clone();
        let clues = [Clue::CountCells {
            selector: CellSelector::Direction {
                name: anchor_name,
                direction: Direction::Below,
            },
            answer: Answer::Innocent,
            count: Count::Number(4),
            filter: CellFilter::Any,
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        for row in 1..5 {
            assert_eq!(forced_at(&analysis, row, 0), ForcedAnswer::Innocent);
        }
        assert_eq!(forced_at(&analysis, 0, 0), ForcedAnswer::Neither);
    }

    #[test]
    fn row_selector_can_force_an_entire_row() {
        let puzzle = test_puzzle();
        let clues = [Clue::CountCells {
            selector: CellSelector::Row { row: 0 },
            answer: Answer::Innocent,
            count: Count::Number(4),
            filter: CellFilter::Any,
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        for col in 0..4 {
            assert_eq!(forced_at(&analysis, 0, col), ForcedAnswer::Innocent);
        }
        assert_eq!(forced_at(&analysis, 1, 0), ForcedAnswer::Neither);
    }

    #[test]
    fn column_selector_can_force_an_entire_column() {
        let puzzle = test_puzzle();
        let clues = [Clue::CountCells {
            selector: CellSelector::Col { col: Column::A },
            answer: Answer::Innocent,
            count: Count::Number(5),
            filter: CellFilter::Any,
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        for row in 0..5 {
            assert_eq!(forced_at(&analysis, row, 0), ForcedAnswer::Innocent);
        }
        assert_eq!(forced_at(&analysis, 0, 1), ForcedAnswer::Neither);
    }

    #[test]
    fn between_selector_can_force_cells_between_two_names() {
        let puzzle = test_puzzle();
        let clues = [Clue::CountCells {
            selector: CellSelector::Between {
                first_name: puzzle.cells[0][0].name.clone(),
                second_name: puzzle.cells[0][3].name.clone(),
            },
            answer: Answer::Innocent,
            count: Count::Number(2),
            filter: CellFilter::Any,
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        assert_eq!(forced_at(&analysis, 0, 1), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 0, 2), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 0, 0), ForcedAnswer::Neither);
    }

    #[test]
    fn shared_neighbor_selector_can_force_common_neighbors() {
        let puzzle = test_puzzle();
        let clues = [Clue::CountCells {
            selector: CellSelector::SharedNeighbor {
                first_name: puzzle.cells[1][1].name.clone(),
                second_name: puzzle.cells[1][2].name.clone(),
            },
            answer: Answer::Innocent,
            count: Count::Number(4),
            filter: CellFilter::Any,
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        for (row, col) in [(0, 1), (0, 2), (2, 1), (2, 2)] {
            assert_eq!(forced_at(&analysis, row, col), ForcedAnswer::Innocent);
        }
        assert_eq!(forced_at(&analysis, 1, 1), ForcedAnswer::Neither);
    }

    #[test]
    fn direct_relation_can_force_a_single_tile() {
        let puzzle = test_puzzle();
        let anchor_name = puzzle.cells[0][1].name.clone();
        let clues = [Clue::DirectRelation {
            name: anchor_name,
            answer: Answer::Innocent,
            direction: Direction::Left,
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        assert_eq!(forced_at(&analysis, 0, 0), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 0, 1), ForcedAnswer::Neither);
    }

    #[test]
    fn impossible_direct_relation_has_no_solution() {
        let puzzle = test_puzzle();
        let anchor_name = puzzle.cells[0][0].name.clone();
        let clues = [Clue::DirectRelation {
            name: anchor_name,
            answer: Answer::Innocent,
            direction: Direction::Left,
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(!analysis.has_solution);
        assert!(
            analysis
                .forced_answers
                .iter()
                .flatten()
                .all(|forced| *forced == ForcedAnswer::Neither)
        );
    }

    #[test]
    fn connected_clue_detects_disconnected_required_criminals() {
        let puzzle = test_puzzle();
        let clues = [
            Clue::CountCells {
                selector: CellSelector::Row { row: 0 },
                answer: Answer::Criminal,
                count: Count::Number(2),
                filter: CellFilter::Any,
            },
            Clue::CountCells {
                selector: CellSelector::Row { row: 0 },
                answer: Answer::Criminal,
                count: Count::Number(2),
                filter: CellFilter::Corner,
            },
            Clue::Connected {
                answer: Answer::Criminal,
                line: Line::Row(0),
            },
        ];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(!analysis.has_solution);
    }

    #[test]
    fn analyze_puzzle_uses_cell_clues() {
        let mut puzzle = test_puzzle();
        puzzle.cells[0][0].clue = Clue::CountCells {
            selector: CellSelector::Board,
            answer: Answer::Criminal,
            count: Count::Number(0),
            filter: CellFilter::Any,
        };

        let analysis = analyze_puzzle(&puzzle).unwrap();

        assert!(analysis.has_solution);
        assert!(
            analysis
                .forced_answers
                .iter()
                .flatten()
                .all(|forced| *forced == ForcedAnswer::Innocent)
        );
    }

    #[test]
    fn role_count_can_force_a_role_column() {
        let puzzle = test_puzzle();
        let clues = [Clue::RoleCount {
            role: "Sleuth".to_string(),
            answer: Answer::Innocent,
            count: Count::Number(5),
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        for row in 0..5 {
            assert_eq!(forced_at(&analysis, row, 0), ForcedAnswer::Innocent);
        }
        assert_eq!(forced_at(&analysis, 0, 1), ForcedAnswer::Neither);
    }

    #[test]
    fn roles_comparison_detects_impossible_role_totals() {
        let puzzle = test_puzzle();
        let clues = [
            Clue::RoleCount {
                role: "Coder".to_string(),
                answer: Answer::Innocent,
                count: Count::Number(5),
            },
            Clue::RolesComparison {
                first_role: "Sleuth".to_string(),
                second_role: "Coder".to_string(),
                answer: Answer::Innocent,
                comparison: Comparison::More,
            },
        ];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(!analysis.has_solution);
    }

    #[test]
    fn line_comparison_detects_impossible_line_totals() {
        let puzzle = test_puzzle();
        let clues = [
            Clue::CountCells {
                selector: CellSelector::Row { row: 1 },
                answer: Answer::Innocent,
                count: Count::Number(4),
                filter: CellFilter::Any,
            },
            Clue::LineComparison {
                first_line: Line::Row(0),
                second_line: Line::Row(1),
                answer: Answer::Innocent,
                comparison: Comparison::More,
            },
        ];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(!analysis.has_solution);
    }

    #[test]
    fn quantified_clue_can_force_matching_neighbors() {
        let puzzle = test_puzzle();
        let clues = [Clue::Quantified {
            quantifier: Quantifier::Exactly(5),
            group: PersonGroup::Role {
                role: "Sleuth".to_string(),
            },
            predicate: PersonPredicate::DirectRelation {
                answer: Answer::Innocent,
                direction: Direction::Left,
            },
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        for row in 0..5 {
            assert_eq!(forced_at(&analysis, row, 1), ForcedAnswer::Innocent);
        }
        assert_eq!(forced_at(&analysis, 0, 0), ForcedAnswer::Neither);
    }

    #[test]
    fn nonsense_clue_adds_no_constraints() {
        let puzzle = test_puzzle();
        let clues = [Clue::Nonsense];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        assert!(
            analysis
                .forced_answers
                .iter()
                .flatten()
                .all(|forced| *forced == ForcedAnswer::Neither)
        );
    }

    #[test]
    fn solver_only_tracks_implicated_unknown_cells() {
        let puzzle = test_puzzle();
        let anchor_name = puzzle.cells[0][1].name.clone();
        let clues = [Clue::DirectRelation {
            name: anchor_name,
            answer: Answer::Innocent,
            direction: Direction::Left,
        }];

        let solved = solve_clues_with_known_mask(&puzzle, &clues, 0, 0).unwrap();

        assert_eq!(solved.assignments.len(), 1);
        assert_eq!(forced_at(&solved.analysis, 0, 0), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&solved.analysis, 4, 3), ForcedAnswer::Neither);
    }

    #[test]
    fn row_and_column_counts_can_force_a_cross_pattern() {
        let puzzle = test_puzzle();
        let clues = [
            Clue::CountCells {
                selector: CellSelector::Row { row: 0 },
                answer: Answer::Innocent,
                count: Count::Number(4),
                filter: CellFilter::Any,
            },
            Clue::CountCells {
                selector: CellSelector::Col { col: Column::A },
                answer: Answer::Criminal,
                count: Count::Number(4),
                filter: CellFilter::Any,
            },
        ];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        for col in 0..4 {
            assert_eq!(forced_at(&analysis, 0, col), ForcedAnswer::Innocent);
        }
        for row in 1..5 {
            assert_eq!(forced_at(&analysis, row, 0), ForcedAnswer::Criminal);
        }
    }

    #[test]
    fn mixed_clues_can_be_inconsistent() {
        let puzzle = test_puzzle();
        let clues = [
            Clue::CountCells {
                selector: CellSelector::Row { row: 0 },
                answer: Answer::Innocent,
                count: Count::Number(4),
                filter: CellFilter::Any,
            },
            Clue::RoleCount {
                role: "Sleuth".to_string(),
                answer: Answer::Criminal,
                count: Count::Number(5),
            },
        ];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(!analysis.has_solution);
    }

    #[test]
    fn analyze_revealed_puzzle_treats_revealed_answers_as_known() {
        let mut puzzle = test_puzzle();
        puzzle.cells[0][0].state = Visibility::Revealed;

        let analysis = analyze_revealed_puzzle(&puzzle).unwrap();

        assert!(analysis.has_solution);
        assert_eq!(forced_at(&analysis, 0, 0), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 0, 1), ForcedAnswer::Neither);
    }

    #[test]
    fn revealed_cells_are_not_reenumerated_when_nothing_else_is_implicated() {
        let mut puzzle = test_puzzle();
        puzzle.cells[0][0].state = Visibility::Revealed;
        let clues = [Clue::Nonsense];
        let (known_mask, known_innocent_mask) = known_masks_from_revealed_cells(&puzzle).unwrap();

        let solved =
            solve_clues_with_known_mask(&puzzle, &clues, known_mask, known_innocent_mask).unwrap();

        assert_eq!(solved.assignments.len(), 1);
        assert_eq!(forced_at(&solved.analysis, 0, 0), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&solved.analysis, 4, 3), ForcedAnswer::Neither);
    }
}
