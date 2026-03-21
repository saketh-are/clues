const boardEl = document.querySelector("#editor-board");
const cellTemplate = document.querySelector("#editor-cell-template");
const rowsSelectEl = document.querySelector("#rows-select");
const colsSelectEl = document.querySelector("#cols-select");
const authorInputEl = document.querySelector("#author-input");
const resetButton = document.querySelector("#reset-editor");
const generateRemainingButton = document.querySelector("#generate-remaining");
const undoButton = document.querySelector("#undo-edit");
const shareButton = document.querySelector("#share-editor");
const promptBarEl = document.querySelector("#editor-prompt-bar");
const promptEl = document.querySelector("#editor-prompt");
const shareLinkEl = document.querySelector("#editor-share-link");
const emptyStateEl = document.querySelector("#editor-empty-state");
const inspectorEl = document.querySelector("#editor-inspector");
const selectedPositionEl = document.querySelector("#selected-position");
const selectedHeadingEl = document.querySelector("#selected-heading");
const inlineDetailsEl = document.querySelector("#editor-inline-details");
const selectedEmojiEl = document.querySelector("#selected-emoji");
const selectedBadgesEl = document.querySelector("#selected-badges");
const editToggleEl = document.querySelector("#edit-toggle");
const renameInputEl = document.querySelector("#rename-input");
const roleInputEl = document.querySelector("#role-input");
const emojiPickerModalEl = document.querySelector("#emoji-picker-modal");
const emojiPickerGridEl = document.querySelector("#emoji-picker-grid");
const emojiCustomInputEl = document.querySelector("#emoji-custom-input");
const emojiDefaultButton = document.querySelector("#emoji-default");
const emojiCloseButton = document.querySelector("#emoji-close");
const customRoleModalEl = document.querySelector("#custom-role-modal");
const customRoleInputEl = document.querySelector("#custom-role-input");
const customRoleCancelButton = document.querySelector("#custom-role-cancel");
const customRoleSaveButton = document.querySelector("#custom-role-save");
const initialAnswerPanelEl = document.querySelector("#initial-answer-panel");
const setInitialInnocentButton = document.querySelector("#set-initial-innocent");
const setInitialCriminalButton = document.querySelector("#set-initial-criminal");
const clueSectionEl = document.querySelector("#clue-section");
const clueKindPickerEl = document.querySelector("#clue-kind-picker");
const clueFormFieldsEl = document.querySelector("#clue-form-fields");
const cluePreviewEl = document.querySelector("#clue-preview");
const suggestClueButton = document.querySelector("#suggest-clue");
const saveClueButton = document.querySelector("#save-clue");
const errorModalEl = document.querySelector("#editor-error-modal");
const errorTitleEl = document.querySelector("#editor-error-title");
const errorMessageEl = document.querySelector("#editor-error-message");
const errorDismissButton = document.querySelector("#editor-error-dismiss");

const maxPublicCellCount = 20;
const maxBoardDimension = 20;
const defaultBoardSize = { rows: 5, cols: 4 };
const editorStorageKey = "clues-editor:v1";
const customRoleOptionValue = "__custom__";
const maxEmojiChars = 32;
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
const emojiPickerChoices = [
  "🧑",
  "🙂",
  "😎",
  "🤠",
  "🕵️",
  "🧑‍🎨",
  "👨‍🍳🥖",
  "👷",
  "🧑‍⚕️",
  "🧑‍🌾",
  "🧑‍🚒",
  "💂",
  "🧑‍⚖️",
  "🧑‍🔧",
  "🧑‍✈️",
  "👮",
  "🧑‍🔬",
  "🧑‍🎤",
  "🧑‍🏫",
  "🧑‍💻",
  "🐱",
  "🦊",
  "🐸",
  "🦉",
];
const clueKindOptions = [
  ["nonsense", "Nonsense"],
  ["declaration", "Declaration"],
  ["count_cells", "Count Cells"],
  ["named_count_cells", "Named Count Cells"],
  ["connected", "Connected"],
  ["direct_relation", "Direct Relation"],
  ["role_count", "Role Count"],
  ["roles_comparison", "Roles Comparison"],
  ["line_comparison", "Line Comparison"],
  ["quantified", "Quantified"],
];

function allRemainingCluelessCellsAreForced() {
  if (!state.response) {
    return false;
  }

  let clueLessCellCount = 0;
  for (const row of state.response.draft.cells) {
    for (const cell of row) {
      if (cell.clue === null) {
        clueLessCellCount += 1;
      }
    }
  }

  return clueLessCellCount > 0 && clueLessCellCount === state.response.next_clue_targets.length;
}

function availableClueKindOptions(view = selectedCellView()) {
  if (isEditableNonsenseTile(view)) {
    return clueKindOptions.filter(([kind]) => kind === "nonsense");
  }

  return allRemainingCluelessCellsAreForced()
    ? clueKindOptions.filter(([kind]) => kind === "nonsense")
    : clueKindOptions;
}

const state = {
  bootstrap: null,
  boardSize: { ...defaultBoardSize },
  pendingBoardSize: { ...defaultBoardSize },
  response: null,
  selected: null,
  progression: [],
  clueForm: null,
  busy: false,
  pendingFocus: null,
  visibleShareUrl: null,
  lastSharedDraftKey: null,
  lastSharedStoredPuzzleId: null,
  isEditingDetails: false,
  editDraft: null,
  emojiPickerOpen: false,
  customRoleModalOpen: false,
  customRoleDraft: "",
  clueSaveValidation: {
    key: null,
    status: "invalid",
    requested: false,
  },
};

function emojiForRole(role) {
  return roleEmojis[role] ?? "🧑";
}

function emojiForCell(cell) {
  if (typeof cell.emoji === "string" && cell.emoji.trim() !== "") {
    return cell.emoji;
  }
  return specialNameEmojis[cell.name] ?? emojiForRole(cell.role);
}

function answerLabel(answer) {
  return answer === "innocent" ? "Innocent" : "Criminal";
}

function answerToneClass(answer) {
  return answer === "innocent" ? "tile-innocent" : "tile-criminal";
}

function badgeToneClass(answer) {
  return answer === "innocent" ? "is-innocent" : "is-criminal";
}

