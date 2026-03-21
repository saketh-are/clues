use std::collections::HashSet;

use clues_core::{
    Answer, BoardShape, Cell, Clue, ForcedAnswer, NAMES, Puzzle, PuzzleValidationError, ROLES,
    Visibility, analyze_revealed_puzzle,
    clue::{CRIMINAL_NONSENSE_TEXTS, NONSENSE_TEXTS, PersonGroup},
    suggest_clue_for_known_tile,
};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

pub const MAX_NONSENSE_TEXT_CHARS: usize = 140;
pub const MAX_EMOJI_CHARS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorDraftCell {
    pub name: String,
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    pub answer: Option<Answer>,
    pub clue: Option<Clue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorPosition {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorDraftPuzzle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub cells: Vec<Vec<EditorDraftCell>>,
    pub initial_reveal: Option<EditorPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EditorAction {
    RenameCell {
        row: usize,
        col: usize,
        name: String,
    },
    ChangeRole {
        row: usize,
        col: usize,
        role: String,
    },
    ChangeEmoji {
        row: usize,
        col: usize,
        emoji: Option<String>,
    },
    SetInitialReveal {
        row: usize,
        col: usize,
        answer: Answer,
    },
    AddClue {
        row: usize,
        col: usize,
        clue: Clue,
    },
    UpdateNonsenseClue {
        row: usize,
        col: usize,
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EditorStateResponse {
    pub draft: EditorDraftPuzzle,
    pub resolved_answers: Vec<Vec<Option<Answer>>>,
    pub rendered_clues: Vec<Vec<Option<String>>>,
    pub referenced_roles: Vec<String>,
    pub next_clue_targets: Vec<EditorPosition>,
    pub share_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorProgressionStep {
    pub kind: String,
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EditorOpenResponse {
    pub draft: EditorDraftPuzzle,
    pub progression: Vec<EditorProgressionStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EditorBootstrapResponse {
    pub roles: Vec<String>,
    pub max_nonsense_text_chars: usize,
    pub nonsense_texts: Vec<String>,
    pub criminal_nonsense_texts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorErrorKind {
    BadRequest,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorError {
    pub kind: EditorErrorKind,
    pub message: String,
}

impl EditorError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: EditorErrorKind::BadRequest,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            kind: EditorErrorKind::Conflict,
            message: message.into(),
        }
    }
}

pub fn editor_bootstrap() -> EditorBootstrapResponse {
    EditorBootstrapResponse {
        roles: ROLES.iter().map(|role| role.to_string()).collect(),
        max_nonsense_text_chars: MAX_NONSENSE_TEXT_CHARS,
        nonsense_texts: NONSENSE_TEXTS.iter().map(|text| text.to_string()).collect(),
        criminal_nonsense_texts: CRIMINAL_NONSENSE_TEXTS
            .iter()
            .map(|text| text.to_string())
            .collect(),
    }
}

pub fn new_random_draft(board: BoardShape) -> Result<EditorStateResponse, EditorError> {
    let mut rng = thread_rng();
    let cell_count = board.rows as usize * board.cols as usize;

    if cell_count == 0 {
        return Err(EditorError::bad_request(
            "editor puzzles must have at least one cell",
        ));
    }

    if cell_count > NAMES.len() {
        return Err(EditorError::bad_request(
            "not enough names available for that board size",
        ));
    }

    let mut names = NAMES
        .iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    names.shuffle(&mut rng);
    names.truncate(cell_count);
    names.sort_unstable();

    let mut roles = ROLES
        .iter()
        .map(|role| role.to_string())
        .collect::<Vec<_>>();
    roles.shuffle(&mut rng);

    let cells = (0..board.rows as usize)
        .map(|row| {
            (0..board.cols as usize)
                .map(|col| {
                    let index = row * board.cols as usize + col;
                    EditorDraftCell {
                        name: names[index].clone(),
                        role: roles[index % roles.len()].clone(),
                        emoji: None,
                        answer: None,
                        clue: None,
                    }
                })
                .collect()
        })
        .collect();

    describe_draft(EditorDraftPuzzle {
        author: None,
        cells,
        initial_reveal: None,
    })
}

pub fn draft_from_puzzle(puzzle: &Puzzle) -> Result<EditorDraftPuzzle, EditorError> {
    puzzle.validate().map_err(map_puzzle_validation_error)?;

    let mut initial_reveal = None;
    let cells = puzzle
        .cells
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            row.iter()
                .enumerate()
                .map(|(col_index, cell)| {
                    if cell.state == Visibility::Revealed {
                        let position = EditorPosition {
                            row: row_index,
                            col: col_index,
                        };
                        if initial_reveal.replace(position).is_some() {
                            return Err(EditorError::bad_request(
                                "playable puzzle must have exactly one starting tile",
                            ));
                        }
                    }

                    Ok(EditorDraftCell {
                        name: cell.name.clone(),
                        role: cell.role.clone(),
                        emoji: cell.emoji.clone(),
                        answer: Some(cell.answer),
                        clue: Some(cell.clue.clone()),
                    })
                })
                .collect::<Result<Vec<_>, EditorError>>()
        })
        .collect::<Result<Vec<_>, EditorError>>()?;

    let initial_reveal = initial_reveal.ok_or_else(|| {
        EditorError::bad_request("playable puzzle must have a revealed starting tile")
    })?;

    Ok(EditorDraftPuzzle {
        author: puzzle.author.clone(),
        cells,
        initial_reveal: Some(initial_reveal),
    })
}

pub fn open_from_puzzle(puzzle: &Puzzle) -> Result<EditorOpenResponse, EditorError> {
    let draft = draft_from_puzzle(puzzle)?;
    let progression = progression_from_complete_draft(&draft)?;
    Ok(EditorOpenResponse { draft, progression })
}

pub fn apply_action(
    draft: EditorDraftPuzzle,
    action: EditorAction,
) -> Result<EditorStateResponse, EditorError> {
    draft.validate_structure()?;

    let mut next = draft.clone();
    match action {
        EditorAction::RenameCell { row, col, name } => {
            let updated_name = normalize_name(&name)?;
            let current_name = next
                .cell(row, col)
                .ok_or_else(|| EditorError::bad_request("tile is out of bounds"))?
                .name
                .clone();

            if current_name == updated_name {
                return describe_draft(next);
            }

            if next
                .cells
                .iter()
                .flatten()
                .any(|cell| cell.name == updated_name)
            {
                return Err(EditorError::conflict("names must stay distinct"));
            }

            next.rename_cell(&current_name, &updated_name)
                .ok_or_else(|| EditorError::bad_request("tile is out of bounds"))?;
        }
        EditorAction::ChangeRole { row, col, role } => {
            let updated_role = normalize_role(&role)?;
            let current_role = next
                .cell(row, col)
                .ok_or_else(|| EditorError::bad_request("tile is out of bounds"))?
                .role
                .clone();

            if current_role == updated_role {
                return describe_draft(next);
            }

            let referenced_roles = next.referenced_roles();
            if referenced_roles.contains(&current_role) || referenced_roles.contains(&updated_role)
            {
                return Err(EditorError::conflict(
                    "that role is already referenced by an existing clue",
                ));
            }

            next.cell_mut(row, col)
                .ok_or_else(|| EditorError::bad_request("tile is out of bounds"))?
                .role = updated_role;
        }
        EditorAction::ChangeEmoji { row, col, emoji } => {
            let updated_emoji = normalize_emoji(emoji.as_deref())?;
            let current_emoji = next
                .cell(row, col)
                .ok_or_else(|| EditorError::bad_request("tile is out of bounds"))?
                .emoji
                .clone();

            if current_emoji == updated_emoji {
                return describe_draft(next);
            }

            next.cell_mut(row, col)
                .ok_or_else(|| EditorError::bad_request("tile is out of bounds"))?
                .emoji = updated_emoji;
        }
        EditorAction::SetInitialReveal { row, col, answer } => {
            if next.initial_reveal.is_some() {
                return Err(EditorError::conflict(
                    "the starting tile can only be assigned once",
                ));
            }

            if next
                .cells
                .iter()
                .flatten()
                .any(|cell| cell.answer.is_some() || cell.clue.is_some())
            {
                return Err(EditorError::conflict(
                    "the starting tile must be the first authoring step",
                ));
            }

            let cell = next
                .cell_mut(row, col)
                .ok_or_else(|| EditorError::bad_request("tile is out of bounds"))?;
            cell.answer = Some(answer);
            next.initial_reveal = Some(EditorPosition { row, col });
        }
        EditorAction::AddClue { row, col, clue } => {
            let current_cell = next
                .cell(row, col)
                .ok_or_else(|| EditorError::bad_request("tile is out of bounds"))?;
            if current_cell.clue.is_some() {
                return Err(EditorError::conflict("that tile already has a clue"));
            }

            let analysis = analyze_draft(&next)?;
            let resolved_answer =
                next.resolved_answer_at(row, col, &analysis)
                    .ok_or_else(|| {
                        EditorError::conflict(
                            "that tile is not determined by the current information yet",
                        )
                    })?;
            if next.all_remaining_clueless_cells_are_forced(&analysis)
                && !matches!(clue, Clue::Nonsense { .. })
            {
                return Err(EditorError::conflict(
                    "once all remaining tiles are forced, only nonsense clues may be added",
                ));
            }

            let cell = next
                .cell_mut(row, col)
                .ok_or_else(|| EditorError::bad_request("tile is out of bounds"))?;
            cell.answer = Some(resolved_answer);
            cell.clue = Some(clue);

            next.validate_structure()?;

            let next_analysis = analyze_draft(&next)?;
            if !next_analysis.has_solution {
                return Err(EditorError::conflict(
                    "that clue conflicts with the current draft",
                ));
            }

            if !next.is_complete() && next.next_clue_targets(&next_analysis).is_empty() {
                return Err(EditorError::conflict(
                    "that clue would leave the puzzle stuck before it is complete",
                ));
            }
        }
        EditorAction::UpdateNonsenseClue { row, col, text } => {
            let updated_text = normalize_nonsense_text(&text)?;
            let cell = next
                .cell_mut(row, col)
                .ok_or_else(|| EditorError::bad_request("tile is out of bounds"))?;

            match &mut cell.clue {
                Some(Clue::Nonsense { text }) => {
                    *text = updated_text;
                }
                Some(_) => {
                    return Err(EditorError::conflict(
                        "only existing nonsense clues can be edited directly",
                    ));
                }
                None => {
                    return Err(EditorError::conflict(
                        "that tile does not have a nonsense clue yet",
                    ));
                }
            }
        }
    }

    describe_draft(next)
}

pub fn describe_draft(draft: EditorDraftPuzzle) -> Result<EditorStateResponse, EditorError> {
    draft.validate_structure()?;
    let analysis = analyze_draft(&draft)?;

    if !analysis.has_solution {
        return Err(EditorError::conflict("the current draft is inconsistent"));
    }

    let resolved_answers = draft.resolved_answers(&analysis);
    let rendered_clues = draft.rendered_clues();
    let referenced_roles = {
        let mut roles = draft.referenced_roles().into_iter().collect::<Vec<_>>();
        roles.sort();
        roles
    };
    let next_clue_targets = draft.next_clue_targets(&analysis);
    let share_ready = finalize_draft(&draft).is_ok();

    Ok(EditorStateResponse {
        draft,
        resolved_answers,
        rendered_clues,
        referenced_roles,
        next_clue_targets,
        share_ready,
    })
}

pub fn suggest_clue(draft: EditorDraftPuzzle, row: usize, col: usize) -> Result<Clue, EditorError> {
    draft.validate_structure()?;
    let analysis = analyze_draft(&draft)?;

    if !analysis.has_solution {
        return Err(EditorError::conflict("the current draft is inconsistent"));
    }

    let cell = draft
        .cell(row, col)
        .ok_or_else(|| EditorError::bad_request("tile is out of bounds"))?;
    if cell.clue.is_some() {
        return Err(EditorError::conflict("that tile already has a clue"));
    }

    let resolved_answer = draft
        .resolved_answer_at(row, col, &analysis)
        .ok_or_else(|| {
            EditorError::conflict("that tile is not determined by the current information yet")
        })?;

    if draft.all_remaining_clueless_cells_are_forced(&analysis) {
        return Ok(random_editor_nonsense_clue(resolved_answer));
    }

    let clues = draft
        .cells
        .iter()
        .flatten()
        .filter_map(|cell| cell.clue.clone())
        .collect::<Vec<_>>();
    let (known_mask, known_innocent_mask) = draft.known_masks();
    let puzzle = draft.to_progress_puzzle();
    let cols = draft
        .cells
        .first()
        .map(|cells| cells.len())
        .unwrap_or_default();
    let index = row * cols + col;

    suggest_clue_for_known_tile(
        &puzzle,
        &clues,
        known_mask,
        known_innocent_mask,
        index,
        resolved_answer,
    )
    .map_err(|error| EditorError::bad_request(format!("failed to suggest a clue: {error:?}")))?
    .ok_or_else(|| {
        EditorError::conflict("no generator-style clue suggestion was found for that tile")
    })
}

pub fn finalize_draft(draft: &EditorDraftPuzzle) -> Result<Puzzle, EditorError> {
    draft.validate_structure()?;
    if !draft.is_complete() {
        return Err(EditorError::conflict("the puzzle is not complete yet"));
    }

    let initial_reveal = draft
        .initial_reveal
        .ok_or_else(|| EditorError::conflict("the puzzle must have a starting tile"))?;
    let puzzle = Puzzle {
        author: normalize_author(draft.author.as_deref()),
        cells: draft
            .cells
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                row.iter()
                    .enumerate()
                    .map(|(col_index, cell)| {
                        Ok(Cell {
                            name: cell.name.clone(),
                            role: cell.role.clone(),
                            emoji: cell.emoji.clone(),
                            clue: cell.clue.clone().ok_or_else(|| {
                                EditorError::conflict("every tile must have a clue before sharing")
                            })?,
                            answer: cell.answer.ok_or_else(|| {
                                EditorError::conflict(
                                    "every tile must have an answer before sharing",
                                )
                            })?,
                            state: if initial_reveal.row == row_index
                                && initial_reveal.col == col_index
                            {
                                Visibility::Revealed
                            } else {
                                Visibility::Hidden
                            },
                        })
                    })
                    .collect::<Result<Vec<_>, EditorError>>()
            })
            .collect::<Result<Vec<_>, EditorError>>()?,
    };

    puzzle.validate().map_err(map_puzzle_validation_error)?;
    let mut playable = puzzle.clone();
    validate_playable_progression(&mut playable)?;
    Ok(puzzle)
}

impl EditorDraftPuzzle {
    fn validate_structure(&self) -> Result<(), EditorError> {
        let rows = self.cells.len();
        let cols = self
            .cells
            .first()
            .ok_or_else(|| EditorError::bad_request("editor puzzle cannot be empty"))?
            .len();
        if cols == 0 {
            return Err(EditorError::bad_request(
                "editor puzzle rows cannot be empty",
            ));
        }

        let mut seen_names = HashSet::new();
        let mut answer_without_clue = None;

        for (row_index, row) in self.cells.iter().enumerate() {
            if row.len() != cols {
                return Err(EditorError::bad_request(
                    "editor puzzle rows must all have the same length",
                ));
            }

            for (col_index, cell) in row.iter().enumerate() {
                let trimmed_name = cell.name.trim();
                if trimmed_name.is_empty() {
                    return Err(EditorError::bad_request("names cannot be empty"));
                }
                if !seen_names.insert(trimmed_name.to_string()) {
                    return Err(EditorError::conflict("names must stay distinct"));
                }

                if cell.role.trim().is_empty() {
                    return Err(EditorError::bad_request("roles cannot be empty"));
                }

                if let Some(emoji) = &cell.emoji {
                    let trimmed_emoji = emoji.trim();
                    if trimmed_emoji.is_empty() {
                        return Err(EditorError::bad_request("emoji cannot be empty"));
                    }
                    if trimmed_emoji.chars().count() > MAX_EMOJI_CHARS {
                        return Err(EditorError::bad_request(format!(
                            "emoji must be at most {MAX_EMOJI_CHARS} characters",
                        )));
                    }
                }

                if let Some(Clue::Nonsense { text }) = &cell.clue {
                    if text.chars().count() > MAX_NONSENSE_TEXT_CHARS {
                        return Err(EditorError::bad_request(format!(
                            "nonsense clues must be at most {MAX_NONSENSE_TEXT_CHARS} characters",
                        )));
                    }
                }

                match (cell.answer, cell.clue.is_some()) {
                    (None, true) => {
                        return Err(EditorError::bad_request(
                            "tiles cannot have a clue before their answer is known",
                        ));
                    }
                    (Some(_), false) => {
                        let position = EditorPosition {
                            row: row_index,
                            col: col_index,
                        };
                        if answer_without_clue.replace(position).is_some() {
                            return Err(EditorError::bad_request(
                                "only the current starting tile may be missing a clue",
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }

        match (self.initial_reveal, answer_without_clue) {
            (None, None) => Ok(()),
            (Some(initial_reveal), maybe_gap) => {
                if initial_reveal.row >= rows || initial_reveal.col >= cols {
                    return Err(EditorError::bad_request("starting tile is out of bounds"));
                }

                let cell = self
                    .cell(initial_reveal.row, initial_reveal.col)
                    .ok_or_else(|| EditorError::bad_request("starting tile is out of bounds"))?;
                if cell.answer.is_none() {
                    return Err(EditorError::bad_request(
                        "starting tile must have an answer",
                    ));
                }

                if let Some(position) = maybe_gap {
                    if position != initial_reveal {
                        return Err(EditorError::bad_request(
                            "only the starting tile may be missing a clue",
                        ));
                    }
                }

                Ok(())
            }
            (None, Some(_)) => Err(EditorError::bad_request(
                "a starting tile must be chosen before any answers are assigned",
            )),
        }
    }

    fn is_complete(&self) -> bool {
        self.initial_reveal.is_some()
            && self
                .cells
                .iter()
                .flatten()
                .all(|cell| cell.answer.is_some() && cell.clue.is_some())
    }

    fn cell(&self, row: usize, col: usize) -> Option<&EditorDraftCell> {
        self.cells.get(row).and_then(|cells| cells.get(col))
    }

    fn cell_mut(&mut self, row: usize, col: usize) -> Option<&mut EditorDraftCell> {
        self.cells.get_mut(row).and_then(|cells| cells.get_mut(col))
    }

    fn rename_cell(&mut self, current_name: &str, updated_name: &str) -> Option<()> {
        let mut renamed = false;

        for row in &mut self.cells {
            for cell in row {
                if cell.name == current_name {
                    cell.name = updated_name.to_string();
                    renamed = true;
                }

                if let Some(clue) = &mut cell.clue {
                    clue.rename_name_references(current_name, updated_name);
                }
            }
        }

        renamed.then_some(())
    }

    fn referenced_roles(&self) -> HashSet<String> {
        let mut roles = HashSet::new();
        for clue in self
            .cells
            .iter()
            .flatten()
            .filter_map(|cell| cell.clue.as_ref())
        {
            collect_referenced_roles(clue, &mut roles);
        }
        roles
    }

    fn resolved_answer_at(
        &self,
        row: usize,
        col: usize,
        analysis: &clues_core::ClueAnalysis,
    ) -> Option<Answer> {
        let cell = self.cell(row, col)?;
        if let Some(answer) = cell.answer {
            return Some(answer);
        }

        match analysis.forced_answers.get(row)?.get(col)? {
            ForcedAnswer::Criminal => Some(Answer::Criminal),
            ForcedAnswer::Innocent => Some(Answer::Innocent),
            ForcedAnswer::Neither => None,
        }
    }

    fn resolved_answers(&self, analysis: &clues_core::ClueAnalysis) -> Vec<Vec<Option<Answer>>> {
        self.cells
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                row.iter()
                    .enumerate()
                    .map(|(col_index, _)| self.resolved_answer_at(row_index, col_index, analysis))
                    .collect()
            })
            .collect()
    }

    fn rendered_clues(&self) -> Vec<Vec<Option<String>>> {
        self.cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.clue.as_ref().map(Clue::text))
                    .collect()
            })
            .collect()
    }

    fn next_clue_targets(&self, analysis: &clues_core::ClueAnalysis) -> Vec<EditorPosition> {
        self.cells
            .iter()
            .enumerate()
            .flat_map(|(row_index, row)| {
                row.iter().enumerate().filter_map(move |(col_index, cell)| {
                    (cell.clue.is_none()
                        && self
                            .resolved_answer_at(row_index, col_index, analysis)
                            .is_some())
                    .then_some(EditorPosition {
                        row: row_index,
                        col: col_index,
                    })
                })
            })
            .collect()
    }

    fn all_remaining_clueless_cells_are_forced(&self, analysis: &clues_core::ClueAnalysis) -> bool {
        self.cells.iter().enumerate().all(|(row_index, row)| {
            row.iter().enumerate().all(|(col_index, cell)| {
                cell.clue.is_some()
                    || self
                        .resolved_answer_at(row_index, col_index, analysis)
                        .is_some()
            })
        })
    }

    fn known_masks(&self) -> (u32, u32) {
        let mut known_mask = 0u32;
        let mut known_innocent_mask = 0u32;
        let cols = self
            .cells
            .first()
            .map(|cells| cells.len())
            .unwrap_or_default();

        for (row_index, row) in self.cells.iter().enumerate() {
            for (col_index, cell) in row.iter().enumerate() {
                let Some(answer) = cell.answer else {
                    continue;
                };

                let bit = 1u32 << (row_index * cols + col_index);
                known_mask |= bit;
                if answer == Answer::Innocent {
                    known_innocent_mask |= bit;
                }
            }
        }

        (known_mask, known_innocent_mask)
    }

    fn clue_presence_mask(&self) -> u32 {
        let cols = self
            .cells
            .first()
            .map(|cells| cells.len())
            .unwrap_or_default();
        let mut clue_mask = 0u32;

        for (row_index, row) in self.cells.iter().enumerate() {
            for (col_index, cell) in row.iter().enumerate() {
                if cell.clue.is_some() {
                    clue_mask |= 1u32 << (row_index * cols + col_index);
                }
            }
        }

        clue_mask
    }

    fn empty_authoring_copy(&self) -> Self {
        Self {
            author: self.author.clone(),
            cells: self
                .cells
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| EditorDraftCell {
                            name: cell.name.clone(),
                            role: cell.role.clone(),
                            emoji: cell.emoji.clone(),
                            answer: None,
                            clue: None,
                        })
                        .collect()
                })
                .collect(),
            initial_reveal: None,
        }
    }

    fn to_progress_puzzle(&self) -> Puzzle {
        Puzzle {
            author: self.author.clone(),
            cells: self
                .cells
                .iter()
                .enumerate()
                .map(|(row_index, row)| {
                    row.iter()
                        .enumerate()
                        .map(|(col_index, cell)| Cell {
                            name: cell.name.clone(),
                            role: cell.role.clone(),
                            emoji: cell.emoji.clone(),
                            clue: cell.clue.clone().unwrap_or_else(placeholder_clue),
                            answer: cell.answer.unwrap_or(Answer::Innocent),
                            state: if cell.clue.is_some()
                                || self.initial_reveal
                                    == Some(EditorPosition {
                                        row: row_index,
                                        col: col_index,
                                    })
                            {
                                Visibility::Revealed
                            } else {
                                Visibility::Hidden
                            },
                        })
                        .collect()
                })
                .collect(),
        }
    }
}

