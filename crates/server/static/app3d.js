const boardMetaEl = document.querySelector("#board-3d-meta");
const statusEl = document.querySelector("#three-d-status");
const boardShellEl = document.querySelector("#board-3d-shell");
const boardSceneEl = document.querySelector("#board-3d-scene");
const newRandomButton = document.querySelector("#new-random-3d");
const resetViewButton = document.querySelector("#reset-view-3d");
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
const maxPublicCellCount = 20;
const defaultBoardSize = { depth: 2, rows: 2, cols: 2 };
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
  boardSize: defaultBoardSize,
  cells: [],
  moves: [],
  hiddenClues: new Set(),
  flashingCells: new Set(),
  flashTimer: null,
  rotationX: -22,
  rotationY: 35,
  drag: null,
  ignoreFaceClicksUntil: 0,
  modalCell: null,
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

function currentSeedFromUrl() {
  return normalizeSeed(new URL(window.location.href).searchParams.get("seed"));
}

function updateUrl(seed, boardSize) {
  const url = new URL(window.location.href);
  url.pathname = "/3d/";
  url.searchParams.set("seed", seed);
  url.searchParams.set("depth", String(boardSize.depth));
  url.searchParams.set("rows", String(boardSize.rows));
  url.searchParams.set("cols", String(boardSize.cols));
  window.history.replaceState({}, "", url);
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

function mentionedCellKeys(clue) {
  const keys = new Set();
  const lowerClue = clue.toLowerCase();

  state.cells.forEach((layer, layerIndex) => {
    layer.forEach((row, rowIndex) => {
      row.forEach((cell, colIndex) => {
        if (new RegExp(`\\b${cell.name.replace(/[.*+?^${}()|[\\]\\\\]/g, "\\$&")}\\b`, "i").test(clue)) {
          keys.add(threeDKey(layerIndex, rowIndex, colIndex));
        }

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

function flashMentionedCells(clue) {
  const keys = mentionedCellKeys(clue);
  state.flashingCells = keys;
  window.clearTimeout(state.flashTimer);
  state.flashTimer = window.setTimeout(() => {
    state.flashingCells = new Set();
    state.flashTimer = null;
    renderBoard();
  }, 900);
  renderBoard();
}

function cellIsRevealed(cell) {
  return cell.revealed === true;
}

function allCellsRevealed() {
  return state.cells.every((layer) =>
    layer.every((row) => row.every((cell) => cellIsRevealed(cell))),
  );
}

function statusText() {
  if (allCellsRevealed()) {
    return "Puzzle Complete";
  }

  return "Drag to rotate";
}

function updateMeta() {
  const { depth, rows, cols } = state.boardSize;
  boardMetaEl.textContent = `${depth} × ${rows} × ${cols}`;
  statusEl.textContent = statusText();
}

function openErrorModal(title, message) {
  errorTitleEl.textContent = title;
  errorMessageEl.textContent = message;
  errorModalEl.hidden = false;
}

function closeErrorModal() {
  errorModalEl.hidden = true;
}

function openGuessModal(layer, row, col) {
  const cell = state.cells[layer][row][col];
  state.modalCell = { layer, row, col };
  guessEmojiEl.textContent = emojiForCell(cell);
  guessTitleEl.textContent = `What is ${cell.name}?`;
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
}

function clueTextForCell(layer, row, col, cell) {
  if (!cellIsRevealed(cell) || !cell.clue) {
    return "";
  }

  return state.hiddenClues.has(threeDKey(layer, row, col)) ? "•••" : cell.clue;
}

function faceClassForCell(cell, key) {
  if (state.flashingCells.has(key)) {
    return "is-flashing";
  }

  if (!cellIsRevealed(cell) || !cell.revealed_answer) {
    return "";
  }

  return cell.revealed_answer === "innocent" ? "tile-innocent" : "tile-criminal";
}

function faceDirectionsForCell(layer, row, col) {
  const directions = [];
  const { depth, rows, cols } = state.boardSize;

  if (layer === 0) directions.push("front");
  if (layer === depth - 1) directions.push("back");
  if (row === 0) directions.push("top");
  if (row === rows - 1) directions.push("bottom");
  if (col === 0) directions.push("left");
  if (col === cols - 1) directions.push("right");

  return directions;
}

function createFace(direction, layer, row, col, cell) {
  const key = threeDKey(layer, row, col);
  const face = document.createElement("button");
  face.type = "button";
  face.className = `three-d-face face-${direction} ${faceClassForCell(cell, key)}`.trim();
  face.dataset.layer = String(layer);
  face.dataset.row = String(row);
  face.dataset.col = String(col);

  const clueText = clueTextForCell(layer, row, col, cell);
  const clueClass =
    clueText === "•••"
      ? "cell-clue clue-hidden"
      : cell.is_nonsense
        ? "cell-clue is-nonsense"
        : "cell-clue";

  face.innerHTML = `
    <div class="cell-top">
      <span class="cell-position">${positionLabel(layer, row, col)}</span>
    </div>
    <div class="cell-header">
      <span class="cell-emoji" aria-hidden="true">${emojiForCell(cell)}</span>
      <span class="cell-name">${cell.name}</span>
      <span class="cell-role">${cell.role}</span>
    </div>
    <p class="${clueClass}">${clueText}</p>
  `;

  return face;
}

function renderBoard() {
  boardSceneEl.replaceChildren();

  const { depth, rows, cols } = state.boardSize;
  const shellRect = boardShellEl.getBoundingClientRect();
  const sceneSize = Math.min(shellRect.width, shellRect.height);
  const extent = Math.max(depth, rows, cols);
  const cellSize = Math.max(84, Math.min(156, Math.floor((sceneSize * 0.62) / extent)));
  const spacing = cellSize + Math.max(10, Math.floor(cellSize * 0.12));
  const halfX = (cols - 1) / 2;
  const halfY = (rows - 1) / 2;
  const halfZ = (depth - 1) / 2;

  boardSceneEl.style.setProperty("--three-d-cell-size", `${cellSize}px`);
  boardSceneEl.style.transform = `rotateX(${state.rotationX}deg) rotateY(${state.rotationY}deg)`;

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

  updateMeta();
}

function updateUrlAndState(seed, puzzle) {
  state.currentSeed = seed;
  state.boardSize = {
    depth: puzzle.depth,
    rows: puzzle.rows,
    cols: puzzle.cols,
  };
  state.cells = puzzle.cells;
  state.moves = [];
  state.hiddenClues = new Set();
  state.flashingCells = new Set();
  updateUrl(seed, state.boardSize);
  renderBoard();
}

async function loadPuzzle(seed = currentSeedFromUrl()) {
  const boardSize = boardSizeFromUrl();
  const params = new URLSearchParams({
    depth: String(boardSize.depth),
    rows: String(boardSize.rows),
    cols: String(boardSize.cols),
  });

  if (seed) {
    params.set("seed", seed);
  }

  const puzzle = await fetchJson(`/api/3d/puzzles/new?${params.toString()}`, {
    method: "GET",
  });
  updateUrlAndState(puzzle.seed, puzzle);
}

async function revealGuess(answer) {
  if (!state.modalCell) {
    return;
  }

  const { layer, row, col } = state.modalCell;
  const response = await fetchJson("/api/3d/puzzles/guess", {
    method: "POST",
    body: JSON.stringify({
      seed: state.currentSeed,
      depth: state.boardSize.depth,
      rows: state.boardSize.rows,
      cols: state.boardSize.cols,
      moves: state.moves,
      layer,
      row,
      col,
      guess: answer,
    }),
  });

  state.moves.push({ layer, row, col, guess: answer });
  const cell = state.cells[layer][row][col];
  cell.revealed = true;
  cell.revealed_answer = answer;
  cell.clue = response.clue;
  cell.is_nonsense = response.is_nonsense;
  closeGuessModal();
  renderBoard();
}

function resetView() {
  state.rotationX = -22;
  state.rotationY = 35;
  renderBoard();
}

function beginDrag(event) {
  state.drag = {
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    startRotationX: state.rotationX,
    startRotationY: state.rotationY,
    moved: false,
  };
  boardShellEl.setPointerCapture?.(event.pointerId);
}

function updateDrag(event) {
  if (!state.drag || state.drag.pointerId !== event.pointerId) {
    return;
  }

  const deltaX = event.clientX - state.drag.startX;
  const deltaY = event.clientY - state.drag.startY;
  if (Math.abs(deltaX) + Math.abs(deltaY) > 6) {
    state.drag.moved = true;
  }

  state.rotationY = state.drag.startRotationY + deltaX * 0.45;
  state.rotationX = Math.max(-85, Math.min(85, state.drag.startRotationX - deltaY * 0.4));
  renderBoard();
}

function endDrag(event) {
  if (!state.drag || state.drag.pointerId !== event.pointerId) {
    return;
  }

  if (state.drag.moved) {
    state.ignoreFaceClicksUntil = Date.now() + 120;
  }
  state.drag = null;
  boardShellEl.releasePointerCapture?.(event.pointerId);
}

function handleFaceClick(event) {
  const face = event.target.closest(".three-d-face");
  if (!face || Date.now() < state.ignoreFaceClicksUntil) {
    return;
  }

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
      flashMentionedCells(cell.clue);
    } else {
      renderBoard();
    }
    return;
  }

  openGuessModal(layer, row, col);
}

function bindEvents() {
  newRandomButton.addEventListener("click", async () => {
    try {
      await loadPuzzle(null);
    } catch (error) {
      openErrorModal("Could not load puzzle.", error.message);
    }
  });
  resetViewButton.addEventListener("click", resetView);
  boardShellEl.addEventListener("pointerdown", beginDrag);
  boardShellEl.addEventListener("pointermove", updateDrag);
  boardShellEl.addEventListener("pointerup", endDrag);
  boardShellEl.addEventListener("pointercancel", endDrag);
  boardShellEl.addEventListener("click", handleFaceClick);
  window.addEventListener("resize", renderBoard);
  guessBackdropEl.addEventListener("click", closeGuessModal);
  guessCancelButton.addEventListener("click", closeGuessModal);
  errorBackdropEl.addEventListener("click", closeErrorModal);
  errorDismissButton.addEventListener("click", closeErrorModal);
  guessInnocentButton.addEventListener("click", () => {
    void revealGuess("innocent").catch((error) => {
      openErrorModal("Guess rejected.", error.message);
    });
  });
  guessCriminalButton.addEventListener("click", () => {
    void revealGuess("criminal").catch((error) => {
      openErrorModal("Guess rejected.", error.message);
    });
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
void loadPuzzle().catch((error) => {
  openErrorModal("Could not load puzzle.", error.message);
});
