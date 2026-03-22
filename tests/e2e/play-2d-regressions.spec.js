const { test, expect } = require("@playwright/test");
const {
  buildAndShareTiny2DPuzzle,
  chooseLongPressColor,
  escapeRegex,
  fetchOpened2DStoredPuzzle,
  start2D,
  startStored2DPuzzle,
  tapLocator,
} = require("./helpers");

function cardByName(page, name) {
  return page
    .locator("#board .cell")
    .filter({
      has: page.locator(".cell-name", {
        hasText: new RegExp(`^${escapeRegex(name)}$`),
      }),
    })
    .first();
}

async function setDocumentHidden(page, hidden) {
  await page.evaluate((nextHidden) => {
    Object.defineProperty(document, "hidden", {
      configurable: true,
      get: () => nextHidden,
    });
    document.dispatchEvent(new Event("visibilitychange"));
  }, hidden);
}

test("seeded 2D share reuses the same stored id on repeated clicks", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"], {
    origin: "http://127.0.0.1:3000",
  });
  await start2D(page);

  await page.click("#share-puzzle");
  await expect
    .poll(() => page.evaluate(() => state.sharedStoredPuzzleId))
    .toMatch(/^[0-9a-f]{6}$/);
  const firstId = await page.evaluate(() => state.sharedStoredPuzzleId);

  await page.click("#share-puzzle");
  const secondId = await page.evaluate(() => state.sharedStoredPuzzleId);

  expect(secondId).toBe(firstId);
});

test("2D open in editor works from both seeded and stored puzzle pages", async ({
  page,
}) => {
  await start2D(page);
  await page.click("#open-editor");
  await page.waitForURL("**/edit");
  await expect(page.locator("#editor-board .editor-cell")).toHaveCount(20);

  const storedPuzzleId = await buildAndShareTiny2DPuzzle(page, "Stored Source");
  await page.goto(`/p/${storedPuzzleId}`);
  const startButton = page.locator("#start-button");
  if (await startButton.isVisible()) {
    await startButton.click();
  }
  await page.click("#open-editor");
  await page.waitForURL("**/edit");
  await expect(page.locator("#author-input")).toHaveValue("Stored Source");
  await expect
    .poll(() =>
      page.evaluate(() => {
        const saved = JSON.parse(window.localStorage.getItem("clues-editor:v1"));
        return saved?.progression?.length ?? 0;
      }),
    )
    .toBeGreaterThan(0);
});

test("2D progress restores notes, hidden clues, and timer state across reload", async ({
  page,
}) => {
  await start2D(page);
  await page.waitForTimeout(250);

  const topNote = page.locator("#board .cell.hidden-clue .cell-note").first();
  const bottomNote = page.locator("#board .cell.hidden-clue .cell-note-secondary").first();
  const revealedClue = page.locator("#board .cell.has-clue .cell-clue").first();

  await tapLocator(page, topNote, 1);
  await chooseLongPressColor(page, bottomNote, "cyan");
  await revealedClue.click();
  await expect(revealedClue).toHaveClass(/clue-hidden/);

  const beforeReload = await page.evaluate(() => currentTimerElapsedMs());
  expect(beforeReload).toBeGreaterThan(0);

  await page.reload();

  await expect(page.locator("#board .cell.hidden-clue .cell-note").first()).toHaveClass(
    /note-yellow/,
  );
  await expect(
    page.locator("#board .cell.hidden-clue .cell-note-secondary").first(),
  ).toHaveClass(/note-cyan/);
  await expect(page.locator("#board .cell.has-clue .cell-clue").first()).toHaveClass(
    /clue-hidden/,
  );
  await expect
    .poll(() => page.evaluate(() => currentTimerElapsedMs()))
    .toBeGreaterThan(0);
});

test("2D timer pauses while the page is hidden and resumes on return", async ({ page }) => {
  await start2D(page);
  await page.waitForTimeout(220);
  const beforePause = await page.evaluate(() => currentTimerElapsedMs());

  await setDocumentHidden(page, true);
  await page.waitForTimeout(500);
  const whileHidden = await page.evaluate(() => currentTimerElapsedMs());

  await setDocumentHidden(page, false);
  await page.waitForTimeout(220);
  const afterResume = await page.evaluate(() => currentTimerElapsedMs());

  expect(whileHidden - beforePause).toBeLessThan(120);
  expect(afterResume - whileHidden).toBeGreaterThan(120);
});