fn analyze_draft(draft: &EditorDraftPuzzle) -> Result<clues_core::ClueAnalysis, EditorError> {
    let puzzle = draft.to_progress_puzzle();
    analyze_revealed_puzzle(&puzzle)
        .map_err(|error| EditorError::bad_request(format!("failed to analyze draft: {error:?}")))
}

fn placeholder_clue() -> Clue {
    Clue::Nonsense {
        text: String::new(),
    }
}

fn random_editor_nonsense_clue(answer: Answer) -> Clue {
    let pool = match answer {
        Answer::Innocent => &NONSENSE_TEXTS[..],
        Answer::Criminal => &CRIMINAL_NONSENSE_TEXTS[..],
    };

    Clue::Nonsense {
        text: pool.choose(&mut thread_rng()).unwrap().to_string(),
    }
}

fn normalize_name(name: &str) -> Result<String, EditorError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(EditorError::bad_request("names cannot be empty"));
    }

    Ok(trimmed.to_string())
}

fn normalize_role(role: &str) -> Result<String, EditorError> {
    let trimmed = role.trim();
    if trimmed.is_empty() {
        return Err(EditorError::bad_request("roles cannot be empty"));
    }

    Ok(trimmed.to_string())
}

fn normalize_author(author: Option<&str>) -> Option<String> {
    let trimmed = author.unwrap_or("").trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn normalize_emoji(emoji: Option<&str>) -> Result<Option<String>, EditorError> {
    let Some(emoji) = emoji else {
        return Ok(None);
    };

    let trimmed = emoji.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_EMOJI_CHARS {
        return Err(EditorError::bad_request(format!(
            "emoji must be at most {MAX_EMOJI_CHARS} characters",
        )));
    }

    Ok(Some(trimmed.to_string()))
}

fn normalize_nonsense_text(text: &str) -> Result<String, EditorError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(EditorError::bad_request("nonsense clues need some text"));
    }
    if trimmed.chars().count() > MAX_NONSENSE_TEXT_CHARS {
        return Err(EditorError::bad_request(format!(
            "nonsense clues must be at most {MAX_NONSENSE_TEXT_CHARS} characters",
        )));
    }

    Ok(trimmed.to_string())
}

