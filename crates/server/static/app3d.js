const boardShellEl = document.querySelector("#board-3d-shell");
const boardSceneEl = document.querySelector("#board-3d-scene");
const axisGizmoEl = document.querySelector("#axis-gizmo");
const clearButton = document.querySelector("#clear-3d");
const newRandomButton = document.querySelector("#new-random-3d");
const newPuzzleModalEl = document.querySelector("#new-puzzle-3d-modal");
const newPuzzleBackdropEl = document.querySelector("#new-puzzle-3d-backdrop");
const newPuzzleDepthUpButton = document.querySelector("#new-puzzle-3d-depth-up");
const newPuzzleDepthValueEl = document.querySelector("#new-puzzle-3d-depth-value");
const newPuzzleDepthDownButton = document.querySelector("#new-puzzle-3d-depth-down");
const newPuzzleRowsUpButton = document.querySelector("#new-puzzle-3d-rows-up");
const newPuzzleRowsValueEl = document.querySelector("#new-puzzle-3d-rows-value");
const newPuzzleRowsDownButton = document.querySelector("#new-puzzle-3d-rows-down");
const newPuzzleColsUpButton = document.querySelector("#new-puzzle-3d-cols-up");
const newPuzzleColsValueEl = document.querySelector("#new-puzzle-3d-cols-value");
const newPuzzleColsDownButton = document.querySelector("#new-puzzle-3d-cols-down");
const newPuzzleCancelButton = document.querySelector("#new-puzzle-3d-cancel");
const newPuzzleConfirmButton = document.querySelector("#new-puzzle-3d-confirm");
const resetViewButton = document.querySelector("#reset-view-3d");
const shareButton = document.querySelector("#share-3d");
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
const finishModalEl = document.querySelector("#finish-modal");
const finishBackdropEl = document.querySelector("#finish-backdrop");
const finishMessageEl = document.querySelector("#finish-message");
const finishCopyButton = document.querySelector("#finish-copy");
const finishDismissButton = document.querySelector("#finish-dismiss");
const cornerNoteMenuEl = document.querySelector("#corner-note-menu");
const cornerNoteMenuCardEl = document.querySelector("#corner-note-menu-card");
const cornerNoteOptionEls = [...document.querySelectorAll("#corner-note-menu [data-color]")];
const progressStoragePrefix = "clues-3d-progress:v1:";
const sharedLinkStoragePrefix = "clues-3d-shared-link:v1:";
const maxPublicCellCount = 20;
const maxBoardDimension = 20;
const noteTapDelayMs = 420;
const cornerNotePressDelayMs = 420;
const suppressedNoteClickDelayMs = 260;
const noteColors = ["yellow", "red", "green", "orange", "magenta", "cyan"];
const invalidMoveMessage = "⚠️ Not enough evidence!";
const textReorientationDelayMs = 180;
const scenePerspective = 1400;
const defaultRotationX = -22;
const defaultRotationY = 35;
const defaultBoardSize = { depth: 2, rows: 2, cols: 2 };
const minUserZoomScale = 0.72;
const maxUserZoomScale = 2.2;
const roleEmojis = {
  Artist: "🧑‍🎨",
  Baker: "👨‍🍳🥖",
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

const state = {
  currentSeed: null,
  currentStoredPuzzleId: null,
  sharedStoredPuzzleId: null,
  boardSize: defaultBoardSize,
  cells: [],
  moves: [],
  notes: new Map(),
  bottomNotes: new Map(),
  timerElapsedMs: 0,
  timerStartedAt: null,
  timerCompletedAt: null,
  completionAcknowledged: false,
  hiddenClues: new Set(),
  flashHighlights: [],
  flashTimer: null,
  shareFeedbackTimer: null,
  shareLinkPromise: null,
  finishCopyFeedbackTimer: null,
  rotationX: defaultRotationX,
  rotationY: defaultRotationY,
  textRotationX: defaultRotationX,
  textRotationY: defaultRotationY,
  textReorientationTimer: null,
  wheelInteractionTimer: null,
  gestureZoomStartScale: null,
  viewOffsetX: 0,
  viewOffsetY: 0,
  viewScale: 1,
  userZoomScale: 1,
  targetViewOffsetX: 0,
  targetViewOffsetY: 0,
  targetViewScale: 1,
  viewCompensationFrame: null,
  sceneCellSize: 0,
  sceneSpacing: 0,
  activeTouchPoints: new Map(),
  pinch: null,
  drag: null,
  modalCell: null,
  pendingNoteTap: null,
  pendingTopNotePress: null,
  pendingBottomNotePress: null,
  suppressedTopNoteClick: null,
  suppressedBottomNoteClick: null,
  cornerNoteMenu: null,
  activeCornerNoteColor: null,
  pendingNewBoardSize: null,
  newPuzzleDrag: null,
};

function emojiForRole(role) {
  return roleEmojis[role] ?? "🧑";
}

function emojiForCell(cell) {
  if (typeof cell.emoji === "string" && cell.emoji.trim() !== "") {
    return cell.emoji;
  }

  return emojiForRole(cell.role);
}

function normalizeSeed(value) {
  if (value === undefined || value === null || value === "") {
    return null;
  }

  const normalized = String(value).trim().toLowerCase();
  return /^[0-9a-f]{1,12}$/.test(normalized) ? normalized.padStart(12, "0") : null;
}

function normalizeStoredPuzzleId(value) {
  if (value === undefined || value === null || value === "") {
    return null;
  }

  const normalized = String(value).trim().toLowerCase();
  return /^[0-9a-f]{6}$/.test(normalized) ? normalized : null;
}

function parseDimension(value, fallback) {
  const parsed = Number.parseInt(String(value ?? ""), 10);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback;
}

function boardSizeFromUrl() {
  const url = new URL(window.location.href);
  const depth = parseDimension(url.searchParams.get("depth"), defaultBoardSize.depth);
  const rows = parseDimension(url.searchParams.get("rows"), defaultBoardSize.rows);
  const maxCols = Math.max(1, Math.floor(maxPublicCellCount / (depth * rows)));
  const cols = Math.min(parseDimension(url.searchParams.get("cols"), defaultBoardSize.cols), maxCols);
  return { depth, rows, cols };
}

function currentBoardSize() {
  return state.boardSize ?? boardSizeFromUrl();
}

function maxRowsForDepth(depth) {
  return Math.max(1, Math.floor(maxPublicCellCount / depth));
}

function maxColsForDepthRows(depth, rows) {
  return Math.max(1, Math.floor(maxPublicCellCount / (depth * rows)));
}

function clampBoardDimension(value, fallback, max = maxBoardDimension) {
  const parsed = Number.parseInt(String(value ?? ""), 10);
  if (!Number.isInteger(parsed)) {
    return Math.min(max, Math.max(1, fallback));
  }

  return Math.min(max, Math.max(1, parsed));
}

function currentNewPuzzleBoardSize() {
  const fallback = currentBoardSize();
  const depth = clampBoardDimension(state.pendingNewBoardSize?.depth, fallback.depth);
  const rows = clampBoardDimension(
    state.pendingNewBoardSize?.rows,
    fallback.rows,
    maxRowsForDepth(depth),
  );
  const cols = clampBoardDimension(
    state.pendingNewBoardSize?.cols,
    fallback.cols,
    maxColsForDepthRows(depth, rows),
  );
  return { depth, rows, cols };
}

function setNewPuzzleSpinValue(element, value, max) {
  element.textContent = String(value);
  element.setAttribute("aria-valuenow", String(value));
  element.setAttribute("aria-valuemax", String(max));
}

function populateNewPuzzleSelectors() {
  const { depth, rows, cols } = currentNewPuzzleBoardSize();
  state.pendingNewBoardSize = { depth, rows, cols };
  const maxRows = maxRowsForDepth(depth);
  const maxCols = maxColsForDepthRows(depth, rows);

  setNewPuzzleSpinValue(newPuzzleDepthValueEl, depth, maxBoardDimension);
  setNewPuzzleSpinValue(newPuzzleRowsValueEl, rows, maxRows);
  setNewPuzzleSpinValue(newPuzzleColsValueEl, cols, maxCols);

  newPuzzleDepthUpButton.disabled = depth >= maxBoardDimension;
  newPuzzleDepthDownButton.disabled = depth <= 1;
  newPuzzleRowsUpButton.disabled = rows >= maxRows;
  newPuzzleRowsDownButton.disabled = rows <= 1;
  newPuzzleColsUpButton.disabled = cols >= maxCols;
  newPuzzleColsDownButton.disabled = cols <= 1;
}

function stepNewPuzzleDimension(dimension, delta) {
  const size = currentNewPuzzleBoardSize();

  if (dimension === "depth") {
    const depth = clampBoardDimension(size.depth + delta, size.depth);
    const rows = Math.min(size.rows, maxRowsForDepth(depth));
    const cols = Math.min(size.cols, maxColsForDepthRows(depth, rows));
    state.pendingNewBoardSize = { depth, rows, cols };
  } else if (dimension === "rows") {
    const rows = clampBoardDimension(size.rows + delta, size.rows, maxRowsForDepth(size.depth));
    state.pendingNewBoardSize = {
      ...size,
      rows,
      cols: Math.min(size.cols, maxColsForDepthRows(size.depth, rows)),
    };
  } else {
    state.pendingNewBoardSize = {
      ...size,
      cols: clampBoardDimension(size.cols + delta, size.cols, maxColsForDepthRows(size.depth, size.rows)),
    };
  }

  populateNewPuzzleSelectors();
}

function beginNewPuzzleDrag(dimension, event) {
  state.newPuzzleDrag = {
    dimension,
    pointerId: event.pointerId,
    lastClientY: event.clientY,
    carriedDeltaY: 0,
  };
  event.currentTarget.setPointerCapture?.(event.pointerId);
}

function updateNewPuzzleDrag(event) {
  if (!state.newPuzzleDrag || state.newPuzzleDrag.pointerId !== event.pointerId) {
    return;
  }

  const threshold = 20;
  state.newPuzzleDrag.carriedDeltaY += event.clientY - state.newPuzzleDrag.lastClientY;
  state.newPuzzleDrag.lastClientY = event.clientY;

  while (state.newPuzzleDrag.carriedDeltaY <= -threshold) {
    stepNewPuzzleDimension(state.newPuzzleDrag.dimension, 1);
    state.newPuzzleDrag.carriedDeltaY += threshold;
  }

  while (state.newPuzzleDrag.carriedDeltaY >= threshold) {
    stepNewPuzzleDimension(state.newPuzzleDrag.dimension, -1);
    state.newPuzzleDrag.carriedDeltaY -= threshold;
  }
}

function endNewPuzzleDrag(event) {
  if (!state.newPuzzleDrag || state.newPuzzleDrag.pointerId !== event.pointerId) {
    return;
  }

  state.newPuzzleDrag = null;
  event.currentTarget.releasePointerCapture?.(event.pointerId);
}

function currentSeedFromUrl() {
  return normalizeSeed(new URL(window.location.href).searchParams.get("seed"));
}

function currentStoredPuzzleIdFromUrl() {
  const match = window.location.pathname.match(/^\/3d\/p\/([0-9a-f]{6})\/?$/i);
  return normalizeStoredPuzzleId(match?.[1] ?? null);
}

function updateUrlGeneratedPuzzle(seed, boardSize) {
  const url = new URL(window.location.href);
  url.pathname = "/3d/";
  url.searchParams.set("seed", seed);
  url.searchParams.set("depth", String(boardSize.depth));
  url.searchParams.set("rows", String(boardSize.rows));
  url.searchParams.set("cols", String(boardSize.cols));
  window.history.replaceState({}, "", url);
}

function updateUrlStoredPuzzle(storedPuzzleId) {
  const url = new URL(window.location.href);
  url.pathname = `/3d/p/${encodeURIComponent(storedPuzzleId)}`;
  url.search = "";
  window.history.replaceState({}, "", url);
}

function progressStorageKey(boardSize = currentBoardSize()) {
  if (state.currentStoredPuzzleId !== null) {
    return `${progressStoragePrefix}stored:${state.currentStoredPuzzleId}`;
  }

  if (state.currentSeed === null) {
    return null;
  }

  return `${progressStoragePrefix}${state.currentSeed}:${boardSize.depth}x${boardSize.rows}x${boardSize.cols}`;
}

function sharedLinkStorageKey(boardSize = currentBoardSize()) {
  if (state.currentSeed === null) {
    return null;
  }

  return `${sharedLinkStoragePrefix}${state.currentSeed}:${boardSize.depth}x${boardSize.rows}x${boardSize.cols}`;
}

function readSharedStoredPuzzleId(boardSize = currentBoardSize()) {
  const storageKey = sharedLinkStorageKey(boardSize);
  if (storageKey === null) {
    return null;
  }

  try {
    return normalizeStoredPuzzleId(window.localStorage.getItem(storageKey));
  } catch {
    return null;
  }
}

function persistSharedStoredPuzzleId(storedPuzzleId, boardSize = currentBoardSize()) {
  const storageKey = sharedLinkStorageKey(boardSize);
  if (storageKey === null) {
    return;
  }

  try {
    window.localStorage.setItem(storageKey, storedPuzzleId);
  } catch {}
}

function storedPuzzleUrl(storedPuzzleId) {
  return `${window.location.origin}/3d/p/${encodeURIComponent(storedPuzzleId)}`;
}

function currentPuzzleSource() {
  if (state.currentStoredPuzzleId !== null) {
    return {
      kind: "stored",
      stored_puzzle_id: state.currentStoredPuzzleId,
    };
  }

  if (state.sharedStoredPuzzleId !== null) {
    return {
      kind: "stored",
      stored_puzzle_id: state.sharedStoredPuzzleId,
    };
  }

  if (state.currentSeed !== null) {
    const boardSize = currentBoardSize();
    return {
      kind: "generated",
      seed: state.currentSeed,
      depth: boardSize.depth,
      rows: boardSize.rows,
      cols: boardSize.cols,
    };
  }

  throw new Error("No puzzle is loaded");
}

function readSavedProgress(boardSize = currentBoardSize()) {
  const storageKey = progressStorageKey(boardSize);
  if (storageKey === null) {
    return null;
  }

  try {
    const raw = window.localStorage.getItem(storageKey);
    if (!raw) {
      return null;
    }

    const parsed = JSON.parse(raw);
    const moves = Array.isArray(parsed?.moves)
      ? parsed.moves.filter(
          (move) =>
            Number.isInteger(move?.layer) &&
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
            noteColors.includes(entry[1]),
        )
      : [];
    const bottomNotes = Array.isArray(parsed?.bottomNotes)
      ? parsed.bottomNotes.filter(
          (entry) =>
            Array.isArray(entry) &&
            entry.length === 2 &&
            typeof entry[0] === "string" &&
            noteColors.includes(entry[1]),
        )
      : [];
    const timerElapsedMs =
      Number.isFinite(parsed?.timerElapsedMs) && parsed.timerElapsedMs >= 0
        ? parsed.timerElapsedMs
        : 0;
    const timerCompletedAt =
      Number.isFinite(parsed?.timerCompletedAt) && parsed.timerCompletedAt >= 0
        ? parsed.timerCompletedAt
        : null;
    const completionAcknowledged = parsed?.completionAcknowledged === true;
    const view =
      parsed?.view && typeof parsed.view === "object"
        ? {
            rotationX: Number.isFinite(parsed.view.rotationX)
              ? normalizeAngle(parsed.view.rotationX)
              : defaultRotationX,
            rotationY: Number.isFinite(parsed.view.rotationY)
              ? normalizeAngle(parsed.view.rotationY)
              : defaultRotationY,
            textRotationX: Number.isFinite(parsed.view.textRotationX)
              ? normalizeAngle(parsed.view.textRotationX)
              : null,
            textRotationY: Number.isFinite(parsed.view.textRotationY)
              ? normalizeAngle(parsed.view.textRotationY)
              : null,
            userZoomScale: Number.isFinite(parsed.view.userZoomScale)
              ? clampUserZoomScale(parsed.view.userZoomScale)
              : 1,
          }
        : null;

    return {
      moves,
      hiddenClues,
      notes,
      bottomNotes,
      timerElapsedMs,
      timerCompletedAt,
      completionAcknowledged,
      view,
    };
  } catch {
    return null;
  }
}

function clearSavedProgress(boardSize = currentBoardSize()) {
  const storageKey = progressStorageKey(boardSize);
  if (storageKey === null) {
    return;
  }

  try {
    window.localStorage.removeItem(storageKey);
  } catch {}
}

function persistProgress() {
  const storageKey = progressStorageKey();
  if (storageKey === null) {
    return;
  }

  const hiddenClues = [...state.hiddenClues];
  const notes = [...state.notes.entries()];
  const bottomNotes = [...state.bottomNotes.entries()];
  const hasTimerState =
    state.timerElapsedMs > 0 ||
    state.timerStartedAt !== null ||
    state.timerCompletedAt !== null ||
    state.completionAcknowledged;
  const hasViewState =
    Math.abs(state.rotationX - defaultRotationX) > 0.01 ||
    Math.abs(state.rotationY - defaultRotationY) > 0.01 ||
    Math.abs(state.userZoomScale - 1) > 0.001 ||
    Math.abs(state.textRotationX - state.rotationX) > 0.01 ||
    Math.abs(state.textRotationY - state.rotationY) > 0.01;

  if (
    state.moves.length === 0 &&
    hiddenClues.length === 0 &&
    notes.length === 0 &&
    bottomNotes.length === 0 &&
    !hasTimerState &&
    !hasViewState
  ) {
    clearSavedProgress();
    return;
  }

  try {
    window.localStorage.setItem(
      storageKey,
      JSON.stringify({
        moves: state.moves,
        hiddenClues,
        notes,
        bottomNotes,
        timerElapsedMs: currentTimerElapsedMs(),
        timerStartedAt: null,
        timerCompletedAt: state.timerCompletedAt,
        completionAcknowledged: state.completionAcknowledged,
        view: {
          rotationX: state.rotationX,
          rotationY: state.rotationY,
          textRotationX: state.textRotationX,
          textRotationY: state.textRotationY,
          userZoomScale: state.userZoomScale,
        },
      }),
    );
  } catch {}
}

function fetchJson(url, options = {}) {
  return fetch(url, {
    headers: {
      "Content-Type": "application/json",
      ...(options.headers ?? {}),
    },
    ...options,
  }).then(async (response) => {
    const body = await response.json().catch(() => ({}));
    if (!response.ok) {
      throw new Error(body.error ?? `Request failed (${response.status})`);
    }
    return body;
  });
}

function threeDKey(layer, row, col) {
  return `${layer}:${row}:${col}`;
}

function colLabel(index) {
  let value = index;
  let label = "";

  while (value >= 0) {
    label = String.fromCharCode(65 + (value % 26)) + label;
    value = Math.floor(value / 26) - 1;
  }

  return label;
}

function positionLabel(layer, row, col) {
  return `L${layer + 1} ${colLabel(col)}${row + 1}`;
}

function mentionedNameKeys(clue) {
  const keys = new Set();

  state.cells.forEach((layer, layerIndex) => {
    layer.forEach((row, rowIndex) => {
      row.forEach((cell, colIndex) => {
        if (new RegExp(`\\b${cell.name.replace(/[.*+?^${}()|[\\]\\\\]/g, "\\$&")}\\b`, "i").test(clue)) {
          keys.add(threeDKey(layerIndex, rowIndex, colIndex));
        }
      });
    });
  });

  return keys;
}

function mentionedRoleKeys(clue) {
  const keys = new Set();
  const lowerClue = clue.toLowerCase();

  state.cells.forEach((layer, layerIndex) => {
    layer.forEach((row, rowIndex) => {
      row.forEach((cell, colIndex) => {
        const role = cell.role.toLowerCase();
        const plural = role.endsWith("s") ? `${role}es` : `${role}s`;
        if (
          new RegExp(`\\b${role.replace(/[.*+?^${}()|[\\]\\\\]/g, "\\$&")}\\b`, "i").test(lowerClue) ||
          new RegExp(`\\b${plural.replace(/[.*+?^${}()|[\\]\\\\]/g, "\\$&")}\\b`, "i").test(lowerClue)
        ) {
          keys.add(threeDKey(layerIndex, rowIndex, colIndex));
        }
      });
    });
  });

  return keys;
}

function prismBoundsForCells(cells) {
  if (!Array.isArray(cells) || cells.length === 0) {
    return null;
  }

  const layers = cells.map((cell) => cell.layer);
  const rows = cells.map((cell) => cell.row);
  const cols = cells.map((cell) => cell.col);
  const minLayer = Math.min(...layers);
  const maxLayer = Math.max(...layers);
  const minRow = Math.min(...rows);
  const maxRow = Math.max(...rows);
  const minCol = Math.min(...cols);
  const maxCol = Math.max(...cols);
  const expectedCount =
    (maxLayer - minLayer + 1) * (maxRow - minRow + 1) * (maxCol - minCol + 1);

  if (expectedCount !== cells.length) {
    return null;
  }

  const keys = new Set(cells.map((cell) => threeDKey(cell.layer, cell.row, cell.col)));
  for (let layer = minLayer; layer <= maxLayer; layer += 1) {
    for (let row = minRow; row <= maxRow; row += 1) {
      for (let col = minCol; col <= maxCol; col += 1) {
        if (!keys.has(threeDKey(layer, row, col))) {
          return null;
        }
      }
    }
  }

  return {
    minLayer,
    maxLayer,
    minRow,
    maxRow,
    minCol,
    maxCol,
  };
}

function normalizedHighlightGroups(groups) {
  if (!Array.isArray(groups)) {
    return [];
  }

  return groups
    .map((group) => {
      const keys = new Set();
      const cells = [];
      if (!Array.isArray(group)) {
        return { keys, cells, prismBounds: null };
      }

      group.forEach((cell) => {
        if (
          Number.isInteger(cell?.layer) &&
          Number.isInteger(cell?.row) &&
          Number.isInteger(cell?.col)
        ) {
          cells.push({
            layer: cell.layer,
            row: cell.row,
            col: cell.col,
          });
          keys.add(threeDKey(cell.layer, cell.row, cell.col));
        }
      });

      return {
        keys,
        cells,
        prismBounds: prismBoundsForCells(cells),
      };
    })
    .filter((group) => group.keys.size > 0);
}

function flashColorForIndex(index) {
  return index % 2 === 0 ? "gold" : "blue";
}

function prismColorForIndex(index) {
  return index % 2 === 0 ? "soft-blue" : "soft-purple";
}

function combinedFlashHighlights(clue, clueHighlightGroups) {
  const groups = normalizedHighlightGroups(clueHighlightGroups);
  const mentionedNames = mentionedNameKeys(clue);

  if (groups.length === 0) {
    const fallback = mentionedNames;
    mentionedRoleKeys(clue).forEach((key) => {
      fallback.add(key);
    });
    return fallback.size > 0 ? [{ kind: "cells", keys: fallback, color: "gold" }] : [];
  }

  const coveredByCellHighlights = new Set();
  const highlights = groups.map((group, index) => {
    if (group.prismBounds && group.keys.size > 1) {
      return {
        kind: "prism",
        bounds: group.prismBounds,
        color: prismColorForIndex(index),
      };
    }

    group.keys.forEach((key) => {
      coveredByCellHighlights.add(key);
    });

    return {
      kind: "cells",
      keys: group.keys,
      color: flashColorForIndex(index),
    };
  });

  const extraNames = new Set();
  mentionedNames.forEach((key) => {
    if (!coveredByCellHighlights.has(key)) {
      extraNames.add(key);
    }
  });

  if (extraNames.size > 0) {
    highlights.push({
      kind: "cells",
      keys: extraNames,
      color: "gold",
    });
  }

  return highlights;
}

function flashMentionedCells(clue, clueHighlightGroups = []) {
  state.flashHighlights = combinedFlashHighlights(clue, clueHighlightGroups);
  window.clearTimeout(state.flashTimer);
  state.flashTimer = window.setTimeout(() => {
    state.flashHighlights = [];
    state.flashTimer = null;
    renderBoard();
  }, 900);
  renderBoard();
}

function allTilesMarked() {
  return state.cells.every((layer) => layer.every((row) => row.every((cell) => cellIsRevealed(cell))));
}

function formatResultDuration(milliseconds) {
  const totalSeconds = Math.max(0, Math.round(milliseconds / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }

  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

function hasStartedTimer() {
  return (
    state.timerElapsedMs > 0 ||
    state.timerStartedAt !== null ||
    state.timerCompletedAt !== null
  );
}

function currentTimerElapsedMs() {
  if (state.timerCompletedAt !== null) {
    return state.timerCompletedAt;
  }

  if (state.timerStartedAt !== null) {
    return state.timerElapsedMs + (Date.now() - state.timerStartedAt);
  }

  return state.timerElapsedMs;
}

function pauseActiveTimer() {
  if (state.timerStartedAt === null || state.timerCompletedAt !== null) {
    return;
  }

  state.timerElapsedMs += Date.now() - state.timerStartedAt;
  state.timerStartedAt = null;
  persistProgress();
}

function resumeActiveTimerIfNeeded() {
  if (
    document.hidden ||
    state.timerCompletedAt !== null ||
    state.timerStartedAt !== null ||
    !hasStartedTimer() ||
    allTilesMarked()
  ) {
    return;
  }

  state.timerStartedAt = Date.now();
  persistProgress();
}

function shareablePuzzleUrl() {
  if (state.currentStoredPuzzleId !== null) {
    return storedPuzzleUrl(state.currentStoredPuzzleId);
  }

  if (state.sharedStoredPuzzleId !== null) {
    return storedPuzzleUrl(state.sharedStoredPuzzleId);
  }

  return window.location.href;
}

async function ensureShareableLink() {
  if (state.currentStoredPuzzleId !== null) {
    return storedPuzzleUrl(state.currentStoredPuzzleId);
  }

  if (state.sharedStoredPuzzleId !== null) {
    return storedPuzzleUrl(state.sharedStoredPuzzleId);
  }

  const cachedStoredPuzzleId = readSharedStoredPuzzleId();
  if (cachedStoredPuzzleId !== null) {
    state.sharedStoredPuzzleId = cachedStoredPuzzleId;
    return storedPuzzleUrl(cachedStoredPuzzleId);
  }

  if (state.currentSeed === null) {
    return window.location.href;
  }

  if (state.shareLinkPromise !== null) {
    return state.shareLinkPromise;
  }

  const boardSize = currentBoardSize();
  const params = new URLSearchParams({
    seed: state.currentSeed,
    depth: String(boardSize.depth),
    rows: String(boardSize.rows),
    cols: String(boardSize.cols),
  });

  state.shareLinkPromise = (async () => {
    const response = await fetch(`/api/3d/stored-puzzles/generate?${params.toString()}`, {
      method: "POST",
    });
    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: "Failed to create share link" }));
      throw new Error(error.error || "Failed to create share link");
    }

    const result = await response.json();
    const storedPuzzleId = normalizeStoredPuzzleId(result.stored_puzzle_id);
    if (storedPuzzleId === null) {
      throw new Error("Failed to create share link");
    }

    state.sharedStoredPuzzleId = storedPuzzleId;
    persistSharedStoredPuzzleId(storedPuzzleId, boardSize);
    return storedPuzzleUrl(storedPuzzleId);
  })().finally(() => {
    state.shareLinkPromise = null;
  });

  return state.shareLinkPromise;
}

