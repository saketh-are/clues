mod editor;

use std::{fs, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use clues_core::{
    Answer, BoardShape, CellFilter, CellSelector, Clue, ClueScoreTerms, DEFAULT_COLS,
    DEFAULT_ROWS, ForcedAnswer, GeneratedPuzzle, GeneratedPuzzle3D, Line, PersonGroup,
    PersonPredicate, Puzzle, Puzzle3D, PuzzleValidationError, StoredPuzzle, Visibility,
    analyze_revealed_puzzle, analyze_revealed_puzzle_3d,
    generate_puzzle_3d_with_seed_and_size, generate_puzzle_with_seed_and_size,
};
use editor::{
    EditorAction, EditorBootstrapResponse, EditorDraftPuzzle, EditorError, EditorErrorKind,
    EditorOpenResponse, EditorStateResponse, apply_action as apply_editor_action_to_draft,
    describe_draft as describe_editor_draft, editor_bootstrap, finalize_draft, new_random_draft,
    open_from_puzzle, suggest_clue as suggest_editor_clue,
};
use rand::random;
use rocksdb::{DB, Options};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

const SEED_MASK: u64 = 0xFFFF_FFFF_FFFF;
const MAX_PUBLIC_CELL_COUNT: usize = 20;
const STORED_PUZZLE_DB_PREFIX: &str = "stored_puzzle:";
const STORED_PUZZLE_ID_MASK: u32 = 0x00FF_FFFF;

#[derive(Clone)]
struct AppState {
    stored_puzzles: Arc<DB>,
}

#[derive(Debug, Deserialize)]
struct NewPuzzleParams {
    seed: Option<String>,
    depth: Option<u8>,
    rows: Option<u8>,
    cols: Option<u8>,
}

#[derive(Debug, Serialize)]
struct PuzzleResponse {
    seed: Option<String>,
    stored_puzzle_id: Option<String>,
    author: Option<String>,
    rows: u8,
    cols: u8,
    cells: Vec<Vec<CellResponse>>,
    generated_score_series: Vec<ClueScoreTerms>,
    generated_clue_texts: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Puzzle3DResponse {
    seed: Option<String>,
    stored_puzzle_id: Option<String>,
    author: Option<String>,
    depth: u8,
    rows: u8,
    cols: u8,
    cells: Vec<Vec<Vec<CellResponse>>>,
}

#[derive(Debug, Serialize)]
struct CellResponse {
    name: String,
    role: String,
    emoji: Option<String>,
    clue: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    clue_highlight_groups: Vec<Vec<HighlightCell3DResponse>>,
    is_nonsense: bool,
    score_terms: Option<ClueScoreTerms>,
    revealed_answer: Option<Answer>,
    revealed: bool,
}

#[derive(Debug, Clone, Serialize)]
struct HighlightCell3DResponse {
    layer: usize,
    row: usize,
    col: usize,
}

#[derive(Debug, Serialize)]
struct GuessResponse {
    clue: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    clue_highlight_groups: Vec<Vec<HighlightCell3DResponse>>,
    is_nonsense: bool,
    score_terms: Option<ClueScoreTerms>,
}

#[derive(Debug, Serialize)]
struct CreateStoredPuzzleResponse {
    stored_puzzle_id: String,
}

#[derive(Debug, Deserialize)]
struct GuessRequest {
    source: PuzzleSource,
    #[serde(default)]
    moves: Vec<AppliedGuess>,
    row: usize,
    col: usize,
    guess: Answer,
}

#[derive(Debug, Deserialize)]
struct Guess3DRequest {
    source: PuzzleSource3D,
    #[serde(default)]
    moves: Vec<AppliedGuess3D>,
    layer: usize,
    row: usize,
    col: usize,
    guess: Answer,
}

#[derive(Debug, Clone, Deserialize)]
struct AppliedGuess3D {
    layer: usize,
    row: usize,
    col: usize,
    guess: Answer,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PuzzleSource {
    Generated { seed: String, rows: u8, cols: u8 },
    Stored { stored_puzzle_id: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PuzzleSource3D {
    Generated {
        seed: String,
        depth: u8,
        rows: u8,
        cols: u8,
    },
    Stored {
        stored_puzzle_id: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct AppliedGuess {
    row: usize,
    col: usize,
    guess: Answer,
}

#[derive(Debug, Deserialize)]
struct NewEditorPuzzleRequest {
    rows: u8,
    cols: u8,
}

#[derive(Debug, Deserialize)]
struct EditorApplyRequest {
    draft: EditorDraftPuzzle,
    action: EditorAction,
}

#[derive(Debug, Deserialize)]
struct EditorDescribeRequest {
    draft: EditorDraftPuzzle,
}

#[derive(Debug, Deserialize)]
struct EditorShareRequest {
    draft: EditorDraftPuzzle,
}

#[derive(Debug, Deserialize)]
struct EditorOpenRequest {
    source: PuzzleSource,
}

#[derive(Debug, Deserialize)]
struct EditorSuggestRequest {
    draft: EditorDraftPuzzle,
    row: usize,
    col: usize,
}

#[derive(Debug, Serialize)]
struct EditorSuggestResponse {
    clue: Clue,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        stored_puzzles: Arc::new(open_stored_puzzle_db().expect("open stored puzzle db")),
    };

    let static_dir = format!("{}/static", env!("CARGO_MANIFEST_DIR"));
    let app = Router::new()
        .route("/", get(index_html))
        .route("/3d", get(index_3d_html))
        .route("/3d/", get(index_3d_html))
        .route("/3d/p/{stored_puzzle_id}", get(index_3d_html))
        .route("/edit", get(edit_html))
        .route("/p/{stored_puzzle_id}", get(index_html))
        .route("/api/puzzles/new", get(new_puzzle))
        .route("/api/puzzles/guess", post(guess_cell))
        .route("/api/3d/puzzles/new", get(new_puzzle_3d))
        .route("/api/3d/puzzles/guess", post(guess_cell_3d))
        .route(
            "/api/3d/stored-puzzles/generate",
            post(create_stored_puzzle_3d),
        )
        .route(
            "/api/3d/stored-puzzles/{stored_puzzle_id}",
            get(load_stored_puzzle_3d),
        )
        .route("/api/editor/bootstrap", get(editor_bootstrap_handler))
        .route("/api/editor/new", post(new_editor_puzzle))
        .route("/api/editor/describe", post(describe_editor_puzzle))
        .route("/api/editor/open", post(open_editor_puzzle))
        .route("/api/editor/apply", post(apply_editor_action))
        .route("/api/editor/suggest", post(suggest_editor_clue_handler))
        .route("/api/editor/share", post(share_editor_puzzle))
        .route("/api/stored-puzzles/generate", post(create_stored_puzzle))
        .route(
            "/api/stored-puzzles/{stored_puzzle_id}",
            get(load_stored_puzzle),
        )
        .fallback_service(ServeDir::new(static_dir))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind localhost server");

    println!("serving on http://127.0.0.1:3000");
    axum::serve(listener, app).await.expect("run server");
}

async fn index_html() -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate")],
        Html(include_str!("../static/index.html")),
    )
}

async fn edit_html() -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate")],
        Html(include_str!("../static/edit.html")),
    )
}

async fn index_3d_html() -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate")],
        Html(include_str!("../static/3d.html")),
    )
}