function rowLabel(index) {
  return String(index + 1);
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

function cellKey(row, col) {
  return `${row}:${col}`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function statusBadgeMarkup(label, className = "") {
  return `<span class="editor-status-badge${className ? ` ${className}` : ""}">${escapeHtml(label)}</span>`;
}

function deepClone(value) {
  return JSON.parse(JSON.stringify(value));
}

function boardNames() {
  if (!state.response) {
    return [];
  }

  return state.response.draft.cells.flat().map((cell) => cell.name);
}

function defaultRoles() {
  return state.bootstrap?.roles ?? [];
}

function roleCatalog() {
  const seen = new Set();
  const roles = [];

  for (const cell of state.response?.draft?.cells?.flat?.() ?? []) {
    if (typeof cell.role !== "string") {
      continue;
    }

    const trimmed = cell.role.trim();
    if (trimmed.length === 0 || seen.has(trimmed)) {
      continue;
    }

    seen.add(trimmed);
    roles.push(trimmed);
  }

  const draftRole = state.isEditingDetails ? state.editDraft?.role?.trim?.() : "";
  if (draftRole && !seen.has(draftRole)) {
    seen.add(draftRole);
    roles.push(draftRole);
  }

  for (const role of defaultRoles()) {
    if (seen.has(role)) {
      continue;
    }

    seen.add(role);
    roles.push(role);
  }

  return roles;
}

function rememberedEmojiForRole(role) {
  const normalizedRole = role.trim();
  if (normalizedRole.length === 0) {
    return null;
  }

  const currentView = selectedCellView();
  if (state.isEditingDetails && state.editDraft && currentView) {
    const roleState = validateRoleInput(currentView, state.editDraft.role);
    const emojiState = validateEmojiInput(state.editDraft.emoji);
    if (
      roleState.valid &&
      emojiState.valid &&
      emojiState.normalized.length > 0 &&
      roleState.normalized === normalizedRole
    ) {
      return emojiState.normalized;
    }
  }

  for (const cell of state.response?.draft?.cells?.flat?.() ?? []) {
    if (cell.role.trim() !== normalizedRole) {
      continue;
    }

    if (typeof cell.emoji === "string" && cell.emoji.trim().length > 0) {
      return cell.emoji.trim();
    }
  }

  return null;
}

function firstName(names) {
  return names[0] ?? "";
}

function secondName(names) {
  return names[1] ?? names[0] ?? "";
}

function firstRole(roles) {
  return roles[0] ?? "";
}

function secondRole(roles) {
  return roles[1] ?? roles[0] ?? "";
}

function clampInt(value, minimum, maximum) {
  const parsed = Number.parseInt(String(value), 10);
  if (!Number.isInteger(parsed)) {
    return minimum;
  }

  return Math.min(maximum, Math.max(minimum, parsed));
}

function lineValue(kind, value) {
  return kind === "row"
    ? clampInt(value, 0, state.boardSize.rows - 1)
    : clampInt(value, 0, state.boardSize.cols - 1);
}

function createDefaultLine() {
  return {
    kind: "row",
    row: 0,
    col: 0,
  };
}

function createDefaultFilter() {
  return {
    kind: "any",
    line: createDefaultLine(),
  };
}

function createDefaultCount() {
  return {
    kind: "number",
    value: 1,
    parity: "odd",
  };
}

function createDefaultSelector(names) {
  return {
    kind: "board",
    name: firstName(names),
    direction: "above",
    row: 0,
    col: 0,
    firstName: firstName(names),
    secondName: secondName(names),
  };
}

function createDefaultGroup(names, roles) {
  return {
    kind: "any",
    filter: createDefaultFilter(),
    line: createDefaultLine(),
    role: firstRole(roles),
    selected: {
      selector: createDefaultSelector(names),
      answer: "innocent",
      filter: createDefaultFilter(),
    },
  };
}

function createDefaultPredicate(names) {
  return {
    kind: "neighbor",
    answer: "innocent",
    count: createDefaultCount(),
    filter: createDefaultFilter(),
    direction: "above",
    name: firstName(names),
  };
}

function createDefaultClueForm() {
  const names = boardNames();
  const roles = roleCatalog();
  return {
    kind: "nonsense",
    nonsense: {
      text: "",
    },
    declaration: {
      name: firstName(names),
      answer: "innocent",
    },
    countCells: {
      selector: createDefaultSelector(names),
      answer: "innocent",
      count: createDefaultCount(),
      filter: createDefaultFilter(),
    },
    namedCountCells: {
      name: firstName(names),
      selector: createDefaultSelector(names),
      answer: "innocent",
      number: 1,
      filter: createDefaultFilter(),
    },
    connected: {
      answer: "innocent",
      line: createDefaultLine(),
    },
    directRelation: {
      name: firstName(names),
      answer: "innocent",
      direction: "above",
    },
    roleCount: {
      role: firstRole(roles),
      answer: "innocent",
      count: createDefaultCount(),
    },
    rolesComparison: {
      firstRole: firstRole(roles),
      secondRole: secondRole(roles),
      answer: "innocent",
      comparison: "more",
    },
    lineComparison: {
      firstLine: createDefaultLine(),
      secondLine: createDefaultLine(),
      answer: "innocent",
      comparison: "more",
    },
    quantified: {
      quantifier: {
        kind: "exactly",
        value: 1,
      },
      group: createDefaultGroup(names, roles),
      predicate: createDefaultPredicate(names),
    },
  };
}

function sanitizeLineState(line) {
  line.kind = line.kind === "col" ? "col" : "row";
  line.row = clampInt(line.row, 0, Math.max(0, state.boardSize.rows - 1));
  line.col = clampInt(line.col, 0, Math.max(0, state.boardSize.cols - 1));
}

function sanitizeFilterState(filter) {
  if (!["any", "edge", "corner", "line"].includes(filter.kind)) {
    filter.kind = "any";
  }
  if (!filter.line) {
    filter.line = createDefaultLine();
  }
  sanitizeLineState(filter.line);
}

function sanitizeCountState(count) {
  if (!["number", "at_least", "parity"].includes(count.kind)) {
    count.kind = "number";
  }
  count.value = clampInt(count.value, 0, maxPublicCellCount);
  count.parity = count.parity === "even" ? "even" : "odd";
}

function sanitizeSelectorState(selector, names) {
  const selectorKinds = [
    "board",
    "neighbor",
    "direction",
    "row",
    "col",
    "between",
    "shared_neighbor",
  ];
  if (!selectorKinds.includes(selector.kind)) {
    selector.kind = "board";
  }
  selector.name = names.includes(selector.name) ? selector.name : firstName(names);
  selector.direction =
    selector.direction === "below" ||
    selector.direction === "left" ||
    selector.direction === "right"
      ? selector.direction
      : "above";
  selector.row = clampInt(selector.row, 0, Math.max(0, state.boardSize.rows - 1));
  selector.col = clampInt(selector.col, 0, Math.max(0, state.boardSize.cols - 1));
  selector.firstName = names.includes(selector.firstName)
    ? selector.firstName
    : firstName(names);
  selector.secondName = names.includes(selector.secondName)
    ? selector.secondName
    : secondName(names);
}

function sanitizeGroupState(group, names, roles) {
  if (!["any", "filter", "line", "role", "selected_cells"].includes(group.kind)) {
    group.kind = "any";
  }
  if (!group.filter) {
    group.filter = createDefaultFilter();
  }
  if (!group.line) {
    group.line = createDefaultLine();
  }
  if (!group.selected) {
    group.selected = {
      selector: createDefaultSelector(names),
      answer: "innocent",
      filter: createDefaultFilter(),
    };
  }
  sanitizeFilterState(group.filter);
  sanitizeLineState(group.line);
  group.role = roles.includes(group.role) ? group.role : firstRole(roles);
  sanitizeSelectorState(group.selected.selector, names);
  group.selected.answer = group.selected.answer === "criminal" ? "criminal" : "innocent";
  sanitizeFilterState(group.selected.filter);
}

function sanitizePredicateState(predicate, names) {
  if (!["neighbor", "direct_relation", "neighboring"].includes(predicate.kind)) {
    predicate.kind = "neighbor";
  }
  predicate.answer = predicate.answer === "criminal" ? "criminal" : "innocent";
  predicate.direction =
    predicate.direction === "below" ||
    predicate.direction === "left" ||
    predicate.direction === "right"
      ? predicate.direction
      : "above";
  predicate.name = names.includes(predicate.name) ? predicate.name : firstName(names);
  sanitizeCountState(predicate.count);
  sanitizeFilterState(predicate.filter);
}

function sanitizeClueFormState() {
  if (!state.clueForm) {
    state.clueForm = createDefaultClueForm();
  }

  const names = boardNames();
  const roles = roleCatalog();
  if (names.length === 0 || roles.length === 0) {
    return;
  }

  const form = state.clueForm;
  if (!availableClueKindOptions().some(([kind]) => kind === form.kind)) {
    form.kind = "nonsense";
  }

  form.nonsense.text = String(form.nonsense.text ?? "").slice(
    0,
    state.bootstrap.max_nonsense_text_chars,
  );
  form.declaration.name = names.includes(form.declaration.name)
    ? form.declaration.name
    : firstName(names);
  form.declaration.answer =
    form.declaration.answer === "criminal" ? "criminal" : "innocent";

  sanitizeSelectorState(form.countCells.selector, names);
  form.countCells.answer = form.countCells.answer === "criminal" ? "criminal" : "innocent";
  sanitizeCountState(form.countCells.count);
  sanitizeFilterState(form.countCells.filter);

  form.namedCountCells.name = names.includes(form.namedCountCells.name)
    ? form.namedCountCells.name
    : firstName(names);
  sanitizeSelectorState(form.namedCountCells.selector, names);
  form.namedCountCells.answer =
    form.namedCountCells.answer === "criminal" ? "criminal" : "innocent";
  form.namedCountCells.number = clampInt(
    form.namedCountCells.number,
    0,
    maxPublicCellCount,
  );
  sanitizeFilterState(form.namedCountCells.filter);

  form.connected.answer = form.connected.answer === "criminal" ? "criminal" : "innocent";
  sanitizeLineState(form.connected.line);

  form.directRelation.name = names.includes(form.directRelation.name)
    ? form.directRelation.name
    : firstName(names);
  form.directRelation.answer =
    form.directRelation.answer === "criminal" ? "criminal" : "innocent";
  form.directRelation.direction =
    form.directRelation.direction === "below" ||
    form.directRelation.direction === "left" ||
    form.directRelation.direction === "right"
      ? form.directRelation.direction
      : "above";

  form.roleCount.role = roles.includes(form.roleCount.role)
    ? form.roleCount.role
    : firstRole(roles);
  form.roleCount.answer = form.roleCount.answer === "criminal" ? "criminal" : "innocent";
  sanitizeCountState(form.roleCount.count);

  form.rolesComparison.firstRole = roles.includes(form.rolesComparison.firstRole)
    ? form.rolesComparison.firstRole
    : firstRole(roles);
  form.rolesComparison.secondRole = roles.includes(form.rolesComparison.secondRole)
    ? form.rolesComparison.secondRole
    : secondRole(roles);
  form.rolesComparison.answer =
    form.rolesComparison.answer === "criminal" ? "criminal" : "innocent";
  form.rolesComparison.comparison = ["more", "fewer", "equal"].includes(
    form.rolesComparison.comparison,
  )
    ? form.rolesComparison.comparison
    : "more";

  sanitizeLineState(form.lineComparison.firstLine);
  sanitizeLineState(form.lineComparison.secondLine);
  form.lineComparison.answer =
    form.lineComparison.answer === "criminal" ? "criminal" : "innocent";
  form.lineComparison.comparison = ["more", "fewer", "equal"].includes(
    form.lineComparison.comparison,
  )
    ? form.lineComparison.comparison
    : "more";

  form.quantified.quantifier.kind = "exactly";
  form.quantified.quantifier.value = clampInt(
    form.quantified.quantifier.value,
    0,
    maxPublicCellCount,
  );
  sanitizeGroupState(form.quantified.group, names, roles);
  sanitizePredicateState(form.quantified.predicate, names);
}

function setNestedValue(target, path, value) {
  const parts = path.split(".");
  let current = target;
  for (let index = 0; index < parts.length - 1; index += 1) {
    current = current[parts[index]];
  }
  current[parts.at(-1)] = value;
}

function readControlValue(target) {
  if (target.dataset.type === "int") {
    return clampInt(target.value, 0, maxPublicCellCount);
  }

  return target.value;
}

function optionMarkup(value, label, selected, disabled = false) {
  return `<option value="${escapeHtml(value)}"${selected ? " selected" : ""}${disabled ? " disabled" : ""}>${escapeHtml(label)}</option>`;
}

function answerOptionsMarkup(selectedAnswer) {
  return [
    optionMarkup("innocent", "Innocent", selectedAnswer === "innocent"),
    optionMarkup("criminal", "Criminal", selectedAnswer === "criminal"),
  ].join("");
}

function directionOptionsMarkup(selectedDirection) {
  return [
    optionMarkup("above", "Above", selectedDirection === "above"),
    optionMarkup("below", "Below", selectedDirection === "below"),
    optionMarkup("left", "Left", selectedDirection === "left"),
    optionMarkup("right", "Right", selectedDirection === "right"),
  ].join("");
}

function comparisonOptionsMarkup(selectedComparison) {
  return [
    optionMarkup("more", "More", selectedComparison === "more"),
    optionMarkup("fewer", "Fewer", selectedComparison === "fewer"),
    optionMarkup("equal", "Equal", selectedComparison === "equal"),
  ].join("");
}

function nameOptionsMarkup(selectedName) {
  return boardNames()
    .map((name) => optionMarkup(name, name, name === selectedName))
    .join("");
}

function roleOptionsMarkup(selectedRole) {
  return roleCatalog()
    .map((role) => optionMarkup(role, role, role === selectedRole))
    .join("");
}

function roleEditOptionsMarkup(selectedRole) {
  const options = [];
  const seen = new Set();
  const roles = roleCatalog();

  if (!roles.includes(selectedRole)) {
    options.push(
      optionMarkup(
        selectedRole,
        selectedRole.trim().length > 0 ? selectedRole : "Custom",
        true,
      ),
    );
    seen.add(selectedRole);
  }

  for (const role of roles) {
    if (seen.has(role)) {
      continue;
    }

    options.push(optionMarkup(role, role, role === selectedRole));
  }

  options.push(optionMarkup(customRoleOptionValue, "Custom", false));
  return options.join("");
}

function nonsensePresetOptionsMarkup() {
  const placeholder = '<option value="" selected>Choose a canned phrase</option>';
  const general = state.bootstrap.nonsense_texts
    .map((text) => optionMarkup(text, text, false))
    .join("");
  const criminal = state.bootstrap.criminal_nonsense_texts
    .map((text) => optionMarkup(text, text, false))
    .join("");

  return `${placeholder}<optgroup label="General">${general}</optgroup><optgroup label="Criminal">${criminal}</optgroup>`;
}

function rowOptionsMarkup(selectedRow) {
  return Array.from({ length: state.boardSize.rows }, (_, index) =>
    optionMarkup(String(index), `Row ${rowLabel(index)}`, index === selectedRow),
  ).join("");
}

function colOptionsMarkup(selectedCol) {
  return Array.from({ length: state.boardSize.cols }, (_, index) =>
    optionMarkup(String(index), `Col ${colLabel(index)}`, index === selectedCol),
  ).join("");
}

function fieldMarkup(label, controlMarkup) {
  return `
    <label class="editor-field">
      <span class="eyebrow">${label}</span>
      ${controlMarkup}
    </label>
  `;
}

function sectionMarkup(title, bodyMarkup) {
  return `
    <section class="editor-subsection">
      <h4 class="editor-subsection-title">${title}</h4>
      <div class="editor-subsection-body">
        ${bodyMarkup}
      </div>
    </section>
  `;
}

function renderLineEditor(path, line) {
  return `
    <div class="editor-subgrid">
      ${fieldMarkup(
        "Line Type",
        `<select class="editor-select" data-path="${path}.kind" data-rerender="true">
          ${optionMarkup("row", "Row", line.kind === "row")}
          ${optionMarkup("col", "Column", line.kind === "col")}
        </select>`,
      )}
      ${
        line.kind === "row"
          ? fieldMarkup(
              "Row",
              `<select class="editor-select" data-path="${path}.row" data-type="int">
                ${rowOptionsMarkup(line.row)}
              </select>`,
            )
          : fieldMarkup(
              "Column",
              `<select class="editor-select" data-path="${path}.col" data-type="int">
                ${colOptionsMarkup(line.col)}
              </select>`,
            )
      }
    </div>
  `;
}

function renderFilterEditor(path, filter) {
  return `
    <div class="editor-subgrid">
      ${fieldMarkup(
        "Filter",
        `<select class="editor-select" data-path="${path}.kind" data-rerender="true">
          ${optionMarkup("any", "Any", filter.kind === "any")}
          ${optionMarkup("edge", "Edge", filter.kind === "edge")}
          ${optionMarkup("corner", "Corner", filter.kind === "corner")}
          ${optionMarkup("line", "Line", filter.kind === "line")}
        </select>`,
      )}
    </div>
    ${filter.kind === "line" ? renderLineEditor(`${path}.line`, filter.line) : ""}
  `;
}

function renderCountEditor(path, count) {
  return `
    <div class="editor-subgrid">
      ${fieldMarkup(
        "Count Kind",
        `<select class="editor-select" data-path="${path}.kind" data-rerender="true">
          ${optionMarkup("number", "Exact Number", count.kind === "number")}
          ${optionMarkup("at_least", "At Least", count.kind === "at_least")}
          ${optionMarkup("parity", "Parity", count.kind === "parity")}
        </select>`,
      )}
      ${
        count.kind === "parity"
          ? fieldMarkup(
              "Parity",
              `<select class="editor-select" data-path="${path}.parity">
                ${optionMarkup("odd", "Odd", count.parity === "odd")}
                ${optionMarkup("even", "Even", count.parity === "even")}
              </select>`,
            )
          : fieldMarkup(
              "Value",
              `<input class="editor-input" type="number" min="0" max="${maxPublicCellCount}" value="${count.value}" data-path="${path}.value" data-type="int" />`,
            )
      }
    </div>
  `;
}

function renderSelectorEditor(path, selector) {
  return `
    <div class="editor-subgrid">
      ${fieldMarkup(
        "Selector",
        `<select class="editor-select" data-path="${path}.kind" data-rerender="true">
          ${optionMarkup("board", "Board", selector.kind === "board")}
          ${optionMarkup("neighbor", "Neighbor", selector.kind === "neighbor")}
          ${optionMarkup("direction", "Direction", selector.kind === "direction")}
          ${optionMarkup("row", "Row", selector.kind === "row")}
          ${optionMarkup("col", "Column", selector.kind === "col")}
          ${optionMarkup("between", "Between", selector.kind === "between")}
          ${optionMarkup("shared_neighbor", "Shared Neighbor", selector.kind === "shared_neighbor")}
        </select>`,
      )}
    </div>
    ${
      selector.kind === "neighbor"
        ? fieldMarkup(
            "Name",
            `<select class="editor-select" data-path="${path}.name">${nameOptionsMarkup(selector.name)}</select>`,
          )
        : ""
    }
    ${
      selector.kind === "direction"
        ? `
          <div class="editor-subgrid">
            ${fieldMarkup(
              "Name",
              `<select class="editor-select" data-path="${path}.name">${nameOptionsMarkup(selector.name)}</select>`,
            )}
            ${fieldMarkup(
              "Direction",
              `<select class="editor-select" data-path="${path}.direction">${directionOptionsMarkup(selector.direction)}</select>`,
            )}
          </div>
        `
        : ""
    }
    ${
      selector.kind === "row"
        ? fieldMarkup(
            "Row",
            `<select class="editor-select" data-path="${path}.row" data-type="int">${rowOptionsMarkup(selector.row)}</select>`,
          )
        : ""
    }
    ${
      selector.kind === "col"
        ? fieldMarkup(
            "Column",
            `<select class="editor-select" data-path="${path}.col" data-type="int">${colOptionsMarkup(selector.col)}</select>`,
          )
        : ""
    }
    ${
      selector.kind === "between" || selector.kind === "shared_neighbor"
        ? `
          <div class="editor-subgrid">
            ${fieldMarkup(
              "First Name",
              `<select class="editor-select" data-path="${path}.firstName">${nameOptionsMarkup(selector.firstName)}</select>`,
            )}
            ${fieldMarkup(
              "Second Name",
              `<select class="editor-select" data-path="${path}.secondName">${nameOptionsMarkup(selector.secondName)}</select>`,
            )}
          </div>
        `
        : ""
    }
  `;
}

function renderGroupEditor(path, group) {
  return `
    <div class="editor-subgrid">
      ${fieldMarkup(
        "Group",
        `<select class="editor-select" data-path="${path}.kind" data-rerender="true">
          ${optionMarkup("any", "Any", group.kind === "any")}
          ${optionMarkup("filter", "Filter", group.kind === "filter")}
          ${optionMarkup("line", "Line", group.kind === "line")}
          ${optionMarkup("role", "Role", group.kind === "role")}
          ${optionMarkup("selected_cells", "Selected Cells", group.kind === "selected_cells")}
        </select>`,
      )}
    </div>
    ${group.kind === "filter" ? renderFilterEditor(`${path}.filter`, group.filter) : ""}
    ${group.kind === "line" ? renderLineEditor(`${path}.line`, group.line) : ""}
    ${
      group.kind === "role"
        ? fieldMarkup(
            "Role",
            `<select class="editor-select" data-path="${path}.role">${roleOptionsMarkup(group.role)}</select>`,
          )
        : ""
    }
    ${
      group.kind === "selected_cells"
        ? `
          ${sectionMarkup("Selected Cells", renderSelectorEditor(`${path}.selected.selector`, group.selected.selector))}
          <div class="editor-subgrid">
            ${fieldMarkup(
              "Answer",
              `<select class="editor-select" data-path="${path}.selected.answer">${answerOptionsMarkup(group.selected.answer)}</select>`,
            )}
          </div>
          ${sectionMarkup("Selected Filter", renderFilterEditor(`${path}.selected.filter`, group.selected.filter))}
        `
        : ""
    }
  `;
}

function renderPredicateEditor(path, predicate) {
  return `
    <div class="editor-subgrid">
      ${fieldMarkup(
        "Predicate",
        `<select class="editor-select" data-path="${path}.kind" data-rerender="true">
          ${optionMarkup("neighbor", "Neighbor Count", predicate.kind === "neighbor")}
          ${optionMarkup("direct_relation", "Direct Relation", predicate.kind === "direct_relation")}
          ${optionMarkup("neighboring", "Neighboring Name", predicate.kind === "neighboring")}
        </select>`,
      )}
    </div>
    ${
      predicate.kind === "neighbor"
        ? `
          <div class="editor-subgrid">
            ${fieldMarkup(
              "Answer",
              `<select class="editor-select" data-path="${path}.answer">${answerOptionsMarkup(predicate.answer)}</select>`,
            )}
          </div>
          ${sectionMarkup("Neighbor Count", renderCountEditor(`${path}.count`, predicate.count))}
          ${sectionMarkup("Neighbor Filter", renderFilterEditor(`${path}.filter`, predicate.filter))}
        `
        : ""
    }
    ${
      predicate.kind === "direct_relation"
        ? `
          <div class="editor-subgrid">
            ${fieldMarkup(
              "Answer",
              `<select class="editor-select" data-path="${path}.answer">${answerOptionsMarkup(predicate.answer)}</select>`,
            )}
            ${fieldMarkup(
              "Direction",
              `<select class="editor-select" data-path="${path}.direction">${directionOptionsMarkup(predicate.direction)}</select>`,
            )}
          </div>
        `
        : ""
    }
    ${
      predicate.kind === "neighboring"
        ? fieldMarkup(
            "Name",
            `<select class="editor-select" data-path="${path}.name">${nameOptionsMarkup(predicate.name)}</select>`,
          )
        : ""
    }
  `;
}

function renderClueFieldsMarkup() {
  sanitizeClueFormState();
  const form = state.clueForm;

  switch (form.kind) {
    case "nonsense":
      return `
        ${fieldMarkup(
          "Preset",
          `<select class="editor-select" data-nonsense-preset="true">${nonsensePresetOptionsMarkup()}</select>`,
        )}
        ${fieldMarkup(
          `Text (max ${state.bootstrap.max_nonsense_text_chars})`,
          `<textarea class="editor-textarea" rows="4" maxlength="${state.bootstrap.max_nonsense_text_chars}" data-path="nonsense.text">${escapeHtml(form.nonsense.text)}</textarea>`,
        )}
      `;
    case "declaration":
      return `
        <div class="editor-subgrid">
          ${fieldMarkup(
            "Name",
            `<select class="editor-select" data-path="declaration.name">${nameOptionsMarkup(form.declaration.name)}</select>`,
          )}
          ${fieldMarkup(
            "Answer",
            `<select class="editor-select" data-path="declaration.answer">${answerOptionsMarkup(form.declaration.answer)}</select>`,
          )}
        </div>
      `;
    case "count_cells":
      return `
        ${sectionMarkup("Selector", renderSelectorEditor("countCells.selector", form.countCells.selector))}
        <div class="editor-subgrid">
          ${fieldMarkup(
            "Answer",
            `<select class="editor-select" data-path="countCells.answer">${answerOptionsMarkup(form.countCells.answer)}</select>`,
          )}
        </div>
        ${sectionMarkup("Count", renderCountEditor("countCells.count", form.countCells.count))}
        ${sectionMarkup("Filter", renderFilterEditor("countCells.filter", form.countCells.filter))}
      `;
    case "named_count_cells":
      return `
        ${fieldMarkup(
          "Named Person",
          `<select class="editor-select" data-path="namedCountCells.name">${nameOptionsMarkup(form.namedCountCells.name)}</select>`,
        )}
        ${sectionMarkup("Selector", renderSelectorEditor("namedCountCells.selector", form.namedCountCells.selector))}
        <div class="editor-subgrid">
          ${fieldMarkup(
            "Answer",
            `<select class="editor-select" data-path="namedCountCells.answer">${answerOptionsMarkup(form.namedCountCells.answer)}</select>`,
          )}
          ${fieldMarkup(
            "Number",
            `<input class="editor-input" type="number" min="0" max="${maxPublicCellCount}" value="${form.namedCountCells.number}" data-path="namedCountCells.number" data-type="int" />`,
          )}
        </div>
        ${sectionMarkup(
          "Filter",
          renderFilterEditor("namedCountCells.filter", form.namedCountCells.filter),
        )}
      `;
    case "connected":
      return `
        <div class="editor-subgrid">
          ${fieldMarkup(
            "Answer",
            `<select class="editor-select" data-path="connected.answer">${answerOptionsMarkup(form.connected.answer)}</select>`,
          )}
        </div>
        ${sectionMarkup("Line", renderLineEditor("connected.line", form.connected.line))}
      `;
    case "direct_relation":
      return `
        <div class="editor-subgrid">
          ${fieldMarkup(
            "Name",
            `<select class="editor-select" data-path="directRelation.name">${nameOptionsMarkup(form.directRelation.name)}</select>`,
          )}
          ${fieldMarkup(
            "Answer",
            `<select class="editor-select" data-path="directRelation.answer">${answerOptionsMarkup(form.directRelation.answer)}</select>`,
          )}
          ${fieldMarkup(
            "Direction",
            `<select class="editor-select" data-path="directRelation.direction">${directionOptionsMarkup(form.directRelation.direction)}</select>`,
          )}
        </div>
      `;
    case "role_count":
      return `
        <div class="editor-subgrid">
          ${fieldMarkup(
            "Role",
            `<select class="editor-select" data-path="roleCount.role">${roleOptionsMarkup(form.roleCount.role)}</select>`,
          )}
          ${fieldMarkup(
            "Answer",
            `<select class="editor-select" data-path="roleCount.answer">${answerOptionsMarkup(form.roleCount.answer)}</select>`,
          )}
        </div>
        ${sectionMarkup("Count", renderCountEditor("roleCount.count", form.roleCount.count))}
      `;
    case "roles_comparison":
      return `
        <div class="editor-subgrid">
          ${fieldMarkup(
            "First Role",
            `<select class="editor-select" data-path="rolesComparison.firstRole">${roleOptionsMarkup(form.rolesComparison.firstRole)}</select>`,
          )}
          ${fieldMarkup(
            "Second Role",
            `<select class="editor-select" data-path="rolesComparison.secondRole">${roleOptionsMarkup(form.rolesComparison.secondRole)}</select>`,
          )}
        </div>
        <div class="editor-subgrid">
          ${fieldMarkup(
            "Answer",
            `<select class="editor-select" data-path="rolesComparison.answer">${answerOptionsMarkup(form.rolesComparison.answer)}</select>`,
          )}
          ${fieldMarkup(
            "Comparison",
            `<select class="editor-select" data-path="rolesComparison.comparison">${comparisonOptionsMarkup(form.rolesComparison.comparison)}</select>`,
          )}
        </div>
      `;
    case "line_comparison":
      return `
        ${sectionMarkup("First Line", renderLineEditor("lineComparison.firstLine", form.lineComparison.firstLine))}
        ${sectionMarkup("Second Line", renderLineEditor("lineComparison.secondLine", form.lineComparison.secondLine))}
        <div class="editor-subgrid">
          ${fieldMarkup(
            "Answer",
            `<select class="editor-select" data-path="lineComparison.answer">${answerOptionsMarkup(form.lineComparison.answer)}</select>`,
          )}
          ${fieldMarkup(
            "Comparison",
            `<select class="editor-select" data-path="lineComparison.comparison">${comparisonOptionsMarkup(form.lineComparison.comparison)}</select>`,
          )}
        </div>
      `;
    case "quantified":
      return `
        <div class="editor-subgrid">
          ${fieldMarkup(
            "Exactly",
            `<input class="editor-input" type="number" min="0" max="${maxPublicCellCount}" value="${form.quantified.quantifier.value}" data-path="quantified.quantifier.value" data-type="int" />`,
          )}
        </div>
        ${sectionMarkup("Group", renderGroupEditor("quantified.group", form.quantified.group))}
        ${sectionMarkup(
          "Predicate",
          renderPredicateEditor("quantified.predicate", form.quantified.predicate),
        )}
      `;
    default:
      return "";
  }
}

function buildLine(line) {
  return line.kind === "col"
    ? { col: clampInt(line.col, 0, state.boardSize.cols - 1) }
    : { row: clampInt(line.row, 0, state.boardSize.rows - 1) };
}

function buildFilter(filter) {
  switch (filter.kind) {
    case "edge":
      return "edge";
    case "corner":
      return "corner";
    case "line":
      return { line: buildLine(filter.line) };
    default:
      return "any";
  }
}

function buildCount(count) {
  if (count.kind === "parity") {
    return {
      kind: "parity",
      value: count.parity === "even" ? "even" : "odd",
    };
  }

  return {
    kind: count.kind === "at_least" ? "at_least" : "number",
    value: clampInt(count.value, 0, maxPublicCellCount),
  };
}

function buildSelector(selector) {
  switch (selector.kind) {
    case "neighbor":
      return { kind: "neighbor", name: selector.name };
    case "direction":
      return {
        kind: "direction",
        name: selector.name,
        direction: selector.direction,
      };
    case "row":
      return { kind: "row", row: clampInt(selector.row, 0, state.boardSize.rows - 1) };
    case "col":
      return { kind: "col", col: clampInt(selector.col, 0, state.boardSize.cols - 1) };
    case "between":
      return {
        kind: "between",
        first_name: selector.firstName,
        second_name: selector.secondName,
      };
    case "shared_neighbor":
      return {
        kind: "shared_neighbor",
        first_name: selector.firstName,
        second_name: selector.secondName,
      };
    default:
      return { kind: "board" };
  }
}

function buildGroup(group) {
  switch (group.kind) {
    case "filter":
      return {
        kind: "filter",
        filter: buildFilter(group.filter),
      };
    case "line":
      return {
        kind: "line",
        line: buildLine(group.line),
      };
    case "role":
      return {
        kind: "role",
        role: group.role,
      };
    case "selected_cells":
      return {
        kind: "selected_cells",
        selector: buildSelector(group.selected.selector),
        answer: group.selected.answer,
        filter: buildFilter(group.selected.filter),
      };
    default:
      return { kind: "any" };
  }
}

function buildPredicate(predicate) {
  switch (predicate.kind) {
    case "direct_relation":
      return {
        kind: "direct_relation",
        answer: predicate.answer,
        direction: predicate.direction,
      };
    case "neighboring":
      return {
        kind: "neighboring",
        name: predicate.name,
      };
    default:
      return {
        kind: "neighbor",
        answer: predicate.answer,
        count: buildCount(predicate.count),
        filter: buildFilter(predicate.filter),
      };
  }
}

function buildClueFromState() {
  sanitizeClueFormState();
  const form = state.clueForm;

  switch (form.kind) {
    case "nonsense": {
      const text = form.nonsense.text.trim();
      if (text.length === 0) {
        throw new Error("Nonsense clues need some text.");
      }
      return {
        kind: "nonsense",
        text,
      };
    }
    case "declaration":
      return {
        kind: "declaration",
        name: form.declaration.name,
        answer: form.declaration.answer,
      };
    case "count_cells":
      return {
        kind: "count_cells",
        selector: buildSelector(form.countCells.selector),
        answer: form.countCells.answer,
        count: buildCount(form.countCells.count),
        filter: buildFilter(form.countCells.filter),
      };
    case "named_count_cells":
      return {
        kind: "named_count_cells",
        name: form.namedCountCells.name,
        selector: buildSelector(form.namedCountCells.selector),
        answer: form.namedCountCells.answer,
        number: clampInt(form.namedCountCells.number, 0, maxPublicCellCount),
        filter: buildFilter(form.namedCountCells.filter),
      };
    case "connected":
      return {
        kind: "connected",
        answer: form.connected.answer,
        line: buildLine(form.connected.line),
      };
    case "direct_relation":
      return {
        kind: "direct_relation",
        name: form.directRelation.name,
        answer: form.directRelation.answer,
        direction: form.directRelation.direction,
      };
    case "role_count":
      return {
        kind: "role_count",
        role: form.roleCount.role,
        answer: form.roleCount.answer,
        count: buildCount(form.roleCount.count),
      };
    case "roles_comparison":
      return {
        kind: "roles_comparison",
        first_role: form.rolesComparison.firstRole,
        second_role: form.rolesComparison.secondRole,
        answer: form.rolesComparison.answer,
        comparison: form.rolesComparison.comparison,
      };
    case "line_comparison":
      return {
        kind: "line_comparison",
        first_line: buildLine(form.lineComparison.firstLine),
        second_line: buildLine(form.lineComparison.secondLine),
        answer: form.lineComparison.answer,
        comparison: form.lineComparison.comparison,
      };
    case "quantified":
      return {
        kind: "quantified",
        quantifier: {
          kind: "exactly",
          value: clampInt(form.quantified.quantifier.value, 0, maxPublicCellCount),
        },
        group: buildGroup(form.quantified.group),
        predicate: buildPredicate(form.quantified.predicate),
      };
    default:
      throw new Error("Unknown clue kind.");
  }
}

function randomChoice(values) {
  if (!Array.isArray(values) || values.length === 0) {
    return null;
  }

  return values[Math.floor(Math.random() * values.length)] ?? null;
}

function randomNonsenseText(answer) {
  const pool =
    answer === "criminal"
      ? state.bootstrap.criminal_nonsense_texts
      : state.bootstrap.nonsense_texts;

  return randomChoice(pool) ?? "";
}

function lineFormFromClue(line) {
  if (line && typeof line === "object" && "col" in line) {
    return {
      kind: "col",
      row: 0,
      col: clampInt(line.col, 0, Math.max(0, state.boardSize.cols - 1)),
    };
  }

  return {
    kind: "row",
    row: clampInt(line?.row ?? 0, 0, Math.max(0, state.boardSize.rows - 1)),
    col: 0,
  };
}

function filterFormFromClue(filter) {
  if (filter === "edge") {
    return { kind: "edge", line: createDefaultLine() };
  }
  if (filter === "corner") {
    return { kind: "corner", line: createDefaultLine() };
  }
  if (filter && typeof filter === "object" && "line" in filter) {
    return {
      kind: "line",
      line: lineFormFromClue(filter.line),
    };
  }

  return {
    kind: "any",
    line: createDefaultLine(),
  };
}

function countFormFromClue(count) {
  if (count?.kind === "at_least") {
    return {
      kind: "at_least",
      value: clampInt(count.value, 0, maxPublicCellCount),
      parity: "odd",
    };
  }
  if (count?.kind === "parity") {
    return {
      kind: "parity",
      value: 1,
      parity: count.value === "even" ? "even" : "odd",
    };
  }

  return {
    kind: "number",
    value: clampInt(count?.value ?? 0, 0, maxPublicCellCount),
    parity: "odd",
  };
}

function selectorFormFromClue(selector) {
  switch (selector?.kind) {
    case "neighbor":
      return {
        kind: "neighbor",
        name: selector.name,
        direction: "above",
        row: 0,
        col: 0,
        firstName: firstName(boardNames()),
        secondName: secondName(boardNames()),
      };
    case "direction":
      return {
        kind: "direction",
        name: selector.name,
        direction: selector.direction,
        row: 0,
        col: 0,
        firstName: firstName(boardNames()),
        secondName: secondName(boardNames()),
      };
    case "row":
      return {
        kind: "row",
        name: firstName(boardNames()),
        direction: "above",
        row: clampInt(selector.row, 0, Math.max(0, state.boardSize.rows - 1)),
        col: 0,
        firstName: firstName(boardNames()),
        secondName: secondName(boardNames()),
      };
    case "col":
      return {
        kind: "col",
        name: firstName(boardNames()),
        direction: "above",
        row: 0,
        col: clampInt(selector.col, 0, Math.max(0, state.boardSize.cols - 1)),
        firstName: firstName(boardNames()),
        secondName: secondName(boardNames()),
      };
    case "between":
      return {
        kind: "between",
        name: firstName(boardNames()),
        direction: "above",
        row: 0,
        col: 0,
        firstName: selector.first_name,
        secondName: selector.second_name,
      };
    case "shared_neighbor":
      return {
        kind: "shared_neighbor",
        name: firstName(boardNames()),
        direction: "above",
        row: 0,
        col: 0,
        firstName: selector.first_name,
        secondName: selector.second_name,
      };
    default:
      return createDefaultSelector(boardNames());
  }
}

function groupFormFromClue(group) {
  switch (group?.kind) {
    case "filter":
      return {
        kind: "filter",
        filter: filterFormFromClue(group.filter),
        line: createDefaultLine(),
        role: firstRole(roleCatalog()),
        selected: {
          selector: createDefaultSelector(boardNames()),
          answer: "innocent",
          filter: createDefaultFilter(),
        },
      };
    case "line":
      return {
        kind: "line",
        filter: createDefaultFilter(),
        line: lineFormFromClue(group.line),
        role: firstRole(roleCatalog()),
        selected: {
          selector: createDefaultSelector(boardNames()),
          answer: "innocent",
          filter: createDefaultFilter(),
        },
      };
    case "role":
      return {
        kind: "role",
        filter: createDefaultFilter(),
        line: createDefaultLine(),
        role: group.role,
        selected: {
          selector: createDefaultSelector(boardNames()),
          answer: "innocent",
          filter: createDefaultFilter(),
        },
      };
    case "selected_cells":
      return {
        kind: "selected_cells",
        filter: createDefaultFilter(),
        line: createDefaultLine(),
        role: firstRole(roleCatalog()),
        selected: {
          selector: selectorFormFromClue(group.selector),
          answer: group.answer,
          filter: filterFormFromClue(group.filter),
        },
      };
    default:
      return createDefaultGroup(boardNames(), roleCatalog());
  }
}

function predicateFormFromClue(predicate) {
  switch (predicate?.kind) {
    case "direct_relation":
      return {
        kind: "direct_relation",
        answer: predicate.answer,
        count: createDefaultCount(),
        filter: createDefaultFilter(),
        direction: predicate.direction,
        name: firstName(boardNames()),
      };
    case "neighboring":
      return {
        kind: "neighboring",
        answer: "innocent",
        count: createDefaultCount(),
        filter: createDefaultFilter(),
        direction: "above",
        name: predicate.name,
      };
    default:
      return {
        kind: "neighbor",
        answer: predicate?.answer === "criminal" ? "criminal" : "innocent",
        count: countFormFromClue(predicate?.count),
        filter: filterFormFromClue(predicate?.filter),
        direction: "above",
        name: firstName(boardNames()),
      };
  }
}

function applySuggestedClue(clue) {
  const form = createDefaultClueForm();
  form.kind = clue.kind;

  switch (clue.kind) {
    case "nonsense":
      form.nonsense.text = clue.text ?? "";
      break;
    case "declaration":
      form.declaration.name = clue.name;
      form.declaration.answer = clue.answer;
      break;
    case "count_cells":
      form.countCells.selector = selectorFormFromClue(clue.selector);
      form.countCells.answer = clue.answer;
      form.countCells.count = countFormFromClue(clue.count);
      form.countCells.filter = filterFormFromClue(clue.filter);
      break;
    case "named_count_cells":
      form.namedCountCells.name = clue.name;
      form.namedCountCells.selector = selectorFormFromClue(clue.selector);
      form.namedCountCells.answer = clue.answer;
      form.namedCountCells.number = clampInt(clue.number, 0, maxPublicCellCount);
      form.namedCountCells.filter = filterFormFromClue(clue.filter);
      break;
    case "connected":
      form.connected.answer = clue.answer;
      form.connected.line = lineFormFromClue(clue.line);
      break;
    case "direct_relation":
      form.directRelation.name = clue.name;
      form.directRelation.answer = clue.answer;
      form.directRelation.direction = clue.direction;
      break;
    case "role_count":
      form.roleCount.role = clue.role;
      form.roleCount.answer = clue.answer;
      form.roleCount.count = countFormFromClue(clue.count);
      break;
    case "roles_comparison":
      form.rolesComparison.firstRole = clue.first_role;
      form.rolesComparison.secondRole = clue.second_role;
      form.rolesComparison.answer = clue.answer;
      form.rolesComparison.comparison = clue.comparison;
      break;
    case "line_comparison":
      form.lineComparison.firstLine = lineFormFromClue(clue.first_line);
      form.lineComparison.secondLine = lineFormFromClue(clue.second_line);
      form.lineComparison.answer = clue.answer;
      form.lineComparison.comparison = clue.comparison;
      break;
    case "quantified":
      form.quantified.quantifier.value = clampInt(
        clue.quantifier?.value ?? 1,
        0,
        maxPublicCellCount,
      );
      form.quantified.group = groupFormFromClue(clue.group);
      form.quantified.predicate = predicateFormFromClue(clue.predicate);
      break;
    default:
      break;
  }

  state.clueForm = form;
  sanitizeClueFormState();
  persistEditorState();
  renderInspector();
}

function lineText(line) {
  if ("row" in line) {
    return `row ${rowLabel(line.row)}`;
  }

  return `col ${colLabel(line.col)}`;
}

function filterSuffixText(filter) {
  if (filter === "edge") {
    return " on the edges";
  }
  if (filter === "corner") {
    return " in the corners";
  }
  if (filter && typeof filter === "object" && "line" in filter) {
    return ` in ${lineText(filter.line)}`;
  }
  return "";
}

function countDescriptionText(count, noun) {
  if (count.kind === "at_least") {
    return `at least ${count.value} ${noun}`;
  }
  if (count.kind === "parity") {
    return `an ${count.value} number of ${noun}`;
  }
  return `${count.value} ${noun}`;
}

function directionText(direction) {
  if (direction === "below") {
    return "below";
  }
  if (direction === "left") {
    return "left of";
  }
  if (direction === "right") {
    return "right of";
  }
  return "above";
}

function comparisonText(comparison, left, right) {
  if (comparison === "fewer") {
    return `there are fewer ${left} than there are ${right}`;
  }
  if (comparison === "equal") {
    return `there are as many ${left} as there are ${right}`;
  }
  return `there are more ${left} than there are ${right}`;
}

function comparisonTextIn(comparison, noun, firstScope, secondScope) {
  if (comparison === "fewer") {
    return `there are fewer ${noun} in ${firstScope} than in ${secondScope}`;
  }
  if (comparison === "equal") {
    return `there are as many ${noun} in ${firstScope} as in ${secondScope}`;
  }
  return `there are more ${noun} in ${firstScope} than in ${secondScope}`;
}

function pluralizeRole(role) {
  const lowered = role.toLowerCase();
  if (
    lowered.endsWith("ch") ||
    lowered.endsWith("sh") ||
    lowered.endsWith("s") ||
    lowered.endsWith("x") ||
    lowered.endsWith("z")
  ) {
    return `${lowered}es`;
  }
  return `${lowered}s`;
}

function answerRolesText(answer, role) {
  return `${answer} ${pluralizeRole(role)}`;
}

function answerWithArticleText(answer) {
  return answer === "criminal" ? "a criminal" : "an innocent";
}

function possessiveText(name) {
  return name.endsWith("s") ? `${name}'` : `${name}'s`;
}

function pluralizeAnswerText(answer, singular) {
  return singular ? answer : `${answer}s`;
}

function directionScopeText(direction, name) {
  if (direction === "below") {
    return `below ${name}`;
  }
  if (direction === "left") {
    return `to the left of ${name}`;
  }
  if (direction === "right") {
    return `to the right of ${name}`;
  }
  return `above ${name}`;
}

function selectorText(selector, answer, count, filter) {
  const suffix = filterSuffixText(filter);
  switch (selector.kind) {
    case "board":
      return `There are ${countDescriptionText(count, `${answer}s`)}${suffix}`;
    case "neighbor":
      return `${selector.name} has ${countDescriptionText(count, answer)} neighbors${suffix}`;
    case "direction":
      return `there are ${countDescriptionText(count, `${answer}s`)} ${directionText(selector.direction)} ${selector.name}${suffix}`;
    case "row":
      return `Row ${rowLabel(selector.row)} has ${countDescriptionText(count, `${answer}s`)}${suffix}`;
    case "col":
      return `Col ${colLabel(selector.col)} has ${countDescriptionText(count, `${answer}s`)}${suffix}`;
    case "between":
      return `there are ${countDescriptionText(count, `${answer}s`)} between ${selector.first_name} and ${selector.second_name}${suffix}`;
    case "shared_neighbor":
      return `${selector.first_name} and ${selector.second_name} share ${countDescriptionText(count, answer)} neighbors${suffix}`;
    default:
      return "";
  }
}

function namedCountCellsText(clue) {
  const suffix = filterSuffixText(clue.filter);
  switch (clue.selector.kind) {
    case "board":
      return `${clue.name} is one of the ${clue.number} ${clue.answer}s${suffix}`;
    case "neighbor":
      return `${clue.name} is one of ${possessiveText(clue.selector.name)} ${clue.number} ${clue.answer} neighbors${suffix}`;
    case "direction":
      return `${clue.name} is one of the ${clue.number} ${clue.answer}s ${directionText(clue.selector.direction)} ${clue.selector.name}${suffix}`;
    case "row":
      return `${clue.name} is one of the ${clue.number} ${clue.answer}s in row ${rowLabel(clue.selector.row)}${suffix}`;
    case "col":
      return `${clue.name} is one of the ${clue.number} ${clue.answer}s in col ${colLabel(clue.selector.col)}${suffix}`;
    case "between":
      return `${clue.name} is one of the ${clue.number} ${clue.answer}s between ${clue.selector.first_name} and ${clue.selector.second_name}${suffix}`;
    case "shared_neighbor":
      return `${clue.name} is one of the ${clue.number} ${clue.answer} neighbors shared by ${clue.selector.first_name} and ${clue.selector.second_name}${suffix}`;
    default:
      return "";
  }
}

function groupText(group, singular) {
  if (group.kind === "any") {
    return singular ? "person" : "people";
  }

  if (group.kind === "filter") {
    if (group.filter === "edge") {
      return singular ? "person on the edges" : "people on the edges";
    }
    if (group.filter === "corner") {
      return singular ? "person in the corners" : "people in the corners";
    }
    if (group.filter && typeof group.filter === "object" && "line" in group.filter) {
      return singular
        ? `person in ${lineText(group.filter.line)}`
        : `people in ${lineText(group.filter.line)}`;
    }
    return singular ? "person" : "people";
  }

  if (group.kind === "line") {
    return singular
      ? `person in ${lineText(group.line)}`
      : `people in ${lineText(group.line)}`;
  }

  if (group.kind === "role") {
    return singular ? group.role.toLowerCase() : pluralizeRole(group.role);
  }

  const answerText = pluralizeAnswerText(group.answer, singular);
  const suffix = filterSuffixText(group.filter);
  switch (group.selector.kind) {
    case "board":
      return `${answerText}${suffix}`;
    case "neighbor":
      return `${answerText} ${singular ? "neighbor" : "neighbors"} of ${group.selector.name}${suffix}`;
    case "direction":
      return `${answerText} ${directionScopeText(group.selector.direction, group.selector.name)}${suffix}`;
    case "row":
      return `${answerText} in row ${rowLabel(group.selector.row)}${suffix}`;
    case "col":
      return `${answerText} in col ${colLabel(group.selector.col)}${suffix}`;
    case "between":
      return `${answerText} between ${group.selector.first_name} and ${group.selector.second_name}${suffix}`;
    case "shared_neighbor":
      return `${answerText} ${singular ? "neighbor" : "neighbors"} shared by ${group.selector.first_name} and ${group.selector.second_name}${suffix}`;
    default:
      return "";
  }
}

function predicateText(predicate, singular) {
  if (predicate.kind === "neighbor") {
    return `${singular ? "has" : "have"} ${countDescriptionText(predicate.count, predicate.answer)} neighbors${filterSuffixText(predicate.filter)}`;
  }
  if (predicate.kind === "direct_relation") {
    return `${singular ? "is" : "are"} directly ${directionText(predicate.direction)} ${answerWithArticleText(predicate.answer)}`;
  }
  return `${singular ? "is" : "are"} neighboring ${predicate.name}`;
}

function clueText(clue) {
  switch (clue.kind) {
    case "nonsense":
      return clue.text;
    case "declaration":
      return `${clue.name} is ${clue.answer}`;
    case "count_cells":
      return selectorText(clue.selector, clue.answer, clue.count, clue.filter);
    case "named_count_cells":
      return namedCountCellsText(clue);
    case "connected":
      return `all ${clue.answer}s in ${lineText(clue.line)} are connected`;
    case "direct_relation":
      return `there is ${answerWithArticleText(clue.answer)} directly ${directionText(clue.direction)} ${clue.name}`;
    case "role_count":
      return `there are ${countDescriptionText(clue.count, answerRolesText(clue.answer, clue.role))}`;
    case "roles_comparison":
      return comparisonText(
        clue.comparison,
        answerRolesText(clue.answer, clue.first_role),
        answerRolesText(clue.answer, clue.second_role),
      );
    case "line_comparison":
      return comparisonTextIn(
        clue.comparison,
        `${clue.answer}s`,
        lineText(clue.first_line),
        lineText(clue.second_line),
      );
    case "quantified": {
      const singular = clue.quantifier.kind === "exactly" && clue.quantifier.value === 1;
      return `Exactly ${clue.quantifier.value} ${groupText(clue.group, singular)} ${predicateText(clue.predicate, singular)}`;
    }
    default:
      return "";
  }
}

function cluePreviewText() {
  try {
    return clueText(buildClueFromState());
  } catch {
    if (state.clueForm?.kind === "nonsense") {
      return state.clueForm.nonsense.text.trim();
    }
    return "";
  }
}

function renderCluePreview() {
  if (!cluePreviewEl) {
    return;
  }

  cluePreviewEl.textContent = cluePreviewText();
}

function currentClueValidationContext(view = selectedCellView()) {
  if (!view || !clueBuilderActiveForView(view) || !state.response) {
    return null;
  }

  const clue = buildClueFromState();
  if (isEditableNonsenseTile(view)) {
    const action = {
      kind: "update_nonsense_clue",
      row: view.row,
      col: view.col,
      text: clue.text,
    };
    return {
      draft: state.response.draft,
      action,
      key: JSON.stringify({ draft: state.response.draft, action }),
    };
  }

  const action = {
    kind: "add_clue",
    row: view.row,
    col: view.col,
    clue,
  };
  const draft =
    isLastAddedClueTile(view) && lastClueProgressionIndex() >= 0
      ? revertProgressionDraft(state.response.draft, lastClueProgressionAction())
      : state.response.draft;
  return {
    draft,
    action,
    key: JSON.stringify({ draft, action }),
  };
}

function requestClueSaveValidation(view = selectedCellView()) {
  if (!view || !clueBuilderActiveForView(view) || !state.response) {
    state.clueSaveValidation = {
      key: null,
      status: "invalid",
      requested: false,
    };
    renderSaveClueButton();
    return;
  }

  let validationContext;
  try {
    validationContext = currentClueValidationContext(view);
  } catch {
    state.clueSaveValidation = {
      key: null,
      status: "invalid",
      requested: false,
    };
    renderSaveClueButton();
    return;
  }

  if (!validationContext) {
    state.clueSaveValidation = {
      key: null,
      status: "invalid",
      requested: false,
    };
    renderSaveClueButton();
    return;
  }

  if (
    state.clueSaveValidation.key === validationContext.key &&
    (state.clueSaveValidation.status !== "pending" ||
      state.clueSaveValidation.requested)
  ) {
    renderSaveClueButton();
    return;
  }

  state.clueSaveValidation = {
    key: validationContext.key,
    status: "pending",
    requested: !state.busy,
  };
  renderSaveClueButton();

  if (state.busy) {
    return;
  }

  void requestApplyAction(validationContext.draft, validationContext.action)
    .then(() => {
      if (state.clueSaveValidation.key !== validationContext.key) {
        return;
      }
      state.clueSaveValidation = {
        key: validationContext.key,
        status: "valid",
        requested: false,
      };
      renderSaveClueButton();
    })
    .catch(() => {
      if (state.clueSaveValidation.key !== validationContext.key) {
        return;
      }
      state.clueSaveValidation = {
        key: validationContext.key,
        status: "invalid",
        requested: false,
      };
      renderSaveClueButton();
    });
}

function selectedCellView() {
  if (!state.response || !state.selected) {
    return null;
  }

  const { row, col } = state.selected;
  const cell = state.response.draft.cells[row]?.[col];
  if (!cell) {
    return null;
  }

  return {
    row,
    col,
    cell,
    renderedClue: state.response.rendered_clues[row][col],
    resolvedAnswer: state.response.resolved_answers[row][col],
    isInitialReveal:
      state.response.draft.initial_reveal?.row === row &&
      state.response.draft.initial_reveal?.col === col,
    canAddClue: state.response.next_clue_targets.some(
      (position) => position.row === row && position.col === col,
    ),
  };
}

function summarizeProgressionAction(action) {
  return {
    kind: action.kind,
    row: Number.isInteger(action.row) ? action.row : null,
    col: Number.isInteger(action.col) ? action.col : null,
  };
}

function lastClueProgressionIndex() {
  for (let index = state.progression.length - 1; index >= 0; index -= 1) {
    if (state.progression[index]?.kind === "add_clue") {
      return index;
    }
  }

  return -1;
}

function lastClueProgressionAction() {
  const index = lastClueProgressionIndex();
  return index >= 0 ? state.progression[index] : null;
}

function isLastAddedClueTile(view) {
  if (!view || view.cell.clue === null) {
    return false;
  }

  const lastAction = lastClueProgressionAction();
  return (
    lastAction?.kind === "add_clue" &&
    lastAction.row === view.row &&
    lastAction.col === view.col
  );
}

function isEditableNonsenseTile(view) {
  return view?.cell.clue?.kind === "nonsense";
}

function clueBuilderActiveForView(view) {
  return (
    (view.cell.clue === null && view.canAddClue) ||
    isLastAddedClueTile(view) ||
    isEditableNonsenseTile(view)
  );
}

function syncClueFormFromView(view) {
  if (!view || !isEditableNonsenseTile(view)) {
    return;
  }

  state.clueForm.kind = "nonsense";
  state.clueForm.nonsense.text = view.cell.clue.text;
}

function currentDraftKey() {
  return state.response ? JSON.stringify(state.response.draft) : null;
}

function clearVisibleShareLink() {
  state.visibleShareUrl = null;
}

function readSavedEditorState() {
  try {
    const raw = window.localStorage.getItem(editorStorageKey);
    if (!raw) {
      return null;
    }

    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") {
      return null;
    }

    return parsed;
  } catch {
    return null;
  }
}

function persistEditorState() {
  try {
    const payload = {
      boardSize: state.boardSize,
      pendingBoardSize: state.pendingBoardSize,
      draft: state.response?.draft ?? null,
      progression: state.progression,
      selected: state.selected,
      clueForm: state.clueForm,
      lastSharedDraftKey: state.lastSharedDraftKey,
      lastSharedStoredPuzzleId: state.lastSharedStoredPuzzleId,
    };
    window.localStorage.setItem(editorStorageKey, JSON.stringify(payload));
  } catch {}
}

function restoreSavedEditorState(saved) {
  if (!saved || typeof saved !== "object") {
    return false;
  }

  const rows = clampInt(saved.boardSize?.rows, 1, maxBoardDimension);
  const cols = clampInt(
    saved.boardSize?.cols,
    1,
    Math.max(1, Math.floor(maxPublicCellCount / rows)),
  );
  const pendingRows = clampInt(saved.pendingBoardSize?.rows ?? saved.boardSize?.rows, 1, maxBoardDimension);
  const pendingCols = clampInt(
    saved.pendingBoardSize?.cols ?? saved.boardSize?.cols,
    1,
    Math.max(1, Math.floor(maxPublicCellCount / pendingRows)),
  );

  state.boardSize = { rows, cols };
  state.pendingBoardSize = { rows: pendingRows, cols: pendingCols };
  const savedDraft = saved.draft ?? saved.response?.draft ?? null;
  state.response = savedDraft ? { draft: savedDraft } : null;
  state.progression = Array.isArray(saved.progression)
    ? saved.progression
        .filter((action) => action && typeof action === "object")
        .map((action) => summarizeProgressionAction(action))
        .filter(
          (action) =>
            action.kind === "add_clue" &&
            Number.isInteger(action.row) &&
            Number.isInteger(action.col),
        )
    : [];
  state.selected =
    Number.isInteger(saved.selected?.row) && Number.isInteger(saved.selected?.col)
      ? { row: saved.selected.row, col: saved.selected.col }
      : null;
  state.clueForm = saved.clueForm ?? createDefaultClueForm();
  state.lastSharedDraftKey =
    typeof saved.lastSharedDraftKey === "string" ? saved.lastSharedDraftKey : null;
  state.lastSharedStoredPuzzleId =
    typeof saved.lastSharedStoredPuzzleId === "string"
      ? saved.lastSharedStoredPuzzleId
      : null;
  state.editDraft = null;
  closeEmojiPicker();
  closeCustomRoleModal();

  return state.response !== null && state.response?.draft !== undefined;
}

async function fetchJson(url, options = {}) {
  const response = await fetch(url, {
    headers: {
      "Content-Type": "application/json",
      ...(options.headers ?? {}),
    },
    ...options,
  });
  const data = await response.json().catch(() => null);
  if (!response.ok) {
    throw new Error(data?.error ?? "Request failed.");
  }
  return data;
}

async function copyText(text) {
  if (!navigator.clipboard?.writeText) {
    return false;
  }

  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

function showError(title, message) {
  errorTitleEl.textContent = title;
  errorMessageEl.textContent = message;
  errorModalEl.hidden = false;
}

function hideError() {
  errorModalEl.hidden = true;
}

function setBusy(busy) {
  state.busy = busy;
  if (state.response) {
    render();
  } else {
    renderTopControls();
  }
}

function ensureSelectionInBounds() {
  if (!state.response || !state.selected) {
    return;
  }

  const { row, col } = state.selected;
  if (!state.response.draft.cells[row]?.[col]) {
    state.selected = null;
  }
}

async function loadBootstrap() {
  state.bootstrap = await fetchJson("/api/editor/bootstrap", { method: "GET" });
}

async function describeDraft(draft) {
  return fetchJson("/api/editor/describe", {
    method: "POST",
    body: JSON.stringify({ draft }),
  });
}

async function requestSuggestedClue(draft, row, col) {
  return fetchJson("/api/editor/suggest", {
    method: "POST",
    body: JSON.stringify({ draft, row, col }),
  });
}

function populateSizeControls() {
  const validCols = Math.max(1, Math.floor(maxPublicCellCount / state.pendingBoardSize.rows));
  if (state.pendingBoardSize.cols > validCols) {
    state.pendingBoardSize.cols = validCols;
  }

  rowsSelectEl.innerHTML = Array.from({ length: maxBoardDimension }, (_, index) =>
    optionMarkup(
      String(index + 1),
      String(index + 1),
      state.pendingBoardSize.rows === index + 1,
    ),
  ).join("");

  colsSelectEl.innerHTML = Array.from({ length: validCols }, (_, index) =>
    optionMarkup(
      String(index + 1),
      String(index + 1),
      state.pendingBoardSize.cols === index + 1,
    ),
  ).join("");
}

async function loadDraft(rows, cols) {
  const preservedAuthor =
    state.response?.draft.author ?? (authorInputEl.value === "" ? null : authorInputEl.value);
  setBusy(true);
  try {
    const response = await fetchJson("/api/editor/new", {
      method: "POST",
      body: JSON.stringify({ rows, cols }),
    });
    response.draft.author = preservedAuthor;
    state.boardSize = { rows, cols };
    state.pendingBoardSize = { rows, cols };
    state.response = response;
    state.progression = [];
    state.selected = null;
    state.isEditingDetails = false;
    state.editDraft = null;
    closeEmojiPicker();
    closeCustomRoleModal();
    state.clueForm = createDefaultClueForm();
    clearVisibleShareLink();
    ensureSelectionInBounds();
    hideError();
    persistEditorState();
    render();
  } finally {
    setBusy(false);
  }
}

async function requestApplyAction(draft, action) {
  return fetchJson("/api/editor/apply", {
    method: "POST",
    body: JSON.stringify({
      draft,
      action,
    }),
  });
}

function renderSaveClueButton() {
  const saveable = state.clueSaveValidation.status === "valid";
  saveClueButton.classList.toggle("button-good", saveable);
  saveClueButton.classList.toggle("button-bad", !saveable);
  saveClueButton.classList.remove("button-primary");
}

async function applyUndoableAction(action) {
  if (!state.response) {
    return;
  }

  setBusy(true);

  try {
    const response = await requestApplyAction(state.response.draft, action);
    if (action.kind === "add_clue") {
      state.progression.push(summarizeProgressionAction(action));
    }
    state.response = response;
    state.isEditingDetails = false;
    state.editDraft = null;
    closeEmojiPicker();
    closeCustomRoleModal();
    sanitizeClueFormState();
    ensureSelectionInBounds();
    clearVisibleShareLink();
    hideError();
    persistEditorState();
    render();
  } finally {
    setBusy(false);
  }
}

async function applyPersistentAction(action, options = {}) {
  if (!state.response) {
    return;
  }

  setBusy(true);

  try {
    state.response = await requestApplyAction(state.response.draft, action);
    if (options.closeDetails !== false) {
      state.isEditingDetails = false;
      state.editDraft = null;
      closeEmojiPicker();
      closeCustomRoleModal();
    }
    sanitizeClueFormState();
    ensureSelectionInBounds();
    clearVisibleShareLink();
    hideError();
    persistEditorState();
    render();
  } finally {
    setBusy(false);
  }
}

function revertProgressionDraft(draft, action) {
  const previous = deepClone(draft);
  const cell = previous.cells[action.row]?.[action.col];
  if (!cell) {
    throw new Error("cannot undo a change for a missing tile");
  }

  if (action.kind === "add_clue") {
    const isInitialReveal =
      previous.initial_reveal?.row === action.row &&
      previous.initial_reveal?.col === action.col;
    cell.clue = null;
    if (!isInitialReveal) {
      cell.answer = null;
    }
    return previous;
  }

  throw new Error("cannot undo that change");
}

async function rewriteLastClue(action) {
  if (!state.response) {
    return;
  }

  const lastIndex = lastClueProgressionIndex();
  if (lastIndex < 0) {
    return;
  }

  const lastAction = state.progression[lastIndex];
  const baseDraft = revertProgressionDraft(state.response.draft, lastAction);
  setBusy(true);

  try {
    state.response = await requestApplyAction(baseDraft, action);
    state.progression[lastIndex] = summarizeProgressionAction(action);
    sanitizeClueFormState();
    clearVisibleShareLink();
    hideError();
    ensureSelectionInBounds();
    persistEditorState();
    render();
  } finally {
    setBusy(false);
  }
}

async function removeLastClue() {
  if (!state.response) {
    return;
  }

  const lastIndex = lastClueProgressionIndex();
  if (lastIndex < 0) {
    return;
  }

  const previousDraft = revertProgressionDraft(state.response.draft, state.progression[lastIndex]);
  setBusy(true);

  try {
    state.response = await describeDraft(previousDraft);
    state.progression.splice(lastIndex, 1);
  } finally {
    setBusy(false);
  }

  state.isEditingDetails = false;
  state.editDraft = null;
  closeEmojiPicker();
  closeCustomRoleModal();
  sanitizeClueFormState();
  ensureSelectionInBounds();
  clearVisibleShareLink();
  persistEditorState();
  render();
}

async function shareDraft() {
  if (!state.response?.share_ready) {
    return;
  }

  const draftKey = currentDraftKey();
  let storedPuzzleId = null;
  setBusy(true);

  try {
    if (state.lastSharedDraftKey === draftKey && state.lastSharedStoredPuzzleId) {
      storedPuzzleId = state.lastSharedStoredPuzzleId;
    } else {
      const response = await fetchJson("/api/editor/share", {
        method: "POST",
        body: JSON.stringify({
          draft: state.response.draft,
        }),
      });
      storedPuzzleId = response.stored_puzzle_id;
      state.lastSharedDraftKey = draftKey;
      state.lastSharedStoredPuzzleId = storedPuzzleId;
    }
  } finally {
    setBusy(false);
  }

  const url = `${window.location.origin}/p/${encodeURIComponent(storedPuzzleId)}`;
  state.visibleShareUrl = url;
  hideError();
  persistEditorState();
  render();
  await copyText(url);
}

function renderTopControls() {
  populateSizeControls();
  const shareReady = Boolean(state.response?.share_ready);
  const canGenerateRemaining =
    state.response &&
    !shareReady &&
    state.response.next_clue_targets.length > 0;
  const canRemoveClue = lastClueProgressionIndex() >= 0;
  generateRemainingButton.disabled = state.busy || !canGenerateRemaining;
  undoButton.disabled = state.busy || !canRemoveClue;
  shareButton.hidden = !shareReady;
  shareButton.disabled = state.busy || !shareReady;
  shareButton.classList.toggle("button-good", shareReady);
  shareButton.classList.toggle("button-primary", !shareReady);
  shareButton.textContent =
    shareReady && state.lastSharedDraftKey === currentDraftKey()
      ? "Copy Share Link"
      : "Share Puzzle";
  resetButton.disabled = state.busy;
  rowsSelectEl.disabled = state.busy;
  colsSelectEl.disabled = state.busy;
  authorInputEl.disabled = state.busy || !state.response;
  const authorValue = state.response?.draft.author ?? "";
  if (authorInputEl.value !== authorValue) {
    authorInputEl.value = authorValue;
  }
}

function promptText() {
  if (!state.response) {
    return "Loading";
  }

  if (state.response.draft.initial_reveal === null) {
    return "Choose a Starting Tile";
  }

  const view = selectedCellView();
  if (state.response.share_ready) {
    return state.visibleShareUrl ? "Share Link Copied" : "Share Your Puzzle";
  }

  if (view && clueBuilderActiveForView(view)) {
    return `Adding a Clue for ${view.cell.name}`;
  }

  return "Choose a Striped Tile to Add a Clue";
}

function renderPromptBar() {
  promptBarEl.hidden = false;
  promptEl.textContent = promptText();

  if (state.visibleShareUrl) {
    shareLinkEl.hidden = false;
    shareLinkEl.innerHTML = `<a href="${state.visibleShareUrl}" target="_blank" rel="noreferrer">Open Shared Puzzle</a>`;
  } else {
    shareLinkEl.hidden = true;
    shareLinkEl.textContent = "";
  }
}

function createEditDraft(view) {
  return {
    row: view.row,
    col: view.col,
    name: view.cell.name,
    role: view.cell.role,
    emoji: view.cell.emoji ?? "",
  };
}

function ensureEditDraft(view) {
  if (!view || !state.isEditingDetails) {
    return null;
  }

  if (
    !state.editDraft ||
    state.editDraft.row !== view.row ||
    state.editDraft.col !== view.col
  ) {
    state.editDraft = createEditDraft(view);
  }

  return state.editDraft;
}

function validateNameInput(view, rawName) {
  const normalized = rawName.trim();
  if (normalized.length === 0) {
    return { valid: false, normalized };
  }

  if (normalized === view.cell.name) {
    return { valid: true, normalized };
  }

  const duplicate = state.response.draft.cells.some((row, rowIndex) =>
    row.some(
      (cell, colIndex) =>
        (rowIndex !== view.row || colIndex !== view.col) && cell.name === normalized,
    ),
  );

  return { valid: !duplicate, normalized };
}

function validateRoleInput(view, rawRole) {
  const normalized = rawRole.trim();
  if (normalized.length === 0) {
    return { valid: false, normalized };
  }

  if (normalized === view.cell.role) {
    return { valid: true, normalized };
  }

  const referencedRoles = new Set(state.response.referenced_roles);
  const blocked =
    referencedRoles.has(view.cell.role) || referencedRoles.has(normalized);

  return { valid: !blocked, normalized };
}

function validateEmojiInput(rawEmoji) {
  const normalized = rawEmoji.trim();
  return {
    valid: Array.from(normalized).length <= maxEmojiChars,
    normalized,
  };
}

function previewBoardCell(row, col, cell) {
  if (
    !state.isEditingDetails ||
    !state.editDraft ||
    state.editDraft.row !== row ||
    state.editDraft.col !== col
  ) {
    return cell;
  }

  const view = selectedCellView();
  if (!view || view.row !== row || view.col !== col) {
    return cell;
  }

  const nameState = validateNameInput(view, state.editDraft.name);
  const roleState = validateRoleInput(view, state.editDraft.role);
  const emojiState = validateEmojiInput(state.editDraft.emoji);

  return {
    ...cell,
    name: nameState.valid ? (nameState.normalized || cell.name) : cell.name,
    role: roleState.valid ? (roleState.normalized || cell.role) : cell.role,
    emoji: emojiState.valid ? (emojiState.normalized || null) : cell.emoji ?? null,
  };
}

function syncInlineEditStyles(view) {
  if (!state.isEditingDetails || !view || !state.editDraft) {
    return;
  }

  const nameState = validateNameInput(view, state.editDraft.name);
  const roleState = validateRoleInput(view, state.editDraft.role);

  renameInputEl.classList.toggle("is-invalid", !nameState.valid);
  renameInputEl.setAttribute("aria-invalid", String(!nameState.valid));
  roleInputEl.classList.toggle("is-invalid", !roleState.valid);
  roleInputEl.setAttribute("aria-invalid", String(!roleState.valid));
}

function updateInlineEditDraft(field, value) {
  const view = selectedCellView();
  const draft = ensureEditDraft(view);
  if (!view || !draft) {
    return;
  }

  draft[field] = value;
  syncInlineEditStyles(view);
  persistEditorState();
  renderBoard();
  if (state.isEditingDetails) {
    selectedEmojiEl.textContent = emojiForCell(previewBoardCell(view.row, view.col, view.cell));
  }
  if (state.emojiPickerOpen) {
    renderEmojiPicker();
  }
}

function updateRoleDraft(nextRole) {
  updateInlineEditDraft("role", nextRole);

  const view = selectedCellView();
  const draft = ensureEditDraft(view);
  if (!draft) {
    return;
  }

  const rememberedEmoji = rememberedEmojiForRole(nextRole);
  if (rememberedEmoji) {
    draft.emoji = rememberedEmoji;
    syncInlineEditStyles(view);
    persistEditorState();
    renderBoard();
    if (state.isEditingDetails) {
      selectedEmojiEl.textContent = emojiForCell(previewBoardCell(view.row, view.col, view.cell));
    }
    if (state.emojiPickerOpen) {
      renderEmojiPicker();
    }
  }
}

function closeCustomRoleModal() {
  state.customRoleModalOpen = false;
}

function openCustomRoleModal(defaultValue) {
  state.customRoleModalDraft = defaultValue;
  state.customRoleModalOpen = true;
  state.pendingFocus = "custom-role-input";
  renderCustomRoleModal();
}

function renderCustomRoleModal() {
  const view = selectedCellView();
  const open = state.customRoleModalOpen && state.isEditingDetails && Boolean(view);
  customRoleModalEl.hidden = !open;
  if (!open) {
    return;
  }

  const roleState = validateRoleInput(view, state.customRoleModalDraft);
  if (customRoleInputEl.value !== state.customRoleModalDraft) {
    customRoleInputEl.value = state.customRoleModalDraft;
  }
  customRoleInputEl.disabled = state.busy;
  customRoleInputEl.classList.toggle("is-invalid", !roleState.valid);
  customRoleInputEl.setAttribute("aria-invalid", String(!roleState.valid));
  customRoleCancelButton.disabled = state.busy;
  customRoleSaveButton.disabled = state.busy || !roleState.valid;
}

function submitCustomRoleModal() {
  const view = selectedCellView();
  if (!view || state.busy) {
    return;
  }

  const roleState = validateRoleInput(view, state.customRoleModalDraft);
  if (!roleState.valid) {
    renderCustomRoleModal();
    return;
  }

  updateRoleDraft(roleState.normalized);
  closeCustomRoleModal();
  persistEditorState();
  render();
}

function currentEmojiPickerChoices() {
  const seen = new Set();
  const choices = [];
  const current = state.editDraft?.emoji?.trim();

  if (current) {
    seen.add(current);
    choices.push(current);
  }

  for (const emoji of [
    ...Object.values(specialNameEmojis),
    ...Object.values(roleEmojis),
    ...emojiPickerChoices,
  ]) {
    if (!emoji || seen.has(emoji)) {
      continue;
    }

    seen.add(emoji);
    choices.push(emoji);
  }

  return choices;
}

function closeEmojiPicker() {
  state.emojiPickerOpen = false;
}

function openEmojiPicker() {
  const view = selectedCellView();
  if (!view || !state.isEditingDetails || state.busy) {
    return;
  }

  ensureEditDraft(view);
  state.emojiPickerOpen = true;
  state.pendingFocus = "emoji-custom-input";
  renderEmojiPicker();
}

function renderEmojiPicker() {
  const view = selectedCellView();
  const open = state.emojiPickerOpen && state.isEditingDetails && Boolean(view);
  emojiPickerModalEl.hidden = !open;
  if (!open) {
    return;
  }

  const draft = ensureEditDraft(view);
  if (!draft) {
    closeEmojiPicker();
    emojiPickerModalEl.hidden = true;
    return;
  }

  const emojiState = validateEmojiInput(draft.emoji);
  const selectedEmoji = emojiState.normalized;

  emojiPickerGridEl.innerHTML = currentEmojiPickerChoices()
    .map(
      (emoji) => `
        <button
          class="button ${selectedEmoji === emoji ? "button-good" : "button-secondary"} editor-emoji-option"
          type="button"
          data-emoji-option="${escapeHtml(emoji)}"
          aria-label="Use ${escapeHtml(emoji)}"
        >
          ${escapeHtml(emoji)}
        </button>
      `,
    )
    .join("");

  if (emojiCustomInputEl.value !== draft.emoji) {
    emojiCustomInputEl.value = draft.emoji;
  }
  emojiCustomInputEl.disabled = state.busy;
  emojiCustomInputEl.classList.toggle("is-invalid", !emojiState.valid);
  emojiCustomInputEl.setAttribute("aria-invalid", String(!emojiState.valid));
  emojiDefaultButton.disabled = state.busy;
  emojiCloseButton.disabled = state.busy;
}

async function finalizeDetailEditing(options = {}) {
  const view = selectedCellView();
  if (!view || !state.isEditingDetails) {
    return true;
  }

  const draft = ensureEditDraft(view);
  if (!draft) {
    return true;
  }

  const nameState = validateNameInput(view, draft.name);
  if (nameState.valid && nameState.normalized !== view.cell.name) {
    draft.name = nameState.normalized;
    try {
      await applyPersistentAction(
        {
          kind: "rename_cell",
          row: view.row,
          col: view.col,
          name: nameState.normalized,
        },
        { closeDetails: false },
      );
    } catch (error) {
      showError("Rename Failed", error.message);
      return false;
    }
  }

  const refreshedView = selectedCellView();
  const refreshedDraft = refreshedView ? ensureEditDraft(refreshedView) : null;
  if (refreshedView && refreshedDraft) {
    const roleState = validateRoleInput(refreshedView, refreshedDraft.role);
    if (roleState.valid && roleState.normalized !== refreshedView.cell.role) {
      refreshedDraft.role = roleState.normalized;
      try {
        await applyPersistentAction(
          {
            kind: "change_role",
            row: refreshedView.row,
            col: refreshedView.col,
            role: roleState.normalized,
          },
          { closeDetails: false },
        );
      } catch (error) {
        showError("Role Change Failed", error.message);
        return false;
      }
    }
  }

  const emojiView = selectedCellView();
  const emojiDraft = emojiView ? ensureEditDraft(emojiView) : null;
  if (emojiView && emojiDraft) {
    const emojiState = validateEmojiInput(emojiDraft.emoji);
    const currentEmoji = (emojiView.cell.emoji ?? "").trim();
    if (emojiState.valid && emojiState.normalized !== currentEmoji) {
      emojiDraft.emoji = emojiState.normalized;
      try {
        await applyPersistentAction(
          {
            kind: "change_emoji",
            row: emojiView.row,
            col: emojiView.col,
            emoji: emojiState.normalized || null,
          },
          { closeDetails: false },
        );
      } catch (error) {
        showError("Emoji Change Failed", error.message);
        return false;
      }
    }
  }

  if (options.close !== false) {
    state.isEditingDetails = false;
    state.editDraft = null;
    closeEmojiPicker();
    closeCustomRoleModal();
    state.pendingFocus = null;
    persistEditorState();
    render();
  } else if (selectedCellView()) {
    state.editDraft = createEditDraft(selectedCellView());
    syncInlineEditStyles(selectedCellView());
    renderEmojiPicker();
    renderCustomRoleModal();
  }

  return true;
}

function renderClueKindPicker() {
  return availableClueKindOptions()
    .map(
      ([value, label]) => `
        <button
          id="clue-kind-${value}"
          class="button ${state.clueForm.kind === value ? "button-good" : "button-secondary"} editor-kind-button"
          type="button"
          data-clue-kind="${value}"
        >
          ${label}
        </button>
      `,
    )
    .join("");
}

function cluePlaceholder(view) {
  if (view.renderedClue) {
    return view.renderedClue;
  }

  return "";
}

function renderBoard() {
  boardEl.replaceChildren();
  if (!state.response) {
    return;
  }

  const rows = state.response.draft.cells.length;
  const cols = state.response.draft.cells[0]?.length ?? 0;
  boardEl.style.setProperty("--rows", String(rows));
  boardEl.style.setProperty("--cols", String(cols));

  state.response.draft.cells.forEach((row, rowIndex) => {
    row.forEach((cell, colIndex) => {
      const displayCell = previewBoardCell(rowIndex, colIndex, cell);
      const fragment = cellTemplate.content.cloneNode(true);
      const article = fragment.querySelector(".editor-cell");
      const positionEl = fragment.querySelector(".cell-position");
      const emojiEl = fragment.querySelector(".cell-emoji");
      const nameEl = fragment.querySelector(".cell-name");
      const roleEl = fragment.querySelector(".cell-role");
      const clueButton = fragment.querySelector(".cell-clue");
      const renderedClue = state.response.rendered_clues[rowIndex][colIndex];
      const resolvedAnswer = state.response.resolved_answers[rowIndex][colIndex];
      const isSelected =
        state.selected?.row === rowIndex && state.selected?.col === colIndex;
      const canAddClue = state.response.next_clue_targets.some(
        (position) => position.row === rowIndex && position.col === colIndex,
      );
      const canReviseLastClue = isLastAddedClueTile({
        row: rowIndex,
        col: colIndex,
        cell,
      });
      const canEditNonsense = isEditableNonsenseTile({
        row: rowIndex,
        col: colIndex,
        cell,
      });
      const isInitialReveal =
        state.response.draft.initial_reveal?.row === rowIndex &&
        state.response.draft.initial_reveal?.col === colIndex;
      const needsClue = resolvedAnswer !== null && cell.clue === null;

      article.classList.add("clickable");
      article.classList.toggle("is-selected", isSelected);
      article.classList.toggle("is-target", canAddClue);
      article.classList.toggle("has-clue", renderedClue !== null);
      article.classList.toggle("is-initial-reveal", isInitialReveal);
      article.classList.toggle("needs-clue", needsClue);
      if (resolvedAnswer) {
        article.classList.add(answerToneClass(resolvedAnswer));
      }
      if (resolvedAnswer && cell.answer === null) {
        article.classList.add("is-forced-answer");
      }

      positionEl.textContent = `${colLabel(colIndex)}${rowLabel(rowIndex)}`;
      emojiEl.textContent = emojiForCell(displayCell);
      nameEl.textContent = displayCell.name;
      roleEl.textContent = displayCell.role;
      clueButton.textContent = cluePlaceholder({
        renderedClue,
        canAddClue,
        isInitialReveal,
      });
      clueButton.classList.toggle("placeholder", renderedClue === null);
      clueButton.classList.toggle(
        "is-nonsense",
        cell.clue?.kind === "nonsense" && renderedClue !== null,
      );
      clueButton.setAttribute(
        "aria-label",
        renderedClue !== null
          ? canReviseLastClue || canEditNonsense
            ? "Edit clue"
            : "Tile clue"
          : canAddClue
            ? "Add clue"
            : isInitialReveal
              ? "Starting tile"
              : "Tile clue",
      );

      article.addEventListener("click", async () => {
        if (!(await finalizeDetailEditing())) {
          return;
        }
        state.selected = { row: rowIndex, col: colIndex };
        state.isEditingDetails = false;
        state.editDraft = null;
        closeEmojiPicker();
        closeCustomRoleModal();
        persistEditorState();
        render();
      });
      clueButton.addEventListener("click", async (event) => {
        event.stopPropagation();
        if (!(await finalizeDetailEditing())) {
          return;
        }
        state.selected = { row: rowIndex, col: colIndex };
        state.isEditingDetails = false;
        state.editDraft = null;
        closeEmojiPicker();
        closeCustomRoleModal();
        state.pendingFocus =
          (cell.clue === null && canAddClue) || canReviseLastClue || canEditNonsense
            ? `clue-kind-${state.clueForm?.kind ?? "nonsense"}`
            : null;
        persistEditorState();
        render();
      });

      boardEl.append(fragment);
    });
  });
}

function renderInspector() {
  const view = selectedCellView();
  if (!view) {
    emptyStateEl.hidden = true;
    inspectorEl.hidden = true;
    return;
  }

  emptyStateEl.hidden = true;
  inspectorEl.hidden = false;

  const roleLocked = state.response.referenced_roles.includes(view.cell.role);
  const badges = [];
  if (view.cell.answer) {
    badges.push(
      statusBadgeMarkup(
        view.isInitialReveal
          ? `Start ${answerLabel(view.cell.answer)}`
          : answerLabel(view.cell.answer),
        badgeToneClass(view.cell.answer),
      ),
    );
  } else if (view.resolvedAnswer) {
    badges.push(
      statusBadgeMarkup(
        `Forced ${answerLabel(view.resolvedAnswer)}`,
        badgeToneClass(view.resolvedAnswer),
      ),
    );
  }
  if (roleLocked) {
    badges.push(statusBadgeMarkup("Role Locked", "is-neutral"));
  }

  selectedPositionEl.textContent = `${colLabel(view.col)}${rowLabel(view.row)}`;
  selectedHeadingEl.textContent = `${emojiForCell(view.cell)} ${view.cell.name}`;
  selectedBadgesEl.innerHTML = badges.join("");
  selectedBadgesEl.hidden = badges.length === 0;
  editToggleEl.disabled = state.busy;
  editToggleEl.textContent = state.isEditingDetails ? "✓" : "✎";
  editToggleEl.setAttribute(
    "aria-label",
    state.isEditingDetails ? "Hide tile editor" : "Edit tile",
  );
  selectedHeadingEl.hidden = state.isEditingDetails;
  inlineDetailsEl.hidden = !state.isEditingDetails;
  if (state.isEditingDetails) {
    const draft = ensureEditDraft(view);
    const previewCell = {
      name: draft.name.trim() || view.cell.name,
      role: draft.role.trim() || view.cell.role,
      emoji: draft.emoji.trim() || null,
    };
    selectedEmojiEl.textContent = emojiForCell(previewCell);
    if (renameInputEl.value !== draft.name) {
      renameInputEl.value = draft.name;
    }
    roleInputEl.innerHTML = roleEditOptionsMarkup(draft.role);
    roleInputEl.value = draft.role;
    selectedEmojiEl.disabled = state.busy;
    renameInputEl.disabled = state.busy;
    roleInputEl.disabled = state.busy;
    syncInlineEditStyles(view);
  }

  const canSetInitialReveal =
    state.response.draft.initial_reveal === null && view.cell.answer === null;
  initialAnswerPanelEl.hidden = !canSetInitialReveal;
  setInitialInnocentButton.disabled = state.busy;
  setInitialCriminalButton.disabled = state.busy;

  const showClueBuilder = clueBuilderActiveForView(view);
  clueSectionEl.hidden = !showClueBuilder;
  clueSectionEl.style.display = showClueBuilder ? "" : "none";
  suggestClueButton.disabled = state.busy;
  saveClueButton.disabled = state.busy;
  renderSaveClueButton();

  if (showClueBuilder) {
    syncClueFormFromView(view);
    sanitizeClueFormState();
    clueKindPickerEl.innerHTML = renderClueKindPicker();
    clueFormFieldsEl.innerHTML = renderClueFieldsMarkup();
    renderCluePreview();
    requestClueSaveValidation(view);
  } else {
    state.clueSaveValidation = {
      key: null,
      status: "invalid",
      requested: false,
    };
    renderSaveClueButton();
    clueKindPickerEl.innerHTML = "";
    clueFormFieldsEl.innerHTML = "";
    if (cluePreviewEl) {
      cluePreviewEl.textContent = "";
    }
  }
}

function applyPendingFocus() {
  if (!state.pendingFocus) {
    return;
  }

  const target = document.querySelector(`#${state.pendingFocus}`);
  state.pendingFocus = null;
  if (target) {
    target.focus();
    if ("select" in target) {
      target.select?.();
    }
  }
}

function render() {
  renderTopControls();
  renderPromptBar();
  renderBoard();
  renderInspector();
  renderEmojiPicker();
  renderCustomRoleModal();
  window.requestAnimationFrame(applyPendingFocus);
}

function handleClueControlEvent(target) {
  if (target.dataset.nonsensePreset === "true") {
    if (target.value === "") {
      return;
    }
    state.clueForm.nonsense.text = target.value;
    persistEditorState();
    renderInspector();
    return;
  }

  const path = target.dataset.path;
  if (!path) {
    return;
  }

  setNestedValue(state.clueForm, path, readControlValue(target));
  sanitizeClueFormState();
  persistEditorState();
  if (target.dataset.rerender === "true") {
    renderInspector();
  } else {
    renderCluePreview();
  }
}

function isTypingTarget(target) {
  if (!(target instanceof Element)) {
    return false;
  }

  return (
    target.closest("input, textarea, select, [contenteditable='true']") !== null
  );
}

async function handleInitialAnswer(answer) {
  const view = selectedCellView();
  if (!view || state.busy) {
    return;
  }

  if (!(await finalizeDetailEditing())) {
    return;
  }

  try {
    await applyPersistentAction({
      kind: "set_initial_reveal",
      row: view.row,
      col: view.col,
      answer,
    });
  } catch (error) {
    showError("Starting Tile Failed", error.message);
  }
}

async function handleSaveClue() {
  const view = selectedCellView();
  if (!view || state.busy) {
    return;
  }

  if (!(await finalizeDetailEditing())) {
    return;
  }

  try {
    const clue = buildClueFromState();
    if (isEditableNonsenseTile(view)) {
      await applyPersistentAction({
        kind: "update_nonsense_clue",
        row: view.row,
        col: view.col,
        text: clue.text,
      });
      return;
    }
    const action = {
      kind: "add_clue",
      row: view.row,
      col: view.col,
      clue,
    };
    if (isLastAddedClueTile(view) && lastClueProgressionIndex() >= 0) {
      await rewriteLastClue(action);
    } else {
      await applyUndoableAction(action);
    }
  } catch (error) {
    showError("Clue Rejected", error.message);
  }
}

async function handleSuggestClue() {
  if (state.busy) {
    return;
  }

  if (!(await finalizeDetailEditing())) {
    return;
  }

  const view = selectedCellView();
  if (!view) {
    return;
  }

  if (isEditableNonsenseTile(view)) {
    state.clueForm.kind = "nonsense";
    state.clueForm.nonsense.text = randomNonsenseText(
      view.cell.answer ?? view.resolvedAnswer ?? "innocent",
    );
    persistEditorState();
    renderInspector();
    return;
  }

  let draft = state.response.draft;
  if (isLastAddedClueTile(view) && lastClueProgressionIndex() >= 0) {
    draft = revertProgressionDraft(state.response.draft, lastClueProgressionAction());
  }

  const response = await requestSuggestedClue(draft, view.row, view.col);
  applySuggestedClue(response.clue);
}

function shufflePositions(positions) {
  for (let index = positions.length - 1; index > 0; index -= 1) {
    const swapIndex = Math.floor(Math.random() * (index + 1));
    [positions[index], positions[swapIndex]] = [positions[swapIndex], positions[index]];
  }
}

async function suggestAndApplyRandomClueOnce() {
  if (!state.response) {
    return false;
  }

  const targets = [...state.response.next_clue_targets];
  if (targets.length === 0) {
    return false;
  }

  shufflePositions(targets);
  let lastError = null;

  for (const target of targets) {
    try {
      const response = await requestSuggestedClue(
        state.response.draft,
        target.row,
        target.col,
      );
      state.selected = { row: target.row, col: target.col };
      await applyUndoableAction({
        kind: "add_clue",
        row: target.row,
        col: target.col,
        clue: response.clue,
      });
      return true;
    } catch (error) {
      lastError = error;
    }
  }

  throw lastError ?? new Error("No valid clue suggestion was found.");
}

async function handleSuggestRandomClue(options = {}) {
  const { complete = false } = options;

  if (!state.response || state.busy) {
    return;
  }

  if (!(await finalizeDetailEditing())) {
    return;
  }

  try {
    do {
      const applied = await suggestAndApplyRandomClueOnce();
      if (!applied) {
        if (complete && state.response && !state.response.share_ready) {
          throw new Error("The draft cannot be completed from its current state.");
        }
        return;
      }
    } while (complete && state.response && !state.response.share_ready);
  } catch (error) {
    showError(complete ? "Generation Failed" : "Suggestion Failed", error.message);
  }
}

function bindEvents() {
  rowsSelectEl.addEventListener("change", () => {
    state.pendingBoardSize.rows = clampInt(rowsSelectEl.value, 1, maxBoardDimension);
    populateSizeControls();
    persistEditorState();
    renderTopControls();
  });

  colsSelectEl.addEventListener("change", () => {
    state.pendingBoardSize.cols = clampInt(
      colsSelectEl.value,
      1,
      Math.max(1, Math.floor(maxPublicCellCount / state.pendingBoardSize.rows)),
    );
    populateSizeControls();
    persistEditorState();
    renderTopControls();
  });

  authorInputEl.addEventListener("input", () => {
    if (!state.response) {
      return;
    }

    state.response.draft.author = authorInputEl.value === "" ? null : authorInputEl.value;
    clearVisibleShareLink();
    persistEditorState();
    renderTopControls();
    renderPromptBar();
  });

  resetButton.addEventListener("click", async () => {
    if (!(await finalizeDetailEditing())) {
      return;
    }
    try {
      await loadDraft(state.pendingBoardSize.rows, state.pendingBoardSize.cols);
    } catch (error) {
      showError("Editor Reset Failed", error.message);
    }
  });

  generateRemainingButton.addEventListener("click", async (event) => {
    await handleSuggestRandomClue({ complete: event.shiftKey });
  });

  undoButton.addEventListener("click", async () => {
    if (!(await finalizeDetailEditing())) {
      return;
    }
    try {
      await removeLastClue();
    } catch (error) {
      showError("Remove Failed", error.message);
    }
  });

  shareButton.addEventListener("click", async () => {
    if (!(await finalizeDetailEditing())) {
      return;
    }
    try {
      await shareDraft();
    } catch (error) {
      showError("Share Failed", error.message);
    }
  });

  editToggleEl.addEventListener("click", async () => {
    if (state.busy) {
      return;
    }

    if (state.isEditingDetails) {
      if (!(await finalizeDetailEditing())) {
        return;
      }
    } else {
      const view = selectedCellView();
      if (!view) {
        return;
      }
      state.isEditingDetails = true;
      state.editDraft = createEditDraft(view);
      closeEmojiPicker();
      state.pendingFocus = "rename-input";
      persistEditorState();
      renderInspector();
    }
  });
  renameInputEl.addEventListener("input", () => {
    updateInlineEditDraft("name", renameInputEl.value);
  });
  roleInputEl.addEventListener("change", () => {
    const view = selectedCellView();
    if (!view || !state.isEditingDetails) {
      return;
    }

    if (roleInputEl.value === customRoleOptionValue) {
      openCustomRoleModal(state.editDraft?.role ?? view.cell.role);
      render();
      return;
    }

    updateRoleDraft(roleInputEl.value);
    persistEditorState();
    renderInspector();
  });
  selectedEmojiEl.addEventListener("click", () => {
    openEmojiPicker();
  });
  emojiPickerGridEl.addEventListener("click", (event) => {
    const button = event.target.closest("[data-emoji-option]");
    if (!button || state.busy) {
      return;
    }

    updateInlineEditDraft("emoji", button.dataset.emojiOption ?? "");
    closeEmojiPicker();
    persistEditorState();
    render();
  });
  emojiCustomInputEl.addEventListener("input", () => {
    updateInlineEditDraft("emoji", emojiCustomInputEl.value);
    renderEmojiPicker();
  });
  emojiDefaultButton.addEventListener("click", () => {
    updateInlineEditDraft("emoji", "");
    closeEmojiPicker();
    persistEditorState();
    render();
  });
  emojiCloseButton.addEventListener("click", () => {
    closeEmojiPicker();
    render();
  });
  emojiPickerModalEl
    .querySelector(".modal-backdrop")
    .addEventListener("click", () => {
      closeEmojiPicker();
      render();
    });
  customRoleInputEl.addEventListener("input", () => {
    state.customRoleModalDraft = customRoleInputEl.value;
    renderCustomRoleModal();
  });
  customRoleInputEl.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      submitCustomRoleModal();
      return;
    }
    if (event.key !== "Escape") {
      return;
    }

    event.preventDefault();
    closeCustomRoleModal();
    render();
  });
  customRoleCancelButton.addEventListener("click", () => {
    closeCustomRoleModal();
    render();
  });
  customRoleSaveButton.addEventListener("click", () => {
    submitCustomRoleModal();
  });
  customRoleModalEl
    .querySelector(".modal-backdrop")
    .addEventListener("click", () => {
      closeCustomRoleModal();
      render();
    });
  renameInputEl.addEventListener("keydown", async (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      await finalizeDetailEditing();
      return;
    }
    if (event.key !== "Escape") {
      return;
    }

    event.preventDefault();
    state.isEditingDetails = false;
    state.editDraft = null;
    closeEmojiPicker();
    closeCustomRoleModal();
    state.pendingFocus = null;
    persistEditorState();
    renderInspector();
  });
  roleInputEl.addEventListener("keydown", async (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      await finalizeDetailEditing();
      return;
    }
    if (event.key !== "Escape") {
      return;
    }

    event.preventDefault();
    state.isEditingDetails = false;
    state.editDraft = null;
    closeEmojiPicker();
    closeCustomRoleModal();
    state.pendingFocus = null;
    persistEditorState();
    renderInspector();
  });
  setInitialInnocentButton.addEventListener("click", () => handleInitialAnswer("innocent"));
  setInitialCriminalButton.addEventListener("click", () => handleInitialAnswer("criminal"));
  suggestClueButton.addEventListener("click", async () => {
    try {
      await handleSuggestClue();
    } catch (error) {
      showError("Suggest Failed", error.message);
    }
  });
  saveClueButton.addEventListener("click", handleSaveClue);

  clueKindPickerEl.addEventListener("click", (event) => {
    const button = event.target.closest("[data-clue-kind]");
    if (!button || state.busy) {
      return;
    }

    state.clueForm.kind = button.dataset.clueKind;
    sanitizeClueFormState();
    persistEditorState();
    renderInspector();
  });
  clueFormFieldsEl.addEventListener("change", (event) => {
    handleClueControlEvent(event.target);
  });
  clueFormFieldsEl.addEventListener("input", (event) => {
    handleClueControlEvent(event.target);
  });

  errorDismissButton.addEventListener("click", hideError);
  errorModalEl.querySelector(".modal-backdrop").addEventListener("click", hideError);

  document.addEventListener("keydown", (event) => {
    if (event.defaultPrevented || state.busy || errorModalEl.hidden === false) {
      return;
    }
    if (state.customRoleModalOpen) {
      if (event.key === "Escape") {
        event.preventDefault();
        closeCustomRoleModal();
        render();
      }
      return;
    }
    if (state.emojiPickerOpen) {
      if (event.key === "Escape") {
        event.preventDefault();
        closeEmojiPicker();
        render();
      }
      return;
    }
    if (event.metaKey || event.ctrlKey || event.altKey) {
      return;
    }
    if (isTypingTarget(event.target)) {
      return;
    }
    if (event.key.toLowerCase() !== "z") {
      return;
    }

    event.preventDefault();
    void removeLastClue().catch((error) => {
      showError("Remove Failed", error.message);
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

async function init() {
  bindEvents();
  renderTopControls();

  try {
    await loadBootstrap();
    const saved = readSavedEditorState();
    const restored = restoreSavedEditorState(saved);

    if (restored) {
      try {
        state.response = await describeDraft(state.response.draft);
        sanitizeClueFormState();
        ensureSelectionInBounds();
        if (
          state.lastSharedDraftKey === currentDraftKey() &&
          state.lastSharedStoredPuzzleId !== null
        ) {
          state.visibleShareUrl = `${window.location.origin}/p/${encodeURIComponent(
            state.lastSharedStoredPuzzleId,
          )}`;
        } else {
          clearVisibleShareLink();
        }
        persistEditorState();
        hideError();
        render();
        return;
      } catch {
        state.response = null;
        state.progression = [];
        state.selected = null;
        state.clueForm = createDefaultClueForm();
        clearVisibleShareLink();
      }
    } else {
      state.clueForm = createDefaultClueForm();
    }

    await loadDraft(state.pendingBoardSize.rows, state.pendingBoardSize.cols);
  } catch (error) {
    showError("Editor Failed To Load", error.message);
  }
}

void init();
