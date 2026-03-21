use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde::{Deserialize, Serialize};

use crate::{
    clue::{
        CRIMINAL_NONSENSE_TEXTS, CellFilter, CellSelector, Clue, Column, Comparison, Count,
        Direction, Line, NONSENSE_TEXTS, Parity,
    },
    geometry::{BoardShape, Position},
    puzzle::{Cell, Puzzle, Puzzle3D, Visibility},
    solver::{
        FlatClueAnalysis, ForcedAnswer, SolutionSet, SolveError, solve_clues_with_known_mask,
        solve_clues_with_known_mask_3d,
    },
    types::{Answer, NAMES, Name, ROLES, Role},
};

pub const DEFAULT_ROWS: u8 = 5;
pub const DEFAULT_COLS: u8 = 4;
pub const MAX_CELL_COUNT: usize = u32::BITS as usize;
#[cfg(test)]
const ROWS: usize = DEFAULT_ROWS as usize;
#[cfg(test)]
const COLS: usize = DEFAULT_COLS as usize;
#[cfg(test)]
const CELL_COUNT: usize = ROWS * COLS;
const MIN_ROLE_POOL_SIZE: usize = 10;
const MAX_ROLE_POOL_SIZE: usize = 15;
const MAX_PUZZLE_ATTEMPTS: usize = 64;
const MAX_CLUE_ATTEMPTS: usize = 128;
const MAX_SCORED_CANDIDATES: usize = 24;
const CLUE_SCORE_TEMPERATURE: f64 = 0.9;
const SID_NAME: &str = "Sid";
const BAKER_ROLE: &str = "Baker";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClueScoreTerms {
    pub combination_size: usize,
    pub combined_new_forced: usize,
    pub standalone_forced: usize,
    pub active_unforced_tiles: usize,
    pub newly_active_unforced_tiles: i32,
    pub active_uncertainty: f64,
    pub active_uncertainty_jump: f64,
    pub combined_gain: f64,
    pub alone_gain: f64,
    pub synergy_gain: f64,
    pub triviality_penalty: f64,
    pub family_weight: f64,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedPuzzle {
    pub puzzle: Puzzle,
    pub first_revealed_name: Name,
    pub first_revealed_answer: Answer,
    pub clue_score_terms: Vec<Vec<ClueScoreTerms>>,
    pub generation_score_series: Vec<ClueScoreTerms>,
    pub generation_clue_texts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedPuzzle3D {
    pub puzzle: Puzzle3D,
    pub first_revealed_name: Name,
    pub first_revealed_answer: Answer,
    pub clue_score_terms: Vec<Vec<Vec<ClueScoreTerms>>>,
    pub generation_score_series: Vec<ClueScoreTerms>,
    pub generation_clue_texts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerateError {
    NotEnoughNames,
    NotEnoughRoles,
    TooManyCells(usize),
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
    generate_puzzle_with_rng(&mut rng, BoardShape::new(DEFAULT_ROWS, DEFAULT_COLS))
}

pub fn generate_puzzle_with_seed(seed: u64) -> Result<GeneratedPuzzle, GenerateError> {
    generate_puzzle_with_seed_and_size(seed, BoardShape::new(DEFAULT_ROWS, DEFAULT_COLS))
}

pub fn generate_puzzle_with_seed_and_size(
    seed: u64,
    board: BoardShape,
) -> Result<GeneratedPuzzle, GenerateError> {
    let mut rng = StdRng::seed_from_u64(seed);
    generate_puzzle_with_rng(&mut rng, board)
}

pub fn generate_puzzle_3d_with_seed_and_size(
    seed: u64,
    board: BoardShape,
) -> Result<GeneratedPuzzle3D, GenerateError> {
    let mut rng = StdRng::seed_from_u64(seed);
    generate_puzzle_3d_with_rng(&mut rng, board)
}

pub fn suggest_clue_for_known_tile(
    puzzle: &Puzzle,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
    current_index: usize,
    current_answer: Answer,
) -> Result<Option<Clue>, GenerateError> {
    let mut rng = rand::thread_rng();
    suggest_clue_for_known_tile_with_rng(
        &mut rng,
        puzzle,
        clues,
        known_mask,
        known_innocent_mask,
        current_index,
        current_answer,
    )
}

pub(crate) fn generate_puzzle_with_rng<R: Rng + ?Sized>(
    rng: &mut R,
    board: BoardShape,
) -> Result<GeneratedPuzzle, GenerateError> {
    generate_puzzle_with_rng_and_instrumentation(rng, board, None)
}

pub(crate) fn generate_puzzle_3d_with_rng<R: Rng + ?Sized>(
    rng: &mut R,
    board: BoardShape,
) -> Result<GeneratedPuzzle3D, GenerateError> {
    generate_puzzle_3d_with_rng_and_instrumentation(rng, board, None)
}

fn suggest_clue_for_known_tile_with_rng<R: Rng + ?Sized>(
    rng: &mut R,
    puzzle: &Puzzle,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
    current_index: usize,
    current_answer: Answer,
) -> Result<Option<Clue>, GenerateError> {
    let rows = puzzle.cells.len();
    let cols = puzzle
        .cells
        .first()
        .map(|row| row.len())
        .unwrap_or_default();
    let cell_count = rows * cols;
    if current_index >= cell_count {
        return Ok(None);
    }

    let layout = Layout::from_puzzle(
        puzzle,
        distinct_roles(
            &puzzle
                .cells
                .iter()
                .flatten()
                .map(|cell| cell.role.clone())
                .collect::<Vec<_>>(),
        ),
    );
    let full_mask = full_mask_for_cell_count(cell_count);
    let explicit_baseline =
        solve_clues_with_known_mask(puzzle, clues, known_mask, known_innocent_mask)?;

    if !explicit_baseline.analysis.has_solution {
        return Ok(None);
    }

    let (closed_known_mask, closed_known_innocent_mask) = closure_known_masks_from_analysis(
        &explicit_baseline.analysis,
        known_mask,
        known_innocent_mask,
    );
    let revealed_only =
        solve_clues_with_known_mask(puzzle, &[], closed_known_mask, closed_known_innocent_mask)?;
    let baseline =
        solve_clues_with_known_mask(puzzle, clues, closed_known_mask, closed_known_innocent_mask)?;

    if !revealed_only.analysis.has_solution || !baseline.analysis.has_solution {
        return Ok(None);
    }

    let require_new_force = has_unforced_unknown(&baseline.analysis, known_mask);
    let mut candidates = Vec::new();

    for _ in 0..MAX_CLUE_ATTEMPTS {
        let Some(witness) = sample_witness_assignment(rng, &baseline, closed_known_mask) else {
            return Ok(None);
        };

        let Some(candidate) = sample_clue(rng, &layout, witness) else {
            continue;
        };

        let mut next_clues = clues.to_vec();
        next_clues.push(candidate.clone());

        let solved = solve_clues_with_known_mask(
            puzzle,
            &next_clues,
            closed_known_mask,
            closed_known_innocent_mask,
        )?;

        if !solved.analysis.has_solution {
            continue;
        }

        let (candidate, solution, newly_forced, forced_unknown) = normalize_generated_clue(
            rng,
            candidate,
            current_answer,
            &baseline,
            &solved,
            closed_known_mask,
        );

        if require_new_force && newly_forced.is_empty() {
            continue;
        }

        if closed_known_mask != full_mask && forced_unknown.is_empty() {
            continue;
        }

        let alone = solve_clues_with_known_mask(
            puzzle,
            &[candidate.clone()],
            closed_known_mask,
            closed_known_innocent_mask,
        )?;

        if !alone.analysis.has_solution {
            continue;
        }

        let target_indices = if !newly_forced.is_empty() {
            &newly_forced
        } else {
            &forced_unknown
        };
        let targets = target_indices
            .iter()
            .filter_map(|index| {
                let forced = forced_answer_at(&solution.analysis, *index);
                (forced != ForcedAnswer::Neither).then_some((*index, forced))
            })
            .collect::<Vec<_>>();
        let combination_size = minimal_forcing_subset_size(
            puzzle,
            &next_clues,
            closed_known_mask,
            closed_known_innocent_mask,
            &targets,
        )?;
        let terms = build_clue_score_terms(
            &layout,
            &candidate,
            &revealed_only,
            &alone,
            &baseline,
            &solution,
            &newly_forced,
            combination_size,
            closed_known_mask,
        );

        candidates.push(ScoredCandidate {
            clue: candidate,
            solution,
            terms,
        });

        if candidates.len() >= MAX_SCORED_CANDIDATES {
            break;
        }
    }

    Ok(choose_scored_candidate(rng, candidates).map(|candidate| candidate.clue))
}

fn generate_puzzle_with_rng_and_instrumentation<R: Rng + ?Sized>(
    rng: &mut R,
    board: BoardShape,
    mut instrumentation: Option<&mut GenerationInstrumentation>,
) -> Result<GeneratedPuzzle, GenerateError> {
    for _ in 0..MAX_PUZZLE_ATTEMPTS {
        let mut attempt_instrumentation = GenerationInstrumentation::default();
        if let Some(generated) =
            try_generate_puzzle(rng, board, Some(&mut attempt_instrumentation))?
        {
            if let Some(global) = instrumentation.as_deref_mut() {
                global.merge(attempt_instrumentation);
            }
            return Ok(generated);
        }
    }

    Err(GenerateError::FailedToGenerate)
}

fn generate_puzzle_3d_with_rng_and_instrumentation<R: Rng + ?Sized>(
    rng: &mut R,
    board: BoardShape,
    mut instrumentation: Option<&mut GenerationInstrumentation>,
) -> Result<GeneratedPuzzle3D, GenerateError> {
    for _ in 0..MAX_PUZZLE_ATTEMPTS {
        let mut attempt_instrumentation = GenerationInstrumentation::default();
        if let Some(generated) =
            try_generate_puzzle_3d(rng, board, Some(&mut attempt_instrumentation))?
        {
            if let Some(global) = instrumentation.as_deref_mut() {
                global.merge(attempt_instrumentation);
            }
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
        let rows = puzzle.cells.len() as u8;
        let cols = puzzle
            .cells
            .first()
            .map(|row| row.len())
            .unwrap_or_default() as u8;
        let board = BoardShape::new(rows, cols);
        let mut names = Vec::with_capacity(rows as usize * cols as usize);
        let mut positions_by_name = HashMap::new();
        let mut indices_by_role: HashMap<Role, Vec<usize>> = HashMap::new();

        for (row_index, row) in puzzle.cells.iter().enumerate() {
            for (col_index, cell) in row.iter().enumerate() {
                let index = row_index * cols as usize + col_index;
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

    fn from_puzzle_3d(puzzle: &Puzzle3D, roles: Vec<Role>) -> Self {
        let depth = puzzle.cells.len() as u8;
        let rows = puzzle
            .cells
            .first()
            .map(|layer| layer.len())
            .unwrap_or_default() as u8;
        let cols = puzzle
            .cells
            .first()
            .and_then(|layer| layer.first())
            .map(|row| row.len())
            .unwrap_or_default() as u8;
        let board = BoardShape::new_3d(depth, rows, cols);
        let mut names = Vec::with_capacity(board.cell_count());
        let mut positions_by_name = HashMap::new();
        let mut indices_by_role: HashMap<Role, Vec<usize>> = HashMap::new();

        for (layer_index, layer) in puzzle.cells.iter().enumerate() {
            for (row_index, row) in layer.iter().enumerate() {
                for (col_index, cell) in row.iter().enumerate() {
                    let position =
                        Position::new_3d(layer_index as i16, row_index as i16, col_index as i16);
                    let index = board.index_of(position);
                    names.push(cell.name.clone());
                    positions_by_name.insert(cell.name.clone(), position);
                    indices_by_role
                        .entry(cell.role.clone())
                        .or_default()
                        .push(index);
                }
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
        self.board.index_of(position)
    }

    fn positions_for_line(&self, line: Line) -> Vec<Position> {
        match line {
            Line::Layer(layer) => self.board.layer_positions(layer),
            Line::Row(row) => self.board.row_positions(row),
            Line::Col(col) => self.board.col_positions(col.index()),
        }
    }

    fn positions_for_selector(&self, selector: &CellSelector) -> Vec<Position> {
        match selector {
            CellSelector::Board => self.board.all_positions(),
            CellSelector::Neighbor { name } => {
                self.board.touching_neighbors(self.position_of_name(name))
            }
            CellSelector::Direction { name, direction } => self
                .board
                .tiles_in_direction(self.position_of_name(name), direction.offset()),
            CellSelector::Layer { layer } => self.board.layer_positions(*layer),
            CellSelector::Row { row } => self.board.row_positions(*row),
            CellSelector::Col { col } => self.board.col_positions(col.index()),
            CellSelector::Between {
                first_name,
                second_name,
            } => {
                let first = self.position_of_name(first_name);
                let second = self.position_of_name(second_name);
                self.board.positions_between(first, second)
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
                CellFilter::Line(line) => self.positions_for_line(line).contains(position),
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
            .filter(|position| {
                answer_for_index(assignment, self.index_of_position(**position)) == answer
            })
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
}

#[derive(Debug, Clone)]
struct ScoredCandidate {
    clue: Clue,
    solution: SolutionSet,
    terms: ClueScoreTerms,
}

#[derive(Debug, Default, Clone)]
struct GenerationInstrumentation {
    candidate_combination_sizes: Vec<usize>,
    selected_combination_sizes: Vec<usize>,
    candidate_newly_forced_count: usize,
    candidate_standalone_zero_count: usize,
    candidate_combination_only_count: usize,
    candidate_combo_eligible_count: usize,
}

impl GenerationInstrumentation {
    fn merge(&mut self, other: Self) {
        self.candidate_combination_sizes
            .extend(other.candidate_combination_sizes);
        self.selected_combination_sizes
            .extend(other.selected_combination_sizes);
        self.candidate_newly_forced_count += other.candidate_newly_forced_count;
        self.candidate_standalone_zero_count += other.candidate_standalone_zero_count;
        self.candidate_combination_only_count += other.candidate_combination_only_count;
        self.candidate_combo_eligible_count += other.candidate_combo_eligible_count;
    }
}

fn cell_count(board: BoardShape) -> usize {
    board.cell_count()
}

fn full_mask_for_cell_count(cell_count: usize) -> u32 {
    if cell_count >= MAX_CELL_COUNT {
        u32::MAX
    } else {
        (1u32 << cell_count) - 1
    }
}

fn try_generate_puzzle<R: Rng + ?Sized>(
    rng: &mut R,
    board: BoardShape,
    mut instrumentation: Option<&mut GenerationInstrumentation>,
) -> Result<Option<GeneratedPuzzle>, GenerateError> {
    let cell_count = cell_count(board);
    if cell_count > MAX_CELL_COUNT {
        return Err(GenerateError::TooManyCells(cell_count));
    }

    if NAMES.len() < cell_count {
        return Err(GenerateError::NotEnoughNames);
    }

    if ROLES.len() < MIN_ROLE_POOL_SIZE {
        return Err(GenerateError::NotEnoughRoles);
    }

    let names = sample_names(rng, cell_count);
    let roles = sample_roles(rng, &names, cell_count)?;
    let mut puzzle = empty_puzzle(board, &names, &roles);
    let layout = Layout::from_puzzle(
        &puzzle,
        distinct_roles(
            &puzzle
                .cells
                .iter()
                .flatten()
                .map(|cell| cell.role.clone())
                .collect::<Vec<_>>(),
        ),
    );

    let first_index = rng.gen_range(0..cell_count);
    let first_answer = random_answer(rng);

    let mut clues = vec![None; cell_count];
    let mut clue_score_terms = vec![None; cell_count];
    let mut generation_score_series = Vec::with_capacity(cell_count);
    let mut generation_clue_texts = Vec::with_capacity(cell_count);
    let mut answers = vec![None; cell_count];
    let mut known_mask = 0u32;
    let mut known_innocent_mask = 0u32;
    let mut pending = vec![first_index];
    let full_mask = full_mask_for_cell_count(cell_count);

    reveal_answer(
        &mut answers,
        &mut known_mask,
        &mut known_innocent_mask,
        first_index,
        first_answer,
    );

    while let Some(current_index) = pending.pop() {
        let current_answer = answers[current_index].unwrap();
        let current_clues = collect_clues(&clues);
        let explicit_baseline =
            solve_clues_with_known_mask(&puzzle, &current_clues, known_mask, known_innocent_mask)?;

        if !explicit_baseline.analysis.has_solution {
            return Ok(None);
        }

        let (closed_known_mask, closed_known_innocent_mask) = closure_known_masks_from_analysis(
            &explicit_baseline.analysis,
            known_mask,
            known_innocent_mask,
        );
        enqueue_forced_closure_indices(
            rng,
            &mut answers,
            &clues,
            &mut known_mask,
            &mut known_innocent_mask,
            &mut pending,
            &explicit_baseline.analysis,
            Some(current_index),
        );
        let revealed_only = solve_clues_with_known_mask(
            &puzzle,
            &[],
            closed_known_mask,
            closed_known_innocent_mask,
        )?;
        let baseline = solve_clues_with_known_mask(
            &puzzle,
            &current_clues,
            closed_known_mask,
            closed_known_innocent_mask,
        )?;

        if !revealed_only.analysis.has_solution || !baseline.analysis.has_solution {
            return Ok(None);
        }

        let require_new_force = has_unforced_unknown(&baseline.analysis, known_mask);
        let mut candidates = Vec::new();

        for _ in 0..MAX_CLUE_ATTEMPTS {
            let Some(witness) = sample_witness_assignment(rng, &baseline, closed_known_mask) else {
                return Ok(None);
            };

            let Some(candidate) = sample_clue(rng, &layout, witness) else {
                continue;
            };

            let mut next_clues = current_clues.clone();
            next_clues.push(candidate.clone());

            let solved = solve_clues_with_known_mask(
                &puzzle,
                &next_clues,
                closed_known_mask,
                closed_known_innocent_mask,
            )?;

            if !solved.analysis.has_solution {
                continue;
            }

            let (candidate, solution, newly_forced, forced_unknown) = normalize_generated_clue(
                rng,
                candidate,
                current_answer,
                &baseline,
                &solved,
                closed_known_mask,
            );

            if require_new_force && newly_forced.is_empty() {
                continue;
            }

            if closed_known_mask != full_mask && forced_unknown.is_empty() {
                continue;
            }

            let alone = solve_clues_with_known_mask(
                &puzzle,
                &[candidate.clone()],
                closed_known_mask,
                closed_known_innocent_mask,
            )?;

            if !alone.analysis.has_solution {
                continue;
            }

            let standalone_forced = forced_unknown_indices(&alone.analysis, closed_known_mask);
            let combination_only_forced = newly_forced
                .iter()
                .copied()
                .filter(|index| forced_answer_at(&alone.analysis, *index) == ForcedAnswer::Neither)
                .collect::<Vec<_>>();

            if let Some(stats) = instrumentation.as_deref_mut() {
                if !newly_forced.is_empty() {
                    stats.candidate_newly_forced_count += 1;
                }
                if standalone_forced.is_empty() {
                    stats.candidate_standalone_zero_count += 1;
                }
                if !combination_only_forced.is_empty() {
                    stats.candidate_combination_only_count += 1;
                }
                if !combination_only_forced.is_empty() && standalone_forced.is_empty() {
                    stats.candidate_combo_eligible_count += 1;
                }
            }

            let mut effective_clues = current_clues.clone();
            effective_clues.push(candidate.clone());
            let target_indices = if !newly_forced.is_empty() {
                &newly_forced
            } else {
                &forced_unknown
            };
            let targets = target_indices
                .iter()
                .filter_map(|index| {
                    let forced = forced_answer_at(&solution.analysis, *index);
                    (forced != ForcedAnswer::Neither).then_some((*index, forced))
                })
                .collect::<Vec<_>>();
            let combination_size = minimal_forcing_subset_size(
                &puzzle,
                &effective_clues,
                closed_known_mask,
                closed_known_innocent_mask,
                &targets,
            )?;
            if let Some(stats) = instrumentation.as_deref_mut() {
                stats.candidate_combination_sizes.push(combination_size);
            }
            let terms = build_clue_score_terms(
                &layout,
                &candidate,
                &revealed_only,
                &alone,
                &baseline,
                &solution,
                &newly_forced,
                combination_size,
                closed_known_mask,
            );

            candidates.push(ScoredCandidate {
                clue: candidate,
                solution,
                terms,
            });

            if candidates.len() >= MAX_SCORED_CANDIDATES {
                break;
            }
        }

        let Some(selected) = choose_scored_candidate(rng, candidates) else {
            return Ok(None);
        };
        let ScoredCandidate {
            clue: candidate,
            solution,
            terms,
        } = selected;
        if let Some(stats) = instrumentation.as_deref_mut() {
            stats
                .selected_combination_sizes
                .push(terms.combination_size);
        }
        let analysis = solution.analysis;

        clues[current_index] = Some(candidate);
        clue_score_terms[current_index] = Some(terms);
        generation_score_series.push(clue_score_terms[current_index].clone().unwrap());
        generation_clue_texts.push(clues[current_index].as_ref().unwrap().text());

        enqueue_forced_closure_indices(
            rng,
            &mut answers,
            &clues,
            &mut known_mask,
            &mut known_innocent_mask,
            &mut pending,
            &analysis,
            None,
        );

        if known_mask == full_mask {
            continue;
        }

        if pending.is_empty() {
            return Ok(None);
        }
    }

    if known_mask != full_mask || clues.iter().any(Option::is_none) {
        return Ok(None);
    }

    for row in 0..board.rows as usize {
        for col in 0..board.cols as usize {
            let index = row * board.cols as usize + col;
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
        first_revealed_name: puzzle.cells[first_index / board.cols as usize]
            [first_index % board.cols as usize]
            .name
            .clone(),
        first_revealed_answer: first_answer,
        clue_score_terms: (0..board.rows as usize)
            .map(|row| {
                (0..board.cols as usize)
                    .map(|col| {
                        clue_score_terms[row * board.cols as usize + col]
                            .clone()
                            .unwrap()
                    })
                    .collect()
            })
            .collect(),
        generation_score_series,
        generation_clue_texts,
        puzzle,
    }))
}

fn try_generate_puzzle_3d<R: Rng + ?Sized>(
    rng: &mut R,
    board: BoardShape,
    mut instrumentation: Option<&mut GenerationInstrumentation>,
) -> Result<Option<GeneratedPuzzle3D>, GenerateError> {
    let cell_count = cell_count(board);
    if cell_count > MAX_CELL_COUNT {
        return Err(GenerateError::TooManyCells(cell_count));
    }

    if NAMES.len() < cell_count {
        return Err(GenerateError::NotEnoughNames);
    }

    if ROLES.len() < MIN_ROLE_POOL_SIZE {
        return Err(GenerateError::NotEnoughRoles);
    }

    let names = sample_names(rng, cell_count);
    let roles = sample_roles(rng, &names, cell_count)?;
    let mut puzzle = empty_puzzle_3d(board, &names, &roles);
    let layout = Layout::from_puzzle_3d(
        &puzzle,
        distinct_roles(
            &puzzle
                .cells
                .iter()
                .flat_map(|layer| layer.iter().flat_map(|row| row.iter()))
                .map(|cell| cell.role.clone())
                .collect::<Vec<_>>(),
        ),
    );

    let first_index = rng.gen_range(0..cell_count);
    let first_answer = random_answer(rng);

    let mut clues = vec![None; cell_count];
    let mut clue_score_terms = vec![None; cell_count];
    let mut generation_score_series = Vec::with_capacity(cell_count);
    let mut generation_clue_texts = Vec::with_capacity(cell_count);
    let mut answers = vec![None; cell_count];
    let mut known_mask = 0u32;
    let mut known_innocent_mask = 0u32;
    let mut pending = vec![first_index];
    let full_mask = full_mask_for_cell_count(cell_count);

    reveal_answer(
        &mut answers,
        &mut known_mask,
        &mut known_innocent_mask,
        first_index,
        first_answer,
    );

    while let Some(current_index) = pending.pop() {
        let current_answer = answers[current_index].unwrap();
        let current_clues = collect_clues(&clues);
        let explicit_baseline = solve_clues_with_known_mask_3d(
            &puzzle,
            &current_clues,
            known_mask,
            known_innocent_mask,
        )?;

        if !explicit_baseline.analysis.has_solution {
            return Ok(None);
        }

        let (closed_known_mask, closed_known_innocent_mask) = closure_known_masks_from_analysis(
            &explicit_baseline.analysis,
            known_mask,
            known_innocent_mask,
        );
        enqueue_forced_closure_indices(
            rng,
            &mut answers,
            &clues,
            &mut known_mask,
            &mut known_innocent_mask,
            &mut pending,
            &explicit_baseline.analysis,
            Some(current_index),
        );
        let revealed_only = solve_clues_with_known_mask_3d(
            &puzzle,
            &[],
            closed_known_mask,
            closed_known_innocent_mask,
        )?;
        let baseline = solve_clues_with_known_mask_3d(
            &puzzle,
            &current_clues,
            closed_known_mask,
            closed_known_innocent_mask,
        )?;

        if !revealed_only.analysis.has_solution || !baseline.analysis.has_solution {
            return Ok(None);
        }

        let require_new_force = has_unforced_unknown(&baseline.analysis, known_mask);
        let mut candidates = Vec::new();

        for _ in 0..MAX_CLUE_ATTEMPTS {
            let Some(witness) = sample_witness_assignment(rng, &baseline, closed_known_mask) else {
                return Ok(None);
            };

            let Some(candidate) = sample_clue(rng, &layout, witness) else {
                continue;
            };

            let mut next_clues = current_clues.clone();
            next_clues.push(candidate.clone());

            let solved = solve_clues_with_known_mask_3d(
                &puzzle,
                &next_clues,
                closed_known_mask,
                closed_known_innocent_mask,
            )?;

            if !solved.analysis.has_solution {
                continue;
            }

            let (candidate, solution, newly_forced, forced_unknown) = normalize_generated_clue(
                rng,
                candidate,
                current_answer,
                &baseline,
                &solved,
                closed_known_mask,
            );

            if require_new_force && newly_forced.is_empty() {
                continue;
            }

            if closed_known_mask != full_mask && forced_unknown.is_empty() {
                continue;
            }

            let alone = solve_clues_with_known_mask_3d(
                &puzzle,
                &[candidate.clone()],
                closed_known_mask,
                closed_known_innocent_mask,
            )?;

            if !alone.analysis.has_solution {
                continue;
            }

            let standalone_forced = forced_unknown_indices(&alone.analysis, closed_known_mask);
            let combination_only_forced = newly_forced
                .iter()
                .copied()
                .filter(|index| forced_answer_at(&alone.analysis, *index) == ForcedAnswer::Neither)
                .collect::<Vec<_>>();

            if let Some(stats) = instrumentation.as_deref_mut() {
                if !newly_forced.is_empty() {
                    stats.candidate_newly_forced_count += 1;
                }
                if standalone_forced.is_empty() {
                    stats.candidate_standalone_zero_count += 1;
                }
                if !combination_only_forced.is_empty() {
                    stats.candidate_combination_only_count += 1;
                }
                if !combination_only_forced.is_empty() && standalone_forced.is_empty() {
                    stats.candidate_combo_eligible_count += 1;
                }
            }

            let mut effective_clues = current_clues.clone();
            effective_clues.push(candidate.clone());
            let target_indices = if !newly_forced.is_empty() {
                &newly_forced
            } else {
                &forced_unknown
            };
            let targets = target_indices
                .iter()
                .filter_map(|index| {
                    let forced = forced_answer_at(&solution.analysis, *index);
                    (forced != ForcedAnswer::Neither).then_some((*index, forced))
                })
                .collect::<Vec<_>>();
            let combination_size = minimal_forcing_subset_size_3d(
                &puzzle,
                &effective_clues,
                closed_known_mask,
                closed_known_innocent_mask,
                &targets,
            )?;
            if let Some(stats) = instrumentation.as_deref_mut() {
                stats.candidate_combination_sizes.push(combination_size);
            }
            let terms = build_clue_score_terms(
                &layout,
                &candidate,
                &revealed_only,
                &alone,
                &baseline,
                &solution,
                &newly_forced,
                combination_size,
                closed_known_mask,
            );

            candidates.push(ScoredCandidate {
                clue: candidate,
                solution,
                terms,
            });

            if candidates.len() >= MAX_SCORED_CANDIDATES {
                break;
            }
        }

        let Some(selected) = choose_scored_candidate(rng, candidates) else {
            return Ok(None);
        };
        let ScoredCandidate {
            clue: candidate,
            solution,
            terms,
        } = selected;
        if let Some(stats) = instrumentation.as_deref_mut() {
            stats
                .selected_combination_sizes
                .push(terms.combination_size);
        }
        let analysis = solution.analysis;

        clues[current_index] = Some(candidate);
        clue_score_terms[current_index] = Some(terms);
        generation_score_series.push(clue_score_terms[current_index].clone().unwrap());
        generation_clue_texts.push(clues[current_index].as_ref().unwrap().text());

        enqueue_forced_closure_indices(
            rng,
            &mut answers,
            &clues,
            &mut known_mask,
            &mut known_innocent_mask,
            &mut pending,
            &analysis,
            None,
        );

        if known_mask == full_mask {
            continue;
        }

        if pending.is_empty() {
            return Ok(None);
        }
    }

    if known_mask != full_mask || clues.iter().any(Option::is_none) {
        return Ok(None);
    }

    for layer in 0..board.depth as usize {
        for row in 0..board.rows as usize {
            for col in 0..board.cols as usize {
                let index =
                    board.index_of(Position::new_3d(layer as i16, row as i16, col as i16));
                puzzle.cells[layer][row][col].clue = clues[index].clone().unwrap();
                puzzle.cells[layer][row][col].answer = answers[index].unwrap();
                puzzle.cells[layer][row][col].state = if index == first_index {
                    Visibility::Revealed
                } else {
                    Visibility::Hidden
                };
            }
        }
    }

    let first_position = board.position_of_index(first_index);

    Ok(Some(GeneratedPuzzle3D {
        first_revealed_name: puzzle.cells[first_position.layer as usize][first_position.row as usize]
            [first_position.col as usize]
            .name
            .clone(),
        first_revealed_answer: first_answer,
        clue_score_terms: (0..board.depth as usize)
            .map(|layer| {
                (0..board.rows as usize)
                    .map(|row| {
                        (0..board.cols as usize)
                            .map(|col| {
                                clue_score_terms[board.index_of(Position::new_3d(
                                    layer as i16,
                                    row as i16,
                                    col as i16,
                                ))]
                                .clone()
                                .unwrap()
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect(),
        generation_score_series,
        generation_clue_texts,
        puzzle,
    }))
}

fn sample_names<R: Rng + ?Sized>(rng: &mut R, cell_count: usize) -> Vec<Name> {
    let mut names = NAMES
        .iter()
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();
    names.shuffle(rng);
    names.truncate(cell_count);
    names.sort_unstable();
    names
}

fn sample_roles<R: Rng + ?Sized>(
    rng: &mut R,
    names: &[Name],
    cell_count: usize,
) -> Result<Vec<Role>, GenerateError> {
    let mut roles = ROLES
        .iter()
        .map(|role| (*role).to_string())
        .collect::<Vec<_>>();
    let pool_size = rng.gen_range(MIN_ROLE_POOL_SIZE..=MAX_ROLE_POOL_SIZE);

    if roles.len() < pool_size {
        return Err(GenerateError::NotEnoughRoles);
    }

    roles.shuffle(rng);
    let mut role_pool = roles.into_iter().take(pool_size).collect::<Vec<_>>();
    let sid_index = names.iter().position(|name| name == SID_NAME);

    if sid_index.is_some() && !role_pool.iter().any(|role| role == BAKER_ROLE) {
        let replacement_index = rng.gen_range(0..role_pool.len());
        role_pool[replacement_index] = BAKER_ROLE.to_string();
    }

    let mut assigned_roles = role_pool.clone();

    while assigned_roles.len() < cell_count {
        assigned_roles.push(role_pool.choose(rng).unwrap().clone());
    }

    assigned_roles.shuffle(rng);

    if let Some(sid_index) = sid_index {
        assigned_roles[sid_index] = BAKER_ROLE.to_string();
    }

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

fn empty_puzzle(board: BoardShape, names: &[Name], roles: &[Role]) -> Puzzle {
    let cells = (0..board.rows as usize)
        .map(|row| {
            (0..board.cols as usize)
                .map(|col| {
                    let index = row * board.cols as usize + col;
                    let role = if names[index] == SID_NAME {
                        BAKER_ROLE.to_string()
                    } else {
                        roles[index].clone()
                    };

                    Cell {
                        name: names[index].clone(),
                        role,
                        emoji: None,
                        clue: default_nonsense_clue(),
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

fn empty_puzzle_3d(board: BoardShape, names: &[Name], roles: &[Role]) -> Puzzle3D {
    let cells = (0..board.depth as usize)
        .map(|layer| {
            (0..board.rows as usize)
                .map(|row| {
                    (0..board.cols as usize)
                        .map(|col| {
                            let index = board.index_of(Position::new_3d(
                                layer as i16,
                                row as i16,
                                col as i16,
                            ));
                            let role = if names[index] == SID_NAME {
                                BAKER_ROLE.to_string()
                            } else {
                                roles[index].clone()
                            };

                            Cell {
                                name: names[index].clone(),
                                role,
                                emoji: None,
                                clue: default_nonsense_clue(),
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

fn closure_known_masks_from_analysis(
    analysis: &FlatClueAnalysis,
    known_mask: u32,
    known_innocent_mask: u32,
) -> (u32, u32) {
    let cell_count = analysis.forced_answers.len();
    let mut closed_known_mask = known_mask;
    let mut closed_known_innocent_mask = known_innocent_mask & known_mask;

    for index in 0..cell_count {
        let bit = 1u32 << index;
        if known_mask & bit != 0 {
            continue;
        }

        match forced_answer_at(analysis, index) {
            ForcedAnswer::Criminal => {
                closed_known_mask |= bit;
                closed_known_innocent_mask &= !bit;
            }
            ForcedAnswer::Innocent => {
                closed_known_mask |= bit;
                closed_known_innocent_mask |= bit;
            }
            ForcedAnswer::Neither => {}
        }
    }

    (closed_known_mask, closed_known_innocent_mask)
}

fn enqueue_forced_closure_indices<R: Rng + ?Sized>(
    rng: &mut R,
    answers: &mut [Option<Answer>],
    clues: &[Option<Clue>],
    known_mask: &mut u32,
    known_innocent_mask: &mut u32,
    pending: &mut Vec<usize>,
    analysis: &FlatClueAnalysis,
    exclude_index: Option<usize>,
) {
    let mut promoted = Vec::new();
    let cell_count = answers.len();

    for index in 0..cell_count {
        if Some(index) == exclude_index || clues[index].is_some() {
            continue;
        }

        let Some(answer) = forced_answer_as_answer(forced_answer_at(analysis, index)) else {
            continue;
        };

        if answers[index].is_none() {
            reveal_answer(answers, known_mask, known_innocent_mask, index, answer);
        }

        if !pending.contains(&index) {
            promoted.push(index);
        }
    }

    promoted.shuffle(rng);
    pending.extend(promoted);
}

fn sample_clue<R: Rng + ?Sized>(rng: &mut R, layout: &Layout, assignment: u32) -> Option<Clue> {
    for _ in 0..32 {
        let candidate = match rng.gen_range(0..10) {
            0..=4 => sample_count_cells_clue(rng, layout, assignment),
            5 => sample_named_count_cells_clue(rng, layout, assignment),
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

fn sample_named_count_cells_clue<R: Rng + ?Sized>(
    rng: &mut R,
    layout: &Layout,
    assignment: u32,
) -> Option<Clue> {
    for _ in 0..32 {
        let selector = sample_selector(rng, layout)?;
        let all_positions = layout.positions_for_selector(&selector);

        if all_positions.is_empty() {
            continue;
        }

        let filter = sample_filter_for_selector(rng, layout, &selector);
        let positions = if filter == CellFilter::Any {
            all_positions.clone()
        } else {
            layout.filtered_positions(&selector, filter)
        };

        if filter_is_redundant(filter, &all_positions, &positions)
            || positions.is_empty()
            || count_scope_is_too_small(&positions)
        {
            continue;
        }

        let answer = random_answer(rng);
        let matching_positions = positions
            .iter()
            .copied()
            .filter(|position| {
                answer_for_index(assignment, layout.index_of_position(*position)) == answer
            })
            .collect::<Vec<_>>();

        if matching_positions.is_empty() {
            continue;
        }

        let member_position = *matching_positions.choose(rng)?;
        let member_name = layout.names[layout.index_of_position(member_position)].clone();

        return Some(Clue::NamedCountCells {
            name: member_name,
            selector,
            answer,
            number: matching_positions.len() as i32,
            filter,
        });
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
        let all_positions = layout.positions_for_selector(&selector);

        if all_positions.is_empty() {
            continue;
        }

        let filter = sample_filter_for_selector(rng, layout, &selector);
        let positions = if filter == CellFilter::Any {
            all_positions.clone()
        } else {
            layout.filtered_positions(&selector, filter)
        };

        if filter_is_redundant(filter, &all_positions, &positions)
            || count_scope_is_too_small(&positions)
        {
            continue;
        }

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

fn filter_is_redundant(
    filter: CellFilter,
    all_positions: &[Position],
    filtered_positions: &[Position],
) -> bool {
    filter != CellFilter::Any && all_positions == filtered_positions
}

fn count_scope_is_too_small(positions: &[Position]) -> bool {
    positions.len() <= 1
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
        Direction::Front,
        Direction::Back,
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
    let compare_rows = rng.gen_bool(0.5);
    let line_family = if layout.board.depth > 1 {
        rng.gen_range(0..3)
    } else if compare_rows {
        1
    } else {
        2
    };
    let lines = match line_family {
        0 => (0..layout.board.depth).map(Line::Layer).collect::<Vec<_>>(),
        1 => (0..layout.board.rows).map(Line::Row).collect::<Vec<_>>(),
        _ => (0..layout.board.cols)
            .map(|index| Line::Col(Column::new(index)))
            .collect::<Vec<_>>(),
    };
    let first_line = *lines.choose(rng)?;
    let second_line = **lines
        .iter()
        .filter(|line| **line != first_line)
        .collect::<Vec<_>>()
        .choose(rng)?;
    let answer = random_answer(rng);
    let first_count = layout.matching_count_in_positions(
        assignment,
        &layout.positions_for_line(first_line),
        answer,
    );
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
            direction: random_direction(rng, layout.board),
        },
        3 if layout.board.depth > 1 => CellSelector::Layer {
            layer: rng.gen_range(0..layout.board.depth),
        },
        3 => CellSelector::Row {
            row: rng.gen_range(0..layout.board.rows),
        },
        4 => CellSelector::Col {
            col: random_column(rng, layout.board.cols),
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

fn sample_filter_for_selector<R: Rng + ?Sized>(
    rng: &mut R,
    layout: &Layout,
    selector: &CellSelector,
) -> CellFilter {
    if matches!(selector, CellSelector::Direction { .. }) {
        return CellFilter::Any;
    }

    match rng.gen_range(0..5) {
        0 => CellFilter::Any,
        1 => CellFilter::Edge,
        2 => CellFilter::Corner,
        3 if layout.board.depth > 1 => {
            CellFilter::Line(Line::Layer(rng.gen_range(0..layout.board.depth)))
        }
        3 => CellFilter::Line(Line::Row(rng.gen_range(0..layout.board.rows))),
        _ => CellFilter::Line(Line::Col(random_column(rng, layout.board.cols))),
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

fn sample_witness_assignment<R: Rng + ?Sized>(
    rng: &mut R,
    solution: &crate::solver::SolutionSet,
    known_mask: u32,
) -> Option<u32> {
    let partial = *solution.assignments.choose(rng)?;
    let cell_count = solution.analysis.forced_answers.len();
    let free_mask = full_mask_for_cell_count(cell_count) & !known_mask & !solution.variable_mask;

    Some(partial | (rng.r#gen::<u32>() & free_mask))
}

fn normalize_generated_clue<R: Rng + ?Sized>(
    rng: &mut R,
    candidate: Clue,
    answer: Answer,
    baseline: &crate::solver::SolutionSet,
    solved: &crate::solver::SolutionSet,
    known_mask: u32,
) -> (Clue, crate::solver::SolutionSet, Vec<usize>, Vec<usize>) {
    if remaining_possibility_count(solved, known_mask)
        < remaining_possibility_count(baseline, known_mask)
    {
        let newly_forced =
            newly_forced_unknown_indices(&baseline.analysis, &solved.analysis, known_mask);
        let forced_unknown = forced_unknown_indices(&solved.analysis, known_mask);

        (candidate, solved.clone(), newly_forced, forced_unknown)
    } else {
        let forced_unknown = forced_unknown_indices(&baseline.analysis, known_mask);

        (
            random_nonsense_clue(rng, answer),
            baseline.clone(),
            Vec::new(),
            forced_unknown,
        )
    }
}

fn default_nonsense_clue() -> Clue {
    Clue::Nonsense {
        text: NONSENSE_TEXTS[0].to_string(),
    }
}

fn random_nonsense_clue<R: Rng + ?Sized>(rng: &mut R, answer: Answer) -> Clue {
    let pool = match answer {
        Answer::Innocent => &NONSENSE_TEXTS[..],
        Answer::Criminal => &CRIMINAL_NONSENSE_TEXTS[..],
    };

    Clue::Nonsense {
        text: pool.choose(rng).unwrap().to_string(),
    }
}

fn remaining_possibility_count(solution: &crate::solver::SolutionSet, known_mask: u32) -> u64 {
    let free_unknown_bits = solution.analysis.forced_answers.len() as u32
        - known_mask.count_ones()
        - solution.variable_mask.count_ones();

    (solution.assignments.len() as u64) * (1u64 << free_unknown_bits)
}

fn log2_information_gain(before: u64, after: u64) -> f64 {
    if before == 0 || after == 0 || after >= before {
        0.0
    } else {
        (before as f64 / after as f64).log2()
    }
}

fn minimal_forcing_subset_size(
    puzzle: &Puzzle,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
    targets: &[(usize, ForcedAnswer)],
) -> Result<usize, SolveError> {
    if targets.is_empty() {
        return Ok(0);
    }

    let greedy_size =
        greedy_forcing_subset_size(puzzle, clues, known_mask, known_innocent_mask, targets)?;
    if greedy_size <= 1 {
        return Ok(greedy_size);
    }

    for subset_size in 1..greedy_size {
        if exists_forcing_subset_of_size(
            puzzle,
            clues,
            subset_size,
            known_mask,
            known_innocent_mask,
            targets,
        )? {
            return Ok(subset_size);
        }
    }

    Ok(greedy_size)
}

fn minimal_forcing_subset_size_3d(
    puzzle: &Puzzle3D,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
    targets: &[(usize, ForcedAnswer)],
) -> Result<usize, SolveError> {
    if targets.is_empty() {
        return Ok(0);
    }

    let greedy_size = greedy_forcing_subset_size_3d(
        puzzle,
        clues,
        known_mask,
        known_innocent_mask,
        targets,
    )?;
    if greedy_size <= 1 {
        return Ok(greedy_size);
    }

    for subset_size in 1..greedy_size {
        if exists_forcing_subset_of_size_3d(
            puzzle,
            clues,
            subset_size,
            known_mask,
            known_innocent_mask,
            targets,
        )? {
            return Ok(subset_size);
        }
    }

    Ok(greedy_size)
}

fn greedy_forcing_subset_size(
    puzzle: &Puzzle,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
    targets: &[(usize, ForcedAnswer)],
) -> Result<usize, SolveError> {
    let mut active = clues.to_vec();

    loop {
        let mut removable_index = None;

        for index in 0..active.len() {
            let mut trial = active.clone();
            trial.remove(index);
            if subset_forces_any_target(puzzle, &trial, known_mask, known_innocent_mask, targets)? {
                removable_index = Some(index);
                break;
            }
        }

        let Some(index) = removable_index else {
            break;
        };

        active.remove(index);
    }

    Ok(active.len())
}

fn greedy_forcing_subset_size_3d(
    puzzle: &Puzzle3D,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
    targets: &[(usize, ForcedAnswer)],
) -> Result<usize, SolveError> {
    let mut active = clues.to_vec();

    loop {
        let mut removable_index = None;

        for index in 0..active.len() {
            let mut trial = active.clone();
            trial.remove(index);
            if subset_forces_any_target_3d(
                puzzle,
                &trial,
                known_mask,
                known_innocent_mask,
                targets,
            )? {
                removable_index = Some(index);
                break;
            }
        }

        let Some(index) = removable_index else {
            break;
        };

        active.remove(index);
    }

    Ok(active.len())
}

fn exists_forcing_subset_of_size(
    puzzle: &Puzzle,
    clues: &[Clue],
    subset_size: usize,
    known_mask: u32,
    known_innocent_mask: u32,
    targets: &[(usize, ForcedAnswer)],
) -> Result<bool, SolveError> {
    if subset_size == 0 || subset_size > clues.len() {
        return Ok(false);
    }

    let mut indices = (0..subset_size).collect::<Vec<_>>();
    let mut trial = Vec::with_capacity(subset_size);

    loop {
        trial.clear();
        trial.extend(indices.iter().map(|&index| clues[index].clone()));

        if subset_forces_any_target(puzzle, &trial, known_mask, known_innocent_mask, targets)? {
            return Ok(true);
        }

        if !advance_combination_indices(&mut indices, clues.len()) {
            return Ok(false);
        }
    }
}

fn exists_forcing_subset_of_size_3d(
    puzzle: &Puzzle3D,
    clues: &[Clue],
    subset_size: usize,
    known_mask: u32,
    known_innocent_mask: u32,
    targets: &[(usize, ForcedAnswer)],
) -> Result<bool, SolveError> {
    if subset_size == 0 || subset_size > clues.len() {
        return Ok(false);
    }

    let mut indices = (0..subset_size).collect::<Vec<_>>();
    let mut trial = Vec::with_capacity(subset_size);

    loop {
        trial.clear();
        trial.extend(indices.iter().map(|&index| clues[index].clone()));

        if subset_forces_any_target_3d(puzzle, &trial, known_mask, known_innocent_mask, targets)? {
            return Ok(true);
        }

        if !advance_combination_indices(&mut indices, clues.len()) {
            return Ok(false);
        }
    }
}

fn advance_combination_indices(indices: &mut [usize], source_len: usize) -> bool {
    let width = indices.len();

    for pivot in (0..width).rev() {
        let max_value = source_len - (width - pivot);
        if indices[pivot] == max_value {
            continue;
        }

        indices[pivot] += 1;
        for index in (pivot + 1)..width {
            indices[index] = indices[index - 1] + 1;
        }
        return true;
    }

    false
}

fn subset_forces_any_target(
    puzzle: &Puzzle,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
    targets: &[(usize, ForcedAnswer)],
) -> Result<bool, SolveError> {
    let solved = solve_clues_with_known_mask(puzzle, clues, known_mask, known_innocent_mask)?;
    Ok(solved.analysis.has_solution && forces_any_target(&solved.analysis, targets))
}

fn subset_forces_any_target_3d(
    puzzle: &Puzzle3D,
    clues: &[Clue],
    known_mask: u32,
    known_innocent_mask: u32,
    targets: &[(usize, ForcedAnswer)],
) -> Result<bool, SolveError> {
    let solved = solve_clues_with_known_mask_3d(puzzle, clues, known_mask, known_innocent_mask)?;
    Ok(solved.analysis.has_solution && forces_any_target(&solved.analysis, targets))
}

fn forces_any_target(
    analysis: &FlatClueAnalysis,
    targets: &[(usize, ForcedAnswer)],
) -> bool {
    targets
        .iter()
        .any(|(index, answer)| forced_answer_at(analysis, *index) == *answer)
}

fn build_clue_score_terms(
    layout: &Layout,
    clue: &Clue,
    revealed_only: &SolutionSet,
    alone: &SolutionSet,
    baseline: &SolutionSet,
    combined: &SolutionSet,
    newly_forced: &[usize],
    combination_size: usize,
    known_mask: u32,
) -> ClueScoreTerms {
    let revealed_count = remaining_possibility_count(revealed_only, known_mask);
    let baseline_count = remaining_possibility_count(baseline, known_mask);
    let alone_count = remaining_possibility_count(alone, known_mask);
    let combined_count = remaining_possibility_count(combined, known_mask);
    let combined_gain = log2_information_gain(baseline_count, combined_count);
    let alone_gain = log2_information_gain(revealed_count, alone_count);
    let synergy_gain = (combined_gain - alone_gain).max(0.0);
    let standalone_forced = forced_unknown_indices(&alone.analysis, known_mask).len();
    let combined_new_forced = newly_forced.len();
    let baseline_active_unforced = active_unforced_tile_count(baseline, known_mask);
    let active_unforced_tiles = active_unforced_tile_count(combined, known_mask);
    let newly_active_unforced_tiles =
        active_unforced_tiles as i32 - baseline_active_unforced as i32;
    let baseline_active_uncertainty = active_uncertainty(baseline, known_mask);
    let active_uncertainty = active_uncertainty(combined, known_mask);
    let active_uncertainty_jump = (active_uncertainty - baseline_active_uncertainty).max(0.0);
    let triviality_penalty = triviality_penalty(layout, clue);
    let family_weight = family_weight(clue);

    let score = combination_size_bonus(combination_size)
        + combined_force_score(combined_new_forced)
        + active_state_bonus(
            active_unforced_tiles,
            newly_active_unforced_tiles,
            active_uncertainty,
        )
        - active_uncertainty_jump_penalty(active_uncertainty_jump)
        + synergy_bonus(synergy_gain, combination_size, standalone_forced)
        + moderate_information_bonus(combined_gain)
        - standalone_force_penalty(standalone_forced)
        - standalone_information_penalty(alone_gain)
        - excess_information_penalty(combined_gain)
        - 1.4 * triviality_penalty
        + family_weight;

    ClueScoreTerms {
        combination_size,
        combined_new_forced,
        standalone_forced,
        active_unforced_tiles,
        newly_active_unforced_tiles,
        active_uncertainty,
        active_uncertainty_jump,
        combined_gain,
        alone_gain,
        synergy_gain,
        triviality_penalty,
        family_weight,
        score,
    }
}

fn combination_size_bonus(size: usize) -> f64 {
    match size {
        0 | 1 => 0.0,
        2 => 6.5,
        3 => 10.0,
        4 => 12.0,
        _ => 12.0 + (size.saturating_sub(4) as f64) * 0.75,
    }
}

fn combined_force_score(count: usize) -> f64 {
    match count {
        0 => -2.0,
        1 => 2.0,
        2 => 0.5,
        3 => -1.5,
        _ => -1.5 - (count as f64 - 3.0) * 1.5,
    }
}

fn active_state_bonus(
    active_unforced_tiles: usize,
    newly_active_unforced_tiles: i32,
    active_uncertainty: f64,
) -> f64 {
    0.08 * active_unforced_tiles as f64
        + gradual_active_tile_bonus(newly_active_unforced_tiles)
        + 0.12 * active_uncertainty.min(6.0)
}

fn gradual_active_tile_bonus(newly_active_unforced_tiles: i32) -> f64 {
    match newly_active_unforced_tiles {
        ..=-1 => -0.4,
        0 => 0.0,
        1 => 0.5,
        2 => 1.4,
        3 => 1.8,
        4 => 0.8,
        5 => -0.2,
        _ => -0.2 - 0.45 * (newly_active_unforced_tiles - 5) as f64,
    }
}

fn active_uncertainty_jump_penalty(active_uncertainty_jump: f64) -> f64 {
    let excess = (active_uncertainty_jump - 1.5).max(0.0);
    excess * excess * 0.9
}

fn synergy_bonus(synergy_gain: f64, combination_size: usize, standalone_forced: usize) -> f64 {
    let capped = synergy_gain.min(2.0);

    if combination_size >= 2 && standalone_forced == 0 {
        1.8 + 1.2 * capped
    } else {
        0.35 * capped
    }
}

fn analysis_cell_count(analysis: &FlatClueAnalysis) -> usize {
    analysis.forced_answers.len()
}

fn active_unforced_tile_count(solution: &SolutionSet, known_mask: u32) -> usize {
    (0..analysis_cell_count(&solution.analysis))
        .filter(|index| solution.implicated_mask & (1u32 << index) != 0)
        .filter(|index| known_mask & (1u32 << index) == 0)
        .filter(|index| forced_answer_at(&solution.analysis, *index) == ForcedAnswer::Neither)
        .count()
}

fn active_uncertainty(solution: &SolutionSet, known_mask: u32) -> f64 {
    if solution.assignments.is_empty() {
        return 0.0;
    }

    let assignment_count = solution.assignments.len() as f64;

    (0..analysis_cell_count(&solution.analysis))
        .filter(|index| solution.implicated_mask & (1u32 << index) != 0)
        .filter(|index| solution.variable_mask & (1u32 << index) != 0)
        .filter(|index| known_mask & (1u32 << index) == 0)
        .map(|index| {
            let innocent_count = solution
                .assignments
                .iter()
                .filter(|assignment| *assignment & (1u32 << index) != 0)
                .count() as f64;
            let probability = innocent_count / assignment_count;

            1.0 - (2.0 * probability - 1.0).abs()
        })
        .sum()
}

fn moderate_information_bonus(combined_gain: f64) -> f64 {
    combined_gain.min(1.25) * 0.45
}

fn standalone_force_penalty(count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        let count = count as f64;
        3.5 + 4.5 * count * count
    }
}

fn standalone_information_penalty(alone_gain: f64) -> f64 {
    alone_gain * 1.1
}

fn excess_information_penalty(combined_gain: f64) -> f64 {
    let excess = (combined_gain - 2.0).max(0.0);
    excess * excess * 1.6
}

fn family_weight(clue: &Clue) -> f64 {
    match clue {
        Clue::Nonsense { .. } => 0.0,
        Clue::Declaration { .. } => -1.6,
        Clue::CountCells {
            selector,
            count,
            filter,
            ..
        } => {
            let filter_bonus = if *filter == CellFilter::Any { 0.0 } else { 0.2 };
            match count {
                Count::Parity(_) => 1.1 + filter_bonus,
                Count::AtLeast(_) => 0.25 + filter_bonus,
                Count::Number(_) => {
                    let selector_weight = match selector {
                        CellSelector::Board => -1.3,
                        CellSelector::Layer { .. }
                        | CellSelector::Row { .. }
                        | CellSelector::Col { .. } => -1.0,
                        CellSelector::Direction { .. } => -0.5,
                        CellSelector::Neighbor { .. }
                        | CellSelector::Between { .. }
                        | CellSelector::SharedNeighbor { .. } => 0.2,
                    };

                    selector_weight + filter_bonus
                }
            }
        }
        Clue::NamedCountCells {
            selector, filter, ..
        } => {
            let filter_bonus = if *filter == CellFilter::Any { 0.0 } else { 0.2 };
            let selector_weight = match selector {
                CellSelector::Board => -1.0,
                CellSelector::Layer { .. }
                | CellSelector::Row { .. }
                | CellSelector::Col { .. } => -0.7,
                CellSelector::Direction { .. } => -0.2,
                CellSelector::Neighbor { .. }
                | CellSelector::Between { .. }
                | CellSelector::SharedNeighbor { .. } => 0.35,
            };

            selector_weight + filter_bonus
        }
        Clue::DirectRelation { .. } => -1.0,
        Clue::RoleCount { count, .. } => match count {
            Count::Parity(_) => 0.9,
            Count::AtLeast(_) => 0.1,
            Count::Number(_) => -0.5,
        },
        Clue::RolesComparison { .. } => 1.1,
        Clue::LineComparison { .. } => 0.9,
        Clue::Connected { .. } => 0.8,
        Clue::Quantified { .. } => 1.2,
    }
}

fn triviality_penalty(layout: &Layout, clue: &Clue) -> f64 {
    let scope_size = match clue {
        Clue::CountCells {
            selector,
            count: Count::Number(_),
            filter,
            ..
        } => Some(layout.filtered_positions(selector, *filter).len() as i32),
        Clue::NamedCountCells {
            selector, filter, ..
        } => Some(layout.filtered_positions(selector, *filter).len() as i32),
        Clue::DirectRelation { .. } => Some(1),
        Clue::RoleCount {
            role,
            count: Count::Number(_),
            ..
        } => Some(
            layout
                .indices_by_role
                .get(role)
                .map(|indices| indices.len())
                .unwrap_or(0) as i32,
        ),
        _ => None,
    };

    match (clue, scope_size) {
        (
            Clue::CountCells {
                count: Count::Number(number),
                ..
            }
            | Clue::NamedCountCells { number, .. }
            | Clue::RoleCount {
                count: Count::Number(number),
                ..
            },
            Some(scope_size),
        ) if scope_size > 0 => exact_count_triviality(*number, scope_size),
        (Clue::DirectRelation { .. }, Some(_)) => 1.5,
        _ => 0.0,
    }
}

fn exact_count_triviality(number: i32, scope_size: i32) -> f64 {
    if number == 0 || number == scope_size {
        2.0
    } else if number == 1 || number == scope_size - 1 {
        1.0
    } else {
        0.0
    }
}

fn choose_scored_candidate<R: Rng + ?Sized>(
    rng: &mut R,
    candidates: Vec<ScoredCandidate>,
) -> Option<ScoredCandidate> {
    if candidates.is_empty() {
        return None;
    }

    let max_score = candidates
        .iter()
        .map(|candidate| candidate.terms.score)
        .fold(f64::NEG_INFINITY, f64::max);
    let weights = candidates
        .iter()
        .map(|candidate| ((candidate.terms.score - max_score) / CLUE_SCORE_TEMPERATURE).exp())
        .collect::<Vec<_>>();
    let total_weight: f64 = weights.iter().sum();

    if !total_weight.is_finite() || total_weight <= 0.0 {
        return candidates.into_iter().max_by(|left, right| {
            left.terms
                .score
                .partial_cmp(&right.terms.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    let mut draw = rng.r#gen::<f64>() * total_weight;
    for (index, weight) in weights.iter().enumerate() {
        draw -= *weight;
        if draw <= 0.0 {
            return Some(candidates.into_iter().nth(index).unwrap());
        }
    }

    candidates.into_iter().last()
}

fn newly_forced_unknown_indices(
    baseline: &FlatClueAnalysis,
    next: &FlatClueAnalysis,
    known_mask: u32,
) -> Vec<usize> {
    (0..analysis_cell_count(next))
        .filter(|index| known_mask & (1u32 << index) == 0)
        .filter(|index| forced_answer_at(baseline, *index) == ForcedAnswer::Neither)
        .filter(|index| forced_answer_at(next, *index) != ForcedAnswer::Neither)
        .collect()
}

fn forced_unknown_indices(analysis: &FlatClueAnalysis, known_mask: u32) -> Vec<usize> {
    (0..analysis_cell_count(analysis))
        .filter(|index| known_mask & (1u32 << index) == 0)
        .filter(|index| forced_answer_at(analysis, *index) != ForcedAnswer::Neither)
        .collect()
}

fn has_unforced_unknown(analysis: &FlatClueAnalysis, known_mask: u32) -> bool {
    (0..analysis_cell_count(analysis))
        .filter(|index| known_mask & (1u32 << index) == 0)
        .any(|index| forced_answer_at(analysis, index) == ForcedAnswer::Neither)
}

fn forced_answer_at(analysis: &FlatClueAnalysis, index: usize) -> ForcedAnswer {
    analysis.forced_answers[index]
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

fn random_direction<R: Rng + ?Sized>(rng: &mut R, board: BoardShape) -> Direction {
    match rng.gen_range(0..if board.depth > 1 { 6 } else { 4 }) {
        0 => Direction::Above,
        1 => Direction::Below,
        2 => Direction::Left,
        3 => Direction::Right,
        4 => Direction::Front,
        _ => Direction::Back,
    }
}

fn random_column<R: Rng + ?Sized>(rng: &mut R, cols: u8) -> Column {
    Column::new(rng.gen_range(0..cols))
}

fn random_distinct_names<R: Rng + ?Sized>(rng: &mut R, names: &[Name]) -> Option<(Name, Name)> {
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
    use std::collections::BTreeMap;

    use rand::{SeedableRng, rngs::StdRng};

    use crate::{
        clue::{
            CRIMINAL_NONSENSE_TEXTS, CellFilter, CellSelector, Clue, Column, Count, Line,
            NONSENSE_TEXTS, Parity,
        },
        geometry::{BoardShape, Position},
        solver::{FlatClueAnalysis, SolutionSet, solve_clues_with_known_mask},
        types::NAMES,
    };

    use super::{
        BAKER_ROLE, CELL_COUNT, COLS, ForcedAnswer, GenerateError, GenerationInstrumentation,
        Layout, ROWS, SID_NAME, active_state_bonus, active_uncertainty,
        active_uncertainty_jump_penalty, active_unforced_tile_count,
        closure_known_masks_from_analysis, combination_size_bonus, count_scope_is_too_small,
        distinct_roles, empty_puzzle, exact_count_triviality, family_weight, filter_is_redundant,
        generate_puzzle_with_rng, generate_puzzle_with_rng_and_instrumentation,
        generate_puzzle_with_seed, gradual_active_tile_bonus, greedy_forcing_subset_size,
        minimal_forcing_subset_size, normalize_generated_clue, sample_filter_for_selector,
        sample_line_comparison_clue, sample_roles, sample_witness_assignment,
        suggest_clue_for_known_tile_with_rng,
    };

    fn blank_analysis() -> FlatClueAnalysis {
        FlatClueAnalysis {
            has_solution: true,
            board: BoardShape::new(ROWS as u8, COLS as u8),
            forced_answers: vec![ForcedAnswer::Neither; CELL_COUNT],
        }
    }

    fn forced_at(analysis: &FlatClueAnalysis, row: usize, col: usize) -> ForcedAnswer {
        analysis.forced_answers[row * COLS + col]
    }

    #[test]
    fn generated_puzzle_is_fully_forced_from_the_first_reveal() {
        let mut rng = StdRng::seed_from_u64(7);
        let generated =
            generate_puzzle_with_rng(&mut rng, BoardShape::new(ROWS as u8, COLS as u8)).unwrap();
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
        let known_innocent_mask =
            if generated.first_revealed_answer == crate::types::Answer::Innocent {
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
                let forced = forced_at(&solved.analysis, row, col);
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
        let generated =
            generate_puzzle_with_rng(&mut rng, BoardShape::new(ROWS as u8, COLS as u8)).unwrap();
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
    fn sid_is_always_assigned_baker_when_present() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut names = (0..CELL_COUNT)
            .map(|index| format!("Person {index}"))
            .collect::<Vec<_>>();
        names[6] = SID_NAME.to_string();

        let roles = sample_roles(&mut rng, &names, CELL_COUNT).unwrap();

        assert_eq!(roles[6], BAKER_ROLE);
        assert!((10..=15).contains(&distinct_roles(&roles).len()));
    }

    #[test]
    fn generated_puzzle_assigns_baker_to_sid() {
        let generated = generate_puzzle_with_seed(3).unwrap();
        let sid = generated
            .puzzle
            .cells
            .iter()
            .flatten()
            .find(|cell| cell.name == SID_NAME)
            .unwrap();

        assert_eq!(sid.role, BAKER_ROLE);
    }

    #[test]
    fn generated_puzzle_supports_2x2_board_size() {
        let generated =
            super::generate_puzzle_with_seed_and_size(7, BoardShape::new(2, 2)).unwrap();

        assert_eq!(generated.puzzle.cells.len(), 2);
        assert_eq!(generated.puzzle.cells[0].len(), 2);
        assert_eq!(generated.clue_score_terms.len(), 2);
        assert_eq!(generated.clue_score_terms[0].len(), 2);
    }

    #[test]
    fn generated_puzzle_supports_2x2x2_board_size() {
        let generated =
            super::generate_puzzle_3d_with_seed_and_size(7, BoardShape::new_3d(2, 2, 2)).unwrap();

        assert_eq!(generated.puzzle.cells.len(), 2);
        assert_eq!(generated.puzzle.cells[0].len(), 2);
        assert_eq!(generated.puzzle.cells[0][0].len(), 2);

        let revealed_count = generated
            .puzzle
            .cells
            .iter()
            .flatten()
            .flatten()
            .filter(|cell| cell.state == crate::puzzle::Visibility::Revealed)
            .count();

        assert_eq!(revealed_count, 1);
    }

    #[test]
    fn suggests_a_generator_style_clue_for_the_first_revealed_tile() {
        let generated = generate_puzzle_with_seed(0x1234).unwrap();
        let rows = generated.puzzle.cells.len();
        let cols = generated.puzzle.cells[0].len();
        let revealed_index = (0..rows * cols)
            .find(|index| {
                generated.puzzle.cells[index / cols][index % cols].state
                    == crate::puzzle::Visibility::Revealed
            })
            .unwrap();
        let revealed_cell = &generated.puzzle.cells[revealed_index / cols][revealed_index % cols];
        let mut rng = StdRng::seed_from_u64(99);
        let known_mask = 1u32 << revealed_index;
        let known_innocent_mask = if revealed_cell.answer == crate::types::Answer::Innocent {
            known_mask
        } else {
            0
        };

        let suggested = suggest_clue_for_known_tile_with_rng(
            &mut rng,
            &generated.puzzle,
            &[],
            known_mask,
            known_innocent_mask,
            revealed_index,
            revealed_cell.answer,
        )
        .unwrap();

        assert!(suggested.is_some());
    }

    #[test]
    fn generator_returns_a_meaningful_error_when_it_cannot_make_progress() {
        let mut rng = StdRng::seed_from_u64(1);
        let result = generate_puzzle_with_rng(&mut rng, BoardShape::new(ROWS as u8, COLS as u8));

        assert!(!matches!(result, Err(GenerateError::NotEnoughNames)));
    }

    #[test]
    fn same_seed_produces_the_same_puzzle() {
        let first = generate_puzzle_with_seed(7).unwrap();
        let second = generate_puzzle_with_seed(7).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn non_narrowing_generated_clues_become_nonsense() {
        let mut rng = StdRng::seed_from_u64(7);
        let candidate = Clue::CountCells {
            selector: CellSelector::Board,
            answer: crate::types::Answer::Innocent,
            count: Count::AtLeast(0),
            filter: CellFilter::Any,
        };
        let baseline = SolutionSet {
            analysis: blank_analysis(),
            assignments: vec![1, 2, 3],
            variable_mask: 0,
            implicated_mask: 0,
        };
        let solved = SolutionSet {
            analysis: blank_analysis(),
            assignments: vec![1, 2, 3],
            variable_mask: 0,
            implicated_mask: 0,
        };

        let (effective, _, newly_forced, forced_unknown) = normalize_generated_clue(
            &mut rng,
            candidate,
            crate::types::Answer::Innocent,
            &baseline,
            &solved,
            0,
        );

        match effective {
            Clue::Nonsense { text } => assert!(NONSENSE_TEXTS.contains(&text.as_str())),
            other => panic!("expected nonsense clue, got {other:?}"),
        }
        assert!(newly_forced.is_empty());
        assert!(forced_unknown.is_empty());
    }

    #[test]
    fn narrowing_generated_clues_are_kept() {
        let mut rng = StdRng::seed_from_u64(7);
        let candidate = Clue::CountCells {
            selector: CellSelector::Board,
            answer: crate::types::Answer::Innocent,
            count: Count::Number(20),
            filter: CellFilter::Any,
        };
        let solved_analysis = FlatClueAnalysis {
            has_solution: true,
            board: BoardShape::new(ROWS as u8, COLS as u8),
            forced_answers: vec![ForcedAnswer::Innocent; CELL_COUNT],
        };
        let baseline = SolutionSet {
            analysis: blank_analysis(),
            assignments: vec![0],
            variable_mask: 0,
            implicated_mask: 0,
        };
        let solved = SolutionSet {
            analysis: solved_analysis.clone(),
            assignments: vec![1],
            variable_mask: 1,
            implicated_mask: 1,
        };

        let (effective, solved, newly_forced, forced_unknown) = normalize_generated_clue(
            &mut rng,
            candidate.clone(),
            crate::types::Answer::Innocent,
            &baseline,
            &solved,
            0,
        );

        assert_eq!(effective, candidate);
        assert_eq!(solved.analysis, solved_analysis);
        assert_eq!(newly_forced.len(), CELL_COUNT);
        assert_eq!(forced_unknown.len(), CELL_COUNT);
    }

    #[test]
    fn criminal_nonsense_clues_use_the_criminal_only_pool() {
        let mut rng = StdRng::seed_from_u64(7);
        let candidate = Clue::CountCells {
            selector: CellSelector::Board,
            answer: crate::types::Answer::Innocent,
            count: Count::AtLeast(0),
            filter: CellFilter::Any,
        };
        let baseline = SolutionSet {
            analysis: blank_analysis(),
            assignments: vec![1, 2, 3],
            variable_mask: 0,
            implicated_mask: 0,
        };
        let solved = SolutionSet {
            analysis: blank_analysis(),
            assignments: vec![1, 2, 3],
            variable_mask: 0,
            implicated_mask: 0,
        };

        let (effective, _, _, _) = normalize_generated_clue(
            &mut rng,
            candidate,
            crate::types::Answer::Criminal,
            &baseline,
            &solved,
            0,
        );

        match effective {
            Clue::Nonsense { text } => {
                assert!(CRIMINAL_NONSENSE_TEXTS.contains(&text.as_str()));
                assert!(!NONSENSE_TEXTS.contains(&text.as_str()));
            }
            other => panic!("expected nonsense clue, got {other:?}"),
        }
    }

    #[test]
    fn witness_sampling_preserves_known_bits_and_can_fill_free_bits() {
        let mut rng = StdRng::seed_from_u64(7);
        let solution = SolutionSet {
            analysis: blank_analysis(),
            assignments: vec![0b0101],
            variable_mask: 0b0001,
            implicated_mask: 0b0101,
        };

        let witness = sample_witness_assignment(&mut rng, &solution, 0b0100).unwrap();

        assert_eq!(witness & 0b0101, 0b0101);
        assert_ne!(witness, 0b0101);
    }

    #[test]
    fn unchanged_filters_are_treated_as_redundant() {
        let left_column = vec![
            Position::new(0, 0),
            Position::new(1, 0),
            Position::new(2, 0),
            Position::new(3, 0),
            Position::new(4, 0),
        ];
        let left_column_corners = vec![Position::new(0, 0), Position::new(4, 0)];

        assert!(filter_is_redundant(
            CellFilter::Edge,
            &left_column,
            &left_column,
        ));
        assert!(!filter_is_redundant(
            CellFilter::Corner,
            &left_column,
            &left_column_corners,
        ));
        assert!(!filter_is_redundant(
            CellFilter::Any,
            &left_column,
            &left_column,
        ));
    }

    #[test]
    fn direction_selectors_only_sample_any_filter() {
        let mut rng = StdRng::seed_from_u64(7);
        let names = NAMES[..CELL_COUNT]
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        let roles = (0..CELL_COUNT)
            .map(|index| format!("Role {}", index % 4))
            .collect::<Vec<_>>();
        let puzzle = empty_puzzle(BoardShape::new(ROWS as u8, COLS as u8), &names, &roles);
        let layout = Layout::from_puzzle(&puzzle, distinct_roles(&roles));
        let selector = CellSelector::Direction {
            name: "Anna".to_string(),
            direction: crate::clue::Direction::Left,
        };

        for _ in 0..32 {
            assert_eq!(
                sample_filter_for_selector(&mut rng, &layout, &selector),
                CellFilter::Any
            );
        }
    }

    #[test]
    fn singleton_count_scopes_are_too_small() {
        assert!(count_scope_is_too_small(&[Position::new(3, 3)]));
        assert!(!count_scope_is_too_small(&[
            Position::new(3, 2),
            Position::new(3, 3)
        ]));
    }

    #[test]
    fn exact_count_triviality_penalizes_extreme_counts() {
        assert_eq!(exact_count_triviality(0, 5), 2.0);
        assert_eq!(exact_count_triviality(5, 5), 2.0);
        assert_eq!(exact_count_triviality(1, 5), 1.0);
        assert_eq!(exact_count_triviality(4, 5), 1.0);
        assert_eq!(exact_count_triviality(2, 5), 0.0);
    }

    #[test]
    fn family_weights_prefer_comparisons_over_exact_row_counts() {
        let exact_row = Clue::CountCells {
            selector: CellSelector::Row { row: 0 },
            answer: crate::types::Answer::Innocent,
            count: Count::Number(2),
            filter: CellFilter::Any,
        };
        let comparison = Clue::LineComparison {
            first_line: crate::clue::Line::Row(0),
            second_line: crate::clue::Line::Row(1),
            answer: crate::types::Answer::Innocent,
            comparison: crate::clue::Comparison::More,
        };

        assert!(family_weight(&comparison) > family_weight(&exact_row));
    }

    #[test]
    fn generated_line_comparisons_only_compare_like_with_like() {
        let mut rng = StdRng::seed_from_u64(7);
        let names = NAMES[..CELL_COUNT]
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        let roles = (0..CELL_COUNT)
            .map(|index| format!("Role {}", index % 4))
            .collect::<Vec<_>>();
        let puzzle = empty_puzzle(BoardShape::new(ROWS as u8, COLS as u8), &names, &roles);
        let layout = Layout::from_puzzle(&puzzle, distinct_roles(&roles));

        for _ in 0..128 {
            let clue = sample_line_comparison_clue(&mut rng, &layout, 0).unwrap();

            match clue {
                Clue::LineComparison {
                    first_line: crate::clue::Line::Row(_),
                    second_line: crate::clue::Line::Row(_),
                    ..
                }
                | Clue::LineComparison {
                    first_line: crate::clue::Line::Col(_),
                    second_line: crate::clue::Line::Col(_),
                    ..
                } => {}
                other => panic!("expected like-with-like line comparison, got {other:?}"),
            }
        }
    }

    #[test]
    fn combination_size_bonus_rewards_two_and_three_clue_dependencies() {
        assert_eq!(combination_size_bonus(1), 0.0);
        assert!(combination_size_bonus(2) > combination_size_bonus(1));
        assert!(combination_size_bonus(3) > combination_size_bonus(2));
    }

    #[test]
    fn active_state_metrics_reward_unforced_implicated_tiles() {
        let mut analysis = blank_analysis();
        analysis.forced_answers[1] = ForcedAnswer::Innocent;
        let solution = SolutionSet {
            analysis,
            assignments: vec![0b0000, 0b0001],
            variable_mask: 0b0011,
            implicated_mask: 0b0011,
        };

        assert_eq!(active_unforced_tile_count(&solution, 0), 1);
        assert!((active_uncertainty(&solution, 0) - 1.0).abs() < 1e-9);
        assert!(active_state_bonus(1, 1, 1.0) > active_state_bonus(0, 0, 0.0));
    }

    #[test]
    fn active_state_bonus_prefers_gradual_tile_activation() {
        assert!(gradual_active_tile_bonus(3) > gradual_active_tile_bonus(1));
        assert!(gradual_active_tile_bonus(3) > gradual_active_tile_bonus(6));
        assert!(active_state_bonus(4, 3, 2.0) > active_state_bonus(8, 6, 2.0));
    }

    #[test]
    fn large_uncertainty_jumps_are_penalized() {
        assert_eq!(active_uncertainty_jump_penalty(1.5), 0.0);
        assert!(active_uncertainty_jump_penalty(3.0) > 0.0);
        assert!(active_uncertainty_jump_penalty(5.0) > active_uncertainty_jump_penalty(3.0));
    }

    #[test]
    fn minimal_forcing_subset_size_measures_two_clue_dependency() {
        let names = NAMES[..CELL_COUNT]
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        let roles = (0..CELL_COUNT)
            .map(|_| "Role".to_string())
            .collect::<Vec<_>>();
        let puzzle = empty_puzzle(BoardShape::new(ROWS as u8, COLS as u8), &names, &roles);
        let anchor_name = puzzle.cells[0][1].name.clone();
        let clues = vec![
            Clue::DirectRelation {
                name: anchor_name,
                answer: crate::types::Answer::Innocent,
                direction: crate::clue::Direction::Left,
            },
            Clue::CountCells {
                selector: CellSelector::Row { row: 0 },
                answer: crate::types::Answer::Innocent,
                count: Count::Number(1),
                filter: CellFilter::Any,
            },
        ];
        let solved = solve_clues_with_known_mask(&puzzle, &clues, 0, 0).unwrap();

        assert_eq!(forced_at(&solved.analysis, 0, 1), ForcedAnswer::Criminal);

        let subset_size =
            minimal_forcing_subset_size(&puzzle, &clues, 0, 0, &[(1, ForcedAnswer::Criminal)])
                .unwrap();

        assert_eq!(subset_size, 2);
    }

    #[test]
    fn minimal_forcing_subset_size_finds_smaller_subset_than_greedy_removal() {
        let names = NAMES[..CELL_COUNT]
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        let roles = (0..CELL_COUNT)
            .map(|_| "Role".to_string())
            .collect::<Vec<_>>();
        let puzzle = empty_puzzle(BoardShape::new(ROWS as u8, COLS as u8), &names, &roles);
        let target_name = puzzle.cells[0][1].name.clone();
        let clues = vec![
            Clue::Declaration {
                name: target_name.clone(),
                answer: crate::types::Answer::Criminal,
            },
            Clue::DirectRelation {
                name: target_name,
                answer: crate::types::Answer::Innocent,
                direction: crate::clue::Direction::Left,
            },
            Clue::CountCells {
                selector: CellSelector::Row { row: 0 },
                answer: crate::types::Answer::Innocent,
                count: Count::Number(1),
                filter: CellFilter::Any,
            },
        ];

        let greedy_subset =
            greedy_forcing_subset_size(&puzzle, &clues, 0, 0, &[(1, ForcedAnswer::Criminal)])
                .unwrap();
        let exact_subset =
            minimal_forcing_subset_size(&puzzle, &clues, 0, 0, &[(1, ForcedAnswer::Criminal)])
                .unwrap();

        assert_eq!(greedy_subset, 2);
        assert_eq!(exact_subset, 1);
    }

    #[test]
    fn closure_forced_tiles_count_as_free_context_for_combination_size() {
        let mut names = (0..CELL_COUNT)
            .map(|index| format!("Person {index}"))
            .collect::<Vec<_>>();
        names[0] = "Albert".to_string();
        names[4] = "Eddy".to_string();
        names[8] = "Isaac".to_string();
        names[16] = "Thrisha".to_string();
        let roles = (0..CELL_COUNT)
            .map(|_| "Role".to_string())
            .collect::<Vec<_>>();
        let puzzle = empty_puzzle(BoardShape::new(ROWS as u8, COLS as u8), &names, &roles);
        let corner_parity = Clue::CountCells {
            selector: CellSelector::Col { col: Column::A },
            answer: crate::types::Answer::Criminal,
            count: Count::Parity(Parity::Even),
            filter: CellFilter::Corner,
        };
        let neighbor_parity = Clue::CountCells {
            selector: CellSelector::Neighbor {
                name: "Eddy".to_string(),
            },
            answer: crate::types::Answer::Criminal,
            count: Count::Parity(Parity::Even),
            filter: CellFilter::Line(Line::Col(Column::A)),
        };
        let explicit_known_mask = 1u32 << 16;
        let explicit_known_innocent_mask = explicit_known_mask;
        let baseline = solve_clues_with_known_mask(
            &puzzle,
            &[corner_parity.clone()],
            explicit_known_mask,
            explicit_known_innocent_mask,
        )
        .unwrap();
        let (closed_known_mask, closed_known_innocent_mask) = closure_known_masks_from_analysis(
            &baseline.analysis,
            explicit_known_mask,
            explicit_known_innocent_mask,
        );
        let clues = vec![corner_parity, neighbor_parity];

        let explicit_subset = minimal_forcing_subset_size(
            &puzzle,
            &clues,
            explicit_known_mask,
            explicit_known_innocent_mask,
            &[(8, ForcedAnswer::Innocent)],
        )
        .unwrap();
        let closed_subset = minimal_forcing_subset_size(
            &puzzle,
            &clues,
            closed_known_mask,
            closed_known_innocent_mask,
            &[(8, ForcedAnswer::Innocent)],
        )
        .unwrap();

        assert_eq!(explicit_subset, 2);
        assert_eq!(closed_subset, 1);
    }

    fn histogram(values: &[usize]) -> BTreeMap<usize, usize> {
        let mut counts = BTreeMap::new();

        for value in values {
            *counts.entry(*value).or_insert(0) += 1;
        }

        counts
    }

    #[test]
    #[ignore]
    fn report_combination_size_distribution_for_generated_puzzles() {
        let puzzle_count = std::env::var("CLUES_COMBO_SAMPLE_SIZE")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(20);
        let mut stats = GenerationInstrumentation::default();

        for seed in 0..puzzle_count as u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            generate_puzzle_with_rng_and_instrumentation(
                &mut rng,
                BoardShape::new(ROWS as u8, COLS as u8),
                Some(&mut stats),
            )
            .unwrap();
        }

        let candidate_histogram = histogram(&stats.candidate_combination_sizes);
        let selected_histogram = histogram(&stats.selected_combination_sizes);
        let candidate_total = stats.candidate_combination_sizes.len();
        let selected_total = stats.selected_combination_sizes.len();
        let candidate_above_one = stats
            .candidate_combination_sizes
            .iter()
            .filter(|size| **size > 1)
            .count();
        let selected_above_one = stats
            .selected_combination_sizes
            .iter()
            .filter(|size| **size > 1)
            .count();

        println!("sampled_puzzles={puzzle_count}");
        println!("candidate_total={candidate_total}");
        println!("selected_total={selected_total}");
        println!(
            "candidate_above_one={} ({:.1}%)",
            candidate_above_one,
            candidate_above_one as f64 * 100.0 / candidate_total as f64
        );
        println!(
            "selected_above_one={} ({:.1}%)",
            selected_above_one,
            selected_above_one as f64 * 100.0 / selected_total as f64
        );
        println!(
            "candidate_newly_forced_count={}",
            stats.candidate_newly_forced_count
        );
        println!(
            "candidate_standalone_zero_count={}",
            stats.candidate_standalone_zero_count
        );
        println!(
            "candidate_combination_only_count={}",
            stats.candidate_combination_only_count
        );
        println!(
            "candidate_combo_eligible_count={}",
            stats.candidate_combo_eligible_count
        );
        println!("candidate_histogram={candidate_histogram:?}");
        println!("selected_histogram={selected_histogram:?}");
    }
}