async fn new_puzzle(
    Query(params): Query<NewPuzzleParams>,
) -> Result<Json<PuzzleResponse>, AppError> {
    let (seed, generated) =
        generate_requested_puzzle(params.seed.as_deref(), params.rows, params.cols)?;
    Ok(Json(PuzzleResponse::from_generated(seed, &generated)))
}

async fn new_puzzle_3d(
    Query(params): Query<NewPuzzleParams>,
) -> Result<Json<Puzzle3DResponse>, AppError> {
    let (seed, generated) = generate_requested_puzzle_3d(
        params.seed.as_deref(),
        params.depth,
        params.rows,
        params.cols,
    )?;
    Ok(Json(Puzzle3DResponse::from_generated(seed, &generated)))
}

async fn create_stored_puzzle(
    State(state): State<AppState>,
    Query(params): Query<NewPuzzleParams>,
) -> Result<Json<CreateStoredPuzzleResponse>, AppError> {
    let (_, generated) =
        generate_requested_puzzle(params.seed.as_deref(), params.rows, params.cols)?;
    let stored_puzzle_id = save_puzzle_as_stored(&state.stored_puzzles, &generated.puzzle)?;

    Ok(Json(CreateStoredPuzzleResponse { stored_puzzle_id }))
}

async fn create_stored_puzzle_3d(
    State(state): State<AppState>,
    Query(params): Query<NewPuzzleParams>,
) -> Result<Json<CreateStoredPuzzleResponse>, AppError> {
    let (_, generated) = generate_requested_puzzle_3d(
        params.seed.as_deref(),
        params.depth,
        params.rows,
        params.cols,
    )?;
    let stored_puzzle_id = save_puzzle_3d_as_stored(&state.stored_puzzles, &generated.puzzle)?;

    Ok(Json(CreateStoredPuzzleResponse { stored_puzzle_id }))
}