test("2D first load shows Start, reload with progress shows Resume, and timer waits for Resume", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.locator("#start-button")).toHaveText("Start");
  await page.waitForTimeout(250);
  expect(await page.evaluate(() => currentTimerElapsedMs())).toBe(0);

  await page.click("#start-button");
  await page.waitForTimeout(220);
  const startedElapsed = await page.evaluate(() => currentTimerElapsedMs());
  expect(startedElapsed).toBeGreaterThan(0);

  await page.reload();
  await expect(page.locator("#start-button")).toHaveText("Resume");
  const beforeResume = await page.evaluate(() => currentTimerElapsedMs());
  await page.waitForTimeout(250);
  const stillPaused = await page.evaluate(() => currentTimerElapsedMs());
  expect(stillPaused - beforeResume).toBeLessThan(80);

  await page.click("#start-button");
  await page.waitForTimeout(220);
  const afterResume = await page.evaluate(() => currentTimerElapsedMs());
  expect(afterResume - stillPaused).toBeGreaterThan(120);
});

test("2D D shortcut closes popups before opening the hidden page", async ({ page }) => {
  await start2D(page);
  await page.locator("#board .cell.hidden-clue").first().click();
  await expect(page.locator("#guess-modal")).toBeVisible();

  await page.keyboard.press("d");

  await expect(page.locator("#guess-modal")).toBeHidden();
  await expect
    .poll(() => page.evaluate(() => state.scoreDebugVisible))
    .toBe(true);
});

test("wrong guesses show the evidence error, correct guesses clear notes, and completed share reopens results", async ({
  page,
  request,
}) => {
  const storedPuzzleId = await buildAndShareTiny2DPuzzle(page);
  const opened = await fetchOpened2DStoredPuzzle(request, storedPuzzleId);
  const initialReveal = opened.draft.initial_reveal;
  const targetStep = opened.progression.find(
    (step) =>
      step.kind === "add_clue" &&
      !(initialReveal && step.row === initialReveal.row && step.col === initialReveal.col),
  );
  const targetCell = opened.draft.cells[targetStep.row][targetStep.col];

  await startStored2DPuzzle(page, storedPuzzleId);
  const targetCard = cardByName(page, targetCell.name);
  await expect(targetCard).toBeVisible();

  const targetTopNote = targetCard.locator(".cell-note");
  const targetBottomNote = targetCard.locator(".cell-note-secondary");
  await tapLocator(page, targetTopNote, 1);
  await chooseLongPressColor(page, targetBottomNote, "cyan");

  await targetCard.click();
  await expect(page.locator("#guess-modal")).toBeVisible();
  await page.click(
    targetCell.answer === "innocent" ? "#guess-criminal" : "#guess-innocent",
  );
  await expect(page.locator("#error-title")).toHaveText("⚠️ Not enough evidence!");
  await page.click("#error-dismiss");
  await expect
    .poll(() => page.evaluate(() => state.mistakeTiles.size))
    .toBe(1);

  await targetCard.click();
  await page.click(
    targetCell.answer === "innocent" ? "#guess-innocent" : "#guess-criminal",
  );
  await expect(targetTopNote).toHaveClass(/note-none/);
  await expect(targetBottomNote).toHaveClass(/note-none/);
  await expect(targetCard.locator(".cell-clue")).not.toHaveText("");

  await page.evaluate(() => {
    state.guesses = state.cells.map((row) =>
      row.map((cell) => cell.answer ?? cell.revealed_answer ?? "innocent"),
    );
    state.timerElapsedMs = 0;
    state.timerStartedAt = Date.now() - 1600;
    state.timerCompletedAt = null;
    state.completionAcknowledged = false;
    completePuzzleIfNeeded();
    renderBoard();
  });
  await expect(page.locator("#finish-modal")).toBeVisible();
  await expect(page.locator("#share-puzzle")).toHaveText("🏆 Share Results");
  await page.click("#finish-dismiss");
  await expect(page.locator("#finish-modal")).toBeHidden();
  await page.click("#share-puzzle");
  await expect(page.locator("#finish-modal")).toBeVisible();
});

test("revealed 2D clue taps hide and unhide clue text immediately", async ({
  page,
  request,
}) => {
  const storedPuzzleId = await buildAndShareTiny2DPuzzle(page);
  await startStored2DPuzzle(page, storedPuzzleId);

  const revealedCard = page.locator("#board .cell.has-clue").first();
  const revealedClue = revealedCard.locator(".cell-clue");

  await revealedCard.click();
  await expect(revealedClue).toHaveClass(/clue-hidden/);

  await revealedCard.click();
  await expect(revealedClue).not.toHaveClass(/clue-hidden/);
});