async function finishShareText() {
  return `${formatResultDuration(currentTimerElapsedMs())}\n${await ensureShareableLink()}`;
}

function resetShareButton() {
  window.clearTimeout(state.shareFeedbackTimer);
  state.shareFeedbackTimer = null;
  syncShareButton();
}

function flashShareButton(label) {
  shareButton.textContent = label;
  window.clearTimeout(state.shareFeedbackTimer);
  state.shareFeedbackTimer = window.setTimeout(() => {
    state.shareFeedbackTimer = null;
    syncShareButton();
  }, 1200);
}

function resetFinishCopyButton() {
  window.clearTimeout(state.finishCopyFeedbackTimer);
  state.finishCopyFeedbackTimer = null;
  finishCopyButton.textContent = "Copy Result";
}

function flashFinishCopyButton(label) {
  finishCopyButton.textContent = label;
  window.clearTimeout(state.finishCopyFeedbackTimer);
  state.finishCopyFeedbackTimer = window.setTimeout(() => {
    state.finishCopyFeedbackTimer = null;
    finishCopyButton.textContent = "Copy Result";
  }, 1200);
}

function openFinishModal() {
  if (!hasStartedTimer() || state.timerCompletedAt === null) {
    return;
  }

  closeCornerNoteMenu();
  closeGuessModal();
  closeErrorModal();
  finishMessageEl.textContent = formatResultDuration(currentTimerElapsedMs());
  resetFinishCopyButton();
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

function defaultShareButtonLabel() {
  return state.timerCompletedAt === null ? "Share" : "🏆 Share Results";
}

function syncShareButton() {
  shareButton.textContent = defaultShareButtonLabel();
}

function completePuzzleIfNeeded() {
  if (!allTilesMarked() || !hasStartedTimer()) {
    return;
  }

  if (state.timerCompletedAt === null) {
    state.timerCompletedAt = currentTimerElapsedMs();
    state.timerStartedAt = null;
  }

  state.completionAcknowledged = false;
  persistProgress();
  syncShareButton();
  openFinishModal();
}

function clearTextReorientationTimer() {
  if (state.textReorientationTimer === null) {
    return;
  }

  window.clearTimeout(state.textReorientationTimer);
  state.textReorientationTimer = null;
}

function clearWheelInteractionTimer() {
  if (state.wheelInteractionTimer === null) {
    return;
  }

  window.clearTimeout(state.wheelInteractionTimer);
  state.wheelInteractionTimer = null;
}

function clampUserZoomScale(value) {
  return Math.max(minUserZoomScale, Math.min(maxUserZoomScale, value));
}

function clearPinchState() {
  state.activeTouchPoints.clear();
  state.pinch = null;
  state.gestureZoomStartScale = null;
}

function pointWithinBoard(clientX, clientY) {
  const rect = boardShellEl.getBoundingClientRect();
  return clientX >= rect.left && clientX <= rect.right && clientY >= rect.top && clientY <= rect.bottom;
}

function eventTargetsBoard(event) {
  if (boardShellEl.contains(event.target)) {
    return true;
  }

  if (typeof event.clientX === "number" && typeof event.clientY === "number") {
    return pointWithinBoard(event.clientX, event.clientY);
  }

  return false;
}

function startBoardZoomInteraction() {
  clearWheelInteractionTimer();
  clearViewCompensationFrame();
}

function finishBoardZoomInteraction(delayMs = 90) {
  clearWheelInteractionTimer();
  state.wheelInteractionTimer = window.setTimeout(() => {
    state.wheelInteractionTimer = null;
    retargetViewCompensation();
    persistProgress();
  }, delayMs);
}

function applyBoardZoom(scale, { immediate = true } = {}) {
  state.userZoomScale = clampUserZoomScale(scale);
  retargetViewCompensation({ immediate });
}

function clearViewCompensationFrame() {
  if (state.viewCompensationFrame === null) {
    return;
  }

  window.cancelAnimationFrame(state.viewCompensationFrame);
  state.viewCompensationFrame = null;
}

function cellIsRevealed(cell) {
  return cell.revealed === true;
}

function clearPendingNoteTap() {
  if (!state.pendingNoteTap) {
    return;
  }

  window.clearTimeout(state.pendingNoteTap.timerId);
  state.pendingNoteTap = null;
}

function clearPendingTopNotePress() {
  if (!state.pendingTopNotePress) {
    return;
  }

  window.clearTimeout(state.pendingTopNotePress.timerId);
  state.pendingTopNotePress = null;
}

function clearPendingBottomNotePress() {
  if (!state.pendingBottomNotePress) {
    return;
  }

  window.clearTimeout(state.pendingBottomNotePress.timerId);
  state.pendingBottomNotePress = null;
}

function suppressTopNoteClick(key) {
  state.suppressedTopNoteClick = {
    key,
    until: Date.now() + suppressedNoteClickDelayMs,
  };
}

function suppressBottomNoteClick(key) {
  state.suppressedBottomNoteClick = {
    key,
    until: Date.now() + suppressedNoteClickDelayMs,
  };
}

function shouldSuppressTopNoteClick(key) {
  if (!state.suppressedTopNoteClick) {
    return false;
  }

  if (state.suppressedTopNoteClick.until < Date.now()) {
    state.suppressedTopNoteClick = null;
    return false;
  }

  if (state.suppressedTopNoteClick.key !== key) {
    return false;
  }

  state.suppressedTopNoteClick = null;
  return true;
}

function shouldSuppressBottomNoteClick(key) {
  if (!state.suppressedBottomNoteClick) {
    return false;
  }

  if (state.suppressedBottomNoteClick.until < Date.now()) {
    state.suppressedBottomNoteClick = null;
    return false;
  }

  if (state.suppressedBottomNoteClick.key !== key) {
    return false;
  }

  state.suppressedBottomNoteClick = null;
  return true;
}

function closeCornerNoteMenu() {
  state.cornerNoteMenu = null;
  state.activeCornerNoteColor = null;
  if (cornerNoteMenuEl) {
    cornerNoteMenuEl.hidden = true;
  }
  if (cornerNoteMenuCardEl) {
    cornerNoteMenuCardEl.style.left = "";
    cornerNoteMenuCardEl.style.top = "";
  }
  updateCornerNoteMenuActiveColor(null);
}

function updateCornerNoteMenuActiveColor(color) {
  state.activeCornerNoteColor = color;
  cornerNoteOptionEls.forEach((option) => {
    option.classList.toggle("active", option.dataset.color === color);
  });
}

function positionCornerNoteMenu(tileRect) {
  const margin = 10;
  const menuRect = cornerNoteMenuCardEl.getBoundingClientRect();
  let left = tileRect.left + 4;
  let top = tileRect.top + (tileRect.height - menuRect.height) / 2;

  if (left < margin) {
    left = margin;
  }
  if (left + menuRect.width > window.innerWidth - margin) {
    left = window.innerWidth - menuRect.width - margin;
  }
  if (top < margin) {
    top = margin;
  }
  if (top + menuRect.height > window.innerHeight - margin) {
    top = Math.max(margin, window.innerHeight - menuRect.height - margin);
  }

  cornerNoteMenuCardEl.style.left = `${left}px`;
  cornerNoteMenuCardEl.style.top = `${top}px`;
}

function openCornerNoteMenu(layer, row, col, slot, tileRect, pointerId) {
  clearPendingTopNotePress();
  clearPendingBottomNotePress();
  const key = threeDKey(layer, row, col);
  const currentColor =
    slot === "top" ? state.notes.get(key) ?? noteColors[0] : state.bottomNotes.get(key) ?? noteColors[0];
  state.cornerNoteMenu = { layer, row, col, slot, pointerId };
  cornerNoteMenuEl.hidden = false;
  cornerNoteMenuCardEl.style.left = "0px";
  cornerNoteMenuCardEl.style.top = "0px";
  positionCornerNoteMenu(tileRect);
  updateCornerNoteMenuActiveColor(currentColor);
}

function setNote(layer, row, col, color) {
  const key = threeDKey(layer, row, col);

  if (color === null) {
    state.notes.delete(key);
  } else {
    state.notes.set(key, color);
  }

  persistProgress();
  renderBoard();
}

function setBottomNote(layer, row, col, color) {
  const key = threeDKey(layer, row, col);

  if (color === null) {
    state.bottomNotes.delete(key);
  } else {
    state.bottomNotes.set(key, color);
  }

  closeCornerNoteMenu();
  persistProgress();
  renderBoard();
}

function setCornerNote(layer, row, col, slot, color) {
  closeCornerNoteMenu();
  if (slot === "top") {
    setNote(layer, row, col, color);
  } else {
    setBottomNote(layer, row, col, color);
  }
}

function cornerNoteColorAtClientY(clientY) {
  const rect = cornerNoteMenuCardEl.getBoundingClientRect();
  const heightPerColor = rect.height / cornerNoteOptionEls.length;
  const clampedY = Math.max(rect.top, Math.min(rect.bottom - 1, clientY));
  const index = Math.max(
    0,
    Math.min(cornerNoteOptionEls.length - 1, Math.floor((clampedY - rect.top) / heightPerColor)),
  );
  return cornerNoteOptionEls[index]?.dataset.color ?? null;
}

function finalizeCornerNoteMenu() {
  if (!state.cornerNoteMenu) {
    return;
  }

  const { layer, row, col, slot } = state.cornerNoteMenu;
  const color = state.activeCornerNoteColor;
  if (color) {
    setCornerNote(layer, row, col, slot, color);
  } else {
    closeCornerNoteMenu();
  }
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

function handleNoteTap(layer, row, col) {
  const key = threeDKey(layer, row, col);
  const current = state.notes.get(key);

  if (state.pendingNoteTap && state.pendingNoteTap.key === key) {
    state.pendingNoteTap.count = Math.min(state.pendingNoteTap.count + 1, 3);
    window.clearTimeout(state.pendingNoteTap.timerId);
    state.pendingNoteTap.timerId = window.setTimeout(() => {
      state.pendingNoteTap = null;
    }, noteTapDelayMs);
    setNote(layer, row, col, noteColorForTapCount(state.pendingNoteTap.count));
    return;
  }

  if (current) {
    clearPendingNoteTap();
    setNote(layer, row, col, null);
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
  setNote(layer, row, col, noteColorForTapCount(1));
}

function handleTopNotePressStart(event, layer, row, col) {
  event.preventDefault();
  event.stopPropagation();

  if (event.pointerType === "mouse" && event.button !== 0) {
    return;
  }

  clearPendingTopNotePress();
  const key = threeDKey(layer, row, col);
  const tileRect = event.currentTarget.closest(".three-d-face").getBoundingClientRect();
  state.pendingTopNotePress = {
    key,
    timerId: window.setTimeout(() => {
      suppressTopNoteClick(key);
      state.pendingTopNotePress = null;
      openCornerNoteMenu(layer, row, col, "top", tileRect, event.pointerId);
    }, cornerNotePressDelayMs),
  };
}

function handleTopNotePressEnd(event) {
  event?.stopPropagation?.();
  clearPendingTopNotePress();
}

function handleTopNoteClick(event, layer, row, col) {
  event.preventDefault();
  event.stopPropagation();

  const key = threeDKey(layer, row, col);
  if (shouldSuppressTopNoteClick(key)) {
    return;
  }

  clearPendingTopNotePress();
  handleNoteTap(layer, row, col);
}

function handleBottomNotePressStart(event, layer, row, col) {
  event.preventDefault();
  event.stopPropagation();

  if (event.pointerType === "mouse" && event.button !== 0) {
    return;
  }

  clearPendingBottomNotePress();
  const key = threeDKey(layer, row, col);
  const tileRect = event.currentTarget.closest(".three-d-face").getBoundingClientRect();
  state.pendingBottomNotePress = {
    key,
    timerId: window.setTimeout(() => {
      suppressBottomNoteClick(key);
      state.pendingBottomNotePress = null;
      openCornerNoteMenu(layer, row, col, "bottom", tileRect, event.pointerId);
    }, cornerNotePressDelayMs),
  };
}

function handleBottomNotePressEnd(event) {
  event?.stopPropagation?.();
  clearPendingBottomNotePress();
}

function handleBottomNoteClick(event, layer, row, col) {
  event.preventDefault();
  event.stopPropagation();

  const key = threeDKey(layer, row, col);
  if (shouldSuppressBottomNoteClick(key)) {
    return;
  }

  clearPendingBottomNotePress();

  if (state.bottomNotes.has(key)) {
    setBottomNote(layer, row, col, null);
  }
}

function openErrorModal(title, message) {
  closeCornerNoteMenu();
  closeGuessModal();
  errorTitleEl.textContent = title;
  errorMessageEl.textContent = message;
  errorModalEl.hidden = false;
}

function closeErrorModal() {
  errorModalEl.hidden = true;
}

function openNewPuzzleModal() {
  closeCornerNoteMenu();
  closeGuessModal();
  closeErrorModal();
  closeFinishModal();
  state.pendingNewBoardSize = { ...currentBoardSize() };
  populateNewPuzzleSelectors();
  newPuzzleModalEl.hidden = false;
}

function closeNewPuzzleModal() {
  state.newPuzzleDrag = null;
  newPuzzleModalEl.hidden = true;
}

function openGuessModal(layer, row, col) {
  closeCornerNoteMenu();
  const cell = state.cells[layer][row][col];
  state.modalCell = { layer, row, col };
  guessEmojiEl.textContent = emojiForCell(cell);
  guessTitleEl.textContent = cell.name;
  guessModalEl.hidden = false;
}

function closeGuessModal() {
  guessModalEl.hidden = true;
  state.modalCell = null;
}

function toggleClueVisibility(layer, row, col) {
  const key = threeDKey(layer, row, col);
  if (state.hiddenClues.has(key)) {
    state.hiddenClues.delete(key);
  } else {
    state.hiddenClues.add(key);
  }

  persistProgress();
}

function clueTextForCell(layer, row, col, cell) {
  if (!cellIsRevealed(cell) || !cell.clue) {
    return "";
  }

  return cell.clue;
}

function faceClassForCell(cell, key) {
  const classes = [];

  const flashHighlight = state.flashHighlights.find(
    (highlight) => highlight.kind === "cells" && highlight.keys.has(key),
  );
  if (flashHighlight) {
    classes.push(flashHighlight.color === "blue" ? "is-flashing-blue" : "is-flashing-gold");
  }

  if (cellIsRevealed(cell) && cell.revealed_answer) {
    classes.push(cell.revealed_answer === "innocent" ? "tile-innocent" : "tile-criminal");
  }

  return classes.join(" ");
}

function faceDirectionsForCell(layer, row, col) {
  return ["front", "back", "left", "right", "top", "bottom"];
}

function faceNormalForDirection(direction) {
  switch (direction) {
    case "front":
      return { x: 0, y: 0, z: 1 };
    case "back":
      return { x: 0, y: 0, z: -1 };
    case "left":
      return { x: -1, y: 0, z: 0 };
    case "right":
      return { x: 1, y: 0, z: 0 };
    case "top":
      return { x: 0, y: -1, z: 0 };
    case "bottom":
      return { x: 0, y: 1, z: 0 };
    default:
      return { x: 0, y: 0, z: 1 };
  }
}

function faceVisibilityScore(direction) {
  return rotatePoint(faceNormalForDirection(direction)).z;
}

function faceIsInteractive(direction) {
  return faceVisibilityScore(direction) > 0.001;
}

function faceAxesForDirection(direction) {
  switch (direction) {
    case "front":
      return {
        right: { x: 1, y: 0, z: 0 },
        down: { x: 0, y: 1, z: 0 },
      };
    case "back":
      return {
        right: { x: -1, y: 0, z: 0 },
        down: { x: 0, y: 1, z: 0 },
      };
    case "left":
      return {
        right: { x: 0, y: 0, z: 1 },
        down: { x: 0, y: 1, z: 0 },
      };
    case "right":
      return {
        right: { x: 0, y: 0, z: -1 },
        down: { x: 0, y: 1, z: 0 },
      };
    case "top":
      return {
        right: { x: 1, y: 0, z: 0 },
        down: { x: 0, y: 0, z: 1 },
      };
    case "bottom":
      return {
        right: { x: 1, y: 0, z: 0 },
        down: { x: 0, y: 0, z: -1 },
      };
    default:
      return {
        right: { x: 1, y: 0, z: 0 },
        down: { x: 0, y: 1, z: 0 },
      };
  }
}

function rotateBasisQuarterTurn(right, down, turns) {
  switch (turns % 4) {
    case 1:
      return {
        right: { x: down.x, y: down.y },
        down: { x: -right.x, y: -right.y },
      };
    case 2:
      return {
        right: { x: -right.x, y: -right.y },
        down: { x: -down.x, y: -down.y },
      };
    case 3:
      return {
        right: { x: -down.x, y: -down.y },
        down: { x: right.x, y: right.y },
      };
    default:
      return {
        right: { x: right.x, y: right.y },
        down: { x: down.x, y: down.y },
      };
  }
}

function textQuarterTurnsForFace(direction) {
  const axes = faceAxesForDirection(direction);
  const projectedRight = rotatePoint(axes.right, state.textRotationX, state.textRotationY);
  const projectedDown = rotatePoint(axes.down, state.textRotationX, state.textRotationY);
  let bestTurns = 0;
  let bestScore = -Infinity;

  for (let turns = 0; turns < 4; turns += 1) {
    const candidate = rotateBasisQuarterTurn(projectedRight, projectedDown, turns);
    const score = candidate.right.x + candidate.down.y;
    if (score > bestScore) {
      bestScore = score;
      bestTurns = turns;
    }
  }

  return bestTurns;
}

function faceTextTransform(direction) {
  return `rotate(${textQuarterTurnsForFace(direction) * 90}deg)`;
}

function createFace(direction, layer, row, col, cell) {
  const key = threeDKey(layer, row, col);
  const note = state.notes.get(key) ?? "none";
  const bottomNote = state.bottomNotes.get(key) ?? "none";
  const textTransform = faceTextTransform(direction);
  const face = document.createElement("article");
  face.className = `three-d-face face-${direction} ${faceClassForCell(cell, key)}`.trim();
  face.dataset.direction = direction;
  face.dataset.layer = String(layer);
  face.dataset.row = String(row);
  face.dataset.col = String(col);
  face.tabIndex = faceIsInteractive(direction) ? 0 : -1;
  face.setAttribute("role", "button");
  face.setAttribute("aria-label", `${cell.name}, ${cell.role}`);
  face.classList.toggle("is-interactive", faceIsInteractive(direction));

  const clueText = clueTextForCell(layer, row, col, cell);
  const clueClass =
    state.hiddenClues.has(threeDKey(layer, row, col))
      ? "cell-clue clue-hidden"
      : cell.is_nonsense
        ? "cell-clue is-nonsense"
        : "cell-clue";

  face.innerHTML = `
    <div class="three-d-face-content" style="transform: ${textTransform};">
      <div class="cell-top">
        <span class="cell-position">${positionLabel(layer, row, col)}</span>
      </div>
      <div class="cell-header">
        <span class="cell-emoji" aria-hidden="true">${emojiForCell(cell)}</span>
        <span class="cell-name">${cell.name}</span>
        <span class="cell-role">${cell.role}</span>
      </div>
      <p class="${clueClass}">${clueText}</p>
    </div>
    <div class="three-d-face-corners" style="transform: ${textTransform};">
      <button class="cell-note note-${note}" type="button" aria-label="${note === "none" ? "Add tile note" : `Clear ${note} note`}"></button>
      <button class="cell-note-secondary note-${bottomNote}" type="button" aria-label="${bottomNote === "none" ? "Add tile color note" : `Clear ${bottomNote} color note`}"></button>
    </div>
  `;

  const noteEl = face.querySelector(".cell-note");
  const bottomNoteEl = face.querySelector(".cell-note-secondary");
  noteEl.addEventListener("pointerdown", (event) => {
    handleTopNotePressStart(event, layer, row, col);
  });
  noteEl.addEventListener("pointerup", handleTopNotePressEnd);
  noteEl.addEventListener("pointercancel", handleTopNotePressEnd);
  noteEl.addEventListener("pointerleave", handleTopNotePressEnd);
  noteEl.addEventListener("click", (event) => {
    handleTopNoteClick(event, layer, row, col);
  });
  noteEl.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });
  bottomNoteEl.addEventListener("pointerdown", (event) => {
    handleBottomNotePressStart(event, layer, row, col);
  });
  bottomNoteEl.addEventListener("pointerup", handleBottomNotePressEnd);
  bottomNoteEl.addEventListener("pointercancel", handleBottomNotePressEnd);
  bottomNoteEl.addEventListener("pointerleave", handleBottomNotePressEnd);
  bottomNoteEl.addEventListener("click", (event) => {
    handleBottomNoteClick(event, layer, row, col);
  });
  bottomNoteEl.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });
  face.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" && event.key !== " ") {
      return;
    }

    event.preventDefault();
    interactWithFace(face);
  });

  return face;
}