async fn editor_bootstrap_handler() -> Json<EditorBootstrapResponse> {
    Json(editor_bootstrap())
}

async fn new_editor_puzzle(
    Json(request): Json<NewEditorPuzzleRequest>,
) -> Result<Json<EditorStateResponse>, AppError> {
    let board = parse_board_shape(Some(request.rows), Some(request.cols))?;
    let draft = new_random_draft(board).map_err(map_editor_error)?;
    Ok(Json(draft))
}

async fn describe_editor_puzzle(
    Json(request): Json<EditorDescribeRequest>,
) -> Result<Json<EditorStateResponse>, AppError> {
    let response = describe_editor_draft(request.draft).map_err(map_editor_error)?;
    Ok(Json(response))
}

async fn open_editor_puzzle(
    State(state): State<AppState>,
    Json(request): Json<EditorOpenRequest>,
) -> Result<Json<EditorOpenResponse>, AppError> {
    let puzzle = match request.source {
        PuzzleSource::Generated { seed, rows, cols } => {
            let (_, generated) =
                generate_requested_puzzle(Some(seed.as_str()), Some(rows), Some(cols))?;
            generated.puzzle
        }
        PuzzleSource::Stored { stored_puzzle_id } => {
            load_stored_puzzle_by_id(&state, &stored_puzzle_id)?
        }
    };

    let response = open_from_puzzle(&puzzle).map_err(map_editor_error)?;
    Ok(Json(response))
}

async fn apply_editor_action(
    Json(request): Json<EditorApplyRequest>,
) -> Result<Json<EditorStateResponse>, AppError> {
    let response =
        apply_editor_action_to_draft(request.draft, request.action).map_err(map_editor_error)?;
    Ok(Json(response))
}

async fn suggest_editor_clue_handler(
    Json(request): Json<EditorSuggestRequest>,
) -> Result<Json<EditorSuggestResponse>, AppError> {
    let clue =
        suggest_editor_clue(request.draft, request.row, request.col).map_err(map_editor_error)?;
    Ok(Json(EditorSuggestResponse { clue }))
}

async fn share_editor_puzzle(
    State(state): State<AppState>,
    Json(request): Json<EditorShareRequest>,
) -> Result<Json<CreateStoredPuzzleResponse>, AppError> {
    let puzzle = finalize_draft(&request.draft).map_err(map_editor_error)?;
    let stored_puzzle_id = save_puzzle_as_stored(&state.stored_puzzles, &puzzle)?;
    Ok(Json(CreateStoredPuzzleResponse { stored_puzzle_id }))
}

async fn load_stored_puzzle(
    State(state): State<AppState>,
    Path(stored_puzzle_id): Path<String>,
) -> Result<Json<PuzzleResponse>, AppError> {
    let puzzle = load_stored_puzzle_by_id(&state, &stored_puzzle_id)?;
    Ok(Json(PuzzleResponse::from_stored(
        &stored_puzzle_id,
        &puzzle,
    )))
}

async fn load_stored_puzzle_3d(
    State(state): State<AppState>,
    Path(stored_puzzle_id): Path<String>,
) -> Result<Json<Puzzle3DResponse>, AppError> {
    let puzzle = load_stored_puzzle_3d_by_id(&state, &stored_puzzle_id)?;
    Ok(Json(Puzzle3DResponse::from_stored(
        &stored_puzzle_id,
        &puzzle,
    )))
}

async fn guess_cell(
    State(state): State<AppState>,
    Json(request): Json<GuessRequest>,
) -> Result<Json<GuessResponse>, AppError> {
    let GuessRequest {
        source,
        moves,
        row,
        col,
        guess,
    } = request;

    let (mut puzzle, generated_score_terms) = match source {
        PuzzleSource::Generated { seed, rows, cols } => {
            let (_, generated) =
                generate_requested_puzzle(Some(seed.as_str()), Some(rows), Some(cols))?;
            (generated.puzzle, Some(generated.clue_score_terms))
        }
        PuzzleSource::Stored { stored_puzzle_id } => {
            (load_stored_puzzle_by_id(&state, &stored_puzzle_id)?, None)
        }
    };

    replay_moves(&mut puzzle, &moves)?;
    validate_and_reveal_guess(&mut puzzle, row, col, guess)?;
    let score_terms = generated_score_terms.map(|rows| rows[row][col].clone());

    Ok(Json(GuessResponse {
        clue: puzzle.cells[row][col].clue.text(),
        clue_highlight_groups: Vec::new(),
        is_nonsense: matches!(
            puzzle.cells[row][col].clue,
            clues_core::Clue::Nonsense { .. }
        ),
        score_terms,
    }))
}

