use std::collections::HashMap;

use crate::{
    clue::{
        CellFilter, CellSelector, Clue, Column, Comparison, Count, Direction, Line, PersonGroup,
        PersonPredicate, Quantifier,
    },
    geometry::{BoardShape, Position},
    puzzle::{Cell, Puzzle, Puzzle3D, Visibility},
    types::{Answer, Name, Role},
};

type BitMask = u32;
const MAX_CELL_COUNT: usize = BitMask::BITS as usize;

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
pub struct ClueAnalysis3D {
    pub has_solution: bool,
    pub forced_answers: Vec<Vec<Vec<ForcedAnswer>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlatClueAnalysis {
    pub has_solution: bool,
    pub board: BoardShape,
    pub forced_answers: Vec<ForcedAnswer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SolutionSet {
    pub analysis: FlatClueAnalysis,
    pub assignments: Vec<u32>,
    pub variable_mask: u32,
    pub implicated_mask: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveError {
    RaggedBoard,
    TooManyCells(usize),
    DuplicateName(Name),
    MissingName(Name),
    InvalidLayer(u8),
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
    CountWithMember {
        mask: BitMask,
        member_mask: BitMask,
        answer: Answer,
        number: i32,
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
        group: CompiledGroup,
        predicate: CompiledPredicate,
        quantifier: Quantifier,
    },
}

#[derive(Debug, Clone)]
enum CompiledGroup {
    Static { mask: BitMask },
    Answered { mask: BitMask, answer: Answer },
}

#[derive(Debug, Clone)]
enum CompiledPredicate {
    Count {
        masks_by_origin: Vec<BitMask>,
        answer: Answer,
        count: Count,
    },
    Structural {
        matching_origins: BitMask,
    },
}

#[derive(Debug, Clone)]
struct CompileContext {
    board: BoardShape,
    cell_count: usize,
    cols: u8,
    full_mask: BitMask,
    edge_mask: BitMask,
    corner_mask: BitMask,
    names: HashMap<Name, usize>,
    role_masks: HashMap<Role, BitMask>,
    orthogonal_masks: Vec<BitMask>,
    positions: Vec<Position>,
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
    let (known_mask, known_innocent_mask) = known_masks_from_revealed_cells_2d(puzzle)?;

    Ok(flat_analysis_to_2d(
        solve_clues_with_known_mask(puzzle, &clues, known_mask, known_innocent_mask)?.analysis,
    ))
}

pub fn analyze_clues(puzzle: &Puzzle, clues: &[Clue]) -> Result<ClueAnalysis, SolveError> {
    Ok(flat_analysis_to_2d(
        solve_clues_with_known_mask(puzzle, clues, 0, 0)?.analysis,
    ))
}

pub fn analyze_puzzle_3d(puzzle: &Puzzle3D) -> Result<ClueAnalysis3D, SolveError> {
    let clues: Vec<Clue> = puzzle
        .cells
        .iter()
        .flat_map(|layer| {
            layer
                .iter()
                .flat_map(|row| row.iter().map(|cell| cell.clue.clone()))
        })
        .collect();

    analyze_clues_3d(puzzle, &clues)
}

pub fn analyze_revealed_puzzle_3d(puzzle: &Puzzle3D) -> Result<ClueAnalysis3D, SolveError> {
    let clues: Vec<Clue> = puzzle
        .cells
        .iter()
        .flat_map(|layer| {
            layer.iter().flat_map(|row| {
                row.iter()
                    .filter(|cell| cell.state == Visibility::Revealed)
                    .map(|cell| cell.clue.clone())
            })
        })
        .collect();
    let (known_mask, known_innocent_mask) = known_masks_from_revealed_cells_3d(puzzle)?;

    Ok(flat_analysis_to_3d(
        solve_clues_with_known_mask_3d(puzzle, &clues, known_mask, known_innocent_mask)?.analysis,
    ))
}

pub fn analyze_clues_3d(puzzle: &Puzzle3D, clues: &[Clue]) -> Result<ClueAnalysis3D, SolveError> {
    Ok(flat_analysis_to_3d(
        solve_clues_with_known_mask_3d(puzzle, clues, 0, 0)?.analysis,
    ))
}

pub(crate) fn solve_clues_with_known_mask(
    puzzle: &Puzzle,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
) -> Result<SolutionSet, SolveError> {
    let context = CompileContext::from_2d_puzzle(puzzle)?;
    solve_clues_with_context(&context, clues, known_mask, known_innocent_mask)
}

pub(crate) fn solve_clues_with_known_mask_3d(
    puzzle: &Puzzle3D,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
) -> Result<SolutionSet, SolveError> {
    let context = CompileContext::from_3d_puzzle(puzzle)?;
    solve_clues_with_context(&context, clues, known_mask, known_innocent_mask)
}

fn solve_clues_with_context(
    context: &CompileContext,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
) -> Result<SolutionSet, SolveError> {
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
    let variable_bits = (0..context.cell_count)
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
        variable_mask,
        implicated_mask,
    })
}

fn known_masks_from_revealed_cells_2d(puzzle: &Puzzle) -> Result<(BitMask, BitMask), SolveError> {
    let context = CompileContext::from_2d_puzzle(puzzle)?;
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

fn known_masks_from_revealed_cells_3d(
    puzzle: &Puzzle3D,
) -> Result<(BitMask, BitMask), SolveError> {
    let context = CompileContext::from_3d_puzzle(puzzle)?;
    let mut known_mask = 0;
    let mut known_innocent_mask = 0;

    for (layer_index, layer) in puzzle.cells.iter().enumerate() {
        for (row_index, row) in layer.iter().enumerate() {
            for (col_index, cell) in row.iter().enumerate() {
                if cell.state != Visibility::Revealed {
                    continue;
                }

                let bit = 1u32
                    << context.board.index_of(Position::new_3d(
                        layer_index as i16,
                        row_index as i16,
                        col_index as i16,
                    ));
                known_mask |= bit;

                if cell.answer == Answer::Innocent {
                    known_innocent_mask |= bit;
                }
            }
        }
    }

    Ok((known_mask, known_innocent_mask))
}

#[cfg(test)]
fn known_masks_from_revealed_cells(puzzle: &Puzzle) -> Result<(BitMask, BitMask), SolveError> {
    known_masks_from_revealed_cells_2d(puzzle)
}

fn flat_analysis_to_2d(analysis: FlatClueAnalysis) -> ClueAnalysis {
    let cols = analysis.board.cols as usize;
    let plane_size = analysis.board.rows as usize * cols;

    let forced_answers = analysis
        .forced_answers
        .chunks(plane_size.max(1))
        .next()
        .unwrap_or(&[])
        .chunks(cols.max(1))
        .map(|row| row.to_vec())
        .collect();

    ClueAnalysis {
        has_solution: analysis.has_solution,
        forced_answers,
    }
}

fn flat_analysis_to_3d(analysis: FlatClueAnalysis) -> ClueAnalysis3D {
    let cols = analysis.board.cols as usize;
    let rows = analysis.board.rows as usize;
    let plane_size = rows * cols;

    let forced_answers = analysis
        .forced_answers
        .chunks(plane_size.max(1))
        .map(|layer| {
            layer
                .chunks(cols.max(1))
                .map(|row| row.to_vec())
                .collect()
        })
        .collect();

    ClueAnalysis3D {
        has_solution: analysis.has_solution,
        forced_answers,
    }
}

impl CompileContext {
    fn analysis_from_assignments(
        &self,
        assignments: &[BitMask],
        known_mask: BitMask,
        known_innocent_mask: BitMask,
        variable_mask: BitMask,
    ) -> FlatClueAnalysis {
        let mut always_innocent = self.full_mask;
        let mut always_criminal = self.full_mask;

        for assignment in assignments {
            always_innocent &= *assignment;
            always_criminal &= (!assignment) & self.full_mask;
        }

        let forced_answers = (0..self.cell_count)
            .map(|index| {
                if assignments.is_empty() {
                    return ForcedAnswer::Neither;
                }

                let bit = 1u32 << index;
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
            .collect();

        FlatClueAnalysis {
            has_solution: !assignments.is_empty(),
            board: self.board,
            forced_answers,
        }
    }

    fn implicated_mask(&self, clue: &CompiledClue) -> BitMask {
        match clue {
            CompiledClue::Nonsense => 0,
            CompiledClue::Count { mask, .. } => *mask,
            CompiledClue::CountWithMember {
                mask, member_mask, ..
            } => *mask | *member_mask,
            CompiledClue::Compare {
                left_mask,
                right_mask,
                ..
            } => *left_mask | *right_mask,
            CompiledClue::Connected { mask, .. } => *mask,
            CompiledClue::Quantified {
                group, predicate, ..
            } => {
                self.implicated_mask_for_group(group)
                    | self.implicated_mask_for_predicate(group, predicate)
            }
        }
    }

    fn new(board: BoardShape, cells: &[&Cell]) -> Result<Self, SolveError> {
        let cell_count = board.cell_count();
        if cell_count > MAX_CELL_COUNT {
            return Err(SolveError::TooManyCells(cell_count));
        }
        let full_mask = if cell_count == MAX_CELL_COUNT {
            BitMask::MAX
        } else {
            (1u32 << cell_count) - 1
        };
        let mut positions = vec![Position::new(0, 0); cell_count];
        let mut names = HashMap::new();
        let mut role_masks: HashMap<Role, BitMask> = HashMap::new();
        let mut edge_mask = 0;
        let mut corner_mask = 0;

        for (index, cell) in cells.iter().enumerate() {
            let position = board.position_of_index(index);
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

        let mut orthogonal_masks = vec![0; cell_count];
        for (index, position) in positions.iter().enumerate() {
            orthogonal_masks[index] = Self::positions_to_mask(&board, &board.orthogonal_neighbors(*position));
        }

        Ok(Self {
            board,
            cell_count,
            cols: board.cols,
            full_mask,
            edge_mask,
            corner_mask,
            names,
            role_masks,
            orthogonal_masks,
            positions,
        })
    }

    fn from_2d_puzzle(puzzle: &Puzzle) -> Result<Self, SolveError> {
        let rows = puzzle.cells.len();
        let cols = puzzle
            .cells
            .first()
            .map(|row| row.len())
            .unwrap_or_default();

        if puzzle.cells.iter().any(|row| row.len() != cols) {
            return Err(SolveError::RaggedBoard);
        }

        let board = BoardShape::new(rows as u8, cols as u8);
        let cells = puzzle
            .cells
            .iter()
            .flat_map(|row| row.iter())
            .collect::<Vec<_>>();
        Self::new(board, &cells)
    }

    fn from_3d_puzzle(puzzle: &Puzzle3D) -> Result<Self, SolveError> {
        let depth = puzzle.cells.len();
        let rows = puzzle
            .cells
            .first()
            .map(|layer| layer.len())
            .unwrap_or_default();
        let cols = puzzle
            .cells
            .first()
            .and_then(|layer| layer.first())
            .map(|row| row.len())
            .unwrap_or_default();

        if puzzle.cells.iter().any(|layer| layer.len() != rows) {
            return Err(SolveError::RaggedBoard);
        }
        if puzzle
            .cells
            .iter()
            .flat_map(|layer| layer.iter())
            .any(|row| row.len() != cols)
        {
            return Err(SolveError::RaggedBoard);
        }

        let board = BoardShape::new_3d(depth as u8, rows as u8, cols as u8);
        let cells = puzzle
            .cells
            .iter()
            .flat_map(|layer| layer.iter().flat_map(|row| row.iter()))
            .collect::<Vec<_>>();
        Self::new(board, &cells)
    }

    fn compile_clue(&self, clue: &Clue) -> Result<CompiledClue, SolveError> {
        match clue {
            Clue::Nonsense { .. } => Ok(CompiledClue::Nonsense),
            Clue::Declaration { name, answer } => Ok(CompiledClue::Count {
                mask: self.position_to_bit(self.name_position(name)?),
                answer: *answer,
                count: Count::Number(1),
            }),
            Clue::CountCells {
                selector,
                answer,
                count,
                filter,
            } => Ok(CompiledClue::Count {
                mask: self.selector_mask(selector)? & self.filter_mask(*filter)?,
                answer: *answer,
                count: *count,
            }),
            Clue::NamedCountCells {
                name,
                selector,
                answer,
                number,
                filter,
            } => Ok(CompiledClue::CountWithMember {
                mask: self.selector_mask(selector)? & self.filter_mask(*filter)?,
                member_mask: self.position_to_bit(self.name_position(name)?),
                answer: *answer,
                number: *number,
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
                group: self.compile_group(group)?,
                predicate: self.compile_predicate(predicate)?,
                quantifier: *quantifier,
            }),
        }
    }

    fn compile_group(&self, group: &PersonGroup) -> Result<CompiledGroup, SolveError> {
        Ok(match group {
            PersonGroup::Any => CompiledGroup::Static {
                mask: self.full_mask,
            },
            PersonGroup::Filter { filter } => CompiledGroup::Static {
                mask: self.filter_mask(*filter)?,
            },
            PersonGroup::Line { line } => CompiledGroup::Static {
                mask: self.line_mask(*line)?,
            },
            PersonGroup::Role { role } => CompiledGroup::Static {
                mask: self.role_mask(role),
            },
            PersonGroup::SelectedCells {
                selector,
                answer,
                filter,
            } => CompiledGroup::Answered {
                mask: self.selector_mask(selector)? & self.filter_mask(*filter)?,
                answer: *answer,
            },
        })
    }

    fn compile_predicate(
        &self,
        predicate: &PersonPredicate,
    ) -> Result<CompiledPredicate, SolveError> {
        let mut masks_by_origin = vec![0; self.cell_count];

        match predicate {
            PersonPredicate::Neighbor {
                answer,
                count,
                filter,
            } => {
                let filter_mask = self.filter_mask(*filter)?;
                for (index, position) in self.positions.iter().enumerate() {
                    masks_by_origin[index] =
                        Self::positions_to_mask(&self.board, &self.board.touching_neighbors(*position))
                            & filter_mask;
                }

                Ok(CompiledPredicate::Count {
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
                    Direction::Front => Direction::Back.offset(),
                    Direction::Back => Direction::Front.offset(),
                };

                for (index, position) in self.positions.iter().enumerate() {
                    let shifted = position.shifted(offset);
                    if self.board.contains(shifted) {
                        masks_by_origin[index] = self.position_to_bit(shifted);
                    }
                }

                Ok(CompiledPredicate::Count {
                    masks_by_origin,
                    answer: *answer,
                    count: Count::Number(1),
                })
            }
            PersonPredicate::Neighboring { name } => Ok(CompiledPredicate::Structural {
                matching_origins: Self::positions_to_mask(
                    &self.board,
                    &self.board.touching_neighbors(self.name_position(name)?),
                ),
            }),
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
            CompiledClue::CountWithMember {
                mask,
                member_mask,
                answer,
                number,
            } => {
                mask & member_mask != 0
                    && self.matches_count(assignment, *mask, *answer, Count::Number(*number))
                    && self.matches_count(assignment, *member_mask, *answer, Count::Number(1))
            }
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
                group,
                predicate,
                quantifier,
            } => {
                let matching_people = (0..self.cell_count)
                    .filter(|index| self.group_contains(assignment, group, *index))
                    .filter(|index| self.predicate_matches(assignment, predicate, *index))
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
            CellSelector::Layer { layer } => self.line_mask(Line::Layer(*layer))?,
            CellSelector::Neighbor { name } => Self::positions_to_mask(
                &self.board,
                &self.board.touching_neighbors(self.name_position(name)?),
            ),
            CellSelector::Direction { name, direction } => Self::positions_to_mask(
                &self.board,
                &self
                    .board
                    .tiles_in_direction(self.name_position(name)?, direction.offset()),
            ),
            CellSelector::Row { row } => self.line_mask(Line::Row(*row))?,
            CellSelector::Col { col } => self.line_mask(Line::Col(*col))?,
            CellSelector::Between {
                first_name,
                second_name,
            } => {
                let first = self.name_position(first_name)?;
                let second = self.name_position(second_name)?;
                Self::positions_to_mask(&self.board, &self.board.positions_between(first, second))
            }
            CellSelector::SharedNeighbor {
                first_name,
                second_name,
            } => Self::positions_to_mask(
                &self.board,
                &self.board.common_neighbors(
                    self.name_position(first_name)?,
                    self.name_position(second_name)?,
                ),
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

    fn group_contains(&self, assignment: BitMask, group: &CompiledGroup, index: usize) -> bool {
        let bit = 1u32 << index;

        match group {
            CompiledGroup::Static { mask } => mask & bit != 0,
            CompiledGroup::Answered { mask, answer } => {
                mask & bit != 0 && self.matches_count(assignment, bit, *answer, Count::Number(1))
            }
        }
    }

    fn predicate_matches(
        &self,
        assignment: BitMask,
        predicate: &CompiledPredicate,
        index: usize,
    ) -> bool {
        match predicate {
            CompiledPredicate::Count {
                masks_by_origin,
                answer,
                count,
            } => self.matches_count(assignment, masks_by_origin[index], *answer, *count),
            CompiledPredicate::Structural { matching_origins } => {
                matching_origins & (1u32 << index) != 0
            }
        }
    }

    fn implicated_mask_for_group(&self, group: &CompiledGroup) -> BitMask {
        match group {
            CompiledGroup::Static { mask } | CompiledGroup::Answered { mask, .. } => *mask,
        }
    }

    fn implicated_mask_for_predicate(
        &self,
        group: &CompiledGroup,
        predicate: &CompiledPredicate,
    ) -> BitMask {
        let candidate_mask = self.implicated_mask_for_group(group);

        match predicate {
            CompiledPredicate::Count {
                masks_by_origin, ..
            } => (0..self.cell_count)
                .filter(|index| candidate_mask & (1u32 << index) != 0)
                .fold(0, |mask, index| mask | masks_by_origin[index]),
            CompiledPredicate::Structural { matching_origins } => *matching_origins,
        }
    }

    fn role_mask(&self, role: &str) -> BitMask {
        self.role_masks.get(role).copied().unwrap_or(0)
    }

    fn line_mask(&self, line: Line) -> Result<BitMask, SolveError> {
        match line {
            Line::Layer(layer) => {
                if layer >= self.board.depth {
                    return Err(SolveError::InvalidLayer(layer));
                }

                Ok(Self::positions_to_mask(
                    &self.board,
                    &self.board.layer_positions(layer),
                ))
            }
            Line::Row(row) => {
                if row >= self.board.rows {
                    return Err(SolveError::InvalidRow(row));
                }

                Ok(Self::positions_to_mask(
                    &self.board,
                    &self.board.row_positions(row),
                ))
            }
            Line::Col(col) => {
                if col.index() >= self.cols {
                    return Err(SolveError::InvalidColumn(col));
                }

                Ok(Self::positions_to_mask(
                    &self.board,
                    &self.board.col_positions(col.index()),
                ))
            }
        }
    }

    fn filter_mask(&self, filter: CellFilter) -> Result<BitMask, SolveError> {
        Ok(match filter {
            CellFilter::Any => self.full_mask,
            CellFilter::Edge => self.edge_mask,
            CellFilter::Corner => self.corner_mask,
            CellFilter::Line(line) => self.line_mask(line)?,
        })
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
        1u32 << self.board.index_of(position)
    }

    fn positions_to_mask(board: &BoardShape, positions: &[Position]) -> BitMask {
        positions
            .iter()
            .fold(0, |mask, position| mask | (1u32 << board.index_of(*position)))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        clue::{
            CellFilter, CellSelector, Clue, Column, Comparison, Count, Direction, Line,
            PersonGroup, PersonPredicate, Quantifier,
        },
        puzzle::{Cell, Puzzle, Puzzle3D, Visibility},
        solver::{
            ClueAnalysis, ClueAnalysis3D, ForcedAnswer, analyze_clues, analyze_clues_3d,
            analyze_puzzle, analyze_revealed_puzzle, known_masks_from_revealed_cells,
            solve_clues_with_known_mask,
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
                            emoji: None,
                            clue: dummy_clue.clone(),
                            answer: Answer::Innocent,
                            state: Visibility::Hidden,
                        }
                    })
                    .collect()
            })
            .collect();

        Puzzle {
            author: None,
            cells,
        }
    }

    fn small_test_puzzle() -> Puzzle {
        let dummy_clue = Clue::CountCells {
            selector: CellSelector::Board,
            answer: Answer::Innocent,
            count: Count::AtLeast(0),
            filter: CellFilter::Any,
        };

        let cells = (0..2)
            .map(|row| {
                (0..2)
                    .map(|col| {
                        let index = row * 2 + col;
                        Cell {
                            name: NAMES[index].to_string(),
                            role: "Role".to_string(),
                            emoji: None,
                            clue: dummy_clue.clone(),
                            answer: Answer::Innocent,
                            state: Visibility::Hidden,
                        }
                    })
                    .collect()
            })
            .collect();

        Puzzle {
            author: None,
            cells,
        }
    }

    fn small_test_puzzle_3d() -> Puzzle3D {
        let dummy_clue = Clue::CountCells {
            selector: CellSelector::Board,
            answer: Answer::Innocent,
            count: Count::AtLeast(0),
            filter: CellFilter::Any,
        };

        let cells = (0..2)
            .map(|layer| {
                (0..2)
                    .map(|row| {
                        (0..2)
                            .map(|col| {
                                let index = layer * 4 + row * 2 + col;
                                Cell {
                                    name: NAMES[index].to_string(),
                                    role: "Role".to_string(),
                                    emoji: None,
                                    clue: dummy_clue.clone(),
                                    answer: Answer::Innocent,
                                    state: Visibility::Hidden,
                                }
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();

        Puzzle3D {
            author: None,
            cells,
        }
    }

    fn forced_at(analysis: &ClueAnalysis, row: usize, col: usize) -> ForcedAnswer {
        analysis.forced_answers[row][col]
    }

    fn forced_at_3d(
        analysis: &ClueAnalysis3D,
        layer: usize,
        row: usize,
        col: usize,
    ) -> ForcedAnswer {
        analysis.forced_answers[layer][row][col]
    }

    fn forced_at_flat(
        analysis: &crate::solver::FlatClueAnalysis,
        row: usize,
        col: usize,
    ) -> ForcedAnswer {
        analysis.forced_answers[row * analysis.board.cols as usize + col]
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
    fn board_wide_innocent_count_supports_non_default_board_size() {
        let puzzle = small_test_puzzle();
        let clues = [Clue::CountCells {
            selector: CellSelector::Board,
            answer: Answer::Innocent,
            count: Count::Number(4),
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
    fn layer_selector_can_force_an_entire_layer_in_3d() {
        let puzzle = small_test_puzzle_3d();
        let clues = [Clue::CountCells {
            selector: CellSelector::Layer { layer: 0 },
            answer: Answer::Innocent,
            count: Count::Number(4),
            filter: CellFilter::Any,
        }];

        let analysis = analyze_clues_3d(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        for row in 0..2 {
            for col in 0..2 {
                assert_eq!(forced_at_3d(&analysis, 0, row, col), ForcedAnswer::Innocent);
                assert_eq!(forced_at_3d(&analysis, 1, row, col), ForcedAnswer::Neither);
            }
        }
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
    fn neighbor_selector_can_be_filtered_to_a_row() {
        let puzzle = test_puzzle();
        let anchor_name = puzzle.cells[1][1].name.clone();
        let clues = [Clue::CountCells {
            selector: CellSelector::Neighbor { name: anchor_name },
            answer: Answer::Innocent,
            count: Count::Number(3),
            filter: CellFilter::Line(Line::Row(2)),
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        assert_eq!(forced_at(&analysis, 2, 0), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 2, 1), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 2, 2), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 0, 0), ForcedAnswer::Neither);
        assert_eq!(forced_at(&analysis, 1, 1), ForcedAnswer::Neither);
    }

    #[test]
    fn named_count_cells_can_force_named_membership() {
        let puzzle = test_puzzle();
        let anchor_name = puzzle.cells[1][1].name.clone();
        let member_name = puzzle.cells[2][0].name.clone();
        let clues = [Clue::NamedCountCells {
            name: member_name,
            selector: CellSelector::Neighbor { name: anchor_name },
            answer: Answer::Innocent,
            number: 3,
            filter: CellFilter::Line(Line::Row(2)),
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        assert_eq!(forced_at(&analysis, 2, 0), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 2, 1), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 2, 2), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 0, 0), ForcedAnswer::Neither);
        assert_eq!(forced_at(&analysis, 1, 1), ForcedAnswer::Neither);
    }

    #[test]
    fn named_count_cells_supports_column_membership() {
        let puzzle = test_puzzle();
        let member_name = puzzle.cells[0][3].name.clone();
        let clues = [Clue::NamedCountCells {
            name: member_name,
            selector: CellSelector::Col { col: Column::D },
            answer: Answer::Criminal,
            number: 5,
            filter: CellFilter::Any,
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        for row in 0..5 {
            assert_eq!(forced_at(&analysis, row, 3), ForcedAnswer::Criminal);
        }
        assert_eq!(forced_at(&analysis, 0, 0), ForcedAnswer::Neither);
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
    fn quantified_clue_can_select_answered_cells_and_apply_structural_predicate() {
        let puzzle = test_puzzle();
        let anchor_name = puzzle.cells[0][0].name.clone();
        let gabe_name = puzzle.cells[1][0].name.clone();
        let clues = [Clue::Quantified {
            quantifier: Quantifier::Exactly(1),
            group: PersonGroup::SelectedCells {
                selector: CellSelector::Direction {
                    name: anchor_name,
                    direction: Direction::Right,
                },
                answer: Answer::Innocent,
                filter: CellFilter::Any,
            },
            predicate: PersonPredicate::Neighboring { name: gabe_name },
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        assert_eq!(forced_at(&analysis, 0, 1), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 0, 2), ForcedAnswer::Neither);
        assert_eq!(forced_at(&analysis, 0, 3), ForcedAnswer::Neither);
    }

    #[test]
    fn nonsense_clue_adds_no_constraints() {
        let puzzle = test_puzzle();
        let clues = [Clue::Nonsense {
            text: "noise".to_string(),
        }];

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
    fn declaration_clue_forces_named_answer() {
        let puzzle = test_puzzle();
        let declared_name = puzzle.cells[0][0].name.clone();
        let clues = [Clue::Declaration {
            name: declared_name,
            answer: Answer::Innocent,
        }];

        let analysis = analyze_clues(&puzzle, &clues).unwrap();

        assert!(analysis.has_solution);
        assert_eq!(forced_at(&analysis, 0, 0), ForcedAnswer::Innocent);
        assert_eq!(forced_at(&analysis, 0, 1), ForcedAnswer::Neither);
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
        assert_eq!(forced_at_flat(&solved.analysis, 0, 0), ForcedAnswer::Innocent);
        assert_eq!(forced_at_flat(&solved.analysis, 4, 3), ForcedAnswer::Neither);
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
        let clues = [Clue::Nonsense {
            text: "noise".to_string(),
        }];
        let (known_mask, known_innocent_mask) = known_masks_from_revealed_cells(&puzzle).unwrap();

        let solved =
            solve_clues_with_known_mask(&puzzle, &clues, known_mask, known_innocent_mask).unwrap();

        assert_eq!(solved.assignments.len(), 1);
        assert_eq!(forced_at_flat(&solved.analysis, 0, 0), ForcedAnswer::Innocent);
        assert_eq!(forced_at_flat(&solved.analysis, 4, 3), ForcedAnswer::Neither);
    }
}