function rotatePoint(point, rotationXDegrees = state.rotationX, rotationYDegrees = state.rotationY) {
  const rotationX = (rotationXDegrees * Math.PI) / 180;
  const rotationY = (rotationYDegrees * Math.PI) / 180;
  const cosY = Math.cos(rotationY);
  const sinY = Math.sin(rotationY);
  const cosX = Math.cos(rotationX);
  const sinX = Math.sin(rotationX);

  const xAfterY = point.x * cosY + point.z * sinY;
  const zAfterY = -point.x * sinY + point.z * cosY;
  const yAfterX = point.y * cosX - zAfterY * sinX;
  const zAfterX = point.y * sinX + zAfterY * cosX;

  return {
    x: xAfterY,
    y: yAfterX,
    z: zAfterX,
  };
}

function normalizeAngle(degrees) {
  let normalized = degrees % 360;
  if (normalized <= -180) {
    normalized += 360;
  } else if (normalized > 180) {
    normalized -= 360;
  }
  return normalized;
}

function updateSceneTransform() {
  const shellRect = boardShellEl.getBoundingClientRect();
  const viewportCenterX = window.innerWidth / 2;
  const viewportCenterY = window.innerHeight / 2;
  const shellCenterX = shellRect.left + shellRect.width / 2;
  const shellCenterY = shellRect.top + shellRect.height / 2;
  const screenCenterOffsetX = viewportCenterX - shellCenterX;
  const screenCenterOffsetY = viewportCenterY - shellCenterY;
  boardSceneEl.style.transform = `
    translateX(${screenCenterOffsetX}px)
    translateY(${screenCenterOffsetY}px)
    translateX(${state.viewOffsetX}px)
    translateY(${state.viewOffsetY}px)
    scale(${state.viewScale})
    rotateX(${state.rotationX}deg)
    rotateY(${state.rotationY}deg)
  `;
}