async fn guess_cell_3d(
    State(state): State<AppState>,
    Json(request): Json<Guess3DRequest>,
) -> Result<Json<GuessResponse>, AppError> {
    let Guess3DRequest {
        source,
        moves,
        layer,
        row,
        col,
        guess,
    } = request;

    let mut puzzle = match source {
        PuzzleSource3D::Generated {
            seed,
            depth,
            rows,
            cols,
        } => {
            let (_, generated) = generate_requested_puzzle_3d(
                Some(seed.as_str()),
                Some(depth),
                Some(rows),
                Some(cols),
            )?;
            generated.puzzle
        }
        PuzzleSource3D::Stored { stored_puzzle_id } => {
            load_stored_puzzle_3d_by_id(&state, &stored_puzzle_id)?
        }
    };

    replay_moves_3d(&mut puzzle, &moves)?;
    validate_and_reveal_guess_3d(&mut puzzle, layer, row, col, guess)?;

    Ok(Json(GuessResponse {
        clue: puzzle.cells[layer][row][col].clue.text(),
        clue_highlight_groups: clue_highlight_groups_3d(
            &puzzle,
            &puzzle.cells[layer][row][col].clue,
        ),
        is_nonsense: matches!(puzzle.cells[layer][row][col].clue, clues_core::Clue::Nonsense { .. }),
        score_terms: None,
    }))
}

fn generate_requested_puzzle(
    seed: Option<&str>,
    rows: Option<u8>,
    cols: Option<u8>,
) -> Result<(u64, GeneratedPuzzle), AppError> {
    let seed = match seed {
        Some(seed) => parse_seed(seed)?,
        None => random::<u64>() & SEED_MASK,
    };
    let board = parse_board_shape(rows, cols)?;
    let generated = generate_puzzle_with_seed_and_size(seed, board)
        .map_err(|error| AppError::internal(format!("failed to generate puzzle: {error:?}")))?;
    Ok((seed, generated))
}

fn generate_requested_puzzle_3d(
    seed: Option<&str>,
    depth: Option<u8>,
    rows: Option<u8>,
    cols: Option<u8>,
) -> Result<(u64, GeneratedPuzzle3D), AppError> {
    let seed = match seed {
        Some(seed) => parse_seed(seed)?,
        None => random::<u64>() & SEED_MASK,
    };
    let board = parse_board_shape_3d(depth, rows, cols)?;
    let generated = generate_puzzle_3d_with_seed_and_size(seed, board)
        .map_err(|error| AppError::internal(format!("failed to generate 3d puzzle: {error:?}")))?;
    Ok((seed, generated))
}

fn load_stored_puzzle_by_id(state: &AppState, stored_puzzle_id: &str) -> Result<Puzzle, AppError> {
    let stored_puzzle = load_stored_puzzle_document_by_id(state, stored_puzzle_id)?;
    let puzzle = match stored_puzzle {
        StoredPuzzle::V1(puzzle) => puzzle,
        StoredPuzzle::V1ThreeD(_) => {
            return Err(AppError::bad_request("stored puzzle is a 3d puzzle"));
        }
    };

    puzzle
        .validate()
        .map_err(map_stored_puzzle_validation_error)?;
    Ok(puzzle)
}

fn load_stored_puzzle_3d_by_id(
    state: &AppState,
    stored_puzzle_id: &str,
) -> Result<Puzzle3D, AppError> {
    let stored_puzzle = load_stored_puzzle_document_by_id(state, stored_puzzle_id)?;
    let puzzle = match stored_puzzle {
        StoredPuzzle::V1(_) => return Err(AppError::bad_request("stored puzzle is a 2d puzzle")),
        StoredPuzzle::V1ThreeD(puzzle) => puzzle,
    };

    puzzle
        .validate()
        .map_err(map_stored_puzzle_validation_error)?;
    Ok(puzzle)
}

fn load_stored_puzzle_document_by_id(
    state: &AppState,
    stored_puzzle_id: &str,
) -> Result<StoredPuzzle, AppError> {
    let encoded = state
        .stored_puzzles
        .get_pinned(stored_puzzle_db_key(stored_puzzle_id))
        .map_err(|error| AppError::internal(format!("failed to read stored puzzle: {error}")))?
        .ok_or_else(|| AppError::not_found("stored puzzle not found"))?;
    serde_json::from_slice(&encoded).map_err(|error| {
        AppError::internal(format!(
            "failed to decode stored puzzle {stored_puzzle_id}: {error}"
        ))
    })
}

fn save_puzzle_as_stored(db: &DB, puzzle: &Puzzle) -> Result<String, AppError> {
    save_stored_puzzle(db, &StoredPuzzle::from(puzzle))
}