fn collect_referenced_roles(clue: &Clue, referenced_roles: &mut HashSet<String>) {
    match clue {
        Clue::RoleCount { role, .. } => {
            referenced_roles.insert(role.clone());
        }
        Clue::RolesComparison {
            first_role,
            second_role,
            ..
        } => {
            referenced_roles.insert(first_role.clone());
            referenced_roles.insert(second_role.clone());
        }
        Clue::Quantified {
            group: PersonGroup::Role { role },
            ..
        } => {
            referenced_roles.insert(role.clone());
        }
        _ => {}
    }
}

fn progression_from_complete_draft(
    completed_draft: &EditorDraftPuzzle,
) -> Result<Vec<EditorProgressionStep>, EditorError> {
    completed_draft.validate_structure()?;
    if !completed_draft.is_complete() {
        return Err(EditorError::bad_request(
            "only complete playable puzzles can be opened with clue progression",
        ));
    }

    let initial_reveal = completed_draft.initial_reveal.ok_or_else(|| {
        EditorError::bad_request("playable puzzle must have a revealed starting tile")
    })?;
    let initial_answer = completed_draft
        .cell(initial_reveal.row, initial_reveal.col)
        .and_then(|cell| cell.answer)
        .ok_or_else(|| EditorError::bad_request("starting tile must have an answer"))?;
    let mut authoring_draft = completed_draft.empty_authoring_copy();
    authoring_draft.initial_reveal = Some(initial_reveal);
    authoring_draft
        .cell_mut(initial_reveal.row, initial_reveal.col)
        .ok_or_else(|| EditorError::bad_request("starting tile is out of bounds"))?
        .answer = Some(initial_answer);

    let mut progression = Vec::new();
    let mut failed_masks = HashSet::new();
    if build_progression_steps(
        completed_draft,
        authoring_draft,
        &mut progression,
        &mut failed_masks,
    )? {
        Ok(progression)
    } else {
        Err(EditorError::conflict(
            "could not recover a valid clue order for this playable puzzle",
        ))
    }
}

