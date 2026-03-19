use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};

use crate::{
    clue::{CellFilter, CellSelector, Clue, Column, Comparison, Count, Direction, Line, Parity},
    geometry::{BoardShape, Position},
    puzzle::{Cell, Puzzle, Visibility},
    solver::{ForcedAnswer, SolveError, solve_clues_with_known_mask},
    types::{Answer, NAMES, Name, ROLES, Role},
};

const ROWS: usize = 5;
const COLS: usize = 4;
const CELL_COUNT: usize = ROWS * COLS;
const MIN_ROLE_POOL_SIZE: usize = 10;
const MAX_ROLE_POOL_SIZE: usize = 15;
const MAX_PUZZLE_ATTEMPTS: usize = 64;
const MAX_CLUE_ATTEMPTS: usize = 128;
const FULL_MASK: u32 = (1u32 << CELL_COUNT) - 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedPuzzle {
    pub puzzle: Puzzle,
    pub first_revealed_name: Name,
    pub first_revealed_answer: Answer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerateError {
    NotEnoughNames,
    NotEnoughRoles,
    Solve(SolveError),
    FailedToGenerate,
}

impl From<SolveError> for GenerateError {
    fn from(error: SolveError) -> Self {
        Self::Solve(error)
    }
}

pub fn generate_puzzle() -> Result<GeneratedPuzzle, GenerateError> {
    let mut rng = rand::thread_rng();
    generate_puzzle_with_rng(&mut rng)
}

pub fn generate_puzzle_with_seed(seed: u64) -> Result<GeneratedPuzzle, GenerateError> {
    let mut rng = StdRng::seed_from_u64(seed);
    generate_puzzle_with_rng(&mut rng)
}

pub(crate) fn generate_puzzle_with_rng<R: Rng + ?Sized>(
    rng: &mut R,
) -> Result<GeneratedPuzzle, GenerateError> {
    for _ in 0..MAX_PUZZLE_ATTEMPTS {
        if let Some(generated) = try_generate_puzzle(rng)? {
            return Ok(generated);
        }
    }

    Err(GenerateError::FailedToGenerate)
}

#[derive(Debug, Clone)]
struct Layout {
    board: BoardShape,
    names: Vec<Name>,
    roles: Vec<Role>,
    positions_by_name: HashMap<Name, Position>,
    indices_by_role: HashMap<Role, Vec<usize>>,
}

impl Layout {
    fn from_puzzle(puzzle: &Puzzle, roles: Vec<Role>) -> Self {
        let board = BoardShape::new(ROWS as u8, COLS as u8);
        let mut names = Vec::with_capacity(CELL_COUNT);
        let mut positions_by_name = HashMap::new();
        let mut indices_by_role: HashMap<Role, Vec<usize>> = HashMap::new();

        for (row_index, row) in puzzle.cells.iter().enumerate() {
            for (col_index, cell) in row.iter().enumerate() {
                let index = row_index * COLS + col_index;
                let position = Position::new(row_index as i16, col_index as i16);
                names.push(cell.name.clone());
                positions_by_name.insert(cell.name.clone(), position);
                indices_by_role
                    .entry(cell.role.clone())
                    .or_default()
                    .push(index);
            }
        }

        Self {
            board,
            names,
            roles,
            positions_by_name,
            indices_by_role,
        }
    }

    fn position_of_name(&self, name: &str) -> Position {
        self.positions_by_name[name]
    }

    fn index_of_position(&self, position: Position) -> usize {
        position.row as usize * COLS + position.col as usize
    }

    fn positions_for_line(&self, line: Line) -> Vec<Position> {
        match line {
            Line::Row(row) => self.board.row_positions(row),
            Line::Col(col) => self.board.col_positions(col.index()),
        }
    }

    fn positions_for_selector(&self, selector: &CellSelector) -> Vec<Position> {
        match selector {
            CellSelector::Board => (0..ROWS)
                .flat_map(|row| (0..COLS).map(move |col| Position::new(row as i16, col as i16)))
                .collect(),
            CellSelector::Neighbor { name } => {
                self.board.touching_neighbors(self.position_of_name(name))
            }
            CellSelector::Direction { name, direction } => self
                .board
                .tiles_in_direction(self.position_of_name(name), direction.offset()),
            CellSelector::Row { row } => self.board.row_positions(*row),
            CellSelector::Col { col } => self.board.col_positions(col.index()),
            CellSelector::Between {
                first_name,
                second_name,
            } => {
                let first = self.position_of_name(first_name);
                let second = self.position_of_name(second_name);

                if first.row == second.row || first.col == second.col {
                    self.board.positions_between(first, second)
                } else {
                    Vec::new()
                }
            }
            CellSelector::SharedNeighbor {
                first_name,
                second_name,
            } => self.board.common_neighbors(
                self.position_of_name(first_name),
                self.position_of_name(second_name),
            ),
        }
    }

    fn filtered_positions(&self, selector: &CellSelector, filter: CellFilter) -> Vec<Position> {
        self.positions_for_selector(selector)
            .into_iter()
            .filter(|position| match filter {
                CellFilter::Any => true,
                CellFilter::Edge => self.board.is_edge(*position),
                CellFilter::Corner => self.board.is_corner(*position),
            })
            .collect()
    }

    fn matching_count_in_positions(
        &self,
        assignment: u32,
        positions: &[Position],
        answer: Answer,
    ) -> usize {
        positions
            .iter()
            .filter(|position| answer_for_index(assignment, self.index_of_position(**position)) == answer)
            .count()
    }

    fn matching_count_in_role(&self, assignment: u32, role: &str, answer: Answer) -> usize {
        self.indices_by_role
            .get(role)
            .into_iter()
            .flatten()
            .filter(|index| answer_for_index(assignment, **index) == answer)
            .count()
    }

    fn all_lines(&self) -> Vec<Line> {
        let mut lines = (0..ROWS as u8).map(Line::Row).collect::<Vec<_>>();
        lines.extend([
            Line::Col(Column::A),
            Line::Col(Column::B),
            Line::Col(Column::C),
            Line::Col(Column::D),
        ]);
        lines
    }
}

fn try_generate_puzzle<R: Rng + ?Sized>(
    rng: &mut R,
) -> Result<Option<GeneratedPuzzle>, GenerateError> {
    if NAMES.len() < CELL_COUNT {
        return Err(GenerateError::NotEnoughNames);
    }

    if ROLES.len() < MIN_ROLE_POOL_SIZE {
        return Err(GenerateError::NotEnoughRoles);
    }

    let names = sample_names(rng);
    let roles = sample_roles(rng)?;
    let mut puzzle = empty_puzzle(&names, &roles);
    let layout = Layout::from_puzzle(&puzzle, distinct_roles(&roles));

    let first_index = rng.gen_range(0..CELL_COUNT);
    let first_answer = random_answer(rng);

    let mut clues = vec![None; CELL_COUNT];
    let mut answers = vec![None; CELL_COUNT];
    let mut known_mask = 0u32;
    let mut known_innocent_mask = 0u32;
    let mut pending = vec![first_index];

    reveal_answer(
        &mut answers,
        &mut known_mask,
        &mut known_innocent_mask,
        first_index,
        first_answer,
    );

    while let Some(current_index) = pending.pop() {
        let current_clues = collect_clues(&clues);
        let baseline =
            solve_clues_with_known_mask(&puzzle, &current_clues, known_mask, known_innocent_mask)?;

        if !baseline.analysis.has_solution {
            return Ok(None);
        }

        let require_new_force = has_unforced_unknown(&baseline.analysis, known_mask);
        let mut accepted = None;

        for _ in 0..MAX_CLUE_ATTEMPTS {
            let Some(&witness) = baseline.assignments.choose(rng) else {
                return Ok(None);
            };

            let Some(candidate) = sample_clue(rng, &layout, witness) else {
                continue;
            };

            let mut next_clues = current_clues.clone();
            next_clues.push(candidate.clone());

            let solved =
                solve_clues_with_known_mask(&puzzle, &next_clues, known_mask, known_innocent_mask)?;

            if !solved.analysis.has_solution {
                continue;
            }

            let newly_forced =
                newly_forced_unknown_indices(&baseline.analysis, &solved.analysis, known_mask);
            let forced_unknown = forced_unknown_indices(&solved.analysis, known_mask);

            if require_new_force && newly_forced.is_empty() {
                continue;
            }

            if known_mask != FULL_MASK && forced_unknown.is_empty() {
                continue;
            }

            accepted = Some((candidate, solved.analysis, newly_forced, forced_unknown));
            break;
        }

        let Some((candidate, analysis, newly_forced, forced_unknown)) = accepted else {
            return Ok(None);
        };

        clues[current_index] = Some(candidate);

        if known_mask == FULL_MASK {
            continue;
        }

        let next_choices = if !newly_forced.is_empty() {
            newly_forced
        } else {
            forced_unknown
        };

        let Some(&next_index) = next_choices.choose(rng) else {
            return Ok(None);
        };

        let Some(next_answer) = forced_answer_as_answer(forced_answer_at(&analysis, next_index))
        else {
            return Ok(None);
        };

        reveal_answer(
            &mut answers,
            &mut known_mask,
            &mut known_innocent_mask,
            next_index,
            next_answer,
        );
        pending.push(next_index);
    }

    if known_mask != FULL_MASK || clues.iter().any(Option::is_none) {
        return Ok(None);
    }

    for row in 0..ROWS {
        for col in 0..COLS {
            let index = row * COLS + col;
            puzzle.cells[row][col].clue = clues[index].clone().unwrap();
            puzzle.cells[row][col].answer = answers[index].unwrap();
            puzzle.cells[row][col].state = if index == first_index {
                Visibility::Revealed
            } else {
                Visibility::Hidden
            };
        }
    }

    Ok(Some(GeneratedPuzzle {
        first_revealed_name: puzzle.cells[first_index / COLS][first_index % COLS]
            .name
            .clone(),
        first_revealed_answer: first_answer,
        puzzle,
    }))
}

fn sample_names<R: Rng + ?Sized>(rng: &mut R) -> Vec<Name> {
    let mut names = NAMES.iter().map(|name| (*name).to_string()).collect::<Vec<_>>();
    names.shuffle(rng);
    names.truncate(CELL_COUNT);
    names
}

fn sample_roles<R: Rng + ?Sized>(rng: &mut R) -> Result<Vec<Role>, GenerateError> {
    let mut roles = ROLES.iter().map(|role| (*role).to_string()).collect::<Vec<_>>();
    let pool_size = rng.gen_range(MIN_ROLE_POOL_SIZE..=MAX_ROLE_POOL_SIZE);

    if roles.len() < pool_size {
        return Err(GenerateError::NotEnoughRoles);
    }

    roles.shuffle(rng);
    let role_pool = roles.into_iter().take(pool_size).collect::<Vec<_>>();
    let mut assigned_roles = role_pool.clone();

    while assigned_roles.len() < CELL_COUNT {
        assigned_roles.push(role_pool.choose(rng).unwrap().clone());
    }

    assigned_roles.shuffle(rng);
    Ok(assigned_roles)
}

fn distinct_roles(roles: &[Role]) -> Vec<Role> {
    let mut distinct = Vec::new();

    for role in roles {
        if !distinct.contains(role) {
            distinct.push(role.clone());
        }
    }

    distinct
}

fn empty_puzzle(names: &[Name], roles: &[Role]) -> Puzzle {
    let cells = (0..ROWS)
        .map(|row| {
            (0..COLS)
                .map(|col| {
                    let index = row * COLS + col;
                    Cell {
                        name: names[index].clone(),
                        role: roles[index].clone(),
                        clue: Clue::Nonsense,
                        answer: Answer::Innocent,
                        state: Visibility::Hidden,
                    }
                })
                .collect()
        })
        .collect();

    Puzzle { cells }
}

fn collect_clues(clues: &[Option<Clue>]) -> Vec<Clue> {
    clues.iter().flatten().cloned().collect()
}

fn reveal_answer(
    answers: &mut [Option<Answer>],
    known_mask: &mut u32,
    known_innocent_mask: &mut u32,
    index: usize,
    answer: Answer,
) {
    answers[index] = Some(answer);
    *known_mask |= 1u32 << index;

    if answer == Answer::Innocent {
        *known_innocent_mask |= 1u32 << index;
    } else {
        *known_innocent_mask &= !(1u32 << index);
    }
}

fn sample_clue<R: Rng + ?Sized>(rng: &mut R, layout: &Layout, assignment: u32) -> Option<Clue> {
    for _ in 0..32 {
        let candidate = match rng.gen_range(0..10) {
            0..=5 => sample_count_cells_clue(rng, layout, assignment),
            6..=7 => sample_direct_relation_clue(rng, layout, assignment),
            8 => sample_role_count_clue(rng, layout, assignment),
            9 => {
                if rng.gen_bool(0.5) {
                    sample_roles_comparison_clue(rng, layout, assignment)
                } else {
                    sample_line_comparison_clue(rng, layout, assignment)
                }
            }
            _ => None,
        };

        if candidate.is_some() {
            return candidate;
        }
    }

    None
}

fn sample_count_cells_clue<R: Rng + ?Sized>(
    rng: &mut R,
    layout: &Layout,
    assignment: u32,
) -> Option<Clue> {
    for _ in 0..32 {
        let selector = sample_selector(rng, layout)?;
        let filter = sample_filter(rng);
        let positions = layout.filtered_positions(&selector, filter);

        if positions.is_empty() {
            continue;
        }

        let answer = random_answer(rng);
        let matching = layout.matching_count_in_positions(assignment, &positions, answer);
        let count = sample_count_from_truth(rng, matching);

        return Some(Clue::CountCells {
            selector,
            answer,
            count,
            filter,
        });
    }

    None
}

fn sample_direct_relation_clue<R: Rng + ?Sized>(
    rng: &mut R,
    layout: &Layout,
    assignment: u32,
) -> Option<Clue> {
    let name = layout.names.choose(rng)?.clone();
    let position = layout.position_of_name(&name);
    let directions = [
        Direction::Above,
        Direction::Below,
        Direction::Left,
        Direction::Right,
    ]
    .into_iter()
    .filter(|direction| layout.board.contains(position.shifted(direction.offset())))
    .collect::<Vec<_>>();
    let direction = *directions.choose(rng)?;
    let target = position.shifted(direction.offset());
    let answer = answer_for_index(assignment, layout.index_of_position(target));

    Some(Clue::DirectRelation {
        name,
        answer,
        direction,
    })
}

fn sample_role_count_clue<R: Rng + ?Sized>(
    rng: &mut R,
    layout: &Layout,
    assignment: u32,
) -> Option<Clue> {
    let role = layout.roles.choose(rng)?.clone();
    let answer = random_answer(rng);
    let matching = layout.matching_count_in_role(assignment, &role, answer);

    Some(Clue::RoleCount {
        role,
        answer,
        count: sample_count_from_truth(rng, matching),
    })
}

fn sample_roles_comparison_clue<R: Rng + ?Sized>(
    rng: &mut R,
    layout: &Layout,
    assignment: u32,
) -> Option<Clue> {
    if layout.roles.len() < 2 {
        return None;
    }

    let first_role = layout.roles.choose(rng)?.clone();
    let second_role = layout
        .roles
        .iter()
        .filter(|role| **role != first_role)
        .cloned()
        .collect::<Vec<_>>()
        .choose(rng)?
        .clone();
    let answer = random_answer(rng);
    let first_count = layout.matching_count_in_role(assignment, &first_role, answer);
    let second_count = layout.matching_count_in_role(assignment, &second_role, answer);

    Some(Clue::RolesComparison {
        first_role,
        second_role,
        answer,
        comparison: comparison_from_truth(first_count, second_count),
    })
}

fn sample_line_comparison_clue<R: Rng + ?Sized>(
    rng: &mut R,
    layout: &Layout,
    assignment: u32,
) -> Option<Clue> {
    let lines = layout.all_lines();
    let first_line = *lines.choose(rng)?;
    let second_line = **lines
        .iter()
        .filter(|line| **line != first_line)
        .collect::<Vec<_>>()
        .choose(rng)?;
    let answer = random_answer(rng);
    let first_count =
        layout.matching_count_in_positions(assignment, &layout.positions_for_line(first_line), answer);
    let second_count = layout.matching_count_in_positions(
        assignment,
        &layout.positions_for_line(second_line),
        answer,
    );

    Some(Clue::LineComparison {
        first_line,
        second_line,
        answer,
        comparison: comparison_from_truth(first_count, second_count),
    })
}

fn sample_selector<R: Rng + ?Sized>(rng: &mut R, layout: &Layout) -> Option<CellSelector> {
    Some(match rng.gen_range(0..7) {
        0 => CellSelector::Board,
        1 => CellSelector::Neighbor {
            name: layout.names.choose(rng)?.clone(),
        },
        2 => CellSelector::Direction {
            name: layout.names.choose(rng)?.clone(),
            direction: random_direction(rng),
        },
        3 => CellSelector::Row {
            row: rng.gen_range(0..ROWS as u8),
        },
        4 => CellSelector::Col {
            col: random_column(rng),
        },
        5 => {
            let (first_name, second_name) = random_distinct_names(rng, &layout.names)?;
            CellSelector::Between {
                first_name,
                second_name,
            }
        }
        6 => {
            let (first_name, second_name) = random_distinct_names(rng, &layout.names)?;
            CellSelector::SharedNeighbor {
                first_name,
                second_name,
            }
        }
        _ => return None,
    })
}

fn sample_filter<R: Rng + ?Sized>(rng: &mut R) -> CellFilter {
    match rng.gen_range(0..3) {
        0 => CellFilter::Any,
        1 => CellFilter::Edge,
        _ => CellFilter::Corner,
    }
}

fn sample_count_from_truth<R: Rng + ?Sized>(rng: &mut R, matching: usize) -> Count {
    match rng.gen_range(0..5) {
        0..=2 => Count::Number(matching as i32),
        3 if matching > 0 => Count::AtLeast(rng.gen_range(1..=matching) as i32),
        _ => Count::Parity(if matching % 2 == 0 {
            Parity::Even
        } else {
            Parity::Odd
        }),
    }
}

fn newly_forced_unknown_indices(
    baseline: &crate::solver::ClueAnalysis,
    next: &crate::solver::ClueAnalysis,
    known_mask: u32,
) -> Vec<usize> {
    (0..CELL_COUNT)
        .filter(|index| known_mask & (1u32 << index) == 0)
        .filter(|index| forced_answer_at(baseline, *index) == ForcedAnswer::Neither)
        .filter(|index| forced_answer_at(next, *index) != ForcedAnswer::Neither)
        .collect()
}

fn forced_unknown_indices(analysis: &crate::solver::ClueAnalysis, known_mask: u32) -> Vec<usize> {
    (0..CELL_COUNT)
        .filter(|index| known_mask & (1u32 << index) == 0)
        .filter(|index| forced_answer_at(analysis, *index) != ForcedAnswer::Neither)
        .collect()
}

fn has_unforced_unknown(analysis: &crate::solver::ClueAnalysis, known_mask: u32) -> bool {
    (0..CELL_COUNT)
        .filter(|index| known_mask & (1u32 << index) == 0)
        .any(|index| forced_answer_at(analysis, index) == ForcedAnswer::Neither)
}

fn forced_answer_at(analysis: &crate::solver::ClueAnalysis, index: usize) -> ForcedAnswer {
    analysis.forced_answers[index / COLS][index % COLS]
}

fn forced_answer_as_answer(forced: ForcedAnswer) -> Option<Answer> {
    match forced {
        ForcedAnswer::Criminal => Some(Answer::Criminal),
        ForcedAnswer::Innocent => Some(Answer::Innocent),
        ForcedAnswer::Neither => None,
    }
}

fn answer_for_index(assignment: u32, index: usize) -> Answer {
    if assignment & (1u32 << index) != 0 {
        Answer::Innocent
    } else {
        Answer::Criminal
    }
}

fn random_answer<R: Rng + ?Sized>(rng: &mut R) -> Answer {
    if rng.gen_bool(0.5) {
        Answer::Innocent
    } else {
        Answer::Criminal
    }
}

fn comparison_from_truth(left: usize, right: usize) -> Comparison {
    if left > right {
        Comparison::More
    } else if left < right {
        Comparison::Fewer
    } else {
        Comparison::Equal
    }
}

fn random_direction<R: Rng + ?Sized>(rng: &mut R) -> Direction {
    match rng.gen_range(0..4) {
        0 => Direction::Above,
        1 => Direction::Below,
        2 => Direction::Left,
        _ => Direction::Right,
    }
}

fn random_column<R: Rng + ?Sized>(rng: &mut R) -> Column {
    match rng.gen_range(0..4) {
        0 => Column::A,
        1 => Column::B,
        2 => Column::C,
        _ => Column::D,
    }
}

fn random_distinct_names<R: Rng + ?Sized>(
    rng: &mut R,
    names: &[Name],
) -> Option<(Name, Name)> {
    if names.len() < 2 {
        return None;
    }

    let first = names.choose(rng)?.clone();
    let second = names
        .iter()
        .filter(|name| **name != first)
        .cloned()
        .collect::<Vec<_>>()
        .choose(rng)?
        .clone();

    Some((first, second))
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use crate::solver::solve_clues_with_known_mask;

    use super::{
        COLS, GenerateError, ForcedAnswer, ROWS, distinct_roles, generate_puzzle_with_rng,
        generate_puzzle_with_seed,
    };

    #[test]
    fn generated_puzzle_is_fully_forced_from_the_first_reveal() {
        let mut rng = StdRng::seed_from_u64(7);
        let generated = generate_puzzle_with_rng(&mut rng).unwrap();
        let clues = generated
            .puzzle
            .cells
            .iter()
            .flat_map(|row| row.iter().map(|cell| cell.clue.clone()))
            .collect::<Vec<_>>();
        let first_index = generated
            .puzzle
            .cells
            .iter()
            .flatten()
            .position(|cell| cell.state == crate::puzzle::Visibility::Revealed)
            .unwrap();
        let known_mask = 1u32 << first_index;
        let known_innocent_mask = if generated.first_revealed_answer == crate::types::Answer::Innocent {
            known_mask
        } else {
            0
        };
        let solved =
            solve_clues_with_known_mask(&generated.puzzle, &clues, known_mask, known_innocent_mask)
                .unwrap();

        assert!(solved.analysis.has_solution);

        for row in 0..ROWS {
            for col in 0..COLS {
                let forced = solved.analysis.forced_answers[row][col];
                let answer = generated.puzzle.cells[row][col].answer;

                assert_eq!(
                    forced,
                    match answer {
                        crate::types::Answer::Criminal => ForcedAnswer::Criminal,
                        crate::types::Answer::Innocent => ForcedAnswer::Innocent,
                    },
                );
            }
        }
    }

    #[test]
    fn generated_puzzle_uses_between_ten_and_fifteen_roles() {
        let mut rng = StdRng::seed_from_u64(7);
        let generated = generate_puzzle_with_rng(&mut rng).unwrap();
        let roles = generated
            .puzzle
            .cells
            .iter()
            .flatten()
            .map(|cell| cell.role.clone())
            .collect::<Vec<_>>();
        let distinct = distinct_roles(&roles);

        assert!((10..=15).contains(&distinct.len()));
    }

    #[test]
    fn generator_returns_a_meaningful_error_when_it_cannot_make_progress() {
        let mut rng = StdRng::seed_from_u64(1);
        let result = generate_puzzle_with_rng(&mut rng);

        assert!(!matches!(result, Err(GenerateError::NotEnoughNames)));
    }

    #[test]
    fn same_seed_produces_the_same_puzzle() {
        let first = generate_puzzle_with_seed(7).unwrap();
        let second = generate_puzzle_with_seed(7).unwrap();

        assert_eq!(first, second);
    }
}
