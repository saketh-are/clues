use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use clues_core::{
    Answer, ForcedAnswer, Puzzle, Visibility, analyze_revealed_puzzle, generate_puzzle_with_seed,
};
use rand::random;
use serde::{Deserialize, Serialize};
use tower_http::services::{ServeDir, ServeFile};

const SEED_MASK: u64 = 0xFFFF_FFFF_FFFF;

#[derive(Clone)]
struct AppState {
    next_id: Arc<AtomicU64>,
    puzzles: Arc<Mutex<HashMap<u64, Puzzle>>>,
}

#[derive(Debug, Deserialize)]
struct NewPuzzleParams {
    seed: Option<String>,
}

#[derive(Debug, Serialize)]
struct PuzzleResponse {
    id: u64,
    seed: String,
    cells: Vec<Vec<CellResponse>>,
}

#[derive(Debug, Serialize)]
struct CellResponse {
    name: String,
    role: String,
    clue: Option<String>,
    is_nonsense: bool,
    revealed_answer: Option<Answer>,
    revealed: bool,
}

#[derive(Debug, Serialize)]
struct GuessResponse {
    clue: String,
    is_nonsense: bool,
}

#[derive(Debug, Deserialize)]
struct GuessRequest {
    guess: Answer,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        next_id: Arc::new(AtomicU64::new(1)),
        puzzles: Arc::new(Mutex::new(HashMap::new())),
    };

    let static_dir = format!("{}/static", env!("CARGO_MANIFEST_DIR"));
    let app = Router::new()
        .route("/api/puzzles/new", get(new_puzzle))
        .route(
            "/api/puzzles/{id}/cells/{row}/{col}/guess",
            post(guess_cell),
        )
        .fallback_service(
            ServeDir::new(static_dir).not_found_service(ServeFile::new(format!(
                "{}/static/index.html",
                env!("CARGO_MANIFEST_DIR")
            ))),
        )
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind localhost server");

    println!("serving on http://127.0.0.1:3000");
    axum::serve(listener, app).await.expect("run server");
}

async fn new_puzzle(
    State(state): State<AppState>,
    Query(params): Query<NewPuzzleParams>,
) -> Result<Json<PuzzleResponse>, AppError> {
    let seed = match params.seed {
        Some(seed) => parse_seed(&seed)?,
        None => random::<u64>() & SEED_MASK,
    };
    let generated = generate_puzzle_with_seed(seed)
        .map_err(|error| AppError::internal(format!("failed to generate puzzle: {error:?}")))?;
    let id = state.next_id.fetch_add(1, Ordering::Relaxed);
    let response = PuzzleResponse::from_puzzle(id, seed, &generated.puzzle);

    state
        .puzzles
        .lock()
        .map_err(|_| AppError::internal("failed to lock puzzle store"))?
        .insert(id, generated.puzzle);

    Ok(Json(response))
}

async fn guess_cell(
    State(state): State<AppState>,
    Path((id, row, col)): Path<(u64, usize, usize)>,
    Json(request): Json<GuessRequest>,
) -> Result<Json<GuessResponse>, AppError> {
    let mut puzzles = state
        .puzzles
        .lock()
        .map_err(|_| AppError::internal("failed to lock puzzle store"))?;
    let puzzle = puzzles
        .get_mut(&id)
        .ok_or_else(|| AppError::not_found("puzzle not found"))?;

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

    match (forced, request.guess) {
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

    Ok(Json(GuessResponse {
        clue: puzzle.cells[row][col].clue.text(),
        is_nonsense: matches!(puzzle.cells[row][col].clue, clues_core::Clue::Nonsense { .. }),
    }))
}

impl PuzzleResponse {
    fn from_puzzle(id: u64, seed: u64, puzzle: &Puzzle) -> Self {
        let cells = puzzle
            .cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| CellResponse {
                        name: cell.name.clone(),
                        role: cell.role.clone(),
                        clue: if cell.state == Visibility::Revealed {
                            Some(cell.clue.text())
                        } else {
                            None
                        },
                        is_nonsense: cell.state == Visibility::Revealed
                            && matches!(cell.clue, clues_core::Clue::Nonsense { .. }),
                        revealed_answer: if cell.state == Visibility::Revealed {
                            Some(cell.answer)
                        } else {
                            None
                        },
                        revealed: cell.state == Visibility::Revealed,
                    })
                    .collect()
            })
            .collect();

        Self {
            id,
            seed: format_seed(seed),
            cells,
        }
    }
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
