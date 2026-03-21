const { test, expect } = require("@playwright/test");
const { openFreshEditor2x2 } = require("./helpers");

function editorCell(page, row, col, cols = 2) {
  return page.locator("#editor-board .editor-cell").nth(row * cols + col);
}

test("editor rows and cols stay pending until reset, and only one starting tile can be assigned", async ({
  page,
}) => {
  await page.goto("/edit");
  await expect(page.locator("#editor-board .editor-cell")).toHaveCount(20);

  await page.selectOption("#rows-select", "2");
  await page.selectOption("#cols-select", "2");
  await expect(page.locator("#editor-board .editor-cell")).toHaveCount(20);
  await expect(page.locator("#share-editor")).toBeHidden();

  await page.click("#reset-editor");
  await expect(page.locator("#editor-board .editor-cell")).toHaveCount(4);

  await editorCell(page, 0, 0).click();
  await page.click("#set-initial-innocent");
  await editorCell(page, 0, 1).click();
  await expect(page.locator("#initial-answer-panel")).toBeHidden();
  await expect(page.locator("#share-editor")).toBeHidden();
});

test("editor only opens clue builder for valid striped targets, and suggest supports one-step and complete flows", async ({
  page,
}) => {
  await openFreshEditor2x2(page);
  await editorCell(page, 0, 0).click();
  await page.click("#set-initial-innocent");

  await expect(page.locator("#editor-board .editor-cell.needs-clue")).toHaveCount(1);
  const invalidTarget = await page.evaluate(() => {
    const selected = state.selected;
    const nextTargets = new Set(
      state.response.next_clue_targets.map((target) => `${target.row}:${target.col}`),
    );

    for (let row = 0; row < state.response.draft.cells.length; row += 1) {
      for (let col = 0; col < state.response.draft.cells[row].length; col += 1) {
        if (selected?.row === row && selected?.col === col) {
          continue;
        }
        if (nextTargets.has(`${row}:${col}`)) {
          continue;
        }
        return { row, col };
      }
    }

    return null;
  });
  expect(invalidTarget).not.toBeNull();
  await editorCell(page, invalidTarget.row, invalidTarget.col).click();
  await expect(page.locator("#clue-section")).toBeHidden();

  const validTarget = await page.evaluate(() => state.response.next_clue_targets[0]);
  expect(validTarget).not.toBeNull();
  await editorCell(page, validTarget.row, validTarget.col).click();
  await expect
    .poll(() =>
      page.evaluate(
        (target) => state.selected?.row === target.row && state.selected?.col === target.col,
        validTarget,
      ),
    )
    .toBe(true);
  await expect(page.locator("#clue-section")).toBeVisible();

  const progressionBeforeSuggest = await page.evaluate(() => state.progression.length);
  await page.click("#generate-remaining");
  await expect
    .poll(() =>
      page.evaluate((baseline) => state.progression.length - baseline, progressionBeforeSuggest),
    )
    .toBe(1);
  await expect(page.locator("#share-editor")).toBeHidden();

  await page.locator("#generate-remaining").click({ modifiers: ["Shift"] });
  await expect.poll(() => page.evaluate(() => state.response.share_ready)).toBe(true);
  await expect(page.locator("#share-editor")).toBeVisible();
});

test("editor duplicate renames revert, role changes persist, and removing a clue does not undo detail edits", async ({
  page,
}) => {
  await openFreshEditor2x2(page);
  await editorCell(page, 0, 0).click();
  await page.click("#set-initial-innocent");
  await page.click("#generate-remaining");

  const duplicateName = await page.evaluate(() => state.response.draft.cells[0][0].name);
  const editTarget = editorCell(page, 0, 1);
  await editTarget.click();
  await page.click("#edit-toggle");
  const originalRole = await page.evaluate(() => state.response.draft.cells[0][1].role);
  const replacementRole = await page.evaluate(
    () => state.bootstrap.roles.find((role) => role !== state.response.draft.cells[0][1].role),
  );

  await page.fill("#rename-input", duplicateName);
  await page.click("#edit-toggle");
  await expect
    .poll(() => page.evaluate(() => state.response.draft.cells[0][1].name))
    .not.toBe(duplicateName);

  await editTarget.click();
  await page.click("#edit-toggle");
  await page.fill("#rename-input", "Zelda");
  await page.selectOption("#role-input", replacementRole);
  await page.click("#edit-toggle");
  await expect
    .poll(() => page.evaluate(() => state.response.draft.cells[0][1].name))
    .toBe("Zelda");
  await expect
    .poll(() => page.evaluate(() => state.response.draft.cells[0][1].role))
    .toBe(replacementRole);

  const progressionBeforeRemove = await page.evaluate(() => state.progression.length);
  await page.click("#undo-edit");
  await expect
    .poll(() => page.evaluate(() => state.progression.length))
    .toBe(progressionBeforeRemove - 1);
  await expect
    .poll(() => page.evaluate(() => state.response.draft.cells[0][1].name))
    .toBe("Zelda");
  await expect
    .poll(() => page.evaluate(() => state.response.draft.cells[0][1].role))
    .toBe(replacementRole);
  expect(originalRole).not.toBe(replacementRole);
});

