const boardEl = document.querySelector("#board");
const template = document.querySelector("#cell-template");
const newRandomButton = document.querySelector("#new-random");
const shareButton = document.querySelector("#share-puzzle");
const clearButton = document.querySelector("#clear-board");
const guessModalEl = document.querySelector("#guess-modal");
const guessBackdropEl = document.querySelector("#guess-backdrop");
const guessEmojiEl = document.querySelector("#guess-emoji");
const guessTitleEl = document.querySelector("#guess-title");
const guessInnocentButton = document.querySelector("#guess-innocent");
const guessCriminalButton = document.querySelector("#guess-criminal");
const guessCancelButton = document.querySelector("#guess-cancel");
const errorModalEl = document.querySelector("#error-modal");
const errorBackdropEl = document.querySelector("#error-backdrop");
const errorTitleEl = document.querySelector("#error-title");
const errorMessageEl = document.querySelector("#error-message");
const errorDismissButton = document.querySelector("#error-dismiss");
const startModalEl = document.querySelector("#start-modal");
const startButton = document.querySelector("#start-button");
const finishModalEl = document.querySelector("#finish-modal");
const finishBackdropEl = document.querySelector("#finish-backdrop");
const finishMessageEl = document.querySelector("#finish-message");
const finishDismissButton = document.querySelector("#finish-dismiss");
const progressStoragePrefix = "clues-progress:v1:";
const noteTapDelayMs = 420;
const noteColors = new Set(["yellow", "red", "green"]);
const invalidMoveMessage = "⚠️ Not enough evidence!";
const roleEmojis = {
  Artist: "🧑‍🎨",
  Builder: "👷",
  Cook: "🧑‍🍳",
  Detective: "🕵️",
  Doctor: "🧑‍⚕️",
  Farmer: "🧑‍🌾",
  Firefighter: "🧑‍🚒",
  Guard: "💂",
  Judge: "🧑‍⚖️",
  Mechanic: "🧑‍🔧",
  Nurse: "🧑‍⚕️",
  Pilot: "🧑‍✈️",
  "Police Officer": "👮",
  Scientist: "🧑‍🔬",
  Singer: "🧑‍🎤",
  Teacher: "🧑‍🏫",
  Technologist: "🧑‍💻",
};

function emojiForRole(role) {
  return roleEmojis[role] ?? "🧑";
}

const rowLabels = ["1", "2", "3", "4", "5"];
const colLabels = ["A", "B", "C", "D"];

const state = {
  puzzleId: null,
  currentSeed: null,
  cells: [],
  guesses: [],
  moves: [],
  notes: new Map(),
  loadingClues: new Set(),
  modalCell: null,
  hiddenClues: new Set(),
  flashingTiles: new Set(),
  flashTimer: null,
  pendingNoteTap: null,
  shareFeedbackTimer: null,
  timerStartedAt: null,
  timerCompletedAt: null,
  completionAcknowledged: false,
};

function normalizeSeed(value) {
  if (value === undefined || value === null || value === "") {
    return null;
  }

  const normalized = String(value).trim().toLowerCase();
  if (normalized.length === 0 || normalized.length > 12) {
    return null;
  }

  if (!/^[0-9a-f]+$/.test(normalized)) {
    return null;
  }

  return normalized.padStart(12, "0");
}

function seedFromUrl() {
  return normalizeSeed(new URL(window.location.href).searchParams.get("seed"));
}

function updateUrlSeed(seed) {
  const url = new URL(window.location.href);
  url.searchParams.set("seed", String(seed));
  window.history.replaceState({}, "", url);
}

function progressStorageKey(seed) {
  return `${progressStoragePrefix}${seed}`;
}

function readSavedProgress(seed) {
  if (seed === null) {
    return null;
  }

  try {
    const raw = window.localStorage.getItem(progressStorageKey(seed));
    if (!raw) {
      return null;
    }

    const parsed = JSON.parse(raw);
    const moves = Array.isArray(parsed?.moves)
      ? parsed.moves.filter(
          (move) =>
            Number.isInteger(move?.row) &&
            Number.isInteger(move?.col) &&
            (move?.guess === "innocent" || move?.guess === "criminal"),
        )
      : [];
    const hiddenClues = Array.isArray(parsed?.hiddenClues)
      ? parsed.hiddenClues.filter((key) => typeof key === "string")
      : [];
    const notes = Array.isArray(parsed?.notes)
      ? parsed.notes.filter(
          (entry) =>
            Array.isArray(entry) &&
            entry.length === 2 &&
            typeof entry[0] === "string" &&
            noteColors.has(entry[1]),
        )
      : [];
    const timerStartedAt =
      Number.isFinite(parsed?.timerStartedAt) && parsed.timerStartedAt >= 0
        ? parsed.timerStartedAt
        : null;
    const timerCompletedAt =
      Number.isFinite(parsed?.timerCompletedAt) && parsed.timerCompletedAt >= 0
        ? parsed.timerCompletedAt
        : null;
    const completionAcknowledged = parsed?.completionAcknowledged === true;

    return { moves, hiddenClues, notes, timerStartedAt, timerCompletedAt, completionAcknowledged };
  } catch {
    return null;
  }
}