function cubeCornerPoints() {
  const { depth, rows, cols } = state.boardSize;
  const halfWidth = ((cols - 1) * state.sceneSpacing + state.sceneCellSize) / 2;
  const halfHeight = ((rows - 1) * state.sceneSpacing + state.sceneCellSize) / 2;
  const halfDepth = ((depth - 1) * state.sceneSpacing + state.sceneCellSize) / 2;
  const points = [];

  [-halfWidth, halfWidth].forEach((x) => {
    [-halfHeight, halfHeight].forEach((y) => {
      [-halfDepth, halfDepth].forEach((z) => {
        points.push({ x, y, z });
      });
    });
  });

  return points;
}

function projectScenePoint(point, rotationXDegrees, rotationYDegrees) {
  const rotated = rotatePoint(point, rotationXDegrees, rotationYDegrees);
  const perspectiveScale = scenePerspective / Math.max(160, scenePerspective - rotated.z);
  return {
    x: rotated.x * perspectiveScale,
    y: rotated.y * perspectiveScale,
    z: rotated.z,
  };
}

function projectedSceneBounds(rotationXDegrees, rotationYDegrees) {
  const corners = cubeCornerPoints().map((point) =>
    projectScenePoint(point, rotationXDegrees, rotationYDegrees),
  );
  const xs = corners.map((point) => point.x);
  const ys = corners.map((point) => point.y);
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minY = Math.min(...ys);
  const maxY = Math.max(...ys);

  return {
    width: maxX - minX,
    height: maxY - minY,
    centerX: (minX + maxX) / 2,
    centerY: (minY + maxY) / 2,
  };
}