fn build_progression_steps(
    completed_draft: &EditorDraftPuzzle,
    current_draft: EditorDraftPuzzle,
    progression: &mut Vec<EditorProgressionStep>,
    failed_masks: &mut HashSet<u32>,
) -> Result<bool, EditorError> {
    if current_draft.is_complete() {
        return Ok(true);
    }

    let clue_mask = current_draft.clue_presence_mask();
    if failed_masks.contains(&clue_mask) {
        return Ok(false);
    }

    let state = describe_draft(current_draft.clone())?;
    let mut next_targets = state.next_clue_targets;
    next_targets.sort_by_key(|position| {
        let clue = completed_draft
            .cell(position.row, position.col)
            .and_then(|cell| cell.clue.as_ref());
        let nonsense_rank = usize::from(matches!(clue, Some(Clue::Nonsense { .. })));
        (nonsense_rank, position.row, position.col)
    });

    for target in next_targets {
        let Some(clue) = completed_draft
            .cell(target.row, target.col)
            .and_then(|cell| cell.clue.clone())
        else {
            continue;
        };

        let Ok(next_state) = apply_action(
            current_draft.clone(),
            EditorAction::AddClue {
                row: target.row,
                col: target.col,
                clue,
            },
        ) else {
            continue;
        };

        progression.push(EditorProgressionStep::add_clue(target.row, target.col));
        if build_progression_steps(completed_draft, next_state.draft, progression, failed_masks)? {
            return Ok(true);
        }
        progression.pop();
    }

    failed_masks.insert(clue_mask);
    Ok(false)
}