fn save_puzzle_3d_as_stored(db: &DB, puzzle: &Puzzle3D) -> Result<String, AppError> {
    save_stored_puzzle(db, &StoredPuzzle::from(puzzle))
}

fn save_stored_puzzle(db: &DB, stored_puzzle: &StoredPuzzle) -> Result<String, AppError> {
    let stored_puzzle_id = generate_stored_puzzle_id(db)?;
    let encoded = serde_json::to_vec(stored_puzzle)
        .map_err(|error| AppError::internal(format!("failed to encode stored puzzle: {error}")))?;

    db.put(stored_puzzle_db_key(&stored_puzzle_id), encoded)
        .map_err(|error| AppError::internal(format!("failed to save stored puzzle: {error}")))?;

    Ok(stored_puzzle_id)
}

fn map_stored_puzzle_validation_error(error: PuzzleValidationError) -> AppError {
    match error {
        PuzzleValidationError::DuplicateName(name) => {
            AppError::internal(format!("stored puzzle has duplicate name: {name}"))
        }
        other => AppError::internal(format!("stored puzzle is invalid: {other:?}")),
    }
}

fn map_editor_error(error: EditorError) -> AppError {
    match error.kind {
        EditorErrorKind::BadRequest => AppError::bad_request(error.message),
        EditorErrorKind::Conflict => AppError::conflict(error.message),
    }
}

fn replay_moves(puzzle: &mut Puzzle, moves: &[AppliedGuess]) -> Result<(), AppError> {
    for prior_move in moves {
        validate_and_reveal_guess(puzzle, prior_move.row, prior_move.col, prior_move.guess)?;
    }

    Ok(())
}

fn replay_moves_3d(puzzle: &mut Puzzle3D, moves: &[AppliedGuess3D]) -> Result<(), AppError> {
    for prior_move in moves {
        validate_and_reveal_guess_3d(
            puzzle,
            prior_move.layer,
            prior_move.row,
            prior_move.col,
            prior_move.guess,
        )?;
    }

    Ok(())
}

fn validate_and_reveal_guess(
    puzzle: &mut Puzzle,
    row: usize,
    col: usize,
    guess: Answer,
) -> Result<(), AppError> {
    {
        let row_cells = puzzle
            .cells
            .get(row)
            .ok_or_else(|| AppError::bad_request("row out of bounds"))?;
        let cell = row_cells
            .get(col)
            .ok_or_else(|| AppError::bad_request("col out of bounds"))?;

        if cell.state == Visibility::Revealed {
            return Err(AppError::bad_request("that cell is already revealed"));
        }
    }

    let analysis = analyze_revealed_puzzle(puzzle).map_err(|error| {
        AppError::internal(format!("failed to analyze revealed clues: {error:?}"))
    })?;

    if !analysis.has_solution {
        return Err(AppError::conflict("The revealed clues are inconsistent."));
    }

    let cell_name = puzzle.cells[row][col].name.clone();
    let forced = analysis.forced_answers[row][col];

    match (forced, guess) {
        (ForcedAnswer::Innocent, Answer::Innocent) | (ForcedAnswer::Criminal, Answer::Criminal) => {
        }
        (ForcedAnswer::Innocent, Answer::Criminal) => {
            return Err(AppError::conflict(format!(
                "{cell_name} is already forced to be innocent.",
            )));
        }
        (ForcedAnswer::Criminal, Answer::Innocent) => {
            return Err(AppError::conflict(format!(
                "{cell_name} is already forced to be criminal.",
            )));
        }
        (ForcedAnswer::Neither, _) => {
            return Err(AppError::conflict(format!(
                "{cell_name} is not forced by the revealed clues yet.",
            )));
        }
    }

    puzzle.cells[row][col].state = Visibility::Revealed;
    Ok(())
}