function sceneVisualBounds() {
  const faces = [...boardSceneEl.querySelectorAll(".three-d-face.is-interactive")];
  if (faces.length === 0) {
    return null;
  }

  const rects = faces
    .map((face) => face.getBoundingClientRect())
    .filter((rect) => rect.width > 0 && rect.height > 0);
  if (rects.length === 0) {
    return null;
  }

  const left = Math.min(...rects.map((rect) => rect.left));
  const right = Math.max(...rects.map((rect) => rect.right));
  const top = Math.min(...rects.map((rect) => rect.top));
  const bottom = Math.max(...rects.map((rect) => rect.bottom));

  return {
    left,
    right,
    top,
    bottom,
    width: right - left,
    height: bottom - top,
    centerX: (left + right) / 2,
    centerY: (top + bottom) / 2,
  };
}

function stepViewCompensation() {
  const ease = 0.12;
  state.viewOffsetX += (state.targetViewOffsetX - state.viewOffsetX) * ease;
  state.viewOffsetY += (state.targetViewOffsetY - state.viewOffsetY) * ease;
  state.viewScale += (state.targetViewScale - state.viewScale) * ease;
  updateSceneTransform();

  const done =
    Math.abs(state.targetViewOffsetX - state.viewOffsetX) < 0.35 &&
    Math.abs(state.targetViewOffsetY - state.viewOffsetY) < 0.35 &&
    Math.abs(state.targetViewScale - state.viewScale) < 0.002;

  if (done) {
    state.viewOffsetX = state.targetViewOffsetX;
    state.viewOffsetY = state.targetViewOffsetY;
    state.viewScale = state.targetViewScale;
    state.viewCompensationFrame = null;
    updateSceneTransform();
    return;
  }

  state.viewCompensationFrame = window.requestAnimationFrame(stepViewCompensation);
}

function retargetViewCompensation({ immediate = false } = {}) {
  if (state.sceneCellSize === 0 || state.sceneSpacing === 0) {
    return;
  }

  const baselineBounds = projectedSceneBounds(defaultRotationX, defaultRotationY);
  const currentBounds = projectedSceneBounds(state.rotationX, state.rotationY);
  const baselineSize = Math.max(baselineBounds.width, baselineBounds.height, 1);
  const currentSize = Math.max(currentBounds.width, currentBounds.height, 1);
  const compensationScale = Math.max(0.82, Math.min(1.18, baselineSize / currentSize));
  const targetScale = compensationScale * state.userZoomScale;

  state.targetViewScale = targetScale;
  updateSceneTransform();
  const visualBounds = sceneVisualBounds();
  if (visualBounds) {
    const viewportCenterX = window.innerWidth / 2;
    const viewportCenterY = window.innerHeight / 2;
    state.targetViewOffsetX = state.viewOffsetX + (viewportCenterX - visualBounds.centerX);
    state.targetViewOffsetY = state.viewOffsetY + (viewportCenterY - visualBounds.centerY);
  } else {
    state.targetViewOffsetX = -currentBounds.centerX * targetScale;
    state.targetViewOffsetY = -currentBounds.centerY * targetScale;
  }

  if (immediate) {
    clearViewCompensationFrame();
    state.viewScale = state.targetViewScale;
    state.viewOffsetX = state.targetViewOffsetX;
    state.viewOffsetY = state.targetViewOffsetY;
    updateSceneTransform();
    return;
  }

  if (state.viewCompensationFrame === null) {
    state.viewCompensationFrame = window.requestAnimationFrame(stepViewCompensation);
  }
}

function projectAxisPoint(point, center, scale) {
  const rotated = rotatePoint(point);
  return {
    x: center + rotated.x * scale,
    y: center + rotated.y * scale,
    z: rotated.z,
  };
}

function lerpProjectedPoint(from, to, ratio) {
  return {
    x: from.x + (to.x - from.x) * ratio,
    y: from.y + (to.y - from.y) * ratio,
    z: from.z + (to.z - from.z) * ratio,
  };
}

function normalizeProjectedVector(dx, dy) {
  const length = Math.hypot(dx, dy) || 1;
  return { x: dx / length, y: dy / length };
}

function pointPastEndpoint(origin, endpoint, distance) {
  const unit = normalizeProjectedVector(endpoint.x - origin.x, endpoint.y - origin.y);
  return {
    x: endpoint.x + unit.x * distance,
    y: endpoint.y + unit.y * distance,
    z: endpoint.z,
  };
}

function offsetPointPerpendicular(from, to, point, distance, sign) {
  const unit = normalizeProjectedVector(to.x - from.x, to.y - from.y);
  return {
    x: point.x + -unit.y * distance * sign,
    y: point.y + unit.x * distance * sign,
    z: point.z,
  };
}

function labelAnchor(point, center) {
  if (point.x > center + 6) {
    return "start";
  }
  if (point.x < center - 6) {
    return "end";
  }
  return "middle";
}

function labelOffset(point, center) {
  if (point.y > center + 4) {
    return 10;
  }
  if (point.y < center - 4) {
    return -6;
  }
  return 4;
}