fn validate_playable_progression(puzzle: &mut Puzzle) -> Result<(), EditorError> {
    loop {
        let analysis = analyze_revealed_puzzle(puzzle).map_err(|error| {
            EditorError::bad_request(format!(
                "failed to validate completed puzzle progression: {error:?}"
            ))
        })?;

        if !analysis.has_solution {
            return Err(EditorError::conflict("the finished puzzle is inconsistent"));
        }

        let mut unrevealed = 0usize;
        let mut revealed_any = false;
        for (row_index, row) in puzzle.cells.iter_mut().enumerate() {
            for (col_index, cell) in row.iter_mut().enumerate() {
                if cell.state == Visibility::Revealed {
                    continue;
                }

                unrevealed += 1;
                match analysis.forced_answers[row_index][col_index] {
                    ForcedAnswer::Criminal | ForcedAnswer::Innocent => {
                        cell.state = Visibility::Revealed;
                        revealed_any = true;
                    }
                    ForcedAnswer::Neither => {}
                }
            }
        }

        if unrevealed == 0 {
            return Ok(());
        }

        if !revealed_any {
            return Err(EditorError::conflict(
                "the finished puzzle still has unrevealed tiles that are not forced",
            ));
        }
    }
}

impl EditorProgressionStep {
    fn add_clue(row: usize, col: usize) -> Self {
        Self {
            kind: "add_clue".to_string(),
            row,
            col,
        }
    }
}