function clearSavedProgress(seed = state.currentSeed) {
  if (seed === null) {
    return;
  }

  window.localStorage.removeItem(progressStorageKey(seed));
}

function persistProgress() {
  if (state.currentSeed === null) {
    return;
  }

  const hiddenClues = [...state.hiddenClues];
  const notes = [...state.notes.entries()];
  const hasTimerState =
    state.timerStartedAt !== null ||
    state.timerCompletedAt !== null ||
    state.completionAcknowledged;
  if (state.moves.length === 0 && hiddenClues.length === 0 && notes.length === 0 && !hasTimerState) {
    clearSavedProgress(state.currentSeed);
    return;
  }

  window.localStorage.setItem(
    progressStorageKey(state.currentSeed),
    JSON.stringify({
      moves: state.moves,
      hiddenClues,
      notes,
      timerStartedAt: state.timerStartedAt,
      timerCompletedAt: state.timerCompletedAt,
      completionAcknowledged: state.completionAcknowledged,
    }),
  );
}

function answerLabel(answer) {
  return answer === "innocent" ? "Innocent" : "Criminal";
}

function shortAnswerLabel(answer) {
  return answer === "innocent" ? "I" : "C";
}

function guessKey(row, col) {
  return `${row}:${col}`;
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function roleVariants(role) {
  const singular = role.toLowerCase();

  if (
    singular.endsWith("ch") ||
    singular.endsWith("sh") ||
    singular.endsWith("s") ||
    singular.endsWith("x") ||
    singular.endsWith("z")
  ) {
    return [singular, `${singular}es`];
  }

  return [singular, `${singular}s`];
}

function mentionsText(text, needle) {
  return new RegExp(`\\b${escapeRegex(needle)}\\b`, "i").test(text);
}

function mentionedTileKeys(clue) {
  const keys = new Set();
  const lowerClue = clue.toLowerCase();

  state.cells.forEach((row, rowIndex) => {
    row.forEach((cell, colIndex) => {
      if (mentionsText(clue, cell.name)) {
        keys.add(guessKey(rowIndex, colIndex));
      }

      if (roleVariants(cell.role).some((variant) => mentionsText(lowerClue, variant))) {
        keys.add(guessKey(rowIndex, colIndex));
      }
    });
  });

  return keys;
}

function flashMentionedTiles(clue) {
  const keys = mentionedTileKeys(clue);
  if (keys.size === 0) {
    return;
  }

  state.flashingTiles = keys;
  window.clearTimeout(state.flashTimer);
  state.flashTimer = window.setTimeout(() => {
    state.flashingTiles = new Set();
    state.flashTimer = null;
    renderBoard();
  }, 900);
  renderBoard();
}

function allTilesMarked() {
  return state.guesses.every((row) => row.every((guess) => guess !== null));
}

function formatElapsedDuration(milliseconds) {
  const totalSeconds = Math.max(0, Math.round(milliseconds / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}h ${String(minutes).padStart(2, "0")}m ${String(seconds).padStart(2, "0")}s`;
  }

  if (minutes > 0) {
    return `${minutes}m ${String(seconds).padStart(2, "0")}s`;
  }

  return `${seconds}s`;
}

function openFinishModal() {
  if (state.timerStartedAt === null) {
    return;
  }

  closeGuessModal();
  closeErrorModal();
  closeStartModal();
  const completedAt = state.timerCompletedAt ?? Date.now();
  const elapsed = formatElapsedDuration(completedAt - state.timerStartedAt);
  finishMessageEl.textContent = `You finished in ${elapsed}.`;
  finishModalEl.hidden = false;
}

function closeFinishModal() {
  finishModalEl.hidden = true;
}

function dismissFinishModal() {
  if (state.timerCompletedAt !== null) {
    state.completionAcknowledged = true;
    persistProgress();
  }

  closeFinishModal();
}

function startPuzzle() {
  if (state.timerStartedAt === null) {
    state.timerStartedAt = Date.now();
    state.timerCompletedAt = null;
    state.completionAcknowledged = false;
    persistProgress();
  }

  closeStartModal();
}

function completePuzzleIfNeeded() {
  if (!allTilesMarked() || state.timerStartedAt === null) {
    return;
  }

  if (state.timerCompletedAt === null) {
    state.timerCompletedAt = Date.now();
  }

  state.completionAcknowledged = false;
  persistProgress();
  openFinishModal();
}

async function loadPuzzle(seed, options = {}) {
  const { forceStartModal = false } = options;
  const normalizedSeed = normalizeSeed(seed);
  const query = normalizedSeed === null ? "" : `?seed=${encodeURIComponent(normalizedSeed)}`;

  const response = await fetch(`/api/puzzles/new${query}`);
  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: "Failed to load puzzle" }));
    throw new Error(error.error || "Failed to load puzzle");
  }

  const puzzle = await response.json();
  state.puzzleId = puzzle.id;
  state.currentSeed = puzzle.seed;
  state.cells = puzzle.cells;
  state.guesses = puzzle.cells.map((row) =>
    row.map((cell) => (cell.revealed_answer ? cell.revealed_answer : null)),
  );
  state.moves = [];
  state.notes = new Map();
  state.loadingClues = new Set();
  state.modalCell = null;
  state.hiddenClues = new Set();
  state.flashingTiles = new Set();
  state.timerStartedAt = null;
  state.timerCompletedAt = null;
  state.completionAcknowledged = false;
  clearPendingNoteTap();
  window.clearTimeout(state.flashTimer);
  state.flashTimer = null;

  updateUrlSeed(puzzle.seed);
  resetShareButton();
  closeGuessModal();
  closeErrorModal();
  closeStartModal();
  closeFinishModal();
  const restoreResult = await restoreProgress();
  renderBoard();
  if (restoreResult.error) {
    openErrorModal(restoreResult.error);
    return;
  }

  if (allTilesMarked() && state.timerStartedAt !== null && !state.completionAcknowledged) {
    openFinishModal();
    return;
  }

  if (forceStartModal || state.timerStartedAt === null) {
    openStartModal();
  }
}

function renderBoard() {
  boardEl.innerHTML = "";

  state.cells.forEach((row, rowIndex) => {
    row.forEach((cell, colIndex) => {
      const fragment = template.content.cloneNode(true);
      const card = fragment.querySelector(".cell");
      const positionEl = fragment.querySelector(".cell-position");
      const emojiEl = fragment.querySelector(".cell-emoji");
      const nameEl = fragment.querySelector(".cell-name");
      const roleEl = fragment.querySelector(".cell-role");
      const clueEl = fragment.querySelector(".cell-clue");
      const noteEl = fragment.querySelector(".cell-note");
      const key = guessKey(rowIndex, colIndex);
      const guess = state.guesses[rowIndex][colIndex];
      const clueHidden = state.hiddenClues.has(key);
      const note = state.notes.get(key) ?? "none";

      positionEl.textContent = `${rowLabels[rowIndex]}${colLabels[colIndex]}`;
      emojiEl.textContent = emojiForRole(cell.role);
      nameEl.textContent = cell.name;
      roleEl.textContent = cell.role.toLowerCase();
      card.classList.add(cell.clue ? "has-clue" : "hidden-clue");
      noteEl.classList.add(`note-${note}`);
      noteEl.setAttribute("aria-label", note === "none" ? "Add tile note" : `Clear ${note} note`);
      noteEl.addEventListener("click", (event) => {
        event.stopPropagation();
        handleNoteTap(rowIndex, colIndex);
      });

      if (cell.revealed && cell.revealed_answer) {
        card.classList.add(`tile-${cell.revealed_answer}`);
      } else if (guess) {
        card.classList.add(`tile-${guess}`);
      }

      if (state.loadingClues.has(key)) {
        card.classList.add("loading");
      }

      if (state.flashingTiles.has(key)) {
        card.classList.add("flash");
      }

      if (cell.clue) {
        clueEl.textContent = cell.clue;
        clueEl.classList.remove("placeholder");
        clueEl.classList.toggle("clue-hidden", clueHidden);
      } else {
        clueEl.textContent = "";
        clueEl.classList.add("placeholder");
        clueEl.classList.remove("clue-hidden");
      }

      if (cell.clue) {
        card.classList.add("clickable");
        card.addEventListener("click", () => {
          toggleClueVisibility(rowIndex, colIndex);
        });
      } else if (!cell.revealed) {
        card.classList.add("clickable");
        card.addEventListener("click", () => {
          openGuessModal(rowIndex, colIndex);
        });
      }

      boardEl.appendChild(fragment);
    });
  });
}

async function fetchValidatedClue(row, col, guess) {
  const response = await fetch(`/api/puzzles/${state.puzzleId}/cells/${row}/${col}/guess`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      guess,
    }),
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: "Failed to validate mark" }));
    const result = new Error(error.error || "Failed to validate mark");
    result.status = response.status;
    result.guess = guess;
    throw result;
  }

  const result = await response.json();
  return result.clue;
}

function applyAcceptedGuess(row, col, guess, clue) {
  state.guesses[row][col] = guess;
  state.cells[row][col].clue = clue;
  state.hiddenClues.delete(guessKey(row, col));
  state.moves = state.moves.filter((move) => move.row !== row || move.col !== col);
  state.moves.push({ row, col, guess });
}

function clearPendingNoteTap() {
  if (!state.pendingNoteTap) {
    return;
  }

  window.clearTimeout(state.pendingNoteTap.timerId);
  state.pendingNoteTap = null;
}

function setNote(row, col, color) {
  const key = guessKey(row, col);

  if (color === null) {
    state.notes.delete(key);
  } else {
    state.notes.set(key, color);
  }

  persistProgress();
  renderBoard();
}

function noteColorForTapCount(count) {
  if (count >= 3) {
    return "green";
  }

  if (count === 2) {
    return "red";
  }

  return "yellow";
}

function handleNoteTap(row, col) {
  const key = guessKey(row, col);
  const current = state.notes.get(key);

  if (state.pendingNoteTap && state.pendingNoteTap.key === key) {
    state.pendingNoteTap.count = Math.min(state.pendingNoteTap.count + 1, 3);
    window.clearTimeout(state.pendingNoteTap.timerId);
    state.pendingNoteTap.timerId = window.setTimeout(() => {
      state.pendingNoteTap = null;
    }, noteTapDelayMs);
    setNote(row, col, noteColorForTapCount(state.pendingNoteTap.count));
    return;
  }

  if (current) {
    clearPendingNoteTap();
    setNote(row, col, null);
    return;
  }

  clearPendingNoteTap();
  state.pendingNoteTap = {
    key,
    count: 1,
    timerId: window.setTimeout(() => {
      state.pendingNoteTap = null;
    }, noteTapDelayMs),
  };
  setNote(row, col, noteColorForTapCount(1));
}

async function restoreProgress() {
  const saved = readSavedProgress(state.currentSeed);
  if (!saved) {
    return { restored: false, error: null };
  }

  state.timerStartedAt = saved.timerStartedAt;
  state.timerCompletedAt = saved.timerCompletedAt;
  state.completionAcknowledged = saved.completionAcknowledged;
  if (
    state.timerStartedAt === null &&
    (saved.moves.length > 0 || saved.hiddenClues.length > 0 || saved.notes.length > 0)
  ) {
    state.timerStartedAt = Date.now();
  }

  try {
    for (const move of saved.moves) {
      const cell = state.cells[move.row]?.[move.col];
      if (!cell || cell.revealed || state.guesses[move.row]?.[move.col]) {
        continue;
      }

      const clue = await fetchValidatedClue(move.row, move.col, move.guess);
      applyAcceptedGuess(move.row, move.col, move.guess, clue);
    }

    state.hiddenClues = new Set(
      saved.hiddenClues.filter((key) => {
        const [row, col] = key.split(":").map(Number);
        return Boolean(state.cells[row]?.[col]?.clue);
      }),
    );
    state.notes = new Map(
      saved.notes.filter(([key]) => {
        const [row, col] = key.split(":").map(Number);
        return Boolean(state.cells[row]?.[col]);
      }),
    );
    if (allTilesMarked()) {
      if (state.timerStartedAt === null) {
        state.timerStartedAt = Date.now();
      }
      if (state.timerCompletedAt === null) {
        state.timerCompletedAt = Date.now();
      }
    } else {
      state.timerCompletedAt = null;
      state.completionAcknowledged = false;
    }
    persistProgress();
  } catch {
    state.moves = [];
    state.notes = new Map();
    state.hiddenClues = new Set();
    state.timerStartedAt = null;
    state.timerCompletedAt = null;
    state.completionAcknowledged = false;
    clearSavedProgress(state.currentSeed);
    return {
      restored: false,
      error: "Saved progress could not be restored and was cleared.",
    };
  }

  return { restored: true, error: null };
}

async function setGuess(row, col, nextGuess) {
  const key = guessKey(row, col);
  const currentGuess = state.guesses[row][col];

  if (currentGuess === nextGuess) {
    closeGuessModal();
    return;
  }

  if (nextGuess === null) {
    state.guesses[row][col] = null;
    closeGuessModal();
    renderBoard();
    return;
  }

  state.loadingClues.add(key);
  renderBoard();

  try {
    const clue = await fetchValidatedClue(row, col, nextGuess);
    applyAcceptedGuess(row, col, nextGuess, clue);
    persistProgress();
    closeGuessModal();
    completePuzzleIfNeeded();
  } finally {
    state.loadingClues.delete(key);
    renderBoard();
  }
}

async function clearBoard() {
  clearPendingNoteTap();
  window.clearTimeout(state.flashTimer);
  clearSavedProgress();
  closeGuessModal();
  closeErrorModal();
  closeFinishModal();

  if (state.currentSeed === null) {
    return;
  }

  await loadPuzzle(state.currentSeed, { forceStartModal: true });
}

function toggleClueVisibility(row, col) {
  const key = guessKey(row, col);
  const clue = state.cells[row]?.[col]?.clue;

  if (state.hiddenClues.has(key)) {
    state.hiddenClues.delete(key);
  } else {
    state.hiddenClues.add(key);
  }

  if (clue) {
    persistProgress();
    flashMentionedTiles(clue);
    return;
  }

  renderBoard();
}

function openGuessModal(row, col) {
  const cell = state.cells[row]?.[col];
  if (!cell || cell.revealed) {
    return;
  }

  state.modalCell = { row, col };
  guessEmojiEl.textContent = emojiForRole(cell.role);
  guessTitleEl.textContent = cell.name;
  guessModalEl.hidden = false;
}

function closeGuessModal() {
  state.modalCell = null;
  guessModalEl.hidden = true;
}

function openErrorModal(title, message = "") {
  closeGuessModal();
  closeFinishModal();
  errorTitleEl.textContent = title;
  errorMessageEl.textContent = message;
  errorMessageEl.hidden = message === "";
  errorModalEl.hidden = false;
}

function closeErrorModal() {
  errorModalEl.hidden = true;
}

function openStartModal() {
  closeGuessModal();
  closeErrorModal();
  closeFinishModal();
  startModalEl.hidden = false;
}

function closeStartModal() {
  startModalEl.hidden = true;
}

function resetShareButton() {
  window.clearTimeout(state.shareFeedbackTimer);
  state.shareFeedbackTimer = null;
  shareButton.textContent = "Share";
}

function flashShareButton(label) {
  shareButton.textContent = label;
  window.clearTimeout(state.shareFeedbackTimer);
  state.shareFeedbackTimer = window.setTimeout(() => {
    state.shareFeedbackTimer = null;
    shareButton.textContent = "Share";
  }, 1200);
}

function showGuessError(error) {
  if (error?.status === 400 || error?.status === 409) {
    const { row, col } = state.modalCell ?? {};
    const cell = Number.isInteger(row) && Number.isInteger(col) ? state.cells[row]?.[col] : null;
    const detail =
      cell && error?.guess
        ? `${cell.name} can't be logically identified as ${error.guess} from the available info.`
        : "";
    openErrorModal(invalidMoveMessage, detail);
    return;
  }

  openErrorModal(error.message);
}

newRandomButton.addEventListener("click", async () => {
  try {
    await loadPuzzle();
  } catch (error) {
    openErrorModal(error.message);
  }
});

shareButton.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(window.location.href);
    flashShareButton("Copied");
  } catch (error) {
    flashShareButton("Failed");
    openErrorModal("Could not copy the link.");
  }
});

clearButton.addEventListener("click", async () => {
  try {
    await clearBoard();
  } catch (error) {
    openErrorModal(error.message);
  }
});
guessBackdropEl.addEventListener("click", closeGuessModal);
guessCancelButton.addEventListener("click", closeGuessModal);
errorBackdropEl.addEventListener("click", closeErrorModal);
errorDismissButton.addEventListener("click", closeErrorModal);
startButton.addEventListener("click", startPuzzle);
finishBackdropEl.addEventListener("click", dismissFinishModal);
finishDismissButton.addEventListener("click", dismissFinishModal);
guessInnocentButton.addEventListener("click", async () => {
  if (!state.modalCell) {
    return;
  }

  try {
    await setGuess(state.modalCell.row, state.modalCell.col, "innocent");
  } catch (error) {
    showGuessError(error);
  }
});
guessCriminalButton.addEventListener("click", async () => {
  if (!state.modalCell) {
    return;
  }

  try {
    await setGuess(state.modalCell.row, state.modalCell.col, "criminal");
  } catch (error) {
    showGuessError(error);
  }
});

loadPuzzle(seedFromUrl()).catch((error) => {
  openErrorModal(error.message);
});
