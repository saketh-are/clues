use std::{env, fs, path::Path};

use clues_core::StoredPuzzle;
use rocksdb::{DB, Options};

const STORED_PUZZLE_DB_PREFIX: &str = "stored_puzzle:";

fn open_db(path: &Path) -> Result<DB, String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("failed to create db directory {}: {error}", path.display()))?;

    let mut options = Options::default();
    options.create_if_missing(true);
    DB::open(&options, path)
        .map_err(|error| format!("failed to open rocksdb at {}: {error}", path.display()))
}

fn db_key(stored_puzzle_id: &str) -> Vec<u8> {
    format!("{STORED_PUZZLE_DB_PREFIX}{stored_puzzle_id}").into_bytes()
}

fn export_json(db_path: &Path, stored_puzzle_id: &str, output_path: &Path) -> Result<(), String> {
    let db = open_db(db_path)?;
    let encoded = db
        .get_pinned(db_key(stored_puzzle_id))
        .map_err(|error| format!("failed to read stored puzzle: {error}"))?
        .ok_or_else(|| format!("stored puzzle {stored_puzzle_id} not found"))?;
    let stored_puzzle: StoredPuzzle = serde_json::from_slice(&encoded)
        .map_err(|error| format!("invalid stored puzzle: {error}"))?;
    let pretty = serde_json::to_vec_pretty(&stored_puzzle)
        .map_err(|error| format!("failed to encode stored puzzle json: {error}"))?;
    fs::write(output_path, pretty)
        .map_err(|error| format!("failed to write {}: {error}", output_path.display()))?;
    Ok(())
}

fn import_json(db_path: &Path, stored_puzzle_id: &str, input_path: &Path) -> Result<(), String> {
    let db = open_db(db_path)?;
    let json = fs::read(input_path)
        .map_err(|error| format!("failed to read {}: {error}", input_path.display()))?;
    let stored_puzzle: StoredPuzzle = serde_json::from_slice(&json)
        .map_err(|error| format!("invalid stored puzzle json: {error}"))?;
    let encoded = serde_json::to_vec(&stored_puzzle)
        .map_err(|error| format!("failed to encode json: {error}"))?;
    db.put(db_key(stored_puzzle_id), encoded)
        .map_err(|error| format!("failed to write stored puzzle: {error}"))?;
    Ok(())
}

fn main() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(
            "usage: db_copy <export-json|import-json> <db_path> <stored_puzzle_id> <path>"
                .to_string(),
        );
    }

    let command = &args[1];
    let db_path = Path::new(&args[2]);
    let stored_puzzle_id = &args[3];
    let path = Path::new(&args[4]);

    match command.as_str() {
        "export-json" => export_json(db_path, stored_puzzle_id, path),
        "import-json" => import_json(db_path, stored_puzzle_id, path),
        _ => Err(format!("unknown command: {command}")),
    }
}