function axisTickSamples(values, maxLabels = 4) {
  if (values.length <= maxLabels) {
    return values.map((value, index) => ({ value, index }));
  }

  const indices = new Set();
  for (let sampleIndex = 0; sampleIndex < maxLabels; sampleIndex += 1) {
    const ratio = sampleIndex / (maxLabels - 1);
    indices.add(Math.round(ratio * (values.length - 1)));
  }

  return [...indices]
    .sort((left, right) => left - right)
    .map((index) => ({ value: values[index], index }));
}

function updateAxisGizmo() {
  if (!axisGizmoEl) {
    return;
  }

  const center = 60;
  const scale = 27;
  const origin = projectAxisPoint({ x: 0, y: 0, z: 0 }, center, scale);
  const endpoints = {
    right: projectAxisPoint({ x: 1, y: 0, z: 0 }, center, scale),
    up: projectAxisPoint({ x: 0, y: -1, z: 0 }, center, scale),
    back: projectAxisPoint({ x: 0, y: 0, z: 1 }, center, scale),
  };
  const axes = [
    {
      key: "x",
      positive: endpoints.right,
      labelText: "Right",
      values: Array.from({ length: state.boardSize.cols }, (_, index) => colLabel(index)),
      tickSign: -1,
    },
    {
      key: "y",
      positive: endpoints.up,
      labelText: "Up",
      values: Array.from({ length: state.boardSize.rows }, (_, index) => String(index + 1)),
      tickSign: 1,
    },
    {
      key: "z",
      positive: endpoints.back,
      labelText: "Back",
      values: Array.from({ length: state.boardSize.depth }, (_, index) => `L${index + 1}`),
      tickSign: -1,
    },
  ];
  const axisSegments = axes
    .map((axis) => ({ key: axis.key, from: origin, to: axis.positive, z: (origin.z + axis.positive.z) / 2 }))
    .sort((left, right) => left.z - right.z);
  const directionLabels = axes
    .map((axis) => ({
      axis: axis.key,
      text: axis.labelText,
      point: pointPastEndpoint(origin, axis.positive, 12),
    }))
    .sort((left, right) => left.point.z - right.point.z);
  const tickLabels = axes
    .flatMap((axis) => {
      const sampledTicks = axisTickSamples(axis.values);
      return sampledTicks.map(({ value, index }) => {
        const ratio = (index + 1) / (axis.values.length + 1);
        const axisPoint = lerpProjectedPoint(origin, axis.positive, ratio);
        return {
          axis: axis.key,
          text: value,
          axisPoint,
          point: offsetPointPerpendicular(origin, axis.positive, axisPoint, 7, axis.tickSign),
        };
      });
    })
    .sort((left, right) => left.axisPoint.z - right.axisPoint.z);

  axisGizmoEl.innerHTML = `
    ${axisSegments
      .map(
        (axis) => `
          <line
            class="axis-gizmo-line axis-${axis.key}"
            x1="${axis.from.x.toFixed(2)}"
            y1="${axis.from.y.toFixed(2)}"
            x2="${axis.to.x.toFixed(2)}"
            y2="${axis.to.y.toFixed(2)}"
          />
          <circle
            class="axis-gizmo-endpoint axis-${axis.key}"
            cx="${axis.from.x.toFixed(2)}"
            cy="${axis.from.y.toFixed(2)}"
            r="2.15"
          />
          <circle
            class="axis-gizmo-endpoint axis-${axis.key}"
            cx="${axis.to.x.toFixed(2)}"
            cy="${axis.to.y.toFixed(2)}"
            r="2.15"
          />
        `,
      )
      .join("")}
    <circle class="axis-gizmo-center" cx="${center}" cy="${center}" r="2.8" />
    ${tickLabels
      .map(
        (label) => `
          <circle
            class="axis-gizmo-tick axis-${label.axis}"
            cx="${label.axisPoint.x.toFixed(2)}"
            cy="${label.axisPoint.y.toFixed(2)}"
            r="1.4"
          />
          <text
            class="axis-gizmo-tick-label axis-${label.axis}"
            x="${label.point.x.toFixed(2)}"
            y="${label.point.y.toFixed(2)}"
            text-anchor="middle"
          >${label.text}</text>
        `,
      )
      .join("")}
    ${directionLabels
      .map((label) => {
        const anchor = labelAnchor(label.point, center);
        const dx = anchor === "start" ? 4 : anchor === "end" ? -4 : 0;
        const dy = labelOffset(label.point, center);
        return `
          <text
            class="axis-gizmo-direction-label axis-${label.axis}"
            x="${(label.point.x + dx).toFixed(2)}"
            y="${(label.point.y + dy).toFixed(2)}"
            text-anchor="${anchor}"
          >${label.text}</text>
        `;
      })
      .join("")}
  `;
}

function updateFaceTextOrientations() {
  document.querySelectorAll(".three-d-face").forEach((face) => {
    const direction = face.dataset.direction;
    if (!direction) {
      return;
    }

    const transform = faceTextTransform(direction);
    face.querySelector(".three-d-face-content")?.style.setProperty("transform", transform);
    face.querySelector(".three-d-face-corners")?.style.setProperty("transform", transform);
  });
}

function updateInteractiveFaces() {
  document.querySelectorAll(".three-d-face").forEach((face) => {
    const direction = face.dataset.direction;
    if (!direction) {
      return;
    }

    const interactive = faceIsInteractive(direction);
    face.classList.toggle("is-interactive", interactive);
    face.tabIndex = interactive ? 0 : -1;
  });
}

function faceAtClientPoint(clientX, clientY) {
  const elements = document.elementsFromPoint(clientX, clientY);
  return (
    elements.find(
      (element) =>
        element instanceof HTMLElement && element.matches(".three-d-face.is-interactive"),
    ) ?? null
  );
}

function scheduleTextReorientation(delayMs = textReorientationDelayMs) {
  clearTextReorientationTimer();
  state.textReorientationTimer = window.setTimeout(() => {
    state.textReorientationTimer = null;
    state.textRotationX = state.rotationX;
    state.textRotationY = state.rotationY;
    updateFaceTextOrientations();
    persistProgress();
  }, delayMs);
}

function createHighlightPrism(bounds, color, cellSize, spacing, halfX, halfY, halfZ) {
  const prismPadding = Math.max(6, Math.round(cellSize * 0.08));
  const minX = (bounds.minCol - halfX) * spacing - cellSize / 2 - prismPadding;
  const maxX = (bounds.maxCol - halfX) * spacing + cellSize / 2 + prismPadding;
  const minY = (bounds.minRow - halfY) * spacing - cellSize / 2 - prismPadding;
  const maxY = (bounds.maxRow - halfY) * spacing + cellSize / 2 + prismPadding;
  const minZ = (bounds.minLayer - halfZ) * spacing - cellSize / 2 - prismPadding;
  const maxZ = (bounds.maxLayer - halfZ) * spacing + cellSize / 2 + prismPadding;
  const width = maxX - minX;
  const height = maxY - minY;
  const depth = maxZ - minZ;
  const centerX = (minX + maxX) / 2;
  const centerY = (minY + maxY) / 2;
  const centerZ = (minZ + maxZ) / 2;
  const prism = document.createElement("div");
  prism.className = `three-d-highlight-prism prism-${color}`;
  prism.style.setProperty("--prism-width", `${width}px`);
  prism.style.setProperty("--prism-height", `${height}px`);
  prism.style.setProperty("--prism-depth", `${depth}px`);
  prism.style.transform = `translate3d(${centerX}px, ${centerY}px, ${centerZ}px)`;
  prism.innerHTML = `
    <div class="three-d-highlight-face prism-${color} face-front"></div>
    <div class="three-d-highlight-face prism-${color} face-back"></div>
    <div class="three-d-highlight-face prism-${color} face-left"></div>
    <div class="three-d-highlight-face prism-${color} face-right"></div>
    <div class="three-d-highlight-face prism-${color} face-top"></div>
    <div class="three-d-highlight-face prism-${color} face-bottom"></div>
  `;
  return prism;
}

function renderBoard() {
  boardSceneEl.replaceChildren();

  const { depth, rows, cols } = state.boardSize;
  const shellRect = boardShellEl.getBoundingClientRect();
  const sceneSize = Math.min(shellRect.width, shellRect.height);
  const extent = Math.max(depth, rows, cols);
  const isCompactMobileLayout = window.innerWidth <= 720;
  const fitRatio = isCompactMobileLayout
    ? extent <= 2
      ? 0.82
      : extent === 3
        ? 0.74
        : 0.66
    : 0.62;
  const minCellSize = isCompactMobileLayout ? 76 : 84;
  const maxCellSize = isCompactMobileLayout ? 170 : 156;
  const cellSize = Math.max(
    minCellSize,
    Math.min(maxCellSize, Math.floor((sceneSize * fitRatio) / extent)),
  );
  const typeScale = isCompactMobileLayout
    ? Math.max(0.56, Math.min(1, cellSize / 165 - Math.max(0, extent - 2) * 0.12))
    : Math.max(0.82, Math.min(1, cellSize / 165));
  const spacing = cellSize + Math.max(
    isCompactMobileLayout ? 6 : 10,
    Math.floor(cellSize * (isCompactMobileLayout ? 0.08 : 0.12)),
  );
  const halfX = (cols - 1) / 2;
  const halfY = (rows - 1) / 2;
  const halfZ = (depth - 1) / 2;

  boardSceneEl.style.setProperty("--three-d-cell-size", `${cellSize}px`);
  boardSceneEl.style.setProperty("--three-d-type-scale", String(typeScale));
  state.sceneCellSize = cellSize;
  state.sceneSpacing = spacing;
  updateSceneTransform();

  state.cells.forEach((layerCells, layer) => {
    layerCells.forEach((rowCells, row) => {
      rowCells.forEach((cell, col) => {
        const cellEl = document.createElement("div");
        cellEl.className = "three-d-cell";
        cellEl.style.width = `${cellSize}px`;
        cellEl.style.height = `${cellSize}px`;
        cellEl.style.transform = `translate3d(${(col - halfX) * spacing}px, ${(row - halfY) * spacing}px, ${(layer - halfZ) * spacing}px)`;

        faceDirectionsForCell(layer, row, col).forEach((direction) => {
          cellEl.append(createFace(direction, layer, row, col, cell));
        });

        boardSceneEl.append(cellEl);
      });
    });
  });

  state.flashHighlights
    .filter((highlight) => highlight.kind === "prism")
    .forEach((highlight) => {
      boardSceneEl.append(
        createHighlightPrism(
        highlight.bounds,
        highlight.color,
        cellSize,
        spacing,
        halfX,
        halfY,
        halfZ,
        ),
      );
    });

  updateAxisGizmo();
  updateInteractiveFaces();
}

function updateUrlAndState(seed, puzzle, storedPuzzleId = null) {
  clearPendingNoteTap();
  clearPendingTopNotePress();
  clearPendingBottomNotePress();
  clearTextReorientationTimer();
  closeCornerNoteMenu();
  window.clearTimeout(state.flashTimer);
  state.currentSeed = seed;
  state.currentStoredPuzzleId =
    normalizeStoredPuzzleId(puzzle.stored_puzzle_id) ?? normalizeStoredPuzzleId(storedPuzzleId);
  state.sharedStoredPuzzleId =
    state.currentStoredPuzzleId ?? readSharedStoredPuzzleId({
      depth: puzzle.depth,
      rows: puzzle.rows,
      cols: puzzle.cols,
    });
  state.shareLinkPromise = null;
  state.boardSize = {
    depth: puzzle.depth,
    rows: puzzle.rows,
    cols: puzzle.cols,
  };
  state.cells = puzzle.cells;
  state.moves = [];
  state.notes = new Map();
  state.bottomNotes = new Map();
  state.hiddenClues = new Set();
  state.flashHighlights = [];
  state.flashTimer = null;
  state.textRotationX = state.rotationX;
  state.textRotationY = state.rotationY;
  clearViewCompensationFrame();
  clearPinchState();
  state.viewOffsetX = 0;
  state.viewOffsetY = 0;
  state.viewScale = 1;
  state.userZoomScale = 1;
  state.targetViewOffsetX = 0;
  state.targetViewOffsetY = 0;
  state.targetViewScale = 1;
  if (state.currentStoredPuzzleId !== null) {
    updateUrlStoredPuzzle(state.currentStoredPuzzleId);
  } else if (seed !== null) {
    updateUrlGeneratedPuzzle(seed, state.boardSize);
  }
  renderBoard();
  retargetViewCompensation({ immediate: true });
}