test("editor custom roles propagate to later dropdowns and reuse remembered emoji", async ({
  page,
}) => {
  await openFreshEditor2x2(page);

  await editorCell(page, 0, 0).click();
  await page.click("#edit-toggle");
  await page.selectOption("#role-input", "__custom__");
  await expect(page.locator("#custom-role-modal")).toBeVisible();
  await page.fill("#custom-role-input", "Astrologer");
  await page.click("#custom-role-save");
  await expect(page.locator("#custom-role-modal")).toBeHidden();
  await page.click("#selected-emoji");
  await page.fill("#emoji-custom-input", "🪐");
  await page.click("#emoji-close");
  await page.click("#edit-toggle");
  await expect
    .poll(() => page.evaluate(() => state.response.draft.cells[0][0].role))
    .toBe("Astrologer");

  await editorCell(page, 0, 1).click();
  await page.click("#edit-toggle");
  await expect(page.locator("#role-input")).toHaveValue(/.*/);
  await page.selectOption("#role-input", "Astrologer");
  await expect(page.locator("#selected-emoji")).toHaveText("🪐");
  await page.click("#edit-toggle");
  await expect
    .poll(() => page.evaluate(() => state.response.draft.cells[0][1].emoji))
    .toBe("🪐");
});

test("editor completed share links are reused until the draft changes", async ({ page }) => {
  await openFreshEditor2x2(page, "Link Reuse");
  await editorCell(page, 0, 0).click();
  await page.click("#set-initial-innocent");
  await page.locator("#generate-remaining").click({ modifiers: ["Shift"] });
  await expect(page.locator("#share-editor")).toBeVisible();

  await page.click("#share-editor");
  const firstHref = await page.locator("#editor-share-link a").getAttribute("href");
  await page.click("#share-editor");
  const secondHref = await page.locator("#editor-share-link a").getAttribute("href");
  expect(secondHref).toBe(firstHref);

  await page.fill("#author-input", "Link Reuse Updated");
  await page.click("#share-editor");
  const thirdHref = await page.locator("#editor-share-link a").getAttribute("href");
  expect(thirdHref).not.toBe(firstHref);
});

test("editor blocks role changes when the current role is already referenced by clues", async ({
  page,
}) => {
  await openFreshEditor2x2(page);
  await editorCell(page, 0, 0).click();
  await page.click("#set-initial-innocent");
  const blocked = await page.evaluate(async () => {
    const role = state.response.draft.cells[0][0].role;
    const replacement = state.bootstrap.roles.find((candidate) => candidate !== role);
    const target = { row: 0, col: 1 };
    const draft = JSON.parse(JSON.stringify(state.response.draft));
    const targetCell = draft.cells[target.row][target.col];
    targetCell.answer =
      targetCell.answer ?? state.response.resolved_answers[target.row][target.col] ?? "innocent";
    targetCell.clue = {
      kind: "role_count",
      role,
      answer: "innocent",
      count: {
        kind: "number",
        value: 1,
      },
    };
    state.response = await describeDraft(draft);
    state.selected = { row: 0, col: 0 };
    render();
    return { row: 0, col: 0, role, replacement };
  });

  await editorCell(page, blocked.row, blocked.col).click();
  await page.click("#edit-toggle");
  await page.selectOption("#role-input", blocked.replacement);
  await expect(page.locator("#role-input")).toHaveClass(/is-invalid/);
  await page.click("#edit-toggle");
  await expect
    .poll(() =>
      page.evaluate(
        ({ row, col }) => state.response.draft.cells[row][col].role,
        { row: blocked.row, col: blocked.col },
      ),
    )
    .toBe(blocked.role);
});

test("editor limits the final remaining clue to nonsense after removing the last clue", async ({
  page,
}) => {
  await openFreshEditor2x2(page);
  await editorCell(page, 0, 0).click();
  await page.click("#set-initial-innocent");
  const availableKinds = await page.evaluate(() => {
    const clueLessTargets = [];
    for (let row = 0; row < state.response.draft.cells.length; row += 1) {
      for (let col = 0; col < state.response.draft.cells[row].length; col += 1) {
        if (state.response.draft.cells[row][col].clue === null) {
          clueLessTargets.push({ row, col });
        }
      }
    }

    state.response = {
      ...state.response,
      next_clue_targets: clueLessTargets,
    };
    state.selected = clueLessTargets[0] ?? null;
    render();
    const view = selectedCellView();
    return availableClueKindOptions(view).map(([, label]) => label);
  });

  expect(availableKinds).toEqual(["Nonsense"]);
});
