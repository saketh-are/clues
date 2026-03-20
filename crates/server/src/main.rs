use std::{fs, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use clues_core::{
    Answer, BoardShape, ClueScoreTerms, DEFAULT_COLS, DEFAULT_ROWS, ForcedAnswer, GeneratedPuzzle,
    Puzzle, PuzzleValidationError, StoredPuzzle, Visibility, analyze_revealed_puzzle,
    generate_puzzle_with_seed_and_size,
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
    rows: Option<u8>,
    cols: Option<u8>,
}

#[derive(Debug, Serialize)]
struct PuzzleResponse {
    seed: Option<String>,
    stored_puzzle_id: Option<String>,
    rows: u8,
    cols: u8,
    cells: Vec<Vec<CellResponse>>,
    generated_score_series: Vec<ClueScoreTerms>,
    generated_clue_texts: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CellResponse {
    name: String,
    role: String,
    clue: Option<String>,
    is_nonsense: bool,
    score_terms: Option<ClueScoreTerms>,
    revealed_answer: Option<Answer>,
    revealed: bool,
}

#[derive(Debug, Serialize)]
struct GuessResponse {
    clue: String,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PuzzleSource {
    Generated { seed: String, rows: u8, cols: u8 },
    Stored { stored_puzzle_id: String },
}

#[derive(Debug, Clone, Deserialize)]
struct AppliedGuess {
    row: usize,
    col: usize,
    guess: Answer,
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
        .route("/p/{stored_puzzle_id}", get(index_html))
        .route("/api/puzzles/new", get(new_puzzle))
        .route("/api/puzzles/guess", post(guess_cell))
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

async fn index_html() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn new_puzzle(
    Query(params): Query<NewPuzzleParams>,
) -> Result<Json<PuzzleResponse>, AppError> {
    let (seed, generated) =
        generate_requested_puzzle(params.seed.as_deref(), params.rows, params.cols)?;
    Ok(Json(PuzzleResponse::from_generated(seed, &generated)))
}

async fn create_stored_puzzle(
    State(state): State<AppState>,
    Query(params): Query<NewPuzzleParams>,
) -> Result<Json<CreateStoredPuzzleResponse>, AppError> {
    let (_, generated) =
        generate_requested_puzzle(params.seed.as_deref(), params.rows, params.cols)?;
    let stored_puzzle = StoredPuzzle::from(&generated.puzzle);
    let stored_puzzle_id = generate_stored_puzzle_id(&state.stored_puzzles)?;
    let encoded = serde_json::to_vec(&stored_puzzle)
        .map_err(|error| AppError::internal(format!("failed to encode stored puzzle: {error}")))?;

    state
        .stored_puzzles
        .put(stored_puzzle_db_key(&stored_puzzle_id), encoded)
        .map_err(|error| AppError::internal(format!("failed to save stored puzzle: {error}")))?;

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
        is_nonsense: matches!(
            puzzle.cells[row][col].clue,
            clues_core::Clue::Nonsense { .. }
        ),
        score_terms,
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

fn load_stored_puzzle_by_id(state: &AppState, stored_puzzle_id: &str) -> Result<Puzzle, AppError> {
    let encoded = state
        .stored_puzzles
        .get_pinned(stored_puzzle_db_key(stored_puzzle_id))
        .map_err(|error| AppError::internal(format!("failed to read stored puzzle: {error}")))?
        .ok_or_else(|| AppError::not_found("stored puzzle not found"))?;
    let stored_puzzle: StoredPuzzle = serde_json::from_slice(&encoded).map_err(|error| {
        AppError::internal(format!(
            "failed to decode stored puzzle {stored_puzzle_id}: {error}"
        ))
    })?;
    let puzzle = match stored_puzzle {
        StoredPuzzle::V1(puzzle) => puzzle,
    };

    puzzle
        .validate()
        .map_err(map_stored_puzzle_validation_error)?;
    Ok(puzzle)
}

fn map_stored_puzzle_validation_error(error: PuzzleValidationError) -> AppError {
    match error {
        PuzzleValidationError::DuplicateName(name) => {
            AppError::internal(format!("stored puzzle has duplicate name: {name}"))
        }
        other => AppError::internal(format!("stored puzzle is invalid: {other:?}")),
    }
}

fn replay_moves(puzzle: &mut Puzzle, moves: &[AppliedGuess]) -> Result<(), AppError> {
    for prior_move in moves {
        validate_and_reveal_guess(puzzle, prior_move.row, prior_move.col, prior_move.guess)?;
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

impl PuzzleResponse {
    fn from_generated(seed: u64, generated: &GeneratedPuzzle) -> Self {
        Self {
            seed: Some(format_seed(seed)),
            stored_puzzle_id: None,
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
                    clue: if cell.state == Visibility::Revealed {
                        Some(cell.clue.text())
                    } else {
                        None
                    },
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