async function applySavedMove(layer, row, col, guess) {
  const response = await fetch("/api/3d/puzzles/guess", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      source: currentPuzzleSource(),
      moves: state.moves,
      layer,
      row,
      col,
      guess,
    }),
  });

  if (!response.ok) {
    throw new Error("Saved progress could not be restored.");
  }

  const result = await response.json();
  applyAcceptedGuess(layer, row, col, guess, result);
}

function applyAcceptedGuess(layer, row, col, guess, clueResult) {
  const key = threeDKey(layer, row, col);
  const cell = state.cells[layer]?.[row]?.[col];
  if (!cell) {
    throw new Error("Saved progress referenced a missing cell.");
  }

  state.hiddenClues.delete(key);
  state.notes.delete(key);
  state.bottomNotes.delete(key);
  state.moves = state.moves.filter(
    (move) => move.layer !== layer || move.row !== row || move.col !== col,
  );
  state.moves.push({ layer, row, col, guess });
  cell.revealed = true;
  cell.revealed_answer = guess;
  cell.clue = clueResult.clue;
  cell.clue_highlight_groups = Array.isArray(clueResult.clue_highlight_groups)
    ? clueResult.clue_highlight_groups
    : [];
  cell.is_nonsense = clueResult.is_nonsense;
}

async function restoreProgress() {
  const saved = readSavedProgress();
  if (!saved) {
    return { restored: false, error: null };
  }

  try {
    for (const move of saved.moves) {
      const cell = state.cells[move.layer]?.[move.row]?.[move.col];
      if (!cell || cell.revealed) {
        continue;
      }

      await applySavedMove(move.layer, move.row, move.col, move.guess);
    }

    state.hiddenClues = new Set(
      saved.hiddenClues.filter((key) => {
        const [layer, row, col] = key.split(":").map(Number);
        return Boolean(state.cells[layer]?.[row]?.[col]?.clue);
      }),
    );
    state.notes = new Map(
      saved.notes.filter(([key]) => {
        const [layer, row, col] = key.split(":").map(Number);
        return Boolean(state.cells[layer]?.[row]?.[col]);
      }),
    );
    state.bottomNotes = new Map(
      saved.bottomNotes.filter(([key]) => {
        const [layer, row, col] = key.split(":").map(Number);
        return Boolean(state.cells[layer]?.[row]?.[col]);
      }),
    );
    state.timerElapsedMs = saved.timerElapsedMs;
    state.timerStartedAt = null;
    state.timerCompletedAt = saved.timerCompletedAt;
    state.completionAcknowledged = saved.completionAcknowledged;
    if (saved.view) {
      state.rotationX = saved.view.rotationX;
      state.rotationY = saved.view.rotationY;
      state.textRotationX =
        saved.view.textRotationX === null ? saved.view.rotationX : saved.view.textRotationX;
      state.textRotationY =
        saved.view.textRotationY === null ? saved.view.rotationY : saved.view.textRotationY;
      state.userZoomScale = saved.view.userZoomScale;
    }
    persistProgress();
    renderBoard();
    retargetViewCompensation({ immediate: true });
  } catch {
    state.moves = [];
    state.notes = new Map();
    state.bottomNotes = new Map();
    state.hiddenClues = new Set();
    state.timerElapsedMs = 0;
    state.timerStartedAt = null;
    state.timerCompletedAt = null;
    state.completionAcknowledged = false;
    clearSavedProgress();
    renderBoard();
    return {
      restored: false,
      error: "Saved 3D progress could not be restored and was cleared.",
    };
  }

  return { restored: true, error: null };
}

async function loadPuzzle(seed = null, requestedBoardSize = boardSizeFromUrl(), storedPuzzleId = null) {
  const boardSize = requestedBoardSize;
  const normalizedStoredPuzzleId = normalizeStoredPuzzleId(storedPuzzleId);
  let puzzle;

  if (normalizedStoredPuzzleId !== null) {
    puzzle = await fetchJson(`/api/3d/stored-puzzles/${encodeURIComponent(normalizedStoredPuzzleId)}`, {
      method: "GET",
    });
  } else {
    const params = new URLSearchParams({
      depth: String(boardSize.depth),
      rows: String(boardSize.rows),
      cols: String(boardSize.cols),
    });

    if (seed) {
      params.set("seed", seed);
    }

    puzzle = await fetchJson(`/api/3d/puzzles/new?${params.toString()}`, {
      method: "GET",
    });
  }
  updateUrlAndState(normalizeSeed(puzzle.seed), puzzle, normalizedStoredPuzzleId);
  state.timerElapsedMs = 0;
  state.timerStartedAt = null;
  state.timerCompletedAt = null;
  state.completionAcknowledged = false;
  resetShareButton();
  resetFinishCopyButton();
  closeFinishModal();
  const restoreResult = await restoreProgress();
  syncShareButton();
  if (restoreResult?.error) {
    return restoreResult;
  }

  if (allTilesMarked() && state.timerCompletedAt !== null && !state.completionAcknowledged) {
    openFinishModal();
  } else {
    resumeActiveTimerIfNeeded();
  }

  return restoreResult;
}

async function clearBoard() {
  closeCornerNoteMenu();
  closeGuessModal();
  closeErrorModal();
  closeNewPuzzleModal();
  closeFinishModal();
  clearSavedProgress();

  if (state.currentStoredPuzzleId !== null) {
    await loadPuzzle(null, state.boardSize, state.currentStoredPuzzleId);
    return;
  }

  if (state.currentSeed === null) {
    return;
  }

  await loadPuzzle(state.currentSeed, state.boardSize, null);
}

async function revealGuess(answer) {
  if (!state.modalCell) {
    return;
  }

  const { layer, row, col } = state.modalCell;
  const response = await fetch("/api/3d/puzzles/guess", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      source: currentPuzzleSource(),
      moves: state.moves,
      layer,
      row,
      col,
      guess: answer,
    }),
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: "Failed to validate mark" }));
    const result = new Error(error.error || "Failed to validate mark");
    result.status = response.status;
    result.guess = answer;
    throw result;
  }

  const result = await response.json();
  if (!hasStartedTimer()) {
    state.timerElapsedMs = 0;
    state.timerStartedAt = Date.now();
    state.timerCompletedAt = null;
    state.completionAcknowledged = false;
  } else if (state.timerStartedAt === null && state.timerCompletedAt === null) {
    state.timerStartedAt = Date.now();
  }
  applyAcceptedGuess(layer, row, col, answer, result);
  persistProgress();
  closeGuessModal();
  renderBoard();
  completePuzzleIfNeeded();
}

function showGuessError(error) {
  if (error?.status === 400 || error?.status === 409) {
    const { layer, row, col } = state.modalCell ?? {};
    const cell =
      Number.isInteger(layer) && Number.isInteger(row) && Number.isInteger(col)
        ? state.cells[layer]?.[row]?.[col]
        : null;
    const detail =
      cell && error?.guess
        ? `${cell.name} can't be logically identified as ${error.guess} from the available info.`
        : "";
    openErrorModal(invalidMoveMessage, detail);
    return;
  }

  openErrorModal("Guess rejected.", error.message);
}

function resetView() {
  state.rotationX = defaultRotationX;
  state.rotationY = defaultRotationY;
  state.textRotationX = state.rotationX;
  state.textRotationY = state.rotationY;
  state.userZoomScale = 1;
  clearPinchState();
  clearTextReorientationTimer();
  clearWheelInteractionTimer();
  clearViewCompensationFrame();
  renderBoard();
  retargetViewCompensation({ immediate: true });
  persistProgress();
}

function handleTrackpadRotate(event) {
  if (event.defaultPrevented) {
    return;
  }

  if (event.ctrlKey) {
    event.preventDefault();
    startBoardZoomInteraction();
    const zoomFactor = Math.exp(-event.deltaY * 0.0025);
    applyBoardZoom(state.userZoomScale * zoomFactor);
    finishBoardZoomInteraction();
    return;
  }

  event.preventDefault();
  clearTextReorientationTimer();
  clearWheelInteractionTimer();
  clearViewCompensationFrame();

  const deltaScale = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? 48 : 1;
  const deltaX = Math.max(-90, Math.min(90, event.deltaX * deltaScale));
  const deltaY = Math.max(-90, Math.min(90, event.deltaY * deltaScale));

  state.rotationY = normalizeAngle(state.rotationY + deltaX * 0.18);
  state.rotationX = normalizeAngle(state.rotationX - deltaY * 0.18);
  updateSceneTransform();
  updateAxisGizmo();
  updateInteractiveFaces();

  state.wheelInteractionTimer = window.setTimeout(() => {
    state.wheelInteractionTimer = null;
    scheduleTextReorientation();
    retargetViewCompensation();
  }, 90);
}

function handleGestureStart(event) {
  if (event.defaultPrevented) {
    return;
  }

  event.preventDefault();
  state.gestureZoomStartScale = state.userZoomScale;
  startBoardZoomInteraction();
}

function handleGestureChange(event) {
  if (event.defaultPrevented) {
    return;
  }

  if (state.gestureZoomStartScale === null) {
    return;
  }

  event.preventDefault();
  applyBoardZoom(state.gestureZoomStartScale * event.scale);
}

function handleGestureEnd(event) {
  if (event.defaultPrevented) {
    return;
  }

  if (state.gestureZoomStartScale === null) {
    return;
  }

  event.preventDefault();
  state.gestureZoomStartScale = null;
  finishBoardZoomInteraction();
}

function updateTouchPoint(event) {
  state.activeTouchPoints.set(event.pointerId, { x: event.clientX, y: event.clientY });
}

function pinchDistance(pointerA, pointerB) {
  return Math.hypot(pointerA.x - pointerB.x, pointerA.y - pointerB.y);
}

function activePinchPointers() {
  if (!state.pinch) {
    return null;
  }

  const first = state.activeTouchPoints.get(state.pinch.pointerIds[0]);
  const second = state.activeTouchPoints.get(state.pinch.pointerIds[1]);
  return first && second ? [first, second] : null;
}

function maybeStartTouchPinch(event) {
  if (event.pointerType !== "touch") {
    return false;
  }

  updateTouchPoint(event);
  if (state.activeTouchPoints.size < 2) {
    return false;
  }

  const pointerIds = [...state.activeTouchPoints.keys()].slice(0, 2);
  const first = state.activeTouchPoints.get(pointerIds[0]);
  const second = state.activeTouchPoints.get(pointerIds[1]);
  if (!first || !second) {
    return false;
  }

  event.preventDefault();
  if (state.drag) {
    boardShellEl.releasePointerCapture?.(state.drag.pointerId);
    state.drag = null;
  }
  clearTextReorientationTimer();
  startBoardZoomInteraction();
  state.pinch = {
    pointerIds,
    startDistance: Math.max(8, pinchDistance(first, second)),
    startScale: state.userZoomScale,
  };
  return true;
}