fn validate_and_reveal_guess_3d(
    puzzle: &mut Puzzle3D,
    layer: usize,
    row: usize,
    col: usize,
    guess: Answer,
) -> Result<(), AppError> {
    {
        let layer_cells = puzzle
            .cells
            .get(layer)
            .ok_or_else(|| AppError::bad_request("layer out of bounds"))?;
        let row_cells = layer_cells
            .get(row)
            .ok_or_else(|| AppError::bad_request("row out of bounds"))?;
        let cell = row_cells
            .get(col)
            .ok_or_else(|| AppError::bad_request("col out of bounds"))?;

        if cell.state == Visibility::Revealed {
            return Err(AppError::bad_request("that cell is already revealed"));
        }
    }

    let analysis = analyze_revealed_puzzle_3d(puzzle).map_err(|error| {
        AppError::internal(format!("failed to analyze revealed 3d clues: {error:?}"))
    })?;

    if !analysis.has_solution {
        return Err(AppError::conflict("The revealed clues are inconsistent."));
    }

    let cell_name = puzzle.cells[layer][row][col].name.clone();
    let forced = analysis.forced_answers[layer][row][col];

    match (forced, guess) {
        (ForcedAnswer::Innocent, Answer::Innocent) | (ForcedAnswer::Criminal, Answer::Criminal) => {
        }
        (ForcedAnswer::Innocent, Answer::Criminal) => {
            return Err(AppError::conflict(format!(
                "{cell_name} is already forced to be innocent.",
            )));
        }
        (ForcedAnswer::Criminal, Answer::Innocent) => {
            return Err(AppError::conflict(format!(
                "{cell_name} is already forced to be criminal.",
            )));
        }
        (ForcedAnswer::Neither, _) => {
            return Err(AppError::conflict(format!(
                "{cell_name} is not forced by the revealed clues yet.",
            )));
        }
    }

    puzzle.cells[layer][row][col].state = Visibility::Revealed;
    Ok(())
}

impl PuzzleResponse {
    fn from_generated(seed: u64, generated: &GeneratedPuzzle) -> Self {
        Self {
            seed: Some(format_seed(seed)),
            stored_puzzle_id: None,
            author: None,
            rows: generated.puzzle.cells.len() as u8,
            cols: generated
                .puzzle
                .cells
                .first()
                .map(|row| row.len())
                .unwrap_or_default() as u8,
            cells: cell_responses(&generated.puzzle, Some(&generated.clue_score_terms)),
            generated_score_series: generated.generation_score_series.clone(),
            generated_clue_texts: generated.generation_clue_texts.clone(),
        }
    }

    fn from_stored(stored_puzzle_id: &str, puzzle: &Puzzle) -> Self {
        Self {
            seed: None,
            stored_puzzle_id: Some(stored_puzzle_id.to_string()),
            author: puzzle.author.clone(),
            rows: puzzle.cells.len() as u8,
            cols: puzzle
                .cells
                .first()
                .map(|row| row.len())
                .unwrap_or_default() as u8,
            cells: cell_responses(puzzle, None),
            generated_score_series: Vec::new(),
            generated_clue_texts: Vec::new(),
        }
    }
}

impl Puzzle3DResponse {
    fn from_generated(seed: u64, generated: &GeneratedPuzzle3D) -> Self {
        Self {
            seed: Some(format_seed(seed)),
            stored_puzzle_id: None,
            author: None,
            depth: generated.puzzle.cells.len() as u8,
            rows: generated
                .puzzle
                .cells
                .first()
                .map(|layer| layer.len())
                .unwrap_or_default() as u8,
            cols: generated
                .puzzle
                .cells
                .first()
                .and_then(|layer| layer.first())
                .map(|row| row.len())
                .unwrap_or_default() as u8,
            cells: cell_responses_3d(&generated.puzzle),
        }
    }

    fn from_stored(stored_puzzle_id: &str, puzzle: &Puzzle3D) -> Self {
        Self {
            seed: None,
            stored_puzzle_id: Some(stored_puzzle_id.to_string()),
            author: puzzle.author.clone(),
            depth: puzzle.cells.len() as u8,
            rows: puzzle
                .cells
                .first()
                .map(|layer| layer.len())
                .unwrap_or_default() as u8,
            cols: puzzle
                .cells
                .first()
                .and_then(|layer| layer.first())
                .map(|row| row.len())
                .unwrap_or_default() as u8,
            cells: cell_responses_3d(puzzle),
        }
    }
}

fn cell_responses(
    puzzle: &Puzzle,
    clue_score_terms: Option<&[Vec<ClueScoreTerms>]>,
) -> Vec<Vec<CellResponse>> {
    puzzle
        .cells
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            row.iter()
                .enumerate()
                .map(|(col_index, cell)| CellResponse {
                    name: cell.name.clone(),
                    role: cell.role.clone(),
                    emoji: cell.emoji.clone(),
                    clue: if cell.state == Visibility::Revealed {
                        Some(cell.clue.text())
                    } else {
                        None
                    },
                    clue_highlight_groups: Vec::new(),
                    is_nonsense: cell.state == Visibility::Revealed
                        && matches!(cell.clue, clues_core::Clue::Nonsense { .. }),
                    score_terms: if cell.state == Visibility::Revealed {
                        clue_score_terms.map(|rows| rows[row_index][col_index].clone())
                    } else {
                        None
                    },
                    revealed_answer: if cell.state == Visibility::Revealed {
                        Some(cell.answer)
                    } else {
                        None
                    },
                    revealed: cell.state == Visibility::Revealed,
                })
                .collect()
        })
        .collect()
}