fn map_puzzle_validation_error(error: PuzzleValidationError) -> EditorError {
    match error {
        PuzzleValidationError::DuplicateName(_) => {
            EditorError::conflict("names must stay distinct")
        }
        other => EditorError::bad_request(format!("invalid puzzle: {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use clues_core::{
        BoardShape,
        clue::{CellFilter, CellSelector, Count},
        generate_puzzle_with_seed_and_size,
        types::Answer,
    };

    use super::{EditorAction, EditorDraftCell, EditorDraftPuzzle, EditorPosition, finalize_draft};

    fn simple_draft() -> EditorDraftPuzzle {
        EditorDraftPuzzle {
            author: None,
            cells: vec![vec![
                EditorDraftCell {
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    emoji: None,
                    answer: Some(Answer::Innocent),
                    clue: Some(clues_core::Clue::CountCells {
                        selector: CellSelector::Neighbor {
                            name: "Ben".to_string(),
                        },
                        answer: Answer::Innocent,
                        count: Count::AtLeast(0),
                        filter: CellFilter::Any,
                    }),
                },
                EditorDraftCell {
                    name: "Ben".to_string(),
                    role: "Baker".to_string(),
                    emoji: None,
                    answer: Some(Answer::Criminal),
                    clue: Some(clues_core::Clue::Nonsense {
                        text: "ok".to_string(),
                    }),
                },
            ]],
            initial_reveal: Some(EditorPosition { row: 0, col: 0 }),
        }
    }

    #[test]
    fn rename_updates_clue_references() {
        let response = super::apply_action(
            simple_draft(),
            EditorAction::RenameCell {
                row: 0,
                col: 1,
                name: "Bianca".to_string(),
            },
        )
        .unwrap();

        assert_eq!(response.draft.cells[0][1].name, "Bianca");
        assert_eq!(
            response.draft.cells[0][0].clue,
            Some(clues_core::Clue::CountCells {
                selector: CellSelector::Neighbor {
                    name: "Bianca".to_string(),
                },
                answer: Answer::Innocent,
                count: Count::AtLeast(0),
                filter: CellFilter::Any,
            })
        );
    }

    #[test]
    fn referenced_roles_block_role_changes() {
        let draft = EditorDraftPuzzle {
            author: None,
            cells: vec![vec![
                EditorDraftCell {
                    name: "Ada".to_string(),
                    role: "Detective".to_string(),
                    emoji: None,
                    answer: Some(Answer::Innocent),
                    clue: Some(clues_core::Clue::RoleCount {
                        role: "Baker".to_string(),
                        answer: Answer::Innocent,
                        count: Count::Number(1),
                    }),
                },
                EditorDraftCell {
                    name: "Ben".to_string(),
                    role: "Guard".to_string(),
                    emoji: None,
                    answer: Some(Answer::Criminal),
                    clue: Some(clues_core::Clue::Nonsense {
                        text: "hmm".to_string(),
                    }),
                },
            ]],
            initial_reveal: Some(EditorPosition { row: 0, col: 0 }),
        };

        let error = super::apply_action(
            draft,
            EditorAction::ChangeRole {
                row: 0,
                col: 1,
                role: "Baker".to_string(),
            },
        )
        .unwrap_err();

        assert_eq!(error.kind, super::EditorErrorKind::Conflict);
    }

    #[test]
    fn custom_roles_are_allowed() {
        let response = super::apply_action(
            simple_draft(),
            EditorAction::ChangeRole {
                row: 0,
                col: 1,
                role: "Coach".to_string(),
            },
        )
        .unwrap();

        assert_eq!(response.draft.cells[0][1].role, "Coach");
    }

    #[test]
    fn new_random_draft_starts_with_alphabetized_names() {
        let response = super::new_random_draft(BoardShape::new(2, 2)).unwrap();
        let names = response
            .draft
            .cells
            .iter()
            .flatten()
            .map(|cell| cell.name.clone())
            .collect::<Vec<_>>();
        let mut sorted = names.clone();
        sorted.sort_unstable();

        assert_eq!(names, sorted);
    }

    #[test]
    fn finalize_requires_clues_on_every_tile() {
        let error = finalize_draft(&EditorDraftPuzzle {
            author: None,
            cells: vec![vec![EditorDraftCell {
                name: "Ada".to_string(),
                role: "Detective".to_string(),
                emoji: None,
                answer: Some(Answer::Innocent),
                clue: None,
            }]],
            initial_reveal: Some(EditorPosition { row: 0, col: 0 }),
        })
        .unwrap_err();

        assert_eq!(error.kind, super::EditorErrorKind::Conflict);
    }

    #[test]
    fn finalize_keeps_only_the_starting_tile_revealed() {
        let generated = generate_puzzle_with_seed_and_size(7, BoardShape::new(2, 2)).unwrap();
        let mut initial_reveal = None;
        let draft = EditorDraftPuzzle {
            author: Some("Ada Lovelace".to_string()),
            cells: generated
                .puzzle
                .cells
                .iter()
                .enumerate()
                .map(|(row_index, row)| {
                    row.iter()
                        .enumerate()
                        .map(|(col_index, cell)| {
                            if cell.state == clues_core::Visibility::Revealed {
                                initial_reveal = Some(EditorPosition {
                                    row: row_index,
                                    col: col_index,
                                });
                            }

                            EditorDraftCell {
                                name: cell.name.clone(),
                                role: cell.role.clone(),
                                emoji: cell.emoji.clone(),
                                answer: Some(cell.answer),
                                clue: Some(cell.clue.clone()),
                            }
                        })
                        .collect()
                })
                .collect(),
            initial_reveal,
        };
        let puzzle = finalize_draft(&draft).unwrap();

        let revealed_tiles = puzzle
            .cells
            .iter()
            .flatten()
            .filter(|cell| cell.state == clues_core::Visibility::Revealed)
            .count();

        assert_eq!(revealed_tiles, 1);
        assert_eq!(puzzle.author.as_deref(), Some("Ada Lovelace"));
    }

    #[test]
    fn open_from_puzzle_reconstructs_a_removable_clue_progression() {
        let generated = generate_puzzle_with_seed_and_size(7, BoardShape::new(2, 2)).unwrap();
        let opened = super::open_from_puzzle(&generated.puzzle).unwrap();

        assert_eq!(opened.progression.len(), 4);

        let initial_reveal = opened.draft.initial_reveal.unwrap();
        let initial_answer = opened.draft.cells[initial_reveal.row][initial_reveal.col]
            .answer
            .unwrap();
        let mut replay = opened.draft.empty_authoring_copy();
        replay.initial_reveal = Some(initial_reveal);
        replay.cells[initial_reveal.row][initial_reveal.col].answer = Some(initial_answer);

        for step in &opened.progression {
            let clue = opened.draft.cells[step.row][step.col].clue.clone().unwrap();
            replay = super::apply_action(
                replay,
                EditorAction::AddClue {
                    row: step.row,
                    col: step.col,
                    clue,
                },
            )
            .unwrap()
            .draft;
        }

        assert_eq!(replay, opened.draft);
    }

    #[test]
    fn only_nonsense_is_allowed_once_all_remaining_tiles_are_forced() {
        let generated = generate_puzzle_with_seed_and_size(7, BoardShape::new(2, 2)).unwrap();
        let mut initial_reveal = None;
        let mut draft = EditorDraftPuzzle {
            author: None,
            cells: generated
                .puzzle
                .cells
                .iter()
                .enumerate()
                .map(|(row_index, row)| {
                    row.iter()
                        .enumerate()
                        .map(|(col_index, cell)| {
                            if cell.state == clues_core::Visibility::Revealed {
                                initial_reveal = Some(EditorPosition {
                                    row: row_index,
                                    col: col_index,
                                });
                            }

                            EditorDraftCell {
                                name: cell.name.clone(),
                                role: cell.role.clone(),
                                emoji: cell.emoji.clone(),
                                answer: Some(cell.answer),
                                clue: Some(cell.clue.clone()),
                            }
                        })
                        .collect()
                })
                .collect(),
            initial_reveal,
        };

        let target = draft
            .cells
            .iter()
            .enumerate()
            .flat_map(|(row_index, row)| {
                row.iter().enumerate().filter_map(move |(col_index, cell)| {
                    (Some(EditorPosition {
                        row: row_index,
                        col: col_index,
                    }) != initial_reveal
                        && !matches!(cell.clue, Some(clues_core::Clue::Nonsense { .. })))
                    .then_some((row_index, col_index))
                })
            })
            .next()
            .unwrap();
        let non_nonsense_clue = draft.cells[target.0][target.1].clue.clone().unwrap();
        draft.cells[target.0][target.1].clue = None;
        draft.cells[target.0][target.1].answer = None;

        let error = super::apply_action(
            draft,
            EditorAction::AddClue {
                row: target.0,
                col: target.1,
                clue: non_nonsense_clue,
            },
        )
        .unwrap_err();

        assert_eq!(error.kind, super::EditorErrorKind::Conflict);
        assert_eq!(
            error.message,
            "once all remaining tiles are forced, only nonsense clues may be added"
        );
    }

    #[test]
    fn existing_nonsense_clues_can_be_edited_directly() {
        let response = super::apply_action(
            simple_draft(),
            EditorAction::UpdateNonsenseClue {
                row: 0,
                col: 1,
                text: "updated".to_string(),
            },
        )
        .unwrap();

        assert_eq!(
            response.draft.cells[0][1].clue,
            Some(clues_core::Clue::Nonsense {
                text: "updated".to_string(),
            })
        );
    }
}