function maybeUpdateTouchPinch(event) {
  if (!state.pinch || !state.activeTouchPoints.has(event.pointerId)) {
    return false;
  }

  updateTouchPoint(event);
  const pinchPointers = activePinchPointers();
  if (!pinchPointers) {
    return false;
  }

  event.preventDefault();
  const [first, second] = pinchPointers;
  const nextDistance = Math.max(8, pinchDistance(first, second));
  applyBoardZoom(state.pinch.startScale * (nextDistance / state.pinch.startDistance));
  return true;
}

function maybeEndTouchPinch(event) {
  if (event.pointerType !== "touch") {
    return false;
  }

  const wasTracking = state.activeTouchPoints.delete(event.pointerId);
  if (!state.pinch) {
    return wasTracking;
  }

  if (!state.pinch.pointerIds.includes(event.pointerId)) {
    return wasTracking;
  }

  event.preventDefault();
  state.pinch = null;
  state.gestureZoomStartScale = null;
  finishBoardZoomInteraction();
  return true;
}

function handleDocumentGestureStart(event) {
  handleGestureStart(event);
}

function handleDocumentGestureChange(event) {
  handleGestureChange(event);
}

function handleDocumentGestureEnd(event) {
  handleGestureEnd(event);
}

function handleDocumentPinchZoom(event) {
  if (!event.ctrlKey || !eventTargetsBoard(event)) {
    return;
  }

  handleTrackpadRotate(event);
}

function interactWithFace(face) {
  if (!face) {
    return;
  }

  closeCornerNoteMenu();
  const layer = Number.parseInt(face.dataset.layer, 10);
  const row = Number.parseInt(face.dataset.row, 10);
  const col = Number.parseInt(face.dataset.col, 10);
  const cell = state.cells[layer]?.[row]?.[col];
  if (!cell) {
    return;
  }

  if (cellIsRevealed(cell)) {
    toggleClueVisibility(layer, row, col);
    if (cell.clue) {
      flashMentionedCells(cell.clue, cell.clue_highlight_groups);
    } else {
      renderBoard();
    }
    return;
  }

  openGuessModal(layer, row, col);
}

function beginDrag(event) {
  if (event.target.closest?.(".three-d-reset-view")) {
    return;
  }

  if (maybeStartTouchPinch(event)) {
    return;
  }

  if (state.pinch) {
    return;
  }

  event.preventDefault();
  clearTextReorientationTimer();
  clearViewCompensationFrame();
  const targetFace =
    event.target.closest?.(".three-d-face.is-interactive") ??
    faceAtClientPoint(event.clientX, event.clientY) ??
    null;
  state.drag = {
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    startRotationX: state.rotationX,
    startRotationY: state.rotationY,
    moved: false,
    face: targetFace,
  };
  boardShellEl.setPointerCapture?.(event.pointerId);
}

function updateDrag(event) {
  if (maybeUpdateTouchPinch(event)) {
    return;
  }

  if (!state.drag || state.drag.pointerId !== event.pointerId) {
    return;
  }

  const deltaX = event.clientX - state.drag.startX;
  const deltaY = event.clientY - state.drag.startY;
  if (Math.abs(deltaX) + Math.abs(deltaY) > 6) {
    state.drag.moved = true;
  }

  state.rotationY = normalizeAngle(state.drag.startRotationY + deltaX * 0.36);
  state.rotationX = normalizeAngle(state.drag.startRotationX - deltaY * 0.32);
  updateSceneTransform();
  updateAxisGizmo();
  updateInteractiveFaces();
}

function endDrag(event) {
  if (maybeEndTouchPinch(event)) {
    return;
  }

  if (!state.drag || state.drag.pointerId !== event.pointerId) {
    return;
  }

  const { moved, face } = state.drag;
  state.drag = null;
  boardShellEl.releasePointerCapture?.(event.pointerId);

  if (moved) {
    scheduleTextReorientation();
    retargetViewCompensation();
    persistProgress();
  }

  if (event.type !== "pointercancel" && !moved && face) {
    interactWithFace(face);
  }
}

function bindEvents() {
  shareButton.addEventListener("click", async () => {
    if (state.timerCompletedAt !== null) {
      openFinishModal();
      return;
    }

    try {
      await navigator.clipboard.writeText(await ensureShareableLink());
      flashShareButton("Copied");
    } catch (error) {
      flashShareButton("Failed");
      openErrorModal("Could not copy the link.", error.message);
    }
  });
  clearButton.addEventListener("click", async () => {
    try {
      await clearBoard();
    } catch (error) {
      openErrorModal("Could not clear the puzzle.", error.message);
    }
  });
  newRandomButton.addEventListener("click", openNewPuzzleModal);
  newPuzzleBackdropEl.addEventListener("click", closeNewPuzzleModal);
  newPuzzleCancelButton.addEventListener("click", closeNewPuzzleModal);
  newPuzzleDepthUpButton.addEventListener("click", () => stepNewPuzzleDimension("depth", 1));
  newPuzzleDepthDownButton.addEventListener("click", () => stepNewPuzzleDimension("depth", -1));
  newPuzzleRowsUpButton.addEventListener("click", () => stepNewPuzzleDimension("rows", 1));
  newPuzzleRowsDownButton.addEventListener("click", () => stepNewPuzzleDimension("rows", -1));
  newPuzzleColsUpButton.addEventListener("click", () => stepNewPuzzleDimension("cols", 1));
  newPuzzleColsDownButton.addEventListener("click", () => stepNewPuzzleDimension("cols", -1));
  newPuzzleDepthValueEl.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    beginNewPuzzleDrag("depth", event);
  });
  newPuzzleRowsValueEl.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    beginNewPuzzleDrag("rows", event);
  });
  newPuzzleColsValueEl.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    beginNewPuzzleDrag("cols", event);
  });
  [newPuzzleDepthValueEl, newPuzzleRowsValueEl, newPuzzleColsValueEl].forEach((element) => {
    element.addEventListener("pointermove", updateNewPuzzleDrag);
    element.addEventListener("pointerup", endNewPuzzleDrag);
    element.addEventListener("pointercancel", endNewPuzzleDrag);
  });
  newPuzzleDepthValueEl.addEventListener("keydown", (event) => {
    if (event.key === "ArrowUp") {
      event.preventDefault();
      stepNewPuzzleDimension("depth", 1);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      stepNewPuzzleDimension("depth", -1);
    }
  });
  newPuzzleRowsValueEl.addEventListener("keydown", (event) => {
    if (event.key === "ArrowUp") {
      event.preventDefault();
      stepNewPuzzleDimension("rows", 1);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      stepNewPuzzleDimension("rows", -1);
    }
  });
  newPuzzleColsValueEl.addEventListener("keydown", (event) => {
    if (event.key === "ArrowUp") {
      event.preventDefault();
      stepNewPuzzleDimension("cols", 1);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      stepNewPuzzleDimension("cols", -1);
    }
  });
  newPuzzleConfirmButton.addEventListener("click", async () => {
    closeNewPuzzleModal();
    try {
      const restoreResult = await loadPuzzle(null, currentNewPuzzleBoardSize(), null);
      if (restoreResult?.error) {
        openErrorModal("Saved progress was cleared.", restoreResult.error);
      }
    } catch (error) {
      openErrorModal("Could not load puzzle.", error.message);
    }
  });
  resetViewButton.addEventListener("click", resetView);
  boardShellEl.addEventListener("wheel", handleTrackpadRotate, { passive: false });
  boardShellEl.addEventListener("gesturestart", handleGestureStart, { passive: false });
  boardShellEl.addEventListener("gesturechange", handleGestureChange, { passive: false });
  boardShellEl.addEventListener("gestureend", handleGestureEnd, { passive: false });
  boardShellEl.addEventListener("pointerdown", beginDrag);
  boardShellEl.addEventListener("dragstart", (event) => {
    event.preventDefault();
  });
  boardShellEl.addEventListener("pointermove", updateDrag);
  boardShellEl.addEventListener("pointerup", endDrag);
  boardShellEl.addEventListener("pointercancel", endDrag);
  window.addEventListener("resize", () => {
    renderBoard();
    retargetViewCompensation({ immediate: true });
  });
  window.addEventListener("resize", closeCornerNoteMenu);
  window.addEventListener("scroll", closeCornerNoteMenu, { passive: true });
  window.addEventListener("pointermove", (event) => {
    if (!state.cornerNoteMenu || state.cornerNoteMenu.pointerId !== event.pointerId) {
      return;
    }

    updateCornerNoteMenuActiveColor(cornerNoteColorAtClientY(event.clientY));
  });
  window.addEventListener(
    "pointerup",
    (event) => {
      if (!state.cornerNoteMenu || state.cornerNoteMenu.pointerId !== event.pointerId) {
        return;
      }

      finalizeCornerNoteMenu();
    },
    true,
  );
  window.addEventListener(
    "pointercancel",
    (event) => {
      if (!state.cornerNoteMenu || state.cornerNoteMenu.pointerId !== event.pointerId) {
        return;
      }

      closeCornerNoteMenu();
    },
    true,
  );
  guessBackdropEl.addEventListener("click", closeGuessModal);
  guessCancelButton.addEventListener("click", closeGuessModal);
  errorBackdropEl.addEventListener("click", closeErrorModal);
  errorDismissButton.addEventListener("click", closeErrorModal);
  finishBackdropEl.addEventListener("click", dismissFinishModal);
  finishDismissButton.addEventListener("click", dismissFinishModal);
  finishCopyButton.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(await finishShareText());
      flashFinishCopyButton("Copied");
    } catch (error) {
      flashFinishCopyButton("Failed");
      openErrorModal("Could not copy the results.", error.message);
    }
  });
  guessInnocentButton.addEventListener("click", () => {
    void revealGuess("innocent").catch(showGuessError);
  });
  guessCriminalButton.addEventListener("click", () => {
    void revealGuess("criminal").catch(showGuessError);
  });
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      pauseActiveTimer();
    } else {
      resumeActiveTimerIfNeeded();
    }
  });
  document.addEventListener("wheel", handleDocumentPinchZoom, {
    capture: true,
    passive: false,
  });
  document.addEventListener("gesturestart", handleDocumentGestureStart, {
    capture: true,
    passive: false,
  });
  document.addEventListener("gesturechange", handleDocumentGestureChange, {
    capture: true,
    passive: false,
  });
  document.addEventListener("gestureend", handleDocumentGestureEnd, {
    capture: true,
    passive: false,
  });
  window.addEventListener("pagehide", () => {
    pauseActiveTimer();
    persistProgress();
  });
  document.addEventListener(
    "dblclick",
    (event) => {
      if (!(event.target instanceof Element)) {
        return;
      }

      if (!event.target.closest("button, input, select, textarea, [role='button'], a, label")) {
        return;
      }

      event.preventDefault();
    },
    { passive: false },
  );
}

bindEvents();
void loadPuzzle(currentSeedFromUrl(), boardSizeFromUrl(), currentStoredPuzzleIdFromUrl())
  .then((restoreResult) => {
    if (restoreResult?.error) {
      openErrorModal("Saved progress was cleared.", restoreResult.error);
    }
  })
  .catch((error) => {
    openErrorModal("Could not load puzzle.", error.message);
  });