fn cell_responses_3d(puzzle: &Puzzle3D) -> Vec<Vec<Vec<CellResponse>>> {
    puzzle
        .cells
        .iter()
        .map(|layer| {
            layer
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| CellResponse {
                            name: cell.name.clone(),
                            role: cell.role.clone(),
                            emoji: cell.emoji.clone(),
                            clue: if cell.state == Visibility::Revealed {
                                Some(cell.clue.text())
                            } else {
                                None
                            },
                            clue_highlight_groups: if cell.state == Visibility::Revealed {
                                clue_highlight_groups_3d(puzzle, &cell.clue)
                            } else {
                                Vec::new()
                            },
                            is_nonsense: cell.state == Visibility::Revealed
                                && matches!(cell.clue, clues_core::Clue::Nonsense { .. }),
                            score_terms: None,
                            revealed_answer: if cell.state == Visibility::Revealed {
                                Some(cell.answer)
                            } else {
                                None
                            },
                            revealed: cell.state == Visibility::Revealed,
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

fn push_highlight_line(lines: &mut Vec<Line>, line: Line) {
    if !lines.contains(&line) {
        lines.push(line);
    }
}

fn collect_selector_highlight_lines(lines: &mut Vec<Line>, selector: &CellSelector) {
    match selector {
        CellSelector::Layer { layer } => push_highlight_line(lines, Line::Layer(*layer)),
        CellSelector::Row { row } => push_highlight_line(lines, Line::Row(*row)),
        CellSelector::Col { col } => push_highlight_line(lines, Line::Col(*col)),
        CellSelector::Board
        | CellSelector::Neighbor { .. }
        | CellSelector::Direction { .. }
        | CellSelector::Between { .. }
        | CellSelector::SharedNeighbor { .. } => {}
    }
}

fn collect_filter_highlight_lines(lines: &mut Vec<Line>, filter: &CellFilter) {
    if let CellFilter::Line(line) = filter {
        push_highlight_line(lines, *line);
    }
}

fn collect_group_highlight_lines(lines: &mut Vec<Line>, group: &PersonGroup) {
    match group {
        PersonGroup::Line { line } => push_highlight_line(lines, *line),
        PersonGroup::Filter { filter } => collect_filter_highlight_lines(lines, filter),
        PersonGroup::SelectedCells {
            selector, filter, ..
        } => {
            collect_selector_highlight_lines(lines, selector);
            collect_filter_highlight_lines(lines, filter);
        }
        PersonGroup::Any | PersonGroup::Role { .. } => {}
    }
}

fn collect_predicate_highlight_lines(lines: &mut Vec<Line>, predicate: &PersonPredicate) {
    if let PersonPredicate::Neighbor { filter, .. } = predicate {
        collect_filter_highlight_lines(lines, filter);
    }
}

fn line_cells_3d(puzzle: &Puzzle3D, line: Line) -> Vec<HighlightCell3DResponse> {
    match line {
        Line::Layer(layer) => puzzle
            .cells
            .get(layer as usize)
            .into_iter()
            .flat_map(|layer_cells| {
                layer_cells.iter().enumerate().flat_map(move |(row_index, row)| {
                    row.iter().enumerate().map(move |(col_index, _)| HighlightCell3DResponse {
                        layer: layer as usize,
                        row: row_index,
                        col: col_index,
                    })
                })
            })
            .collect(),
        Line::Row(row) => puzzle
            .cells
            .iter()
            .enumerate()
            .flat_map(move |(layer_index, layer_cells)| {
                layer_cells
                    .get(row as usize)
                    .into_iter()
                    .flat_map(move |row_cells| {
                        row_cells.iter().enumerate().map(move |(col_index, _)| {
                            HighlightCell3DResponse {
                                layer: layer_index,
                                row: row as usize,
                                col: col_index,
                            }
                        })
                    })
            })
            .collect(),
        Line::Col(col) => puzzle
            .cells
            .iter()
            .enumerate()
            .flat_map(move |(layer_index, layer_cells)| {
                layer_cells.iter().enumerate().filter_map(move |(row_index, row_cells)| {
                    row_cells
                        .get(col.index() as usize)
                        .map(|_| HighlightCell3DResponse {
                            layer: layer_index,
                            row: row_index,
                            col: col.index() as usize,
                        })
                })
            })
            .collect(),
    }
}

fn clue_highlight_groups_3d(puzzle: &Puzzle3D, clue: &Clue) -> Vec<Vec<HighlightCell3DResponse>> {
    let mut lines = Vec::new();

    match clue {
        Clue::CountCells {
            selector, filter, ..
        }
        | Clue::NamedCountCells {
            selector, filter, ..
        } => {
            collect_selector_highlight_lines(&mut lines, selector);
            collect_filter_highlight_lines(&mut lines, filter);
        }
        Clue::Connected { line, .. } => {
            push_highlight_line(&mut lines, *line);
        }
        Clue::LineComparison {
            first_line,
            second_line,
            ..
        } => {
            push_highlight_line(&mut lines, *first_line);
            push_highlight_line(&mut lines, *second_line);
        }
        Clue::Quantified {
            group, predicate, ..
        } => {
            collect_group_highlight_lines(&mut lines, group);
            collect_predicate_highlight_lines(&mut lines, predicate);
        }
        Clue::Nonsense { .. }
        | Clue::Declaration { .. }
        | Clue::DirectRelation { .. }
        | Clue::RoleCount { .. }
        | Clue::RolesComparison { .. } => {}
    }

    lines
        .into_iter()
        .map(|line| line_cells_3d(puzzle, line))
        .filter(|group| !group.is_empty())
        .collect()
}

fn parse_board_shape(rows: Option<u8>, cols: Option<u8>) -> Result<BoardShape, AppError> {
    let rows = rows.unwrap_or(DEFAULT_ROWS);
    let cols = cols.unwrap_or(DEFAULT_COLS);

    if rows == 0 || cols == 0 {
        return Err(AppError::bad_request("rows and cols must be at least 1"));
    }

    let cell_count = rows as usize * cols as usize;
    if cell_count > MAX_PUBLIC_CELL_COUNT {
        return Err(AppError::bad_request(format!(
            "rows * cols must be at most {MAX_PUBLIC_CELL_COUNT}",
        )));
    }

    Ok(BoardShape::new(rows, cols))
}

fn parse_board_shape_3d(
    depth: Option<u8>,
    rows: Option<u8>,
    cols: Option<u8>,
) -> Result<BoardShape, AppError> {
    let depth = depth.unwrap_or(2);
    let rows = rows.unwrap_or(2);
    let cols = cols.unwrap_or(2);

    if depth == 0 || rows == 0 || cols == 0 {
        return Err(AppError::bad_request("depth, rows, and cols must be at least 1"));
    }

    let cell_count = depth as usize * rows as usize * cols as usize;
    if cell_count > MAX_PUBLIC_CELL_COUNT {
        return Err(AppError::bad_request(format!(
            "depth * rows * cols must be at most {MAX_PUBLIC_CELL_COUNT}",
        )));
    }

    Ok(BoardShape::new_3d(depth, rows, cols))
}

fn parse_seed(value: &str) -> Result<u64, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 12 || !trimmed.chars().all(|ch| ch.is_ascii_hexdigit())
    {
        return Err(AppError::bad_request(
            "seed must be 1 to 12 hexadecimal characters",
        ));
    }

    u64::from_str_radix(trimmed, 16)
        .map(|seed| seed & SEED_MASK)
        .map_err(|_| AppError::bad_request("seed must be 1 to 12 hexadecimal characters"))
}

fn format_seed(seed: u64) -> String {
    format!("{:012x}", seed & SEED_MASK)
}

fn format_stored_puzzle_id(value: u32) -> String {
    format!("{:06x}", value & STORED_PUZZLE_ID_MASK)
}

fn stored_puzzle_db_key(stored_puzzle_id: &str) -> Vec<u8> {
    format!("{STORED_PUZZLE_DB_PREFIX}{stored_puzzle_id}").into_bytes()
}

fn generate_stored_puzzle_id(db: &DB) -> Result<String, AppError> {
    for _ in 0..16 {
        let candidate = format_stored_puzzle_id(random::<u32>());
        let exists = db
            .get_pinned(stored_puzzle_db_key(&candidate))
            .map_err(|error| {
                AppError::internal(format!("failed to check stored puzzle id: {error}"))
            })?
            .is_some();

        if !exists {
            return Ok(candidate);
        }
    }

    Err(AppError::internal("failed to allocate stored puzzle id"))
}

fn open_stored_puzzle_db() -> Result<DB, String> {
    let path = stored_puzzle_db_path();
    fs::create_dir_all(&path)
        .map_err(|error| format!("failed to create db directory {}: {error}", path.display()))?;

    let mut options = Options::default();
    options.create_if_missing(true);
    DB::open(&options, &path)
        .map_err(|error| format!("failed to open rocksdb at {}: {error}", path.display()))
}

fn stored_puzzle_db_path() -> PathBuf {
    std::env::var_os("CLUES_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("data")
                .join("rocksdb")
        })
}

struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}
