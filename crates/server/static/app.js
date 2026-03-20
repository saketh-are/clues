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
const confirmModalEl = document.querySelector("#confirm-modal");
const confirmBackdropEl = document.querySelector("#confirm-backdrop");
const confirmTitleEl = document.querySelector("#confirm-title");
const confirmMessageEl = document.querySelector("#confirm-message");
const confirmAcceptButton = document.querySelector("#confirm-accept");
const confirmCancelButton = document.querySelector("#confirm-cancel");
const finishModalEl = document.querySelector("#finish-modal");
const finishBackdropEl = document.querySelector("#finish-backdrop");
const finishMessageEl = document.querySelector("#finish-message");
const finishDismissButton = document.querySelector("#finish-dismiss");
const scoreDebugEl = document.querySelector("#score-debug");
const progressStoragePrefix = "clues-progress:v1:";
const noteTapDelayMs = 420;
const clueTapDelayMs = 260;
const noteColors = new Set(["yellow", "red", "green"]);
const invalidMoveMessage = "⚠️ Not enough evidence!";
const specialNameEmojis = {
  Coriander: "₍^. .^₎⟆",
};
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

function emojiForRole(role) {
  return roleEmojis[role] ?? "🧑";
}

function emojiForCell(cell) {
  return specialNameEmojis[cell.name] ?? emojiForRole(cell.role);
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
  pendingClueTap: null,
  shareFeedbackTimer: null,
  timerStartedAt: null,
  timerCompletedAt: null,
  completionAcknowledged: false,
  pendingConfirmAction: null,
  scoreDebugVisible: false,
  hoveredScoreMetric: null,
  scoreDebugTab: "generated",
  generatedScoreSeries: [],
  generatedClueTexts: [],
  initialRevealed: null,
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
    renderBoard();
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
  closeConfirmModal();
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

function openConfirmModal(action) {
  closeGuessModal();
  closeErrorModal();
  closeFinishModal();
  state.pendingConfirmAction = action;
  confirmAcceptButton.className = "button button-secondary";
  confirmCancelButton.className = "button button-secondary";
  confirmCancelButton.textContent = "Keep Playing";

  if (action === "clear") {
    confirmTitleEl.textContent = "Clear this puzzle?";
    confirmMessageEl.textContent = "This will wipe your current progress for this seed.";
    confirmAcceptButton.textContent = "Clear";
    confirmCancelButton.classList.add("button-recommended");
  } else {
    confirmTitleEl.textContent = "Load a new puzzle?";
    confirmMessageEl.textContent = "This will leave the current puzzle. Your progress will be saved.";
    confirmAcceptButton.textContent = "New Puzzle";
    confirmCancelButton.classList.add("button-recommended");
  }

  confirmModalEl.hidden = false;
}

function closeConfirmModal() {
  state.pendingConfirmAction = null;
  confirmModalEl.hidden = true;
}

async function confirmPendingAction() {
  const action = state.pendingConfirmAction;
  closeConfirmModal();

  if (action === "clear") {
    await clearBoard();
    return;
  }

  if (action === "new") {
    await loadPuzzle();
  }
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
  const { forceStartModal = false, suppressStartModal = false } = options;
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
  state.generatedScoreSeries = Array.isArray(puzzle.generated_score_series)
    ? puzzle.generated_score_series
    : [];
  state.generatedClueTexts = Array.isArray(puzzle.generated_clue_texts)
    ? puzzle.generated_clue_texts
    : [];
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
  state.scoreDebugTab = "generated";
  state.initialRevealed =
    puzzle.cells.flatMap((row, rowIndex) =>
      row.map((cell, colIndex) => ({ cell, rowIndex, colIndex })),
    ).find(({ cell }) => cell.revealed && cell.score_terms) ?? null;
  clearPendingNoteTap();
  clearPendingClueTap();
  window.clearTimeout(state.flashTimer);
  state.flashTimer = null;

  updateUrlSeed(puzzle.seed);
  resetShareButton();
  closeGuessModal();
  closeConfirmModal();
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

  if (!suppressStartModal && (forceStartModal || state.timerStartedAt === null)) {
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
      emojiEl.textContent = emojiForCell(cell);
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
        clueEl.classList.toggle("is-nonsense", cell.is_nonsense === true);
      } else {
        clueEl.textContent = "";
        clueEl.classList.add("placeholder");
        clueEl.classList.remove("clue-hidden");
        clueEl.classList.remove("is-nonsense");
      }

      if (cell.clue) {
        card.classList.add("clickable");
        card.addEventListener("click", () => {
          handleClueTap(rowIndex, colIndex);
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

  renderScoreDebugPanel();
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
  return result;
}

function applyAcceptedGuess(row, col, guess, clueResult) {
  state.guesses[row][col] = guess;
  state.cells[row][col].clue = clueResult.clue;
  state.cells[row][col].is_nonsense = clueResult.is_nonsense === true;
  state.cells[row][col].score_terms = clueResult.score_terms ?? null;
  state.hiddenClues.delete(guessKey(row, col));
  state.moves = state.moves.filter((move) => move.row !== row || move.col !== col);
  state.moves.push({ row, col, guess });
}

function formatScoreValue(value) {
  if (!Number.isFinite(value)) {
    return "0";
  }

  return Math.abs(value) >= 10 ? value.toFixed(1) : value.toFixed(2);
}

function createSvgElement(name) {
  return document.createElementNS("http://www.w3.org/2000/svg", name);
}

function metricKey(label) {
  return label.toLowerCase().replace(/[^a-z0-9]+/g, "-");
}

function applyScoreMetricFocus(metricKeyValue) {
  state.hoveredScoreMetric = metricKeyValue;

  scoreDebugEl
    .querySelectorAll(
      ".score-debug-series, .score-debug-series-connector, .score-debug-series-label, .score-debug-legend-item",
    )
    .forEach((element) => {
      const key = element.dataset.metric;
      const shouldFade =
        metricKeyValue !== null && key !== metricKeyValue && key !== "score";
      element.classList.toggle("is-faded", shouldFade);
    });
}

function revealedScoreEntries() {
  const entries = [];
  const seen = new Set();

  if (state.initialRevealed) {
    const { cell, rowIndex, colIndex } = state.initialRevealed;
    const key = guessKey(rowIndex, colIndex);
    seen.add(key);
    entries.push({ cell, rowIndex, colIndex, key });
  }

  state.moves.forEach((move) => {
    const cell = state.cells[move.row]?.[move.col];
    if (!cell?.score_terms) {
      return;
    }

    const key = guessKey(move.row, move.col);
    if (seen.has(key)) {
      return;
    }

    seen.add(key);
    entries.push({ cell, rowIndex: move.row, colIndex: move.col, key });
  });

  return entries;
}

function valuesRange(values) {
  const min = Math.min(...values);
  const max = Math.max(...values);

  if (min === max) {
    if (min === 0) {
      return { min: -1, max: 1 };
    }

    const padding = Math.abs(min) * 0.2 || 1;
    return { min: min - padding, max: max + padding };
  }

  return { min, max };
}

function valueToChartY(value, min, max, height, paddingTop, paddingBottom) {
  const usableHeight = height - paddingTop - paddingBottom;
  return height - paddingBottom - ((value - min) / (max - min)) * usableHeight;
}

function createCombinedChart(title, metricSeries, clueTexts) {
  const sectionEl = document.createElement("section");
  sectionEl.className = "score-debug-metric";

  const titleEl = document.createElement("h3");
  titleEl.className = "score-debug-chart-title";
  titleEl.textContent = title;
  sectionEl.appendChild(titleEl);

  const width = 760;
  const height = 240;
  const chartPadding = {
    top: 14,
    right: 132,
    bottom: 34,
    left: 48,
  };
  const svgEl = createSvgElement("svg");
  svgEl.setAttribute("class", "score-debug-chart");
  svgEl.setAttribute("viewBox", `0 0 ${width} ${height}`);
  svgEl.setAttribute("preserveAspectRatio", "none");

  const allValues = metricSeries.flatMap((metric) => metric.values);
  const { min, max } = valuesRange(allValues);
  const plotRight = width - chartPadding.right;
  const plotBottom = height - chartPadding.bottom;
  const xLabel = plotRight + 10;
  const yAtValue = (value) =>
    valueToChartY(value, min, max, height, chartPadding.top, chartPadding.bottom);
  const chartCount = metricSeries[0]?.values.length ?? 0;
  const xStep =
    chartCount <= 1 ? 0 : (plotRight - chartPadding.left) / (chartCount - 1);
  const xAtIndex = (index) => chartPadding.left + xStep * index;

  if (min < 0 && max > 0) {
    const baseline = createSvgElement("line");
    const baselineY = yAtValue(0);
    baseline.setAttribute("x1", String(chartPadding.left));
    baseline.setAttribute("x2", String(plotRight));
    baseline.setAttribute("y1", String(baselineY));
    baseline.setAttribute("y2", String(baselineY));
    baseline.setAttribute("class", "score-debug-baseline");
    svgEl.appendChild(baseline);
  }

  const tickValues = [max];
  if (min < 0 && max > 0) {
    tickValues.push(0);
  }
  if (min !== max) {
    tickValues.push(min);
  }

  [...new Set(tickValues)].forEach((value) => {
    const y = yAtValue(value);
    const gridLine = createSvgElement("line");
    gridLine.setAttribute("x1", String(chartPadding.left));
    gridLine.setAttribute("x2", String(plotRight));
    gridLine.setAttribute("y1", String(y));
    gridLine.setAttribute("y2", String(y));
    gridLine.setAttribute("class", "score-debug-grid");
    svgEl.appendChild(gridLine);

    const textEl = createSvgElement("text");
    textEl.setAttribute("x", String(chartPadding.left - 8));
    textEl.setAttribute("y", String(y + 3));
    textEl.setAttribute("text-anchor", "end");
    textEl.setAttribute("class", "score-debug-axis-label");
    textEl.textContent = formatScoreValue(value);
    svgEl.appendChild(textEl);
  });

  const axisLine = createSvgElement("line");
  axisLine.setAttribute("x1", String(chartPadding.left));
  axisLine.setAttribute("x2", String(chartPadding.left));
  axisLine.setAttribute("y1", String(chartPadding.top));
  axisLine.setAttribute("y2", String(plotBottom));
  axisLine.setAttribute("class", "score-debug-axis");
  svgEl.appendChild(axisLine);

  const xAxisLine = createSvgElement("line");
  xAxisLine.setAttribute("x1", String(chartPadding.left));
  xAxisLine.setAttribute("x2", String(plotRight));
  xAxisLine.setAttribute("y1", String(plotBottom));
  xAxisLine.setAttribute("y2", String(plotBottom));
  xAxisLine.setAttribute("class", "score-debug-axis");
  svgEl.appendChild(xAxisLine);

  const tooltipEl = document.createElement("div");
  tooltipEl.className = "score-debug-tooltip";
  tooltipEl.hidden = true;
  sectionEl.appendChild(tooltipEl);

  const showTooltip = (event, text) => {
    if (!text) {
      return;
    }

    tooltipEl.textContent = text;
    tooltipEl.hidden = false;
    const rect = sectionEl.getBoundingClientRect();
    const x = event.clientX - rect.left + 10;
    const y = event.clientY - rect.top - 30;
    tooltipEl.style.left = `${x}px`;
    tooltipEl.style.top = `${Math.max(8, y)}px`;
  };

  const hideTooltip = () => {
    tooltipEl.hidden = true;
  };

  clueTexts.forEach((text, index) => {
    const x = xAtIndex(index);

    const tick = createSvgElement("line");
    tick.setAttribute("x1", String(x));
    tick.setAttribute("x2", String(x));
    tick.setAttribute("y1", String(plotBottom));
    tick.setAttribute("y2", String(plotBottom + 7));
    tick.setAttribute("class", "score-debug-clue-tick");
    svgEl.appendChild(tick);

    const marker = createSvgElement("circle");
    marker.setAttribute("cx", String(x));
    marker.setAttribute("cy", String(plotBottom + 14));
    marker.setAttribute("r", "3.2");
    marker.setAttribute("class", "score-debug-clue-marker");
    marker.addEventListener("mouseenter", (event) => {
      showTooltip(event, text);
    });
    marker.addEventListener("mousemove", (event) => {
      showTooltip(event, text);
    });
    marker.addEventListener("mouseleave", hideTooltip);
    svgEl.appendChild(marker);

    const label = createSvgElement("text");
    label.setAttribute("x", String(x));
    label.setAttribute("y", String(plotBottom + 28));
    label.setAttribute("text-anchor", "middle");
    label.setAttribute("class", "score-debug-x-label");
    label.textContent = String(index + 1);
    label.addEventListener("mouseenter", (event) => {
      showTooltip(event, text);
    });
    label.addEventListener("mousemove", (event) => {
      showTooltip(event, text);
    });
    label.addEventListener("mouseleave", hideTooltip);
    svgEl.appendChild(label);
  });

  const labelAnchors = [];

  metricSeries.forEach((metric, metricIndex) => {
    const key = metricKey(metric.label);
    const seriesEl = createSvgElement("g");
    seriesEl.setAttribute("class", "score-debug-series");
    seriesEl.dataset.metric = key;

    const points = metric.values
      .map((value, index) => {
        const x = xAtIndex(index);
        const y = yAtValue(value);
        return `${x},${y}`;
      })
      .join(" ");

    const lineEl = createSvgElement("polyline");
    lineEl.setAttribute("points", points);
    lineEl.setAttribute("class", "score-debug-line");
    lineEl.style.stroke = metric.color;
    seriesEl.appendChild(lineEl);

    metric.values.forEach((value, index) => {
      const dotEl = createSvgElement("circle");
      const x = xAtIndex(index);
      const y = yAtValue(value);
      dotEl.setAttribute("cx", String(x));
      dotEl.setAttribute("cy", String(y));
      dotEl.setAttribute("r", index === metric.values.length - 1 ? "2.6" : "1.8");
      dotEl.setAttribute("class", "score-debug-dot");
      dotEl.style.fill = metric.color;
      dotEl.style.opacity = index === metric.values.length - 1 ? "1" : "0.78";
      dotEl.style.stroke = metricIndex === 0 && index === metric.values.length - 1 ? "rgba(255,255,255,0.85)" : "transparent";
      dotEl.style.strokeWidth = "0.9";
      seriesEl.appendChild(dotEl);
    });

    svgEl.appendChild(seriesEl);

    if (metric.values.length > 0) {
      labelAnchors.push({
        key,
        color: metric.color,
        label: metric.label,
        x: xAtIndex(metric.values.length - 1),
        y: yAtValue(metric.values[metric.values.length - 1]),
      });
    }
  });

  labelAnchors.sort((left, right) => left.y - right.y);
  let previousY = chartPadding.top + 8;
  labelAnchors.forEach((anchor) => {
    anchor.labelY = Math.max(anchor.y, previousY);
    previousY = anchor.labelY + 11;
  });

  for (let index = labelAnchors.length - 1; index >= 0; index -= 1) {
    const nextY =
      index === labelAnchors.length - 1
        ? plotBottom - 4
        : labelAnchors[index + 1].labelY - 11;
    labelAnchors[index].labelY = Math.min(labelAnchors[index].labelY, nextY);
  }

  labelAnchors.forEach((anchor) => {
    const connector = createSvgElement("line");
    connector.setAttribute("x1", String(anchor.x + 2));
    connector.setAttribute("x2", String(xLabel - 4));
    connector.setAttribute("y1", String(anchor.y));
    connector.setAttribute("y2", String(anchor.labelY));
    connector.setAttribute("class", "score-debug-series-connector");
    connector.dataset.metric = anchor.key;
    connector.style.stroke = anchor.color;
    svgEl.appendChild(connector);

    const labelEl = createSvgElement("text");
    labelEl.setAttribute("x", String(xLabel));
    labelEl.setAttribute("y", String(anchor.labelY + 3));
    labelEl.setAttribute("class", "score-debug-series-label");
    labelEl.dataset.metric = anchor.key;
    labelEl.style.fill = anchor.color;
    labelEl.textContent = anchor.label;
    if (anchor.key !== "score") {
      labelEl.addEventListener("mouseenter", () => {
        applyScoreMetricFocus(anchor.key);
      });
      labelEl.addEventListener("mouseleave", () => {
        applyScoreMetricFocus(null);
      });
    }
    svgEl.appendChild(labelEl);
  });

  sectionEl.appendChild(svgEl);

  const legendEl = document.createElement("div");
  legendEl.className = "score-debug-legend";

  metricSeries.forEach((metric) => {
    const key = metricKey(metric.label);
    const itemEl = document.createElement("div");
    itemEl.className = "score-debug-legend-item";
    itemEl.dataset.metric = key;

    const swatchEl = document.createElement("span");
    swatchEl.className = "score-debug-swatch";
    swatchEl.style.background = metric.color;

    const labelEl = document.createElement("span");
    labelEl.className = "score-debug-legend-label";
    labelEl.textContent = metric.label;

    itemEl.append(swatchEl, labelEl);
    if (key !== "score") {
      itemEl.addEventListener("mouseenter", () => {
        applyScoreMetricFocus(key);
      });
      itemEl.addEventListener("mouseleave", () => {
        applyScoreMetricFocus(null);
      });
    }
    legendEl.appendChild(itemEl);
  });

  sectionEl.appendChild(legendEl);
  return sectionEl;
}

function scoreDebugDataForActiveTab() {
  if (state.scoreDebugTab === "generated") {
    return {
      series: state.generatedScoreSeries,
      clueTexts: state.generatedClueTexts,
    };
  }

  const entries = revealedScoreEntries();
  return {
    series: entries.map(({ cell }) => cell.score_terms),
    clueTexts: entries.map(({ cell }) => cell.clue ?? ""),
  };
}

function renderScoreDebugPanel() {
  if (!state.scoreDebugVisible) {
    scoreDebugEl.hidden = true;
    scoreDebugEl.innerHTML = "";
    return;
  }

  const { series: activeSeries, clueTexts } = scoreDebugDataForActiveTab();
  if (activeSeries.length === 0) {
    scoreDebugEl.hidden = true;
    scoreDebugEl.innerHTML = "";
    return;
  }

  const metrics = [
    ["combination_size", "combination_size", "#2f7a61"],
    ["combined_new_forced", "combined_new_forced", "#cf5f3f"],
    ["standalone_forced", "standalone_forced", "#b43c2f"],
    ["active_unforced_tiles", "active_unforced_tiles", "#2f6fb4"],
    ["newly_active_unforced_tiles", "newly_active_unforced_tiles", "#1f8a63"],
    ["active_uncertainty", "active_uncertainty", "#9153c6"],
    ["active_uncertainty_jump", "active_uncertainty_jump", "#b24f86"],
    ["combined_gain", "combined_gain", "#6f58c9"],
    ["alone_gain", "alone_gain", "#8b6b2d"],
    ["synergy_gain", "synergy_gain", "#208a8d"],
    ["triviality_penalty", "triviality_penalty", "#58606f"],
    ["family_weight", "family_weight", "#d07d1f"],
    ["score", "score", "#1f2430"],
  ];

  scoreDebugEl.hidden = false;
  scoreDebugEl.innerHTML = "";

  const cardEl = document.createElement("div");
  cardEl.className = "score-debug-card";

  const headerEl = document.createElement("div");
  headerEl.className = "score-debug-top";

  const headingEl = document.createElement("div");
  headingEl.className = "score-debug-heading";

  const titleEl = document.createElement("h2");
  titleEl.className = "score-debug-title";
  titleEl.textContent = "Clue Score";

  const metaEl = document.createElement("span");
  metaEl.className = "score-debug-meta";
  metaEl.textContent =
    state.scoreDebugTab === "generated"
      ? `${activeSeries.length} generated`
      : `${activeSeries.length} revealed`;

  headingEl.append(titleEl, metaEl);

  const quickNewButton = document.createElement("button");
  quickNewButton.type = "button";
  quickNewButton.className = "button button-secondary score-debug-action";
  quickNewButton.textContent = "New Puzzle";
  quickNewButton.addEventListener("click", async () => {
    try {
      state.scoreDebugVisible = true;
      state.scoreDebugTab = "generated";
      await loadPuzzle(undefined, { suppressStartModal: true });
    } catch (error) {
      openErrorModal(error.message);
    }
  });

  headerEl.append(headingEl, quickNewButton);
  cardEl.appendChild(headerEl);

  const tabsEl = document.createElement("div");
  tabsEl.className = "score-debug-tabs";

  [
    ["revealed", "Revealed"],
    ["generated", "Generated"],
  ].forEach(([value, label]) => {
    const tabEl = document.createElement("button");
    tabEl.type = "button";
    tabEl.className = "score-debug-tab";
    if (state.scoreDebugTab === value) {
      tabEl.classList.add("is-active");
    }
    tabEl.textContent = label;
    tabEl.addEventListener("click", () => {
      state.scoreDebugTab = value;
      state.hoveredScoreMetric = null;
      renderScoreDebugPanel();
    });
    tabsEl.appendChild(tabEl);
  });

  cardEl.appendChild(tabsEl);

  const chartsEl = document.createElement("div");
  chartsEl.className = "score-debug-charts";
  const metricSeries = metrics.map(([key, label, color]) => ({
    label,
    color,
    values: activeSeries.map((terms) => Number(terms?.[key] ?? 0)),
  }));
  const scoreSeries = metricSeries.filter((metric) => metric.label === "score");
  const factorSeries = metricSeries.filter((metric) => metric.label !== "score");
  chartsEl.appendChild(createCombinedChart("Factors", factorSeries, clueTexts));
  chartsEl.appendChild(createCombinedChart("Score", scoreSeries, clueTexts));

  cardEl.appendChild(chartsEl);
  scoreDebugEl.appendChild(cardEl);
  applyScoreMetricFocus(state.hoveredScoreMetric);
}

function shouldIgnoreDebugShortcut(event) {
  if (event.metaKey || event.ctrlKey || event.altKey) {
    return true;
  }

  const target = event.target;
  if (!(target instanceof HTMLElement)) {
    return false;
  }

  return (
    target.isContentEditable ||
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA" ||
    target.tagName === "SELECT"
  );
}

function toggleScoreDebugOverlay() {
  state.scoreDebugVisible = !state.scoreDebugVisible;
  if (!state.scoreDebugVisible) {
    applyScoreMetricFocus(null);
  }
  renderScoreDebugPanel();
}

function clearPendingNoteTap() {
  if (!state.pendingNoteTap) {
    return;
  }

  window.clearTimeout(state.pendingNoteTap.timerId);
  state.pendingNoteTap = null;
}

function clearPendingClueTap() {
  if (!state.pendingClueTap) {
    return;
  }

  window.clearTimeout(state.pendingClueTap.timerId);
  state.pendingClueTap = null;
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

function handleClueTap(row, col) {
  const key = guessKey(row, col);
  const clue = state.cells[row]?.[col]?.clue;

  if (!clue) {
    return;
  }

  if (state.pendingClueTap && state.pendingClueTap.key === key) {
    clearPendingClueTap();
    flashMentionedTiles(clue);
    return;
  }

  clearPendingClueTap();
  state.pendingClueTap = {
    key,
    timerId: window.setTimeout(() => {
      state.pendingClueTap = null;
      toggleClueVisibility(row, col);
    }, clueTapDelayMs),
  };
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

      const clueResult = await fetchValidatedClue(move.row, move.col, move.guess);
      applyAcceptedGuess(move.row, move.col, move.guess, clueResult);
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
    const clueResult = await fetchValidatedClue(row, col, nextGuess);
    applyAcceptedGuess(row, col, nextGuess, clueResult);
    if (state.timerStartedAt === null) {
      state.timerStartedAt = Date.now();
      state.timerCompletedAt = null;
      state.completionAcknowledged = false;
    }
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
  clearPendingClueTap();
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

  if (state.hiddenClues.has(key)) {
    state.hiddenClues.delete(key);
  } else {
    state.hiddenClues.add(key);
  }

  persistProgress();
  renderBoard();
}

function canSkipNewPuzzleConfirmation() {
  return (
    allTilesMarked() ||
    (state.moves.length === 0 && state.hiddenClues.size === 0 && state.notes.size === 0)
  );
}

function openGuessModal(row, col) {
  const cell = state.cells[row]?.[col];
  if (!cell || cell.revealed) {
    return;
  }

  state.modalCell = { row, col };
  guessEmojiEl.textContent = emojiForCell(cell);
  guessTitleEl.textContent = cell.name;
  guessModalEl.hidden = false;
}

function closeGuessModal() {
  state.modalCell = null;
  guessModalEl.hidden = true;
}

function openErrorModal(title, message = "") {
  closeGuessModal();
  closeConfirmModal();
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
  closeConfirmModal();
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
  if (canSkipNewPuzzleConfirmation()) {
    try {
      await loadPuzzle();
    } catch (error) {
      openErrorModal(error.message);
    }
    return;
  }

  openConfirmModal("new");
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

clearButton.addEventListener("click", () => {
  openConfirmModal("clear");
});
guessBackdropEl.addEventListener("click", closeGuessModal);
guessCancelButton.addEventListener("click", closeGuessModal);
confirmBackdropEl.addEventListener("click", closeConfirmModal);
confirmCancelButton.addEventListener("click", closeConfirmModal);
confirmAcceptButton.addEventListener("click", async () => {
  try {
    await confirmPendingAction();
  } catch (error) {
    openErrorModal(error.message);
  }
});
errorBackdropEl.addEventListener("click", closeErrorModal);
errorDismissButton.addEventListener("click", closeErrorModal);
startButton.addEventListener("click", startPuzzle);
finishBackdropEl.addEventListener("click", dismissFinishModal);
finishDismissButton.addEventListener("click", dismissFinishModal);
scoreDebugEl.addEventListener("click", (event) => {
  if (event.target === scoreDebugEl) {
    toggleScoreDebugOverlay();
  }
});
scoreDebugEl.addEventListener("mouseleave", () => {
  applyScoreMetricFocus(null);
});
window.addEventListener("keydown", (event) => {
  if (shouldIgnoreDebugShortcut(event)) {
    return;
  }

  if (event.key.toLowerCase() === "d") {
    event.preventDefault();
    toggleScoreDebugOverlay();
  }
});
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
